use crate::net::bind_stream;
use crate::net::FramedStream;
use crate::net::NetworkError;
use bytes::Bytes;
use futures::{future::select_all, SinkExt, StreamExt};
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::select;
use tokio::sync::mpsc;

/// A server that works with framed streams.
/// It listens for connection attempts, manages a set of connected clients and provides methods for communicating with them.
pub struct Server {
    pub local_addr: SocketAddr,
    connections_rx: mpsc::Receiver<(SocketAddr, FramedStream)>,
    connected_clients: HashMap<SocketAddr, FramedStream>,
}

impl Server {
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

    fn accept_queued(&mut self) {
        while let Ok((addr, stream)) = self.connections_rx.try_recv() {
            self.connected_clients.insert(addr, stream);
        }
    }

    async fn accept_one(&mut self) {
        if let Some((addr, stream)) = self.connections_rx.recv().await {
            self.connected_clients.insert(addr, stream);
        }
    }

    /// Disconnects the clients for whom there are networking errors.
    pub async fn broadcast_frame(&mut self, frame: Bytes) -> Result<(), NetworkError> {
        self.accept_queued();
        let mut errors = Vec::new();

        for (&addr, stream) in &mut self.connected_clients {
            let result = stream.send(frame.clone()).await;

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

    // TODO: Make this function disconnect clients for whom there are networking errors.
    // (This requires keeping track of the addresses of clients in the order in which they are given to `select_all`.)
    pub async fn recv_frame(&mut self) -> Result<Bytes, NetworkError> {
        if self.connected_clients.is_empty() {
            self.accept_one().await;
        }

        loop {
            let clients = &mut self.connected_clients;
            let client_frame_futures = clients.values_mut().map(|v| v.next());

            select! {
                next_frame = select_all(client_frame_futures) => {
                    if let (Some(msg), ..) = next_frame {
                        let msg = msg.map_err(NetworkError::ReadFrame)?;
                        return Ok(msg.into());
                    } else {
                        // ... Can this happen at all?
                    }
                }
                new_client = self.connections_rx.recv() => {
                    if let Some((addr, stream)) = new_client {
                        self.connected_clients.insert(addr, stream);
                    }
                }
            }
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

async fn try_accept_connection(
    listener: &mut TcpListener,
) -> Result<(SocketAddr, FramedStream), NetworkError> {
    let (stream, addr) = listener.accept().await.map_err(NetworkError::Accept)?;
    let stream = bind_stream(stream);
    Ok((addr, stream))
}

// TODO: stop listening when the channel is closed (select on the channel and the listener)
async fn listen(mut listener: TcpListener, tx: mpsc::Sender<(SocketAddr, FramedStream)>) {
    loop {
        if let Ok((addr, stream)) = try_accept_connection(&mut listener).await {
            if tx.send((addr, stream)).await.is_err() {
                break; // channel closed, stop listening
            }
        } else {
            // connection attempt failed, ignore for now
        }
    }
}
