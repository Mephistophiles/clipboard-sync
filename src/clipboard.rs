use crossbeam_channel::Sender;
use log::{debug, trace};
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
    time::Duration,
};
use x11_clipboard::Clipboard;

use crate::error::{ClipboardError, Error, Result};

struct InnerClipboardContext {
    clipboard: Clipboard,
    current: String,
}

#[derive(Clone)]
pub struct ClipboardContext {
    inner: Arc<Mutex<InnerClipboardContext>>,
}

impl ClipboardContext {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(InnerClipboardContext {
                clipboard: Clipboard::new().unwrap(),
                current: Default::default(),
            })),
        }
    }

    pub fn get(&self) -> Result<String> {
        let mut inner = self.inner.lock().unwrap();

        let text = String::from_utf8(
            inner
                .clipboard
                .load(
                    inner.clipboard.getter.atoms.clipboard,
                    inner.clipboard.getter.atoms.utf8_string,
                    inner.clipboard.getter.atoms.property,
                    Some(Duration::from_millis(10)),
                )
                .map_err(Error::from)?,
        )
        .map_err(Error::from)?;

        if inner.current == text {
            return Err(Error::ClipboardError(ClipboardError::DuplicatedValue));
        }

        if text.is_empty() {
            return Err(Error::ClipboardError(ClipboardError::EmptyValue));
        }

        inner.current = text.clone();

        Ok(text)
    }

    pub fn set(&self, str: String) -> Result<()> {
        if str.is_empty() {
            return Err(Error::ClipboardError(ClipboardError::EmptyValue));
        }

        let mut inner = self.inner.lock().unwrap();

        if inner.current == str {
            return Err(Error::ClipboardError(ClipboardError::DuplicatedValue));
        }

        debug!("Update clipboard content: {:?}", str);
        inner
            .clipboard
            .store(
                inner.clipboard.getter.atoms.clipboard,
                inner.clipboard.getter.atoms.utf8_string,
                str.clone(),
            )
            .map_err(Error::from)?;
        inner.current = str;

        Ok(())
    }
}

impl Future for &ClipboardContext {
    type Output = String;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        trace!("Wait clipboard updates...");

        match self.get() {
            Ok(update) => {
                debug!("Got new clipboard contents: {:?}", update);
                Poll::Ready(update)
            }
            Err(_) => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }
}

async fn get_next_clipboard(clipboard: &ClipboardContext) -> String {
    clipboard.await
}

pub async fn clipboard_loop(context: ClipboardContext, sender: Sender<String>) -> Result<()> {
    loop {
        let text = get_next_clipboard(&context).await;

        debug!("Add {:?} to updates", text);
        sender.send(text).unwrap();
    }
}
