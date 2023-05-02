use crate::event::Event;
use crate::notification;
use crate::pomodoro;
use crate::tui::{DisplayData, Tui, TuiError};
use crossbeam::{
    channel::{after, tick, unbounded, Receiver, RecvError, SendError, Sender, TryRecvError},
    select,
};
use std::fmt::Display;
use std::ops::Deref;
use std::time::{Duration, Instant};

pub struct App {
    pomodoro_state: pomodoro::State,
    events_tx: Sender<Event>,
    events_rx: Receiver<Event>,
    tui: Tui,
}

impl App {
    pub fn new(pomodoro_state: pomodoro::State) -> Result<Self, AppError> {
        let (events_tx, events_rx) = unbounded::<Event>();
        let tui = Tui::new()?;

        Ok(Self {
            pomodoro_state,
            events_tx,
            events_rx,
            tui,
        })
    }

    pub fn run(&mut self) -> Result<(), AppError> {
        let reset_clock = || (Instant::now(), after(Duration::from_millis(100)));

        let (mut start_time, mut pomodoro_clock_rx) = reset_clock();
        let event_poll_clock_rx = tick(Duration::from_millis(20));

        self.tui.enable()?;

        loop {
            self.tui.render(&DisplayData::from(&self.pomodoro_state))?;

            select! {
                recv(pomodoro_clock_rx) -> _ => {
                    if self.pomodoro_state.timer_is_active() {
                        let activity_before = self.pomodoro_state.current_activity();

                        self.pomodoro_state.increase_progress(start_time.elapsed());
                        (start_time, pomodoro_clock_rx) = reset_clock();

                        let activity_after = self.pomodoro_state.current_activity();
                        if activity_before != activity_after {
                            self.show_desktop_notification();
                        }
                    }
                }
                recv(self.events_rx) -> event => {
                    let event = event?;
                    let timer_was_stopped = !self.pomodoro_state.timer_is_active();

                    if *self.handle_event(&event) {
                        break;
                    }

                    let timer_is_active_now = self.pomodoro_state.timer_is_active();
                    if timer_was_stopped && timer_is_active_now {
                        (start_time, pomodoro_clock_rx) = reset_clock();
                    }
                }
                recv(event_poll_clock_rx) -> _ => {
                    for event in self.tui.try_read_events(10)? {
                        self.events_tx.send(event)?;
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
#[derive(Debug)]
pub enum AppError {
    ChannelRecv(RecvError),
    ChannelTryRecv(TryRecvError),
    ChannelSendEvent(SendError<Event>),
    TuiError(TuiError),
}

impl From<RecvError> for AppError {
    fn from(error: RecvError) -> Self {
        AppError::ChannelRecv(error)
    }
}

impl From<TryRecvError> for AppError {
    fn from(error: TryRecvError) -> Self {
        AppError::ChannelTryRecv(error)
    }
}

impl From<SendError<Event>> for AppError {
    fn from(error: SendError<Event>) -> Self {
        AppError::ChannelSendEvent(error)
    }
}

impl From<TuiError> for AppError {
    fn from(error: TuiError) -> Self {
        AppError::TuiError(error)
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner_error = match self {
            AppError::ChannelRecv(error) => error.to_string(),
            AppError::ChannelTryRecv(error) => error.to_string(),
            AppError::ChannelSendEvent(error) => error.to_string(),
            AppError::TuiError(error) => error.to_string(),
        };
        write!(f, "unrecoverable error: {inner_error}")
    }
}
