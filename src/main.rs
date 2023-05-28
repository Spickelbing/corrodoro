use crate::app::{App, ClientApp, UnrecoverableError};
use crate::args::{Args, IpVersion, Parser};
use rand::{seq::IteratorRandom, thread_rng};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::process::ExitCode;

mod app;
mod args;
mod net;
mod notification;
mod pomodoro;
mod tui;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let args = Args::parse();

    let result = match args.command {
        args::Command::Offline { focus, short, long } => run_offline(focus, short, long).await,
        args::Command::Connect {
            server_address,
            ip_version,
        } => run_client(server_address.resolved(), ip_version).await,
        args::Command::Host {
            port,
            ip_version,
            focus,
            short,
            long,
        } => run_server(port, ip_version, focus, short, long).await,
    };

    if let Err(err) = result {
        eprintln!("{err}");
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

async fn run_offline(
    work: pomodoro::SessionDuration,
    short: pomodoro::SessionDuration,
    long: pomodoro::SessionDuration,
) -> Result<(), UnrecoverableError> {
    let settings = pomodoro::Settings::new(work, short, long, false);
    let state = pomodoro::State::new(settings);
    let mut app = App::new(state)?;

    app.run().await?;

    Ok(())
}

async fn run_client(
    server_addresses: Vec<SocketAddr>,
    ip_version: IpVersion,
) -> Result<(), UnrecoverableError> {
    let version_filter = match ip_version {
        IpVersion::V4 => |addr: &&SocketAddr| addr.is_ipv4(),
        IpVersion::V6 => |addr: &&SocketAddr| addr.is_ipv6(),
    };

    let server_address = match server_addresses
        .iter()
        .filter(version_filter)
        .choose(&mut thread_rng())
    {
        Some(random_addr) => *random_addr,
        None => match server_addresses.len() {
            0 => return Err(UnrecoverableError::HostHasNoDnsRecords),
            _ => {
                return Err(if ip_version.is_v4() {
                    UnrecoverableError::HostHasOnlyIpv6Records
                } else {
                    UnrecoverableError::HostHasOnlyIpv4Records
                })
            }
        },
    };

    let mut app = ClientApp::connect(server_address).await?;

    app.run().await?;

    Ok(())
}

async fn run_server(
    port: u16,
    ip_version: IpVersion,
    work: pomodoro::SessionDuration,
    short: pomodoro::SessionDuration,
    long: pomodoro::SessionDuration,
) -> Result<(), UnrecoverableError> {
    let settings = pomodoro::Settings::new(work, short, long, false);
    let socket = SocketAddr::new(
        match ip_version {
            IpVersion::V4 => Ipv4Addr::UNSPECIFIED.into(),
            IpVersion::V6 => Ipv6Addr::UNSPECIFIED.into(),
        },
        port,
    );
    let state = pomodoro::State::new(settings);
    let mut app = App::new(state)?;

    app.start_server(socket).await?;
    app.run().await?;
    let _ = app.stop_server().await;

    Ok(())
}
