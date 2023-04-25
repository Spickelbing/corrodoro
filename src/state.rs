use crate::args::SessionDuration;
use std::fmt::Display;
use std::time::Duration;

#[derive(Clone)]
pub struct PomodoroState {
    activity: Activity,
    progress: SessionDuration,
    num_activity: u32,
    pub timer_is_stopped: bool,
    settings: Settings,
}

impl PomodoroState {
    pub fn new(settings: Settings) -> PomodoroState {
        PomodoroState {
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

    pub fn handle_event(&mut self, event : Event) {
        match event {
            Event::Pause => self.timer_is_stopped = true,
            Event::Resume => self.timer_is_stopped = false,
        }
    }
}

impl Display for PomodoroState {
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

pub enum Event {
    Pause,
    Resume,
}
