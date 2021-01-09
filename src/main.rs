use client::client;
use clipboard::ClipboardContext;
use config::{Config, Mode};
use crossbeam_channel::Receiver;
use log::info;

mod client;
mod clipboard;
mod config;
mod error;
mod server;

async fn client_mode(host: &str, port: u16, ctx: ClipboardContext, receiver: Receiver<String>) {
    info!("run in client mode");
    client(host, port, ctx, receiver).await;
}

async fn server_mode(host: &str, port: u16, ctx: ClipboardContext, receiver: Receiver<String>) {
    info!("run in server mode");
    server::server(&host, port, ctx, receiver).await.unwrap()
}

#[actix_web::main]
async fn main() {
    let (sender, receiver) = crossbeam_channel::unbounded();

    let config = Config::from_args();

    flexi_logger::Logger::with_env_or_str(config.default_log_level)
        .start()
        .expect("logger");

    let clipboard_ctx = ClipboardContext::new();

    let ctx = clipboard_ctx.clone();
    actix_rt::spawn(async move { clipboard::clipboard_loop(ctx, sender).await.unwrap() });

    match config.mode {
        Mode::Server => server_mode(&config.host, config.port, clipboard_ctx, receiver).await,
        Mode::Client => client_mode(&config.host, config.port, clipboard_ctx, receiver).await,
    }
}
