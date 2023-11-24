mod handle_image;
mod image_type;
mod process_images;

pub use crate::handle_image::*;
pub use crate::image_type::*;
pub use crate::process_images::*;

pub const SERVER_ADDRESS: &'static str = env!("SERVER_ADDRESS"); // 127.0.0.1:4000
pub const EXTERN_LOCATION_IMAGES_STORAGE_PATH: &'static str =
    env!("EXTERN_LOCATION_IMAGES_STORAGE_PATH"); // https://site.com/images/
pub const LOCAL_IMAGES_STORAGE_PATH: &'static str = env!("LOCAL_IMAGES_STORAGE_PATH"); // ./images/
pub const THUMBNAIL_SMALL_WIDTH: u32 = env!("THUMBNAIL_SMALL_WIDTH")
    .parse()
    .expect("`THUMBNAIL_SMALL_WIDTH` should be `u32`"); // 250
pub const THUMBNAIL_MEDIUM_WIDTH: u32 = env!("THUMBNAIL_MEDIUM_WIDTH")
    .parse()
    .expect("`THUMBNAIL_MEDIUM_WIDTH` should be `u32`"); // 750
pub const THUMBNAIL_HEIGHT_MULTIPLIER: u32 = env!("THUMBNAIL_HEIGHT_MULTIPLIER")
    .parse()
    .expect("`THUMBNAIL_HEIGHT_MULTIPLIER` should be `u32`"); // 3

// ------------------------------

use actix_web::{get, web, App, HttpResponse, HttpServer};
use std::collections::HashMap;
use std::default::Default;
use std::sync::{mpsc, Mutex};

#[get("/small/{base64_url}")]
pub async fn handle_small_thumbnail_image(
    base64_url: web::Path<String>,
    tx: web::Data<mpsc::Sender<(ImageType, String, flume::Sender<()>)>>,
    in_progress_storage: web::Data<Mutex<HashMap<String, flume::Receiver<()>>>>,
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
    tx: web::Data<mpsc::Sender<(ImageType, String, flume::Sender<()>)>>,
    in_progress_storage: web::Data<Mutex<HashMap<String, flume::Receiver<()>>>>,
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
    tx: web::Data<mpsc::Sender<(ImageType, String, flume::Sender<()>)>>,
    in_progress_storage: web::Data<Mutex<HashMap<String, flume::Receiver<()>>>>,
) -> HttpResponse {
    handle_image(ImageType::Normal, base64_url, tx, in_progress_storage).await
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let (tx, rx) = mpsc::channel();
    let in_progress_storage = web::Data::new(Mutex::<HashMap<String, flume::Receiver<()>>>::new(
        Default::default(),
    ));

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
