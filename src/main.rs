use crate::args::{Args, Parser};
use std::fmt::Display;
use std::io;
use std::ops::Deref;
use crossbeam::channel::{unbounded, RecvError, TryRecvError, SendError, Receiver, Sender};
use std::thread;
use std::time::Duration;
use std::time::Instant;
use std::vec::Vec;

mod animations;
mod args;
mod pomodoro;
mod ui;

fn main() {
    let args = Args::parse();

    match args.command {
        args::Command::Local { work, short, long } => run_local_session(work, short, long),
        _ => todo!(),
    }
}

fn run_local_session(
    work: pomodoro::SessionDuration,
    short: pomodoro::SessionDuration,
    long: pomodoro::SessionDuration,
) {
    let pomodoro_settings = pomodoro::Settings {
        focus_duration: work,
        short_break_duration: short,
        long_break_duration: long,
        start_automatically: false,
    };
    let pomodoro_state = pomodoro::State::new(pomodoro_settings);

    let timer_update_interval = Duration::from_millis(50);

    let (events_tx, events_rx) = unbounded::<ui::Event>();
    let (ui_tx, ui_rx) = unbounded::<pomodoro::State>();
    let (close_ui_tx, close_ui_rx) = unbounded::<CloseThreadNotificiation>();

    let mut thread_handles = Vec::new();

    thread_handles.push(thread::spawn(move || {
        ui::ui_thread(ui_rx, events_tx, close_ui_rx)
    }));
    thread_handles.push(thread::spawn(move || {
        state_transformer_thread(
            pomodoro_state,
            timer_update_interval,
            events_rx,
            ui_tx,
            close_ui_tx,
        )
    }));

    for handle in thread_handles {
        handle.join().expect("failed to join thread"); // TODO: handle join error, handle `AppError`s
    }
}

pub struct CloseThreadNotificiation;

pub enum AppError {
    Io(io::Error),
    ChannelRecv(RecvError),
    ChannelTryRecv(TryRecvError),
    ChannelSendPomodoroState(SendError<pomodoro::State>),
    ChannelSendUiEvent(SendError<ui::Event>),
    ChannelSendCloseNotification(SendError<CloseThreadNotificiation>),
}

impl From<io::Error> for AppError {
    fn from(error: io::Error) -> Self {
        AppError::Io(error)
    }
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

impl From<SendError<pomodoro::State>> for AppError {
    fn from(error: SendError<pomodoro::State>) -> Self {
        AppError::ChannelSendPomodoroState(error)
    }
}

impl From<SendError<ui::Event>> for AppError {
    fn from(error: SendError<ui::Event>) -> Self {
        AppError::ChannelSendUiEvent(error)
    }
}

impl From<SendError<CloseThreadNotificiation>> for AppError {
    fn from(error: SendError<CloseThreadNotificiation>) -> Self {
        AppError::ChannelSendCloseNotification(error)
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Io(error) => write!(f, "I/O error: {error}"),
            AppError::ChannelRecv(error) => write!(f, "channel receive error: {error}"),
            AppError::ChannelTryRecv(error) => write!(f, "channel receive error: {error}"),
            AppError::ChannelSendPomodoroState(error) => {
                write!(f, "send error for state channel: {error}")
            }
            AppError::ChannelSendUiEvent(error) => {
                write!(f, "send error for event channel: {error}")
            }
            AppError::ChannelSendCloseNotification(error) => {
                write!(f, "send error for close-notification channel: {error}")
            }
        }
    }
}

// TODO: separate into timer and event handler thread, where event handler answers to events immediately
fn state_transformer_thread(
    mut state: pomodoro::State,
    timer_update_interval: Duration,
    events_rx: Receiver<ui::Event>,
    ui_tx: Sender<pomodoro::State>,
    close_ui_tx: Sender<CloseThreadNotificiation>,
) -> Result<(), AppError> {
    loop {
        let now = Instant::now();
        ui_tx.send(state.clone())?;

        if state.timer_is_stopped {
            // this blocks the thread until a ui event is received
            for event in [events_rx.recv()?]
                .into_iter()
                .chain(events_rx.try_iter().into_iter())
            {
                if *handle_event(&event, &mut state) {
                    close_ui_tx.send(CloseThreadNotificiation)?;
                    return Ok(());
                }
            }
            continue;
        }

        // TODO: handle hangup
        for event in events_rx.try_iter() {
            if *handle_event(&event, &mut state) {
                close_ui_tx.send(CloseThreadNotificiation)?;
                return Ok(());
            }
        }

        let elapsed = now.elapsed();
        if elapsed < timer_update_interval {
            thread::sleep(timer_update_interval - elapsed);
        }

        state.increase_progress(&now.elapsed());
    }
}

struct AppShouldQuit(bool);

impl Deref for AppShouldQuit {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn handle_event(event: &ui::Event, state: &mut pomodoro::State) -> AppShouldQuit {
    match event {
        ui::Event::ToggleTimer => {
            state.toggle_timer();
        }
        ui::Event::ExtendActivity(duration) => {
            state.extend_activity(duration);
        }
        ui::Event::ReduceActivity(duration) => {
            state.reduce_activity(duration);
        }
        ui::Event::SkipActivity => {
            state.skip_activity();
        }
        ui::Event::Quit => return AppShouldQuit(true),
        _ => (),
    }

    AppShouldQuit(false)
}
