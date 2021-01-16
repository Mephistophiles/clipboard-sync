use config::Config;
use log::{info, warn};
use tokio::sync::Mutex;

use tonic::{transport::Server, Request, Response, Status};

use clipboard_sync_lib::clipboard::{
    clipboard_server::{Clipboard, ClipboardServer},
    GetRequest, GetResponse, SetRequest, SetResponse,
};

mod config;

#[derive(Default)]
struct SimpleDB {
    epoch: u64,
    content: String,
}

#[derive(Default)]
struct MyClipboard {
    db: Mutex<SimpleDB>,
}

#[tonic::async_trait]
impl Clipboard for MyClipboard {
    async fn get(&self, _request: Request<GetRequest>) -> Result<Response<GetResponse>, Status> {
        let db = self.db.lock().await;
        let reply = GetResponse {
            epoch: db.epoch,
            content: db.content.clone(),
        };

        Ok(Response::new(reply))
    }

    async fn set(&self, request: Request<SetRequest>) -> Result<Response<SetResponse>, Status> {
        let mut db = self.db.lock().await;
        let request = request.into_inner();

        if db.epoch != request.epoch {
            warn!("Outdated request!");
            return Ok(Response::new(SetResponse {
                success: false,
                message: "Outdated".to_string(),
            }));
        }

        db.epoch += 1;
        db.content = request.content;
        info!("next epoch {}: {:?}", db.epoch, db.content);

        return Ok(Response::new(SetResponse {
            success: true,
            message: "".to_string(),
        }));
    }
}

#[tokio::main]
async fn main() {
    let config = Config::from_args();
    flexi_logger::Logger::with_env_or_str(config.default_log_level)
        .start()
        .expect("logger");

    info!("Listen on {} port", config.port);

    let addr = format!("{}:{}", config.host, config.port).parse().unwrap();
    let clipboard = MyClipboard::default();

    Server::builder()
        .add_service(ClipboardServer::new(clipboard))
        .serve(addr)
        .await
        .unwrap();
}
