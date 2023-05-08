use std::net::{Ipv4Addr, SocketAddr};

use crate::app::{App, ClientApp, UnrecoverableError};
use crate::args::{Args, Parser};
use std::process::ExitCode;

mod app;
mod args;
mod notification;
mod pomodoro;
mod tui;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let args = Args::parse();

    let result = match args.command {
        args::Command::Local { work, short, long } => run_local_session(work, short, long).await,
        args::Command::Client { server_address } => run_client_session(server_address).await,
        args::Command::Server { port } => run_server_session(port).await,
    };

    if let Err(err) = result {
        eprintln!("{err}");
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

async fn run_local_session(
    work: pomodoro::SessionDuration,
    short: pomodoro::SessionDuration,
    long: pomodoro::SessionDuration,
) -> Result<(), UnrecoverableError> {
    let state = pomodoro::State::new(pomodoro::Settings {
        focus_duration: work,
        short_break_duration: short,
        long_break_duration: long,
        start_automatically: false,
    });
    let mut app = App::new(state)?;

    app.run().await?;

    Ok(())
}

async fn run_client_session(
    server_address: std::net::SocketAddr,
) -> Result<(), UnrecoverableError> {
    let mut app = ClientApp::connect(server_address).await?;

    app.run().await?;

    Ok(())
}

async fn run_server_session(port: u16) -> Result<(), UnrecoverableError> {
    let state = pomodoro::State::new(pomodoro::Settings::default());
    let mut app = App::new(state)?;
    let socket = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port);
    app.start_server(socket).await?;
    app.run().await?;
    let _ = app.stop_server().await;

    Ok(())
}
