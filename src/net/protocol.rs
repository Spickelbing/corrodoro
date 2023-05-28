use crate::net::NetworkError;
use crate::pomodoro::{Activity, SessionDuration};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize)]
pub enum ClientMessage {
    Event(Event),
}

#[derive(Serialize, Deserialize)]
pub enum ServerMessage {
    Display(TimerVisuals),
    Notify(Activity),
}

#[derive(Serialize, Deserialize)]
pub enum Event {
    Quit,
    ToggleTimer,
    ResetTimer,
    SkipActivity,
    ExtendActivity(Duration),
    ReduceActivity(Duration),
}

#[derive(Serialize, Deserialize)]
pub struct TimerVisuals {
    pub time_remaining: SessionDuration,
    pub timer_is_paused: bool,
    pub activity: Activity,
    pub progress_percentage: f64,
    pub completed_focus_sessions: u32,
}

impl ClientMessage {
    pub fn as_bytes(&self) -> Result<Bytes, NetworkError> {
        self.try_into()
    }

    pub fn from_bytes(bytes: Bytes) -> Result<Self, NetworkError> {
        bytes.try_into()
    }
}

impl TryFrom<Bytes> for ClientMessage {
    type Error = NetworkError;

    fn try_from(bytes: Bytes) -> Result<Self, Self::Error> {
        bincode::deserialize(&bytes).map_err(NetworkError::Deserialize)
    }
}

impl TryFrom<&ClientMessage> for Bytes {
    type Error = NetworkError;

    fn try_from(msg: &ClientMessage) -> Result<Self, Self::Error> {
        bincode::serialize(&msg)
            .map_err(NetworkError::Serialize)
            .map(|v| v.into())
    }
}

impl ServerMessage {
    pub fn as_bytes(&self) -> Result<Bytes, NetworkError> {
        self.try_into()
    }

    pub fn from_bytes(bytes: Bytes) -> Result<Self, NetworkError> {
        bytes.try_into()
    }
}

impl TryFrom<Bytes> for ServerMessage {
    type Error = NetworkError;

    fn try_from(bytes: Bytes) -> Result<Self, Self::Error> {
        bincode::deserialize(&bytes).map_err(NetworkError::Deserialize)
    }
}

impl TryFrom<&ServerMessage> for Bytes {
    type Error = NetworkError;

    fn try_from(msg: &ServerMessage) -> Result<Self, Self::Error> {
        bincode::serialize(&msg)
            .map_err(NetworkError::Serialize)
            .map(|v| v.into())
    }
}
