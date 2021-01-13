use std::sync::Arc;

use clipboard::ClipboardContext;
use config::Config;
use log::info;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::{sync::Mutex, time, time::Duration};

mod clipboard;
mod config;

struct GlobalContext<'a> {
    get_url: &'a str,
    set_url: &'a str,
    http_client: &'a Client,
    clipboard: &'a ClipboardContext,
    db: Mutex<SimpleDB>,
}

// TODO: move out to the library
#[derive(Serialize, Deserialize, Default, Debug)]
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
    message: String,
}

#[derive(Serialize, Deserialize)]
struct GetResponse {
    #[serde(flatten)]
    db: SimpleDB,
}

async fn get_clipboard(context: &GlobalContext<'_>) -> Option<GetResponse> {
    context
        .http_client
        .get(context.get_url)
        .send()
        .await
        .ok()?
        .json::<GetResponse>()
        .await
        .ok()
}

async fn set_clipboard(context: &GlobalContext<'_>, c: SetRequest) -> Option<SetResponse> {
    info!("try to push epoch {}: {}", c.db.epoch, c.db.content);
    context
        .http_client
        .post(context.set_url)
        .json(&c)
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()
}

async fn check_clipboard<'a>(context: &GlobalContext<'_>) -> Option<()> {
    let mut db = context.db.lock().await;
    let server_clipboard = get_clipboard(context).await?;

    if db.epoch < server_clipboard.db.epoch {
        context
            .clipboard
            .set(server_clipboard.db.content.clone())
            .await;
        db.content = server_clipboard.db.content;
        db.epoch = server_clipboard.db.epoch;
        info!(
            "Got update from server: epoch {} {:?}",
            db.epoch, db.content,
        );

        // skip next clip update
        context.clipboard.get().await;
        return Some(());
    }

    let update = context.clipboard.get().await?;

    if update == db.content {
        return Some(());
    }

    info!("Got update from x11: now {:?}", update);
    set_clipboard(
        context,
        SetRequest {
            db: SimpleDB {
                epoch: db.epoch,
                content: update.clone(),
            },
        },
    )
    .await;

    db.epoch += 1;
    db.content = update;

    Some(())
}

#[tokio::main]
async fn main() {
    let config = Config::from_args();

    flexi_logger::Logger::with_env_or_str(config.default_log_level)
        .start()
        .expect("logger");

    let get_url = format!("http://{}:{}/get", config.host, config.port);
    let set_url = format!("http://{}:{}/set", config.host, config.port);
    let client = reqwest::Client::new();

    let clipboard_ctx = ClipboardContext::new();

    let global_context = GlobalContext {
        get_url: &get_url,
        set_url: &set_url,
        http_client: &client,
        clipboard: &clipboard_ctx,
        db: Default::default(),
    };

    loop {
        check_clipboard(&global_context).await;
        time::sleep(Duration::from_millis(200)).await;
    }
}
