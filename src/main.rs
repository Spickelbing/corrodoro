use ui::UiUpdate;
use crate::args::{Args, Parser, SessionDuration};
use crate::state::Settings;
use crate::state::{Event, PomodoroState};
use std::fmt::Display;
use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use std::vec::Vec;

mod args;
mod state;
mod ui;

fn main() {
    let args = Args::parse();

    match args.command {
        args::Command::Local { work, short, long } => run_local_session(work, short, long),
        _ => todo!(),
    }
}

fn run_local_session(work: SessionDuration, short: SessionDuration, long: SessionDuration) {
    let pomodoro_settings = Settings {
        focus_duration: work,
        short_break_duration: short,
        long_break_duration: long,
        start_automatically: false, // TODO: set to true later, this is for debugging purposes
    };
    let pomodoro_state = PomodoroState::new(pomodoro_settings);

    let timer_update_interval = Duration::from_millis(100);

    let (events_tx, events_rx) = mpsc::channel::<Event>();
    let (ui_tx, ui_rx) = mpsc::channel::<UiUpdate>();
    let ui_txs = (ui_tx.clone(), ui_tx.clone());

    let mut thread_handles = Vec::new();

    thread_handles.push(thread::spawn(move || ui::ui_thread(ui_txs.0, ui_rx, events_tx)));
    //thread_handles.push(thread::spawn(move || event_handler_thread(control_tx)));
    thread_handles.push(thread::spawn(move || {
        state_transformer_thread(pomodoro_state, timer_update_interval, events_rx, ui_txs.1)
    }));

    for handle in thread_handles {
        handle.join().expect("failed to join thread"); // TODO: handle error
    }
}

pub enum ThreadError {
    Io(io::Error),
    Recv(mpsc::RecvError),
    SendUi(mpsc::SendError<UiUpdate>),
    SendEvent(mpsc::SendError<crossterm::event::Event>),
}

impl From<io::Error> for ThreadError {
    fn from(error: io::Error) -> Self {
        ThreadError::Io(error)
    }
}

impl From<mpsc::RecvError> for ThreadError {
    fn from(error: mpsc::RecvError) -> Self {
        ThreadError::Recv(error)
    }
}

impl From<mpsc::SendError<UiUpdate>> for ThreadError {
    fn from(error: mpsc::SendError<UiUpdate>) -> Self {
        ThreadError::SendUi(error)
    }
}

impl From<mpsc::SendError<crossterm::event::Event>> for ThreadError {
    fn from(error: mpsc::SendError<crossterm::event::Event>) -> Self {
        ThreadError::SendEvent(error)
    }
}

impl Display for ThreadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThreadError::Io(error) => write!(f, "I/O error: {error}"),
            ThreadError::Recv(error) => write!(f, "channel receive error: {error}"),
            ThreadError::SendUi(error) => write!(f, "ui channel send error: {error}"),
            ThreadError::SendEvent(error) => write!(f, "event channel send error: {error}"),
        }
    }
}

fn state_transformer_thread(
    mut state: PomodoroState,
    timer_update_interval: Duration,
    events_rx: mpsc::Receiver<Event>,
    ui_tx: mpsc::Sender<UiUpdate>,
) -> Result<(), ThreadError> {
    loop {
        let now = Instant::now();

        ui_tx.send(UiUpdate::StateUpdate(state.clone()))?;

        // TODO: handle hangup
        events_rx
            .try_iter()
            .for_each(|event| state.handle_event(event));

        if state.timer_is_stopped {
            let event = events_rx.recv()?;
            state.handle_event(event);
            // TODO: handle hangup
            events_rx
                .try_iter()
                .for_each(|event| state.handle_event(event));

            continue;
        }

        let elapsed = now.elapsed();
        if elapsed < timer_update_interval {
            thread::sleep(timer_update_interval - elapsed);
        }

        state.increase_progress(&now.elapsed());
    }
}
