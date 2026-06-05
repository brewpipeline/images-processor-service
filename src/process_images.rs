use crate::*;

use base64::engine::general_purpose;
use base64::Engine;
use std::error::Error;
use std::fs;
use std::io::{Cursor, Read, Write};
use std::net::{IpAddr, ToSocketAddrs};
use std::ptr;
use std::sync::{mpsc, LazyLock};
use std::time::Duration;

pub type ProcessResult = Result<(), Box<dyn Error + Send + Sync>>;

const DECODER_MAX_ALLOC: u64 = 256 * 1024 * 1024;
const MAX_DOWNLOAD_BYTES: u64 = 16 * 1024 * 1024;
const MAX_REDIRECTS: usize = 5;

fn decoder_limits() -> image::Limits {
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(DECODER_MAX_ALLOC);
    limits
}

static HTTP_CLIENT: LazyLock<reqwest::blocking::Client> = LazyLock::new(|| {
    reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0 (compatible; BlogImageBot/1.0)")
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= MAX_REDIRECTS {
                attempt.error("too many redirects")
            } else if url_is_public(attempt.url()) {
                attempt.follow()
            } else {
                attempt.error("redirect to a non-public or non-https address")
            }
        }))
        .timeout(Duration::from_secs(15))
        .connect_timeout(Duration::from_secs(5))
        .build()
        .expect("failed to build HTTP client")
});

fn is_public_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            !(v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_documentation()
                || v4.is_unspecified()
                || o[0] == 0
                || (o[0] == 100 && (o[1] & 0xc0) == 64)
                || (o[0] == 192 && o[1] == 0 && o[2] == 0)
                || o[0] >= 240)
        }
        IpAddr::V6(v6) => {
            if v6.is_loopback() || v6.is_unspecified() {
                return false;
            }
            if let Some(mapped) = v6.to_ipv4_mapped() {
                return is_public_ip(&IpAddr::V4(mapped));
            }
            let seg = v6.segments();
            if (seg[0] & 0xffc0) == 0xfe80 {
                return false;
            }
            if (seg[0] & 0xfe00) == 0xfc00 {
                return false;
            }
            true
        }
    }
}

fn url_is_public(url: &reqwest::Url) -> bool {
    if url.scheme() != "https" {
        return false;
    }
    let Some(host) = url.host_str() else {
        return false;
    };
    let port = url.port_or_known_default().unwrap_or(443);
    let Ok(addrs) = (host, port).to_socket_addrs() else {
        return false;
    };
    let mut resolved_any = false;
    for addr in addrs {
        resolved_any = true;
        if !is_public_ip(&addr.ip()) {
            return false;
        }
    }
    resolved_any
}

fn fetch_remote_image_bytes(url: &str) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    let parsed = url::Url::parse(url)?;
    if !url_is_public(&parsed) {
        return Err(Box::from("image source is not a public https address"));
    }

    let res = HTTP_CLIENT.get(parsed).send()?;
    if !res.status().is_success() {
        return Err(Box::from("image source returned a non-success status"));
    }
    if let Some(content_type) = res.headers().get(reqwest::header::CONTENT_TYPE) {
        let is_image = content_type
            .to_str()
            .map(|v| v.trim_start().starts_with("image/"))
            .unwrap_or(false);
        if !is_image {
            return Err(Box::from("image source returned a non-image content type"));
        }
    }
    if let Some(len) = res.content_length() {
        if len > MAX_DOWNLOAD_BYTES {
            return Err(Box::from("image source is too large"));
        }
    }

    let mut bytes = Vec::new();
    res.take(MAX_DOWNLOAD_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_DOWNLOAD_BYTES {
        return Err(Box::from("image source is too large"));
    }
    Ok(bytes)
}

pub fn process_images(rx: mpsc::Receiver<(ImageType, String, flume::Sender<ProcessResult>)>) {
    while let Ok((image_type, base64_url, tx)) = rx.recv() {
        let result = download_and_process_image(&image_type, &base64_url);
        if let Some(err) = result.as_ref().err() {
            println!(
                "Image (name: `{name}`, type: `{type}`) process error: {reason}",
                name = base64_url,
                type = image_type,
                reason = err.to_string()
            )
        }
        let _ = tx.send(result);
        purge_jemalloc_arenas();
        if LOG_MEMORY_STATS {
            log_jemalloc_stats();
            log_kernel_memory();
        }
    }
}

