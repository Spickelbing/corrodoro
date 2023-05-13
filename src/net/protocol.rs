use crate::app::{Display, Event};
use crate::net::NetworkError;
use bytes::Bytes;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum ClientMessage {
    Event(Bytes),
}

#[derive(Serialize, Deserialize)]
pub enum ServerMessage {
    Display(Bytes),
}

impl ClientMessage {
    pub fn new(event: Event) -> Result<Self, NetworkError> {
        let serialized = bincode::serialize(&event).map_err(NetworkError::Serialize)?;
        Ok(Self::Event(serialized.into()))
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Event, NetworkError> {
        bincode::deserialize(bytes).map_err(NetworkError::Deserialize)
    }
}

impl From<ClientMessage> for Bytes {
    fn from(message: ClientMessage) -> Self {
        match message {
            ClientMessage::Event(bytes) => bytes,
        }
    }
}

impl ServerMessage {
    pub fn new(visuals: Display) -> Result<Self, NetworkError> {
        let serialized = bincode::serialize(&visuals).map_err(NetworkError::Serialize)?;
        Ok(Self::Display(serialized.into()))
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Display, NetworkError> {
        bincode::deserialize(bytes).map_err(NetworkError::Deserialize)
    }
}

impl From<ServerMessage> for Bytes {
    fn from(message: ServerMessage) -> Self {
        match message {
            ServerMessage::Display(bytes) => bytes,
        }
    }
}
