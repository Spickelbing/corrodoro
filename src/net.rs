use std::io;
use std::net::SocketAddr;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

mod client;
mod protocol;
mod server;

pub use client::Client;
pub use protocol::{ClientMessage, ServerMessage};
pub use server::Server;

type FramedStream = Framed<TcpStream, LengthDelimitedCodec>;

fn bind_stream(stream: TcpStream) -> FramedStream {
    Framed::new(stream, LengthDelimitedCodec::new())
}

#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("error while receiving frame: {0}")]
    ReadFrame(io::Error),
    #[error("error while sending frame: {0}")]
    SendFrame(io::Error),
    #[error(
        "one or more clients could not receive broadcast message and were disconnected: {0:?}"
    )]
    Broadcast(Vec<(SocketAddr, io::Error)>),
    #[error("failed to accept connection attempt: {0}")]
    Accept(io::Error),
    #[error("failed to bind to socket {0}: {1}")]
    Bind(SocketAddr, io::Error),
    #[error("failed to connect to remote server {0}: {1}")]
    Connect(SocketAddr, io::Error),
    #[error("remote server disconnected")]
    ServerDisconnect,
    #[error("received unexpected message type")]
    UnexpectedMessage,
    #[error("failed to serialize message: {0}")]
    Serialize(bincode::Error),
    #[error("failed to deserialize message: {0}")]
    Deserialize(bincode::Error),
}
