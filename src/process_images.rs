use crate::*;

use base64::engine::general_purpose;
use base64::Engine;
use std::sync::mpsc;

pub fn process_images(rx: mpsc::Receiver<(ImageType, String, flume::Sender<()>)>) {
    while let Ok((image_type, base64_url, tx)) = rx.recv() {
        let _ = download_and_process_image(image_type, base64_url);
        let _ = tx.send(());
    }
}

fn download_and_process_image(image_type: ImageType, base64_url: String) -> Result<(), ()> {
    let url_vec = general_purpose::URL_SAFE
        .decode(&base64_url)
        .map_err(|_| ())?;
    let url = String::from_utf8(url_vec).map_err(|_| ())?;
    let res = reqwest::blocking::get(url).map_err(|_| ())?;
    let bytes = res.bytes().map_err(|_| ())?;

    let image = image::load_from_memory(&bytes).map_err(|_| ())?;

    image_type
        .process_and_save_with_name(&base64_url, image)
        .map_err(|_| ())?;

    Ok(())
}
