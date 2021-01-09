use std::{collections::HashMap, pin::Pin, time::Duration};

use crossbeam_channel::Receiver;
use futures::{
    stream::Stream,
    task::{Context, Poll},
    StreamExt,
};
use log::{debug, trace};
use reqwest::ClientBuilder;

use crate::{clipboard::ClipboardContext, server::ClipboardResponse};

#[derive(Clone)]
struct FutureReceiver<T> {
    receiver: Receiver<T>,
}

impl<T> Stream for FutureReceiver<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.receiver.try_recv() {
            Ok(i) => Poll::Ready(Some(i)),
            Err(crossbeam_channel::TryRecvError::Empty) => Poll::Pending,
            Err(_) => Poll::Ready(None),
        }
    }
}

async fn fetch_updates(host: &str, port: u16, ctx: ClipboardContext) {
    let poll_url = format!("http://{}:{}/get_clipboard", host, port);

    let client = ClientBuilder::new()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();

    loop {
        trace!("Try to get updates");

        let response = match client.get(&poll_url).send().await {
            Ok(r) => r,
            Err(_) => continue,
        };
        let response = match response.json::<ClipboardResponse>().await {
            Ok(r) => r,
            Err(_) => continue,
        };
        match response.contents {
            Some(c) => ctx.set(c).ok(),
            None => continue,
        };
    }
}

pub async fn client(
    host: &str,
    port: u16,
    clipboard: ClipboardContext,
    receiver: Receiver<String>,
) {
    let fetch_host = host.to_string();
    actix_rt::spawn(async move { fetch_updates(&fetch_host, port, clipboard).await });

    let poll_url = format!("http://{}:{}/push_clipboard", host, port);
    let client = ClientBuilder::new()
        .timeout(Duration::from_secs(60))
        .build()
        .unwrap();

    let mut receiver = FutureReceiver { receiver };

    debug!("enter in the loop");
    loop {
        let update = receiver.next().await.unwrap();
        debug!("got update: {:?}", update);
        let mut map = HashMap::new();
        map.insert("contents", update);

        debug!("try update server {}:{}", host, port);
        client.post(&poll_url).json(&map).send().await.unwrap();
    }
}
