use crate::args::SessionDuration;
use std::fmt::Display;
use std::time::Duration;

pub struct State {
    activity: Activity,
    progress: SessionDuration,
    num_activity: u32,
    pub is_stopped: bool,
    settings: Settings,
}

impl State {
    pub fn new(settings: Settings) -> State {
        State {
            activity: Activity::Focus,
            progress: SessionDuration(Duration::from_secs(0)),
            num_activity: 0,
            is_stopped: !settings.start_automatically,
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
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {}",
            self.activity,
            self.progress,
            if self.is_stopped { "||" } else { "" }
        )
    }
}

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

pub struct Settings {
    pub focus_duration: SessionDuration,
    pub short_break_duration: SessionDuration,
    pub long_break_duration: SessionDuration,
    pub start_automatically: bool,
}

pub enum Message {
    Start,
    Stop,
    Reset,
    Skip,
    //Quit,
    IncreaseProgress(SessionDuration),
    DecreaseProgress(SessionDuration),
}
