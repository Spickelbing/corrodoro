use crate::app::{Event, ForeverPending, UiData};
use futures::{future::select_all, SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use thiserror::Error;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

#[derive(Serialize, Deserialize)]
pub enum Message {
    UiData(UiData),
    Event(Event),
}

type FramedStream = Framed<TcpStream, LengthDelimitedCodec>;

fn bind_stream(stream: TcpStream) -> FramedStream {
    Framed::new(stream, LengthDelimitedCodec::new())
}

pub struct RemoteServer {
    stream: FramedStream,
    pub remote_addr: SocketAddr,
}

impl RemoteServer {
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

    pub async fn next_message(&mut self) -> Result<Message, NetworkError> {
        let msg = self
            .stream
            .next()
            .await
            .ok_or(NetworkError::ServerDisconnect)?
            .map_err(|e| NetworkError::Read(self.remote_addr, e))?;

        let msg: Message = bincode::deserialize(&msg)
            .map_err(|e| NetworkError::Deserialize(self.remote_addr, e))?;

        Ok(msg)
    }

    pub async fn send_message(&mut self, msg: Message) -> Result<(), NetworkError> {
        let msg = bincode::serialize(&msg).map_err(NetworkError::Serialize)?;

        self.stream
            .send(msg.into())
            .await
            .map_err(|e| NetworkError::Send(self.remote_addr, e))?;

        Ok(())
    }
}

pub struct Server {
    pub local_addr: SocketAddr,
    connections_rx: mpsc::Receiver<(SocketAddr, FramedStream)>,
    connected_clients: HashMap<SocketAddr, FramedStream>,
}

async fn try_accept_connection(
    listener: &mut TcpListener,
) -> Result<(SocketAddr, FramedStream), NetworkError> {
    let (stream, addr) = listener.accept().await.map_err(NetworkError::Accept)?;
    let stream = bind_stream(stream);
    Ok((addr, stream))
}

async fn listen(mut listener: TcpListener, tx: mpsc::Sender<(SocketAddr, FramedStream)>) {
    loop {
        if let Ok((addr, stream)) = try_accept_connection(&mut listener).await {
            if tx.send((addr, stream)).await.is_err() {
                break; // channel closed
            }
        } else {
            // connection attempt failed, ignore for now
        }
    }
}

impl Server {
    /// Starts to listen for connections in a separate task.
    /// Connections are queued and have to be put into effect by calling `accept_pending_connections`.
    pub async fn bind(socket: SocketAddr) -> Result<Self, NetworkError> {
        let listener = TcpListener::bind(socket)
            .await
            .map_err(|e| NetworkError::Bind(socket, e))?;
        let local_addr = listener
            .local_addr()
            .map_err(|e| NetworkError::Bind(socket, e))?;

        let (tx, rx) = mpsc::channel(32);

        tokio::spawn(listen(listener, tx));

        Ok(Self {
            local_addr,
            connections_rx: rx,
            connected_clients: HashMap::new(),
        })
    }

    pub async fn accept_pending_connections(&mut self) {
        while let Ok((addr, stream)) = self.connections_rx.try_recv() {
            self.connected_clients.insert(addr, stream);
        }
    }

    /// Disconnects the clients for whom there are networking errors.
    pub async fn broadcast_message(&mut self, msg: Message) -> Result<(), NetworkError> {
        let serialized = bincode::serialize(&msg)?;
        let mut errors = Vec::new();

        for (&addr, stream) in &mut self.connected_clients {
            let result = stream.send(serialized.clone().into()).await;

            if let Err(err) = result {
                errors.push((addr, err));
            }
        }

        for (addr, _) in &errors {
            self.disconnect_client(*addr);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(NetworkError::Broadcast(errors))
        }
    }

    /// Pends forever if there are no connected clients. Does not wake up when a new client connects.
    // TODO: Make this function disconnect clients for whom there are networking errors.
    // (This requires keeping track of the addresses of clients in the order in which they are given to `select_all`.)
    pub async fn next_message(&mut self) -> Result<Message, NetworkError> {
        if self.connected_clients.is_empty() {
            ForeverPending.await.forever()
        }

        let futures = self.connected_clients.values_mut().map(|v| v.next());
        let next = select_all(futures).await;

        if let (Some(msg), ..) = next {
            let msg = msg.map_err(NetworkError::ReadUnknownSender)?;
            let msg: Message =
                bincode::deserialize(&msg).map_err(NetworkError::DeserializeUnknownSender)?;

            Ok(msg)
        } else {
            ForeverPending.await.forever()
        }
    }

    fn disconnect_client(&mut self, addr: SocketAddr) {
        self.connected_clients.remove(&addr);
    }

    pub fn disconnect_all(&mut self) {
        self.connected_clients.clear();
    }

    pub fn clients(&self) -> Vec<SocketAddr> {
        self.connected_clients.keys().copied().collect()
    }
}

#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("error while receiving message from {0}: {1}")]
    Read(SocketAddr, io::Error),
    #[error("error while receiving message: {0}")]
    ReadUnknownSender(io::Error),
    #[error("error while sending message to {0}: {1}")]
    Send(SocketAddr, io::Error),
    #[error("failed to serialize message: {0}")]
    Serialize(#[from] bincode::Error),
    #[error("failed to deserialize message from {0}: {1}")]
    Deserialize(SocketAddr, bincode::Error),
    #[error("failed to deserialize message: {0}")]
    DeserializeUnknownSender(bincode::Error),
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
}
