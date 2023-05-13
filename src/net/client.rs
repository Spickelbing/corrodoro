use crate::net::NetworkError;
use crate::net::{bind_stream, FramedStream};
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use std::net::SocketAddr;
use tokio::net::TcpStream;

/// A client that works with framed streams.
/// Manages a connection to a remote server and provides methods to communicate with it.
pub struct Client {
    stream: FramedStream,
    pub remote_addr: SocketAddr,
}

impl Client {
    pub async fn connect(socket: SocketAddr) -> Result<Self, NetworkError> {
        let stream = TcpStream::connect(socket)
            .await
            .map_err(|e| NetworkError::Connect(socket, e))?;
        let framed = bind_stream(stream);
        Ok(Self {
            stream: framed,
            remote_addr: socket,
        })
    }

    pub async fn recv_frame(&mut self) -> Result<Bytes, NetworkError> {
        let frame = self
            .stream
            .next()
            .await
            .ok_or(NetworkError::ServerDisconnect)?
            .map_err(NetworkError::ReadFrame)?;
        Ok(frame.into())
    }

    pub async fn send_frame(&mut self, frame: Bytes) -> Result<(), NetworkError> {
        self.stream
            .send(frame)
            .await
            .map_err(NetworkError::SendFrame)?;
        Ok(())
    }
}
