use std::error::Error;
use std::fmt::{Display, Formatter};
use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use std::time::Duration;

#[derive(Clone)]
pub struct State {
    activity: Activity,
    progress: SessionDuration,
    num_activity: u32,
    pub timer_is_stopped: bool,
    settings: Settings,
}

impl State {
    pub fn new(settings: Settings) -> State {
        State {
            activity: Activity::Focus,
            progress: SessionDuration(Duration::from_secs(0)),
            num_activity: 0,
            timer_is_stopped: !settings.start_automatically,
            settings,
        }
    }

    pub fn increase_progress(&mut self, duration: &Duration) {
        *self.progress += *duration;

        let max_duration = &match self.activity {
            Activity::Focus => self.settings.focus_duration,
            Activity::ShortBreak => self.settings.short_break_duration,
            Activity::LongBreak => self.settings.long_break_duration,
        };

        if *self.progress >= **max_duration {
            *self.progress -= **max_duration;
            self.num_activity += 1;
            self.activity = match self.activity {
                Activity::Focus => {
                    if self.num_activity % 4 == 0 {
                        Activity::LongBreak
                    } else {
                        Activity::ShortBreak
                    }
                }
                Activity::ShortBreak => Activity::Focus,
                Activity::LongBreak => Activity::Focus,
            }
        }
    }

    pub fn time_remaining(&self) -> SessionDuration {
        let duration = match self.activity {
            Activity::Focus => self.settings.focus_duration,
            Activity::ShortBreak => self.settings.short_break_duration,
            Activity::LongBreak => self.settings.long_break_duration,
        };

        SessionDuration::from(*duration - *self.progress)
    }

    pub fn start_timer(&mut self) {
        self.timer_is_stopped = false;
    }

    pub fn stop_timer(&mut self) {
        self.timer_is_stopped = true;
    }

    pub fn toggle_timer(&mut self) {
        self.timer_is_stopped = !self.timer_is_stopped;
    }
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {}",
            self.activity,
            self.progress,
            if self.timer_is_stopped { "||" } else { "" }
        )
    }
}

#[derive(Clone)]
pub enum Activity {
    Focus,
    ShortBreak,
    LongBreak,
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

#[derive(Clone)]
pub struct Settings {
    pub focus_duration: SessionDuration,
    pub short_break_duration: SessionDuration,
    pub long_break_duration: SessionDuration,
    pub start_automatically: bool,
}

#[derive(Copy, Clone, PartialEq, Eq)]
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

#[derive(Debug)]
pub enum ParseSessionDurationError {
    InvalidFormat,
    TooManySeconds,
    NotTwoDigitsForSeconds,
    ParseIntError(std::num::ParseIntError),
}

impl Display for ParseSessionDurationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseSessionDurationError::InvalidFormat => {
                write!(f, "expected \"minutes\" or \"minutes:seconds\"")
            }
            ParseSessionDurationError::TooManySeconds => {
                write!(f, "seconds must be less than 60")
            }
            ParseSessionDurationError::ParseIntError(e) => {
                write!(f, "failed to parse integer: {e}")
            }
            ParseSessionDurationError::NotTwoDigitsForSeconds => {
                write!(f, "seconds must be two digits")
            }
        }
    }
}

impl Error for ParseSessionDurationError {}

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
        let minutes = self.0.as_secs() / 60;
        let seconds = self.0.as_secs() % 60;
        write!(f, "{minutes}:{seconds:02}")
    }
}
