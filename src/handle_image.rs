use crate::*;

use actix_web::{web, HttpResponse};
use std::collections::HashMap;
use std::sync::{mpsc, Mutex};

pub async fn handle_image(
    image_type: ImageType,
    base64_url: web::Path<String>,
    tx: web::Data<mpsc::Sender<(ImageType, String, flume::Sender<()>)>>,
    in_progress_storage: web::Data<Mutex<HashMap<String, flume::Receiver<()>>>>,
) -> HttpResponse {
    let base64_url = base64_url.to_string();

    let extern_location = image_type.extern_location(&base64_url);

    let result = if image_type.is_local_image_exists(&base64_url) {
        Ok(())
    } else {
        tokio::task::spawn_blocking(move || {
            let res_rx = {
                let mut in_progress_storage = in_progress_storage.lock().unwrap();
                if let Some(res_rx) = in_progress_storage.get(&base64_url) {
                    let res_rx = res_rx.clone();
                    drop(in_progress_storage);
                    res_rx
                } else {
                    let (res_tx, res_rx) = flume::bounded(1);
                    in_progress_storage.insert(base64_url.clone(), res_rx.clone());
                    drop(in_progress_storage);
                    let _ = tx.send((image_type, base64_url.clone(), res_tx));
                    res_rx
                }
            };
            let _ = res_rx.recv();
            {
                let mut in_progress_storage = in_progress_storage.lock().unwrap();
                in_progress_storage.remove(&base64_url.to_string());
                drop(in_progress_storage);
            }
        })
        .await
    };

    match result {
        Ok(_) => HttpResponse::MovedPermanently()
            .append_header(("Location", extern_location))
            .finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}
