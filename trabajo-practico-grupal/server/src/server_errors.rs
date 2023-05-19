use std::{
    io::{Error, ErrorKind},
    sync::{
        mpsc::{RecvError, SendError},
        PoisonError,
    },
};

use model::{dcc::DccMessageError, message::MessageError};

#[derive(Debug, PartialEq, Eq)]
pub enum ServerError {
    EmptyCommand,
    EmptyMessage,
    InvalidCommand,
    InvalidArgs,
    InvalidMessage,
    InvalidParameters,
    InvalidPort,
    ClientAlreadyRegistered,
    ClientMustSetPassword,
    ClientMustRegisterOrAuthenticate,
    ClientCannotPrivmsgToSelf,
    ClientNotFound,
    ReceiverNotFound,
    PoisonedLock,
    PassMustBeSetBeforeUser,
    PassMustBeSetBeforeNickname,
    ChannelNotFound,
    UserAlreadyInChannel,
    UserNotOperator,
    UserNotInChannel,
    ChannelMustStartWithHashOrAmpersand,
    ClientConnected,
    ClientNotConnected,
    LockError,
    CannotWriteSocket,
    CannotReadFromSocket,
    ChannelIsInviteOnly,
    ChannelIsSecret,
    MustInsertPassword,
    CannotRemoveLastOperator,
    InvalidFlags,
    IncorrectPassword,
    UserIsBanned,
    InvalidCredentials,
    CannotPersistClient,
    CannotLoadClients,
    CannotPersistChannel,
    CannotLoadChannels,
    ClientNotOnChannel,
    CannotReadFromFile,
    ChannelIsFull,
    ChannelIsModerated,
    TopicOnlySetableByOperators,
    CannotSendToChannel,
    CannotReceiveFromChannel,
    CannotChangeModesFromOtherUsers,
    ServerAlreadyRegistered,
    ServerNotFound,
    NicknameInUse(String),
    InvalidPassword,
    NotOnChannel,
    ErroneusNickname,
    Other,
}

impl From<MessageError> for ServerError {
    fn from(error: MessageError) -> Self {
        match error {
            MessageError::EmptyCommand => ServerError::EmptyCommand,
            MessageError::EmptyMessage => ServerError::EmptyMessage,
            MessageError::InvalidCommand => ServerError::InvalidCommand,
        }
    }
}

impl From<DccMessageError> for ServerError {
    fn from(error: DccMessageError) -> Self {
        match error {
            DccMessageError::InvalidCommand => ServerError::InvalidCommand,
            DccMessageError::CannotParsePrefix => ServerError::InvalidMessage,
            DccMessageError::NoParameters => ServerError::InvalidParameters,
            DccMessageError::InvalidMessage => ServerError::InvalidMessage,
        }
    }
}

impl From<Error> for ServerError {
    fn from(e: Error) -> Self {
        match e.kind() {
            ErrorKind::InvalidInput => ServerError::InvalidPort,
            _ => ServerError::Other,
        }
    }
}

impl<T> From<SendError<T>> for ServerError {
    fn from(_: SendError<T>) -> Self {
        ServerError::CannotSendToChannel
    }
}

impl From<RecvError> for ServerError {
    fn from(_: RecvError) -> Self {
        ServerError::CannotReceiveFromChannel
    }
}

impl<T> From<PoisonError<T>> for ServerError {
    fn from(_: PoisonError<T>) -> Self {
        ServerError::LockError
    }
}
