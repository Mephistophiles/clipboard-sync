use super::Result;
use actix_web::{get, post, web, App, HttpServer};
use crossbeam_channel::Receiver;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use x11_clipboard::Clipboard;

use super::LAST_CLIPBOARD_CONTENT;

#[derive(Serialize, Deserialize)]
struct ClipboardUpdate {
    contents: String,
}

#[derive(Serialize, Deserialize)]
pub struct ClipboardResponse {
    pub contents: Option<String>,
    pub updated: bool,
}

#[derive(Serialize, Deserialize)]
struct Response {
    success: bool,
    error: Option<String>,
}

fn get_contents(ClipboardUpdate { contents }: ClipboardUpdate) -> String {
    contents
}

#[post("/push_clipboard")]
async fn push_clipboard(update: web::Json<ClipboardUpdate>) -> web::Json<Response> {
    let contents = get_contents(update.into_inner());
    {
        let mut last_selection = LAST_CLIPBOARD_CONTENT.lock().unwrap();
        debug!("Get content: {}", contents);

        if *last_selection == contents {
            return web::Json(Response {
                success: false,
                error: Some("Duplicated value".to_string()),
            });
        }
        *last_selection = contents.clone();
    }

    let clipboard = match Clipboard::new() {
        Ok(clipboard) => clipboard,
        Err(e) => {
            return web::Json(Response {
                success: false,
                error: Some(e.to_string()),
            })
        }
    };

    debug!("update clipboard");
    clipboard
        .store(
            clipboard.getter.atoms.clipboard,
            clipboard.getter.atoms.utf8_string,
            contents,
        )
        .unwrap();

    web::Json(Response {
        success: true,
        error: None,
    })
}

#[get("/get_clipboard")]
async fn get_clipboard(receiver: web::Data<Receiver<String>>) -> web::Json<ClipboardResponse> {
    debug!("Try to get clipboard");
    let contents = receiver.recv_timeout(Duration::from_secs(30)).ok();
    let updated = contents.is_some();

    debug!("Update: {:?}", contents);

    web::Json(ClipboardResponse { contents, updated })
}

pub async fn server(host: &str, port: u16, receiver: Receiver<String>) -> Result<()> {
    info!("Listen on {} port", port);

    HttpServer::new(move || {
        App::new()
            .data(receiver.clone())
            .service(push_clipboard)
            .service(get_clipboard)
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await?;

    Ok(())
}
