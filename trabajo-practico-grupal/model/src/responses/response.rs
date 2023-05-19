use std::fmt::Display;

use super::{
    dcc, errors, message,
    replies::{self, CommandResponse},
};

/// Enum that represents the different types of responses that the server can send.
/// # Fields
/// * `CommandResponse`: The response to a command sent by the client.
/// * `Message`: The message that was sent by a user.
/// * `ErrorResponse`: The error that was sent by the server.
/// * `MessageResponse`: PRIVMSG and notifications messages.
pub enum Response {
    CommandResponse { response: replies::CommandResponse },
    ErrorResponse { response: errors::ErrorResponse },
    MessageResponse { response: message::MessageResponse },
    DccResponse { response: dcc::DccResponse },
}

impl Display for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let r = match self {
            Response::CommandResponse { response } => format!("{}", response),
            Response::ErrorResponse { response } => format!("{}", response),
            Response::MessageResponse { response } => format!("{}", response),
            Response::DccResponse { response } => format!("{}", response),
        };
        write!(f, "{}", r)
    }
}

impl Response {
    /// Creates a new instance of the response enum, it will parse the string and return the correct enum variant.
    pub fn serialize(msg: String) -> Option<Response> {
        let response = CommandResponse::serialize(msg.clone());
        match response {
            Some(r) => Some(Response::CommandResponse { response: r }),
            None => match errors::ErrorResponse::serialize(msg.clone()) {
                Some(r) => Some(Response::ErrorResponse { response: r }),
                None => match dcc::DccResponse::serialize(msg.clone()) {
                    Some(r) => Some(Response::DccResponse { response: r }),
                    None => message::MessageResponse::serialize(msg)
                        .map(|r| Response::MessageResponse { response: r }),
                },
            },
        }
    }
}
