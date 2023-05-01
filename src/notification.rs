use std::{error::Error, fmt::Display};

pub fn show_desktop_notification(title: &str, message: &str) -> Result<(), NotificationError> {
    notify_rust::Notification::new()
        .summary(title)
        .body(message)
        .show()?;
    Ok(())
}

#[derive(Debug)]
pub struct NotificationError;

impl Error for NotificationError {}

impl Display for NotificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "failed to show desktop notification")
    }
}

impl From<notify_rust::error::Error> for NotificationError {
    fn from(_: notify_rust::error::Error) -> Self {
        NotificationError
    }
}
