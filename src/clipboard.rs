use crossbeam_channel::Sender;
use log::{debug, trace, warn};
use x11_clipboard::Clipboard;

use super::{Result, LAST_CLIPBOARD_CONTENT};

fn get_next_clipboard(clipboard: &Clipboard) -> String {
    loop {
        trace!("Wait clipboard updates...");

        let mut text = match clipboard.load_wait(
            clipboard.getter.atoms.clipboard,
            clipboard.getter.atoms.utf8_string,
            clipboard.getter.atoms.property,
        ) {
            Ok(text) => text,
            Err(_) => {
                trace!("load_wait failed!");
                continue;
            }
        };

        text.retain(|c| c != &0);

        let text = match String::from_utf8(text) {
            Ok(text) => text,
            Err(e) => {
                warn!("UTF8 convert failed: {:?}", e);

                continue;
            }
        };

        debug!("Got new clipboard contents: {:?}", text);

        if text.is_empty() {
            continue;
        }

        return text;
    }
}

pub fn clipboard_loop(sender: Sender<String>) -> Result<()> {
    let clipboard = Clipboard::new().unwrap();

    loop {
        let text = get_next_clipboard(&clipboard);

        trace!("Add {:?} to updates", text);
        sender.send(text).unwrap();
    }
}

pub fn clipboard_update(content: String) {
    let clipboard = Clipboard::new().unwrap();

    debug!("Update clipboard content: {:?}", content);

    *LAST_CLIPBOARD_CONTENT.lock().unwrap() = content.clone();

    clipboard
        .store(
            clipboard.getter.atoms.clipboard,
            clipboard.getter.atoms.utf8_string,
            content,
        )
        .unwrap();
}
