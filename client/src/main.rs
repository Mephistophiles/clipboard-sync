use clipboard::ClipboardContext;
use clipboard_sync_lib::{
    clipboard::{
        clipboard_client::{self, ClipboardClient},
        GetRequest, GetResponse, SetRequest, SetResponse,
    },
    config::{Config, Type},
};
use log::info;
use tokio::{time, time::Duration};
use tonic::transport::{Certificate, Channel, ClientTlsConfig};

mod clipboard;

#[derive(Default)]
struct SimpleDB {
    epoch: u64,
    content: String,
}

struct GlobalContext<'a> {
    proto_client: ClipboardClient<Channel>,
    clipboard: &'a ClipboardContext,
    db: SimpleDB,
}

async fn get_clipboard(context: &mut GlobalContext<'_>) -> Option<GetResponse> {
    let response = context.proto_client.get(GetRequest {}).await.ok()?;
    Some(response.into_inner())
}

async fn set_clipboard(context: &mut GlobalContext<'_>, req: SetRequest) -> Option<SetResponse> {
    info!("try to push epoch {}: {}", req.epoch, req.content);
    Some(context.proto_client.set(req).await.ok()?.into_inner())
}

async fn check_clipboard<'a>(context: &mut GlobalContext<'_>) -> Option<()> {
    let server_clipboard = get_clipboard(context).await?;

    if context.db.epoch < server_clipboard.epoch {
        context
            .clipboard
            .set(server_clipboard.content.clone())
            .await;
        context.db.content = server_clipboard.content;
        context.db.epoch = server_clipboard.epoch;
        info!(
            "Got update from server: epoch {} {:?}",
            context.db.epoch, context.db.content,
        );

        // skip next clip update
        context.clipboard.get().await;
        return Some(());
    }

    let update = context.clipboard.get().await?;

    if update == context.db.content {
        return Some(());
    }

    info!("Got update from x11: now {:?}", update);
    set_clipboard(
        context,
        SetRequest {
            epoch: context.db.epoch,
            content: update.clone(),
        },
    )
    .await;

    context.db.epoch += 1;
    context.db.content = update;

    Some(())
}

#[tokio::main]
async fn main() {
    let config = Config::load(Type::Client);

    flexi_logger::Logger::with_env_or_str(&config.client.log_level)
        .start()
        .expect("logger");

    let clipboard_ctx = ClipboardContext::new();
    let url = format!(
        "{}://{}:{}",
        if config.client.root_ca.is_some() {
            "https"
        } else {
            "http"
        },
        config.client.host,
        config.client.port
    );

    let client = if config.client.root_ca.is_some() {
        let cert = tokio::fs::read(config.client.root_ca.unwrap())
            .await
            .unwrap();
        let cert = Certificate::from_pem(cert);

        let tls_ops = ClientTlsConfig::new().ca_certificate(cert);

        let channel = Channel::from_shared(url.clone())
            .unwrap()
            .tls_config(tls_ops)
            .unwrap()
            .connect()
            .await
            .unwrap();

        info!("Connecting to {}", url);
        clipboard_client::ClipboardClient::new(channel)
    } else {
        clipboard_client::ClipboardClient::connect(url)
            .await
            .unwrap()
    };

    let mut global_context = GlobalContext {
        proto_client: client,
        clipboard: &clipboard_ctx,
        db: Default::default(),
    };

    loop {
        check_clipboard(&mut global_context).await;
        time::sleep(Duration::from_millis(200)).await;
    }
}
