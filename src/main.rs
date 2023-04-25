use state::Settings;

use crate::args::{Args, Parser, SessionDuration};
use crate::state::State;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
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
    let settings = Settings {
        focus_duration: work,
        short_break_duration: short,
        long_break_duration: long,
        start_automatically: true,
    };
    let shared_state = Arc::new(Mutex::new(State::new(settings)));
    let shared_state = (
        Arc::clone(&shared_state),
        Arc::clone(&shared_state),
        Arc::clone(&shared_state),
    );
    let control_channel = mpsc::channel::<bool>();
    let mut thread_handles = Vec::new();

    // TODO: start tui thread
    thread_handles.push(thread::spawn(move || {
        let state = shared_state.0;

        loop {
            println!("{}", &state.lock().unwrap());
            thread::sleep(Duration::from_millis(100));
        }
    }));

    // TODO: start event handler thread

    // TODO: start state transformer (mostly timer) thread
    thread_handles.push(thread::spawn(move || {
        let state = shared_state.2;
        let rx = control_channel.1;
        let timer_update_interval = Duration::from_millis(100);

        loop {
            let mut now = Instant::now();

            // TODO: handle error
            let mut is_stopped = state.lock().unwrap().is_stopped;
            while is_stopped {
                // TODO: handle error
                is_stopped = rx
                    .recv()
                    .expect("channel to the event handler thread was closed");

                if is_stopped == false {
                    state.lock().unwrap().is_stopped = is_stopped;
                    now = Instant::now();
                }
            }

            if !is_stopped {
                let should_stop = rx.try_iter().any(|e| e);
                if should_stop {
                    // TODO: handle error
                    state.lock().unwrap().is_stopped = should_stop;
                    continue;
                }
            }

            let elapsed = now.elapsed();
            if elapsed < timer_update_interval {
                thread::sleep(timer_update_interval - elapsed);
            }

            // TODO: handle error
            state.lock().unwrap().increase_progress(&now.elapsed());
        }
    }));

    for handle in thread_handles {
        handle.join().expect("failed to join thread"); // TODO: handle error
    }
}
