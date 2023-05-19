use std::fmt::Display;

/// Message that is sent by the server to the client. It is used to send notifications and private messages to the client.
#[derive(Debug)]
pub enum MessageResponse {
    UserPrivMsg {
        sender: String,
        message: String,
    },
    ChannelPrivMsg {
        channel: String,
        sender: String,
        message: String,
    },
    KickMsg {
        message: String,
    },
    InviteMsg {
        message: String,
    },
}

impl Display for MessageResponse {
    /// Formats the message response into string.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let r = match self {
            MessageResponse::UserPrivMsg { sender, message } => {
                format!("002 {} {}", sender, message)
            }
            MessageResponse::ChannelPrivMsg {
                channel,
                sender,
                message,
            } => {
                format!("003 {} {} {}", channel, sender, message)
            }
            MessageResponse::KickMsg { message } => {
                format!("004 {}", message)
            }
            MessageResponse::InviteMsg { message } => {
                format!("005 {}", message)
            }
        };
        write!(f, "{}", r)
    }
}

impl MessageResponse {
    /// Creates a new instance of the message response enum, it will parse the string and return the correct enum variant.
    /// It is the opposite of the `Display` trait.
    /// It will return none if the string is not a valid message response.
    pub fn serialize(response: String) -> Option<MessageResponse> {
        let msg = response
            .split_whitespace()
            .map(|x| x.to_string())
            .collect::<Vec<_>>();
        let command = msg[0].as_str();
        match command {
            "002" => Some(MessageResponse::UserPrivMsg {
                sender: msg[1].clone(),
                message: msg[2..].to_owned().join(" "),
            }),
            "003" => {
                match msg[1].strip_prefix(':') {
                    Some(p) => p.to_owned(),
                    None => "".to_owned(),
                };
                Some(MessageResponse::ChannelPrivMsg {
                    channel: msg[1].clone(),
                    sender: msg[2].clone(),
                    message: msg[3..].to_owned().join(" "),
                })
            }
            "004" => Some(MessageResponse::KickMsg {
                message: msg[1..].to_owned().join(" "),
            }),
            "005" => Some(MessageResponse::InviteMsg {
                message: msg[1..].to_owned().join(" "),
            }),
            _ => None,
        }
    }
}
