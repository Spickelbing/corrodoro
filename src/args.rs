use crate::pomodoro::SessionDuration;
pub use clap::Parser;
use clap::Subcommand;
use rand::{seq::IteratorRandom, thread_rng};
use std::net::{SocketAddr, ToSocketAddrs};
use std::ops::Deref;
use std::str::FromStr;
use std::time::Duration;
use thiserror::Error;
use url::Host;

#[derive(Parser)]
#[command(version)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Start an offline session
    Offline {
        /// Duration of a work session
        #[arg(short, long, default_value_t = SessionDuration(Duration::from_secs(25 * 60)))]
        work: SessionDuration,

        /// Duration of a short break
        #[arg(short, long, default_value_t = SessionDuration(Duration::from_secs(5 * 60)))]
        short: SessionDuration,

        /// Duration of a long break
        #[arg(short, long, default_value_t = SessionDuration(Duration::from_secs(20 * 60)))]
        long: SessionDuration,
    },

    /// Connect to a session
    Connect {
        /// Address of the server to connect to
        #[arg(id = "HOSTNAME:PORT")]
        server_address: ServerAddress,
    },

    /// Host a session
    Host {
        /// Port to listen on
        port: u16,

        /// Duration of a work session
        #[arg(short, long, default_value_t = SessionDuration(Duration::from_secs(25 * 60)))]
        work: SessionDuration,

        /// Duration of a short break
        #[arg(short, long, default_value_t = SessionDuration(Duration::from_secs(5 * 60)))]
        short: SessionDuration,

        /// Duration of a long break
        #[arg(short, long, default_value_t = SessionDuration(Duration::from_secs(20 * 60)))]
        long: SessionDuration,
    },
}

#[derive(Clone)]
pub struct ServerAddress(SocketAddr);

impl Deref for ServerAddress {
    type Target = SocketAddr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Error)]
pub enum ServerAddressConversionError {
    #[error("invalid hostname: {0}")]
    InvalidHost(#[from] url::ParseError),
    #[error("missing port")]
    MissingPort,
    #[error("failed to resolve hostname: {0}")]
    CannotResolveHost(#[from] std::io::Error),
    #[error("invalid port: {0}")]
    InvalidPort(#[from] std::num::ParseIntError),
    #[error("failed to resolve hostname")]
    HostHasNoDnsRecords,
}

impl FromStr for ServerAddress {
    type Err = ServerAddressConversionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v: Vec<&str> = s.split(':').collect();
        let host = Host::parse(v[0])?;

        match v.len() {
            2 => {
                let port = v[1].parse::<u16>()?;
                let socket_addr_string = format!("{host}:{port}");
                let addr = socket_addr_string
                    .to_socket_addrs()?
                    .into_iter()
                    .choose(&mut thread_rng())
                    .ok_or(ServerAddressConversionError::HostHasNoDnsRecords)?;
                let addr = SocketAddr::new(addr.ip(), port);

                Ok(ServerAddress(addr))
            }
            _ => Err(ServerAddressConversionError::MissingPort),
        }
    }
}
