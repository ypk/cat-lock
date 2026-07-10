//! System notification support using `notify-rust`.

use notify_rust::Notification;

/// Send a desktop notification if possible.
/// Silently fails if notifications are unavailable or unsupported.
pub fn send(title: &str, body: &str) {
    let result = Notification::new()
        .appname("CatLock")
        .summary(title)
        .body(body)
        .show();

    if let Err(e) = result {
        log::debug!("Failed to send desktop notification: {}", e);
    }
}
