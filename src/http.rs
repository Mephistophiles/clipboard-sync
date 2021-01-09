use crate::error::{ClipboardError, Error, Result};
use actix_web::{get, post, web, App, HttpServer};
use crossbeam_channel::Receiver;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::clipboard::ClipboardContext;

#[derive(Clone)]
struct Context {
    receiver: Receiver<String>,
    clipboard: ClipboardContext,
}

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
async fn push_clipboard(
    ctx: web::Data<Context>,
    update: web::Json<ClipboardUpdate>,
) -> web::Json<Response> {
    let contents = get_contents(update.into_inner());

    match ctx.clipboard.set(contents) {
        Ok(()) => web::Json(Response {
            success: true,
            error: None,
        }),
        Err(Error::ClipboardError(ClipboardError::DuplicatedValue)) => web::Json(Response {
            success: false,
            error: Some("Duplicated value".to_string()),
        }),
        Err(e) => web::Json(Response {
            success: false,
            error: Some(format!("Internal error: {:?}", e)),
        }),
    }
}

#[get("/get_clipboard")]
async fn get_clipboard(ctx: web::Data<Context>) -> web::Json<ClipboardResponse> {
    debug!("Try to get clipboard");
    let contents = ctx.receiver.recv_timeout(Duration::from_secs(30)).ok();
    let updated = contents.is_some();

    debug!("Update: {:?}", contents);

    web::Json(ClipboardResponse { contents, updated })
}

pub async fn server(
    host: &str,
    port: u16,
    clipboard: ClipboardContext,
    receiver: Receiver<String>,
) -> Result<()> {
    info!("Listen on {} port", port);

    let ctx = Context {
        receiver,
        clipboard,
    };

    HttpServer::new(move || {
        App::new()
            .data(ctx.clone())
            .service(push_clipboard)
            .service(get_clipboard)
    })
    .bind(format!("{}:{}", host, port))
    .map_err(Error::from)?
    .run()
    .await
    .map_err(Error::from)?;

    Ok(())
}
