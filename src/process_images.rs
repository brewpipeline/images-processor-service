use crate::*;

use base64::engine::general_purpose;
use base64::Engine;
use std::error::Error;
use std::fs;
use std::sync::mpsc;

pub type ProcessResult = Result<(), Box<dyn Error + Send + Sync>>;

pub fn process_images(rx: mpsc::Receiver<(ImageType, String, flume::Sender<ProcessResult>)>) {
    loop {
        let (image_type, base64_url, tx) = rx.recv().unwrap();
        let result = download_and_process_image(&image_type, &base64_url);
        if let Some(err) = result.as_ref().err() {
            println!(
                "Image (name: `{name}`, type: `{type}`) process error: {reason}",
                name = base64_url,
                type = image_type,
                reason = err.to_string()
            )
        }
        tx.send(result).unwrap();
    }
}

fn download_and_process_image(image_type: &ImageType, base64_url: &String) -> ProcessResult {
    let url_vec = general_purpose::URL_SAFE.decode(base64_url)?;
    let url = String::from_utf8(url_vec)?;
    let res = reqwest::blocking::get(url)?;
    let bytes = res.bytes()?;
    let image = image::load_from_memory(&bytes)?;
    let image = image_type.process_image(image);
    let path = image_type.local_path(base64_url);
    let temp_path = format!("{path}.tmp");
    image.save_with_format(&temp_path, image_type.file_format())?;
    fs::rename(&temp_path, path)?;
    Ok(())
}
