use crate::pomodoro::SessionDuration;
pub use clap::Parser;
use clap::{Subcommand, ValueEnum};
use std::fmt::{Display, Formatter};
use std::net::{SocketAddr, ToSocketAddrs};
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

        #[arg(short, long, default_value_t = IpVersion::V4)]
        ip_version: IpVersion,
    },

    /// Host a session
    Host {
        /// Port to listen on
        port: u16,

        #[arg(short, long, default_value_t = IpVersion::V4)]
        ip_version: IpVersion,

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

#[derive(Clone, ValueEnum)]
pub enum IpVersion {
    /// IPv4
    V4,
    /// IPv6
    V6,
}

impl IpVersion {
    pub fn is_v4(&self) -> bool {
        matches!(self, IpVersion::V4)
    }
}

impl Display for IpVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            IpVersion::V4 => write!(f, "v4"),
            IpVersion::V6 => write!(f, "v6"),
        }
    }
}

#[derive(Clone)]
pub struct ServerAddress(Vec<SocketAddr>);

impl ServerAddress {
    pub fn resolved(self) -> Vec<SocketAddr> {
        self.0
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
                let addr = socket_addr_string.to_socket_addrs()?.collect();

                Ok(ServerAddress(addr))
            }
            _ => Err(ServerAddressConversionError::MissingPort),
        }
    }
}