fn log_kernel_memory() {
    fn parse_kb(line: &str) -> u64 {
        line.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0)
    }
    fn parse_bytes(line: &str) -> u64 {
        line.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0)
    }

    if let Ok(status) = fs::read_to_string("/proc/self/status") {
        let mut rss_kb = 0u64;
        let mut anon_kb = 0u64;
        let mut file_kb = 0u64;
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                rss_kb = parse_kb(line);
            } else if line.starts_with("RssAnon:") {
                anon_kb = parse_kb(line);
            } else if line.starts_with("RssFile:") {
                file_kb = parse_kb(line);
            }
        }
        println!(
            "proc: VmRSS={} MiB, RssAnon={} MiB, RssFile={} MiB",
            rss_kb / 1024,
            anon_kb / 1024,
            file_kb / 1024,
        );
    }

    if let Ok(stat) = fs::read_to_string("/sys/fs/cgroup/memory.stat") {
        let mut anon_b = 0u64;
        let mut file_b = 0u64;
        for line in stat.lines() {
            if line.starts_with("anon ") {
                anon_b = parse_bytes(line);
            } else if line.starts_with("file ") {
                file_b = parse_bytes(line);
            }
        }
        println!(
            "cgroup: anon={} MiB, file={} MiB",
            anon_b / (1024 * 1024),
            file_b / (1024 * 1024),
        );
    }
}

fn log_jemalloc_stats() {
    use tikv_jemalloc_ctl::{epoch, stats};
    if epoch::advance().is_err() {
        return;
    }
    let allocated = stats::allocated::read().unwrap_or(0);
    let resident = stats::resident::read().unwrap_or(0);
    println!(
        "jemalloc: allocated={} MiB, resident={} MiB",
        allocated / (1024 * 1024),
        resident / (1024 * 1024),
    );
}

fn purge_jemalloc_arenas() {
    // MALLCTL_ARENAS_ALL == 4096; targets every arena in a single call.
    let name = c"arena.4096.purge";
    unsafe {
        tikv_jemalloc_sys::mallctl(
            name.as_ptr(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            0,
        );
    }
}

fn download_and_process_image(image_type: &ImageType, base64_url: &String) -> ProcessResult {
    let external_to_local_paths_map: HashMap<&str, &str> = EXTERNAL_TO_LOCAL_PATHS_MAP
        .split(',')
        .filter_map(|pair| {
            let parts: Vec<&str> = pair.split('|').collect();
            if parts.len() == 2 {
                Some((parts[0], parts[1]))
            } else {
                None
            }
        })
        .collect();

    let url_vec = general_purpose::URL_SAFE.decode(base64_url)?;
    let url = String::from_utf8(url_vec)?;
    let image = if let Some((external_path_component, local_path_component)) =
        external_to_local_paths_map
            .iter()
            .find(|&(k, &_)| url.contains(k))
    {
        let local_path = url.replace(external_path_component, local_path_component);
        if local_path.contains("..") {
            return Err(Box::from("local image path traversal rejected"));
        }
        let mut reader = image::ImageReader::open(local_path)?;
        reader.limits(decoder_limits());
        reader.decode()?
    } else {
        let bytes = fetch_remote_image_bytes(&url)?;
        let mut reader = image::ImageReader::new(Cursor::new(&bytes)).with_guessed_format()?;
        reader.limits(decoder_limits());
        reader.decode()?
    };
    let image = image_type.process_image(image);
    let path = image_type.local_path(base64_url);
    let temp_path = format!("{path}.tmp");
    match image_type.file_format() {
        image::ImageFormat::WebP => {
            let webp_data = webp::Encoder::from_image(&image)?
                .encode_simple(false, 100f32)
                .map_err(|e| format!("simple encode error: {:?}", e))?;
            let mut output_file = fs::File::create(&temp_path)?;
            output_file.write_all(&webp_data)?;
            output_file.sync_all()?;
            drop_from_page_cache(&output_file);
        },
        file_format => image.save_with_format(&temp_path, file_format)?,
    }
    fs::rename(&temp_path, path)?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn drop_from_page_cache(file: &fs::File) {
    use std::os::unix::io::AsRawFd;
    unsafe {
        libc::posix_fadvise(file.as_raw_fd(), 0, 0, libc::POSIX_FADV_DONTNEED);
    }
}

#[cfg(not(target_os = "linux"))]
fn drop_from_page_cache(_file: &fs::File) {}
