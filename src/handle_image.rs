use crate::*;

use actix_web::{web, HttpResponse};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{mpsc, Mutex};

pub async fn handle_image(
    image_type: ImageType,
    base64_url: web::Path<String>,
    tx: web::Data<mpsc::Sender<(ImageType, String, flume::Sender<ProcessResult>)>>,
    in_progress_storage: web::Data<Mutex<HashMap<String, flume::Receiver<ProcessResult>>>>,
) -> HttpResponse {
    let base64_url = base64_url.to_string();

    let extern_path = image_type.extern_path(&base64_url);

    let result = if !Path::new(&image_type.local_path(&base64_url)).exists() {
        tokio::task::spawn_blocking(move || {
            let res_rx = {
                let maybe_rx = in_progress_storage.lock().unwrap_or_else(|e| e.into_inner()).get(&base64_url).cloned();
                if let Some(res_rx) = maybe_rx {
                    res_rx
                } else {
                    let (res_tx, res_rx) = flume::bounded(1);
                    in_progress_storage.lock().unwrap_or_else(|e| e.into_inner()).insert(base64_url.clone(), res_rx.clone());
                    if tx.send((image_type, base64_url.clone(), res_tx)).is_err() {
                        in_progress_storage.lock().unwrap_or_else(|e| e.into_inner()).remove(&base64_url);
                        return Err(Box::from("worker thread is dead"));
                    }
                    res_rx
                }
            };
            let result = match res_rx.recv() {
                Ok(r) => r,
                Err(_) => {
                    if Path::new(&image_type.local_path(&base64_url)).exists() {
                        Ok(())
                    } else {
                        Err(Box::from("processing taken by another request and failed"))
                    }
                }
            };
            in_progress_storage.lock().unwrap_or_else(|e| e.into_inner()).remove(&base64_url);
            result
        })
        .await
        .unwrap_or_else(|_| Err(Box::from("worker thread panicked")))
    } else {
        Ok(())
    };

    match result {
        Ok(_) => HttpResponse::MovedPermanently()
            .append_header(("Location", extern_path))
            .finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}
