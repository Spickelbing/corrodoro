pub use clap::Parser;
use clap::Subcommand;
use std::error::Error;
use std::fmt::Display;
use std::fmt::Formatter;
use std::ops::Deref;
use std::ops::DerefMut;
use std::str::FromStr;
use std::time::Duration;

#[derive(Parser)]
#[command(version)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Start a local session without networking
    Local {
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

    /// Attach to a session
    Client,

    /// Host a session
    Server,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct SessionDuration(pub Duration);

impl Deref for SessionDuration {
    type Target = Duration;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SessionDuration {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Duration> for SessionDuration {
    fn from(duration: Duration) -> Self {
        SessionDuration(duration)
    }
}

#[derive(Debug)]
pub enum ParseSessionDurationError {
    InvalidFormat,
    TooManySeconds,
    NotTwoDigitsForSeconds,
    ParseIntError(std::num::ParseIntError),
}

impl Display for ParseSessionDurationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseSessionDurationError::InvalidFormat => {
                write!(f, "expected \"minutes\" or \"minutes:seconds\"")
            }
            ParseSessionDurationError::TooManySeconds => {
                write!(f, "seconds must be less than 60")
            }
            ParseSessionDurationError::ParseIntError(e) => {
                write!(f, "failed to parse integer: {e}")
            }
            ParseSessionDurationError::NotTwoDigitsForSeconds => {
                write!(f, "seconds must be two digits")
            }
        }
    }
}

impl Error for ParseSessionDurationError {}

impl FromStr for SessionDuration {
    type Err = ParseSessionDurationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v: Vec<&str> = s.split(':').collect();
        let minutes = v[0]
            .parse::<u64>()
            .map_err(ParseSessionDurationError::ParseIntError)?;

        match v.len() {
            1 => Ok(SessionDuration(Duration::from_secs(minutes * 60))),
            2 => {
                let seconds = v[1]
                    .parse::<u64>()
                    .map_err(ParseSessionDurationError::ParseIntError)?;
                if seconds > 59 {
                    return Err(ParseSessionDurationError::TooManySeconds);
                } else if v[1].len() != 2 {
                    return Err(ParseSessionDurationError::NotTwoDigitsForSeconds);
                }

                Ok(SessionDuration(Duration::from_secs(minutes * 60 + seconds)))
            }
            _ => Err(ParseSessionDurationError::InvalidFormat),
        }
    }
}

impl Display for SessionDuration {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let minutes = self.0.as_secs() / 60;
        let seconds = self.0.as_secs() % 60;
        write!(f, "{minutes}:{seconds:02}")
    }
}
