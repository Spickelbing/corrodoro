use crate::pomodoro::SessionDuration;
pub use clap::Parser;
use clap::Subcommand;
use std::net::SocketAddr;
use std::time::Duration;

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
        #[arg(id = "ADDRESS:PORT")]
        server_address: SocketAddr
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
