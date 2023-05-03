use crate::app::{App, AppError};
use crate::args::{Args, Parser};

mod app;
mod args;
mod event;
mod notification;
mod pomodoro;
mod tui;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), AppError> {
    let args = Args::parse();

    match args.command {
        args::Command::Local { work, short, long } => run_local_session(work, short, long).await,
        _ => todo!(),
    }
}

async fn run_local_session(
    work: pomodoro::SessionDuration,
    short: pomodoro::SessionDuration,
    long: pomodoro::SessionDuration,
) -> Result<(), AppError> {
    let mut app = App::new(pomodoro::State::new(pomodoro::Settings {
        focus_duration: work,
        short_break_duration: short,
        long_break_duration: long,
        start_automatically: false,
    }))?;

    app.run().await?;

    Ok(())
}
