use std::sync::Arc;

use actix_web::{get, post, web, App, HttpServer};
use config::Config;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use web::Json;

mod config;

#[derive(Serialize, Deserialize, Default)]
struct SimpleDB {
    epoch: u64,
    content: String,
}

#[derive(Clone, Default)]
struct Context {
    db: Arc<Mutex<SimpleDB>>,
}

#[derive(Serialize, Deserialize)]
pub struct SetRequest {
    #[serde(flatten)]
    db: SimpleDB,
}

#[derive(Serialize, Deserialize)]
struct SetResponse {
    success: bool,
    message: &'static str,
}

#[derive(Serialize, Deserialize)]
struct GetResponse {
    #[serde(flatten)]
    db: SimpleDB,
}

#[post("/set")]
async fn set_clipboard(
    ctx: web::Data<Context>,
    request: web::Json<SetRequest>,
) -> web::Json<SetResponse> {
    let mut db = ctx.db.lock().await;

    if db.epoch != request.db.epoch {
        warn!("Outdated request!");
        return Json(SetResponse {
            success: false,
            message: "Outdated",
        });
    }

    db.epoch += 1;
    db.content = request.db.content.clone();

    info!("next epoch {}: {:?}", db.epoch, db.content);

    Json(SetResponse {
        success: true,
        message: "Success",
    })
}

#[get("/get")]
async fn get_clipboard(ctx: web::Data<Context>) -> web::Json<GetResponse> {
    let db = ctx.db.lock().await;

    Json(GetResponse {
        db: SimpleDB {
            epoch: db.epoch,
            content: db.content.clone(),
        },
    })
}

#[actix_web::main]
async fn main() {
    let config = Config::from_args();
    flexi_logger::Logger::with_env_or_str(config.default_log_level)
        .start()
        .expect("logger");

    info!("Listen on {} port", config.port);

    let context: Context = Default::default();

    HttpServer::new(move || {
        App::new()
            .data(context.clone())
            .service(get_clipboard)
            .service(set_clipboard)
    })
    .bind(format!("{}:{}", config.host, config.port))
    .unwrap()
    .run()
    .await
    .unwrap();
}
