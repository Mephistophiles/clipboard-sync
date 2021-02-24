use clipboard_sync_lib::{
    clipboard::{
        clipboard_server::{Clipboard, ClipboardServer},
        GetRequest, GetResponse, SetRequest, SetResponse,
    },
    config::{Config, Type},
};
use log::{info, warn};
use std::net::SocketAddr;
use tokio::sync::Mutex;
use tonic::{
    transport::{Identity, Server, ServerTlsConfig},
    Request, Response, Status,
};

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
    let config = Config::load(Type::Server);
    flexi_logger::Logger::with_env_or_str(&config.server.log_level)
        .start()
        .expect("logger");

    info!("Listening on {}:{}", config.server.host, config.server.port);

    let addr = SocketAddr::new(config.server.host.parse().unwrap(), config.server.port);
    let clipboard = MyClipboard::default();

    let mut server = if config.server.cert.is_some() {
        let cert = tokio::fs::read(config.server.cert.unwrap()).await.unwrap();
        let key = tokio::fs::read(config.server.key.unwrap()).await.unwrap();
        let server_identity = Identity::from_pem(cert, key);

        let tls = ServerTlsConfig::new().identity(server_identity);

        Server::builder().tls_config(tls).unwrap()
    } else {
        Server::builder()
    };

    server
        .add_service(ClipboardServer::new(clipboard))
        .serve(addr)
        .await
        .unwrap();
}
