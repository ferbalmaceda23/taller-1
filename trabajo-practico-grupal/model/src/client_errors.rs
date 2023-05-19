use std::{
    io::{Error, ErrorKind},
    sync::PoisonError,
};

use crate::{dcc::DccMessageError, message::MessageError};

/// A custom error type for the client.
/// * InvalidPort -> The port is not a valid number
/// * InvalidArgs -> The arguments are not valid
/// * ErrorWhileConnecting -> The client could not connect to the server
/// * ErrorWhileConnectingWithInterface -> The client could send message to the interface
/// * CannotPrivmsgToSelf -> The client tried to send a message to itself
/// * CannotPrivmsgToServer -> The client tried to send a message to the server
/// * CannotWriteSocket -> The client could not write to the socket
/// * ConnectionFinished -> The connection with the server finished

#[derive(Debug)]
pub enum ClientError {
    InvalidPort,
    InvalidArgs,
    ErrorWhileConnecting,
    ErrorWhileConnectingWithInterface,
    Other,
    CannotPrivmsgToSelf,
    CannotWriteSocket,
    ConnectionFinished,
    MessageError,
    LockError,
    EmptyCommand,
    EmptyMessage,
    InvalidCommand,
    SocketError,
    FileError,
    GuiCommunicationError,
    NoOngoingTransfer,
    OngoingTransfer,
}

impl From<Error> for ClientError {
    fn from(e: Error) -> Self {
        match e.kind() {
            ErrorKind::ConnectionRefused => ClientError::InvalidPort,
            _ => ClientError::Other,
        }
    }
}

impl From<MessageError> for ClientError {
    fn from(error: MessageError) -> Self {
        match error {
            MessageError::EmptyCommand => ClientError::EmptyCommand,
            MessageError::EmptyMessage => ClientError::EmptyMessage,
            MessageError::InvalidCommand => ClientError::InvalidCommand,
        }
    }
}

impl From<DccMessageError> for ClientError {
    fn from(error: DccMessageError) -> Self {
        match error {
            DccMessageError::InvalidCommand => ClientError::InvalidCommand,
            DccMessageError::CannotParsePrefix => ClientError::MessageError,
            DccMessageError::NoParameters => ClientError::EmptyMessage,
            DccMessageError::InvalidMessage => ClientError::MessageError,
        }
    }
}

impl<T> From<PoisonError<T>> for ClientError {
    fn from(_: PoisonError<T>) -> Self {
        ClientError::LockError
    }
}
