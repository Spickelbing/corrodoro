use state::Settings;

use crate::args::{Args, Parser, SessionDuration};
use crate::state::{Event, PomodoroState};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use std::vec::Vec;

mod args;
mod state;

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
        start_automatically: false, // TODO: set to true later, this is for debug purposes
    };
    let pomodoro_state = PomodoroState::new(pomodoro_settings);

    let timer_update_interval = Duration::from_millis(100);

    let (events_tx, events_rx) = mpsc::channel::<Event>();
    let (tui_tx, tui_rx) = mpsc::channel::<PomodoroState>();

    let mut thread_handles = Vec::new();

    thread_handles.push(thread::spawn(move || tui_thread(tui_rx)));
    //thread_handles.push(thread::spawn(move || event_handler_thread(control_tx)));
    thread_handles.push(thread::spawn(move || {
        state_transformer_thread(pomodoro_state, timer_update_interval, events_rx, tui_tx)
    }));

    for handle in thread_handles {
        handle.join().expect("failed to join thread"); // TODO: handle error
    }
}

// TODO: this is a mock-up, replace with a real TUI
fn tui_thread(tui_rx: mpsc::Receiver<PomodoroState>) {
    loop {
        let state = tui_rx
            .recv()
            .expect("channel to the state transformer thread was closed");
        println!("{state}");
    }
}

fn event_handler_thread(events_tx: mpsc::Sender<Event>) {
    todo!()
}

fn state_transformer_thread(
    mut state: PomodoroState,
    timer_update_interval: Duration,
    events_rx: mpsc::Receiver<Event>,
    tui_tx: mpsc::Sender<PomodoroState>,
) {
    loop {
        let now = Instant::now();

        // TODO: handle error
        tui_tx
            .send(state.clone())
            .expect("failed to send state to tui thread");

        // TODO: handle hangup
        events_rx
            .try_iter()
            .for_each(|event| state.handle_event(event));

        if state.timer_is_stopped {
            // TODO: handle hangup
            if let Ok(event) = events_rx.recv() {
                state.handle_event(event);
                // TODO: handle hangup
                events_rx
                    .try_iter()
                    .for_each(|event| state.handle_event(event));
            }
            continue;
        }

        let elapsed = now.elapsed();
        if elapsed < timer_update_interval {
            thread::sleep(timer_update_interval - elapsed);
        }

        state.increase_progress(&now.elapsed());
    }
}
