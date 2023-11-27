mod handle_image;
mod image_type;
mod process_images;

pub use crate::handle_image::*;
pub use crate::image_type::*;
pub use crate::process_images::*;

pub const fn parse_u32(s: &str) -> u32 {
    let mut out: u32 = 0;
    let mut i: usize = 0;
    while i < s.len() {
        out *= 10;
        out += (s.as_bytes()[i] - b'0') as u32;
        i += 1;
    }
    out
}

// 127.0.0.1:4000
pub const SERVER_ADDRESS: &'static str = env!("SERVER_ADDRESS");

// https://site.com/images/
pub const EXTERN_LOCATION_IMAGES_STORAGE_PATH: &'static str =
    env!("EXTERN_LOCATION_IMAGES_STORAGE_PATH");

// ./images/
pub const LOCAL_IMAGES_STORAGE_PATH: &'static str = env!("LOCAL_IMAGES_STORAGE_PATH");

// 250
pub const THUMBNAIL_SMALL_WIDTH: u32 = parse_u32(env!("THUMBNAIL_SMALL_WIDTH"));

// 750
pub const THUMBNAIL_MEDIUM_WIDTH: u32 = parse_u32(env!("THUMBNAIL_MEDIUM_WIDTH"));

// 3
pub const THUMBNAIL_HEIGHT_MULTIPLIER: u32 = parse_u32(env!("THUMBNAIL_HEIGHT_MULTIPLIER"));

// https://site.com/|/var/www/site.com/,https://site.ru/|/var/www/site.ru/
pub const EXTERNAL_TO_LOCAL_PATHS_MAP: &'static str = env!("EXTERNAL_TO_LOCAL_PATHS_MAP");

// ------------------------------

use actix_web::{get, web, App, HttpResponse, HttpServer};
use std::collections::HashMap;
use std::default::Default;
use std::sync::{mpsc, Mutex};

#[get("/small/{base64_url}")]
pub async fn handle_small_thumbnail_image(
    base64_url: web::Path<String>,
    tx: web::Data<mpsc::Sender<(ImageType, String, flume::Sender<ProcessResult>)>>,
    in_progress_storage: web::Data<Mutex<HashMap<String, flume::Receiver<ProcessResult>>>>,
) -> HttpResponse {
    handle_image(
        ImageType::Thumbnail {
            nwidth: THUMBNAIL_SMALL_WIDTH,
            nheight: THUMBNAIL_SMALL_WIDTH * THUMBNAIL_HEIGHT_MULTIPLIER,
        },
        base64_url,
        tx,
        in_progress_storage,
    )
    .await
}

#[get("/medium/{base64_url}")]
pub async fn handle_medium_thumbnail_image(
    base64_url: web::Path<String>,
    tx: web::Data<mpsc::Sender<(ImageType, String, flume::Sender<ProcessResult>)>>,
    in_progress_storage: web::Data<Mutex<HashMap<String, flume::Receiver<ProcessResult>>>>,
) -> HttpResponse {
    handle_image(
        ImageType::Thumbnail {
            nwidth: THUMBNAIL_MEDIUM_WIDTH,
            nheight: THUMBNAIL_MEDIUM_WIDTH * THUMBNAIL_HEIGHT_MULTIPLIER,
        },
        base64_url,
        tx,
        in_progress_storage,
    )
    .await
}

#[get("/{base64_url}")]
pub async fn handle_normal_image(
    base64_url: web::Path<String>,
    tx: web::Data<mpsc::Sender<(ImageType, String, flume::Sender<ProcessResult>)>>,
    in_progress_storage: web::Data<Mutex<HashMap<String, flume::Receiver<ProcessResult>>>>,
) -> HttpResponse {
    handle_image(ImageType::Normal, base64_url, tx, in_progress_storage).await
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let (tx, rx) = mpsc::channel();
    let in_progress_storage = web::Data::new(Mutex::<
        HashMap<String, flume::Receiver<ProcessResult>>,
    >::new(Default::default()));

    std::thread::spawn(move || {
        process_images(rx);
    });

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(tx.clone()))
            .app_data(in_progress_storage.clone())
            .service(handle_small_thumbnail_image)
            .service(handle_medium_thumbnail_image)
            .service(handle_normal_image)
    })
    .bind(SERVER_ADDRESS)?
    .run()
    .await
}
