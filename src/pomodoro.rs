use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use std::time::Duration;
use thiserror::Error;

#[derive(Clone, Serialize, Deserialize)]
pub struct State {
    activity: Activity,
    progress: SessionDuration,
    completed_focus_sessions: u32,
    timer_is_active: bool,
    settings: Settings,
    current_activity_duration_override: Option<SessionDuration>,
}

impl State {
    pub fn new(settings: Settings) -> State {
        State {
            activity: Activity::Focus,
            progress: SessionDuration(Duration::from_secs(0)),
            completed_focus_sessions: 0,
            timer_is_active: settings.start_automatically,
            settings,
            current_activity_duration_override: None,
        }
    }

    pub fn increase_progress(&mut self, duration: Duration) {
        *self.progress += duration;

        let max_duration = self.current_activity_duration();

        if *self.progress >= *max_duration {
            if self.settings.start_automatically {
                *self.progress -= *max_duration;
                self.start_timer();
            } else {
                self.progress = Duration::from_secs(0).into();
                self.stop_timer();
            }

            self.current_activity_duration_override = None;

            if self.activity.is_focus() {
                self.completed_focus_sessions += 1;
            }

            self.activity = self.next_activity();
        }
    }

    pub fn time_remaining(&self) -> SessionDuration {
        (*self.current_activity_duration() - *self.progress).into()
    }

    pub fn start_timer(&mut self) {
        self.timer_is_active = true;
    }

    pub fn stop_timer(&mut self) {
        self.timer_is_active = false;
    }

    pub fn toggle_timer(&mut self) {
        self.timer_is_active = !self.timer_is_active;
    }

    pub fn timer_is_active(&self) -> bool {
        self.timer_is_active
    }

    pub fn skip_activity(&mut self) {
        self.progress = Duration::from_secs(0).into();
        self.current_activity_duration_override = None;
        if self.activity.is_focus() {
            self.completed_focus_sessions += 1;
        }

        if self.settings.start_automatically {
            self.start_timer();
        } else {
            self.stop_timer();
        }

        self.activity = self.next_activity();
    }

    /// Does nothing if the extension in duration would lead to an overflow (probably about 3.5 billion seconds).
    pub fn extend_activity(&mut self, duration: &Duration) {
        if let Some(sum) = self.current_activity_duration().checked_add(*duration) {
            self.current_activity_duration_override = Some(sum.into());
        }
    }

    /// Does nothing if the reducement in duration would yield 0 or lead to a negative duration.
    pub fn reduce_activity(&mut self, duration: &Duration) {
        if *self.time_remaining() > *duration {
            self.current_activity_duration_override =
                Some((*self.current_activity_duration() - *duration).into());
        }
    }

    pub fn progress_percentage(&self) -> f64 {
        self.progress.as_secs_f64() / self.current_activity_duration().as_secs_f64()
    }

    pub fn current_activity(&self) -> Activity {
        self.activity
    }

    pub fn completed_focus_sessions(&self) -> u32 {
        self.completed_focus_sessions
    }

    fn current_activity_duration(&self) -> SessionDuration {
        match self.current_activity_duration_override {
            Some(duration) => duration,
            None => match self.activity {
                Activity::Focus => self.settings.focus_duration,
                Activity::ShortBreak => self.settings.short_break_duration,
                Activity::LongBreak => self.settings.long_break_duration,
            },
        }
    }

    fn next_activity(&self) -> Activity {
        match self.activity {
            Activity::Focus => {
                if self.completed_focus_sessions % 4 == 0 {
                    Activity::LongBreak
                } else {
                    Activity::ShortBreak
                }
            }
            Activity::ShortBreak => Activity::Focus,
            Activity::LongBreak => Activity::Focus,
        }
    }

    pub fn reset(&mut self) {
        self.activity = Activity::Focus;
        self.progress = Duration::from_secs(0).into();
        self.completed_focus_sessions = 0;
        self.timer_is_active = self.settings.start_automatically;
        self.current_activity_duration_override = None;
    }
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\n{} {}",
            self.time_remaining(),
            self.activity,
            if self.timer_is_active { "⏸" } else { "▶" }
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Activity {
    Focus,
    ShortBreak,
    LongBreak,
}

impl Activity {
    pub fn is_focus(&self) -> bool {
        matches!(self, Activity::Focus)
    }
}

impl Display for Activity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Activity::Focus => write!(f, "focus"),
            Activity::ShortBreak => write!(f, "short break"),
            Activity::LongBreak => write!(f, "long break"),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    pub focus_duration: SessionDuration,
    pub short_break_duration: SessionDuration,
    pub long_break_duration: SessionDuration,
    pub start_automatically: bool,
}

impl Settings {
    pub fn new(
        focus: SessionDuration,
        short_break: SessionDuration,
        long_break: SessionDuration,
        start_automatically: bool,
    ) -> Self {
        Settings {
            focus_duration: focus,
            short_break_duration: short_break,
            long_break_duration: long_break,
            start_automatically,
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            focus_duration: SessionDuration(Duration::from_secs(25 * 60)),
            short_break_duration: SessionDuration(Duration::from_secs(5 * 60)),
            long_break_duration: SessionDuration(Duration::from_secs(15 * 60)),
            start_automatically: false,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SessionDuration(pub Duration);

impl Deref for SessionDuration {
    type Target = Duration;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SessionDuration {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Duration> for SessionDuration {
    fn from(duration: Duration) -> Self {
        SessionDuration(duration)
    }
}

#[derive(Debug, Error)]
pub enum ParseSessionDurationError {
    #[error("expected \"minutes\" or \"minutes:seconds\"")]
    InvalidFormat,
    #[error("seconds must be less than 60")]
    TooManySeconds,
    #[error("seconds must be two digits")]
    NotTwoDigitsForSeconds,
    #[error("failed to parse integer: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
}

impl FromStr for SessionDuration {
    type Err = ParseSessionDurationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v: Vec<&str> = s.split(':').collect();
        let minutes = v[0]
            .parse::<u64>()
            .map_err(ParseSessionDurationError::ParseIntError)?;

        match v.len() {
            1 => Ok(SessionDuration(Duration::from_secs(minutes * 60))),
            2 => {
                let seconds = v[1]
                    .parse::<u64>()
                    .map_err(ParseSessionDurationError::ParseIntError)?;
                if seconds > 59 {
                    return Err(ParseSessionDurationError::TooManySeconds);
                } else if v[1].len() != 2 {
                    return Err(ParseSessionDurationError::NotTwoDigitsForSeconds);
                }

                Ok(SessionDuration(Duration::from_secs(minutes * 60 + seconds)))
            }
            _ => Err(ParseSessionDurationError::InvalidFormat),
        }
    }
}

impl Display for SessionDuration {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let display_secs = self.0.as_secs_f64().ceil() as u64;

        let minutes = display_secs / 60;
        let seconds = display_secs % 60;
        write!(f, "{minutes}:{seconds:02}")
    }
}
