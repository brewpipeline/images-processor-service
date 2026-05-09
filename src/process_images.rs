use crate::*;

use base64::engine::general_purpose;
use base64::Engine;
use std::error::Error;
use std::fs;
use std::io::{Cursor, Write};
use std::sync::{mpsc, LazyLock};

pub type ProcessResult = Result<(), Box<dyn Error + Send + Sync>>;

const DECODER_MAX_ALLOC: u64 = 256 * 1024 * 1024;

fn decoder_limits() -> image::Limits {
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(DECODER_MAX_ALLOC);
    limits
}

static HTTP_CLIENT: LazyLock<reqwest::blocking::Client> = LazyLock::new(|| {
    reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0 (compatible; BlogImageBot/1.0)")
        .build()
        .expect("failed to build HTTP client")
});

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
        let mut reader = image::ImageReader::open(local_path)?;
        reader.limits(decoder_limits());
        reader.decode()?
    } else {
        let res = HTTP_CLIENT.get(url).send()?;
        let bytes = res.bytes()?;
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
        },
        file_format => image.save_with_format(&temp_path, file_format)?,
    }
    fs::rename(&temp_path, path)?;
    Ok(())
}
