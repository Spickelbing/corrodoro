use crate::pomodoro::{Activity, SessionDuration};
use bincode::{deserialize, serialize};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use zwiesel::{Message, MessageError};

#[derive(Serialize, Deserialize)]
pub enum NetworkProtocol {
    Event(Event),
    Display(TimerVisuals),
    Notify(Activity),
}

impl Message for NetworkProtocol {
    fn serialize(&self) -> Result<Bytes, MessageError> {
        Ok(serialize(self).map_err(|_| MessageError::Serialize)?.into())
    }

    fn deserialize(bytes: Bytes) -> Result<Self, MessageError> {
        deserialize(&bytes).map_err(|_| MessageError::Deserialize)
    }
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

// TODO: change (remaining time, progress percentage) to (progressed time) and (total time)
#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct TimerVisuals {
    pub time_remaining: SessionDuration,
    pub timer_is_paused: bool,
    pub activity: Activity,
    pub progress_percentage: f64,
    pub completed_focus_sessions: u32,
}
