use log::{debug, warn};
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;
use x11_clipboard::Clipboard;

struct InnerClipboardContext {
    clipboard: Clipboard,
    current: String,
}

#[derive(Clone)]
pub struct ClipboardContext {
    inner: Arc<Mutex<InnerClipboardContext>>,
}

impl Default for ClipboardContext {
    fn default() -> Self {
        Self::new()
    }
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

    pub async fn get(&self) -> Option<String> {
        let mut inner = self.inner.lock().await;

        let text = String::from_utf8(
            inner
                .clipboard
                .load(
                    inner.clipboard.getter.atoms.clipboard,
                    inner.clipboard.getter.atoms.utf8_string,
                    inner.clipboard.getter.atoms.property,
                    Some(Duration::from_millis(10)),
                )
                .ok()?,
        )
        .ok()?;

        if text.is_empty() || inner.current == text {
            return None;
        }

        inner.current = text.clone();

        Some(text)
    }

    pub async fn set(&self, str: String) -> Option<()> {
        if str.is_empty() {
            return None;
        }

        let mut inner = self.inner.lock().await;

        if inner.current == str {
            warn!("set :: Duplicated value: {:?}", str);
            return None;
        }

        debug!("set :: Update clipboard content: {:?}", str);
        inner
            .clipboard
            .store(
                inner.clipboard.getter.atoms.clipboard,
                inner.clipboard.getter.atoms.utf8_string,
                str.clone(),
            )
            .ok()?;
        inner.current = str;

        Some(())
    }
}
