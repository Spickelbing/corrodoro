use crate::event::Event;
use crate::notification;
use crate::pomodoro;
use crate::tui::{DisplayData, Tui, TuiError};
use std::ops::Deref;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::select;
use tokio::time::interval;

pub struct App {
    pomodoro_state: pomodoro::State,
    tui: Tui,
}

impl App {
    pub fn new(pomodoro_state: pomodoro::State) -> Result<Self, AppError> {
        let tui = Tui::new()?;

        Ok(Self {
            pomodoro_state,
            tui,
        })
    }

    pub async fn run(&mut self) -> Result<(), AppError> {
        let mut pomodoro_clock = interval(Duration::from_millis(100));
        let mut pomodoro_start_time = Instant::now();

        self.tui.enable()?;

        loop {
            self.tui.render(&DisplayData::from(&self.pomodoro_state))?;

            select! {
                _ = pomodoro_clock.tick() => {
                    if self.pomodoro_state.timer_is_active() {
                        let activity_before = self.pomodoro_state.current_activity();

                        self.pomodoro_state.increase_progress(pomodoro_start_time.elapsed());
                        pomodoro_start_time = Instant::now();

                        let activity_after = self.pomodoro_state.current_activity();
                        if activity_before != activity_after {
                            self.show_desktop_notification();
                        }
                    }
                }
                event = self.tui.read_event() => {
                    let event = event?;
                    let timer_was_stopped = !self.pomodoro_state.timer_is_active();

                    if *self.handle_event(&event) {
                        break;
                    }

                    let timer_is_active_now = self.pomodoro_state.timer_is_active();
                    if timer_was_stopped && timer_is_active_now {
                        pomodoro_clock.reset();
                        pomodoro_start_time = Instant::now();
                    }
                }
            }
        }

        self.tui.disable()?;

        Ok(())
    }

    fn show_desktop_notification(&self) {
        // ignore errors for now, shouldn't crash but also don't know how to handle
        let _ = notification::show_desktop_notification(
            "",
            &self.pomodoro_state.current_activity().to_string(),
        );
    }

    fn handle_event(&mut self, event: &Event) -> AppShouldQuit {
        match event {
            Event::ToggleTimer => {
                self.pomodoro_state.toggle_timer();
            }
            Event::ExtendActivity(duration) => {
                self.pomodoro_state.extend_activity(duration);
            }
            Event::ReduceActivity(duration) => {
                self.pomodoro_state.reduce_activity(duration);
            }
            Event::SkipActivity => {
                self.pomodoro_state.skip_activity();
            }
            Event::Quit => return AppShouldQuit(true),
            _ => (),
        };

        AppShouldQuit(false)
    }
}

struct AppShouldQuit(bool);

impl Deref for AppShouldQuit {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Represents errors the app has no control over. They are unrecoverable.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("unrecoverable tui error: {0}")]
    TuiError(#[from] TuiError),
}
