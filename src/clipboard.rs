use crossbeam_channel::Sender;
use log::{debug, trace};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::{future::Future, time::Duration};
use x11_clipboard::Clipboard;

use super::{Result, LAST_CLIPBOARD_CONTENT};

struct ClipboardContext {
    clipboard: Clipboard,
}

impl ClipboardContext {
    fn new() -> Self {
        Self {
            clipboard: Clipboard::new().unwrap(),
        }
    }
}

impl Future for &ClipboardContext {
    type Output = String;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        trace!("Wait clipboard updates...");

        let event = self.clipboard.load(
            self.clipboard.getter.atoms.clipboard,
            self.clipboard.getter.atoms.utf8_string,
            self.clipboard.getter.atoms.property,
            Some(Duration::from_millis(10)),
        );
        trace!("event: {:?}", event);

        let bytes = match event {
            Ok(e) => e,
            Err(_) => {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
        };

        let text = String::from_utf8(bytes).unwrap();
        let mut last_clip = LAST_CLIPBOARD_CONTENT.lock().unwrap();

        if last_clip.eq(&text) {
            trace!("Previous content, retry");
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }

        *last_clip = text.clone();

        if text.is_empty() {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }

        debug!("Got new clipboard contents: {:?}", text);

        Poll::Ready(text)
    }
}

async fn get_next_clipboard(clipboard: &ClipboardContext) -> String {
    clipboard.await
}

pub async fn clipboard_loop(sender: Sender<String>) -> Result<()> {
    let clipboard = ClipboardContext::new();

    loop {
        let text = get_next_clipboard(&clipboard).await;

        debug!("Add {:?} to updates", text);
        sender.send(text).unwrap();
    }
}

pub fn clipboard_update(content: String) {
    let clipboard = Clipboard::new().unwrap();

    debug!("Update clipboard content: {:?}", content);

    clipboard
        .store(
            clipboard.getter.atoms.clipboard,
            clipboard.getter.atoms.utf8_string,
            content,
        )
        .unwrap();
}
