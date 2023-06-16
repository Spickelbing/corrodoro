use rodio::{Decoder, OutputStream, Sink};
use std::io::Cursor;
use std::thread;
use thiserror::Error;

pub fn show_desktop_notification(title: &str, message: &str) -> Result<(), NotificationError> {
    notify_rust::Notification::new()
        .summary(title)
        .body(message)
        .show()?;
    Ok(())
}

pub fn play_notification_sound() {
    thread::spawn(move || {
        // ignore errors, too insignificant for crash
        let _ = play_notification_sound_sync();
    });
}

fn play_notification_sound_sync() -> Result<(), NotificationError> {
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let audio_file = Cursor::new(include_bytes!("../media/notification.wav"));
    let audio = Decoder::new(audio_file)?;

    let sink = Sink::try_new(&stream_handle)?;
    sink.append(audio);
    sink.set_volume(1.0);
    sink.sleep_until_end();

    Ok(())
}

#[derive(Debug, Error)]
pub enum NotificationError {
    #[error("failed to show desktop notification")]
    Show(#[from] notify_rust::error::Error),
    #[error("failed to create audio stream for notification sound: {0}")]
    StreamCreation(#[from] rodio::StreamError),
    #[error("failed to play notification sound: {0}")]
    Play(#[from] rodio::PlayError),
    #[error("failed to decode notification sound: {0}")]
    Decoding(#[from] rodio::decoder::DecoderError),
}
