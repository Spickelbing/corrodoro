use crate::app::{Event, UiData, ForeverPending};
use futures::{future::select_all, SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use thiserror::Error;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
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

// There are two client maps so that one can be used to accept connections and the other to receive messages,
// without having to lock the same list for both operations.
// I use interior mutability so that the accepting and receiving methods can be non-mut,
// and therefore can be used in the same `select!` block.
// Clients will will be moved from `accepted_clients` to `connected_clients` when the next message is broadcast.
// Livelocks are impossible because the broadcast method is mut, and therefore cannot be awaited concurrently
// with the accepting and receiving methods.
pub struct Server {
    listener: TcpListener,
    pub local_addr: SocketAddr,
    connected_clients: Mutex<HashMap<SocketAddr, FramedStream>>,
    accepted_clients: Mutex<HashMap<SocketAddr, FramedStream>>,
    pub clients: Vec<SocketAddr>, // always a copy of connected_clients.keys()
}

impl Server {
    pub async fn bind(socket: SocketAddr) -> Result<Self, NetworkError> {
        let listener = TcpListener::bind(socket)
            .await
            .map_err(|e| NetworkError::Bind(socket, e))?;
        let local_addr = listener
            .local_addr()
            .map_err(|e| NetworkError::Bind(socket, e))?;

        Ok(Self {
            listener,
            local_addr,
            connected_clients: Mutex::new(HashMap::new()),
            accepted_clients: Mutex::new(HashMap::new()),
            clients: Vec::new(),
        })
    }

    // This is the only method that locks `accepted_clients`.
    pub async fn accept_connection(&self) -> Result<(), NetworkError> {
        let (stream, addr) = self.listener.accept().await.map_err(NetworkError::Accept)?;
        let stream = bind_stream(stream);
        self.accepted_clients.lock().await.insert(addr, stream);
        Ok(())
    }

    /// Disconnects the clients for whom there are networking errors.
    pub async fn broadcast_message(&mut self, msg: Message) -> Result<(), NetworkError> {
        self.merge_client_collections();

        let serialized = bincode::serialize(&msg)?;
        let mut errors = Vec::new();

        for (&addr, stream) in self.connected_clients.get_mut() {
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
    // This and `connected_clients()` are the only methods that lock `connected_clients`.
    pub async fn next_message(&self) -> Result<Message, NetworkError> {
        let mut connected_clients = self.connected_clients.lock().await;
        if connected_clients.is_empty() {
            ForeverPending.await.forever()
        }

        let futures = connected_clients.values_mut().map(|v| v.next());
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
        self.connected_clients.get_mut().remove(&addr);
    }

    pub fn disconnect_all(&mut self) {
        self.connected_clients.get_mut().clear();
    }

    pub fn clients(&self) -> Vec<SocketAddr> {
        self.clients.clone()
    }

    fn merge_client_collections(&mut self) {
        let accepted = self.accepted_clients.get_mut();
        self.connected_clients.get_mut().extend(accepted.drain());
        self.clients = self.connected_clients.get_mut().keys().copied().collect();
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
