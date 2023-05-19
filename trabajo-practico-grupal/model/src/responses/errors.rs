use std::fmt::Display;

/// Enum that represents the different types of errors that the server can send.
/// according to the irc protocol.
#[derive(Debug, Clone)]
pub enum ErrorResponse {
    NoSuchNick { nickname: String },
    NoSuchServer { servername: String },
    NoSuchChannel { channel: String },
    CannotSendToChannel { channel: String },
    TooManyChannels { channel: String },
    UnknownCommand { command: String },
    NickInUse { nickname: String },
    NoNicknameGiven,
    UserNotInChannel { nickname: String, channel: String },
    NotOnChannel { channel: String },
    UserOnChannel { nickname: String, channel: String },
    NoLogin { nickname: String },
    NeedMoreParams { command: String },
    AlreadyRegistered { nickname: String },
    PasswordMismatch,
    YouAreBanned,
    ChannelIsFull { channel: String },
    UnknownMode { character: char },
    InviteOnlyChannel { channel: String },
    BannedFromChannel { channel: String },
    BadChannelKey { channel: String },
    NoPrivileges,
    ChanOPrivsNeeded { channel: String },
    UnknownModeFlag,
    NotRegistered,
    ErrorWhileConnecting,
    UsersDontMatch,
    KeySet { channel: String },
    ClientDisconnected { nickname: String },
}

impl Display for ErrorResponse {
    /// Formats the error response according to the irc protocol into a string.
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let e = match self {
            ErrorResponse::UnknownMode { character } => {
                format!("472 {} :is unknown mode char to me", character)
            }
            ErrorResponse::BadChannelKey { channel } => {
                format!("475 {} :Cannot join channel (+k)", channel)
            }
            ErrorResponse::BannedFromChannel { channel } => {
                format!("474 {} :Cannot join channel (+b)", channel)
            }
            ErrorResponse::InviteOnlyChannel { channel } => {
                format!("473 {} :Cannot join channel (+i)", channel)
            }
            ErrorResponse::ChannelIsFull { channel } => {
                format!("471 {} :Cannot join channel (+l)", channel)
            }
            ErrorResponse::YouAreBanned => "465 :You are banned from this server".to_string(),
            ErrorResponse::PasswordMismatch => "464 :Password incorrect".to_string(),
            ErrorResponse::AlreadyRegistered { nickname } => {
                format!("462 {} :You may not reregister", nickname)
            }
            ErrorResponse::NeedMoreParams { command } => {
                format!("461 {} :Not enough parameters", command)
            }
            ErrorResponse::NoLogin { nickname } => format!("444 {} :User not logged in", nickname),
            ErrorResponse::UserOnChannel { nickname, channel } => {
                format!("443 {} {} :is already on channel", nickname, channel)
            }
            ErrorResponse::NotOnChannel { channel } => {
                format!("442 {} :You're not on that channel", channel)
            }
            ErrorResponse::UserNotInChannel { nickname, channel } => {
                format!("441 {} {} :They aren't on that channel", nickname, channel)
            }
            ErrorResponse::NotRegistered => "432 You are not registered".to_string(),
            ErrorResponse::NoNicknameGiven => "431 :No nickname given".to_string(),
            ErrorResponse::UnknownCommand { command } => {
                format!("421 {} :Unknown command", command)
            }
            ErrorResponse::TooManyChannels { channel } => {
                format!("405 {} :You have joined too many channels", channel)
            }
            ErrorResponse::CannotSendToChannel { channel } => {
                format!("404 {} :Cannot send to channel", channel)
            }
            ErrorResponse::NoSuchChannel { channel } => format!("403 {} :No such channel", channel),
            ErrorResponse::NoSuchServer { servername } => {
                format!("402 {} :No such server", servername)
            }
            ErrorResponse::NoSuchNick { nickname } => {
                format!("401 {} :No such nick/channel", nickname)
            }
            ErrorResponse::NoPrivileges => {
                "481 :Permission Denied- You're not an IRC operator".to_string()
            }
            ErrorResponse::ChanOPrivsNeeded { channel } => {
                format!("482 {} :You're not channel operator", channel)
            }
            ErrorResponse::UnknownModeFlag => "501 :Unknown MODE flag".to_string(),
            ErrorResponse::ErrorWhileConnecting => "".to_string(),
            ErrorResponse::NickInUse { nickname } => {
                format!("433 {} :Nickname is already in use", nickname)
            }
            ErrorResponse::UsersDontMatch => "502 :Cant change mode for other users".to_string(),
            ErrorResponse::KeySet { channel } => {
                format!("467 {} :Channel key already set", channel)
            }
            ErrorResponse::ClientDisconnected { nickname } => {
                // format with a number that havent been used yet
                format!("999 {} :client disconnected", nickname)
            }
        };
        write!(f, "{}", e)
    }
}

impl ErrorResponse {
    /// Returns the enum variant that corresponds to the given error string.
    /// If the error string is not recognized, it returns None.
    pub fn serialize(response: String) -> Option<ErrorResponse> {
        let msg = response
            .split_whitespace()
            .map(|x| x.to_string())
            .collect::<Vec<_>>();
        let error = msg[0].as_str();
        match error {
            "401" => Some(ErrorResponse::NoSuchNick {
                nickname: msg[1].clone(),
            }),
            "402" => Some(ErrorResponse::NoSuchServer {
                servername: msg[1].clone(),
            }),
            "403" => Some(ErrorResponse::NoSuchChannel {
                channel: msg[1].clone(),
            }),
            "404" => Some(ErrorResponse::CannotSendToChannel {
                channel: msg[1].clone(),
            }),
            "405" => Some(ErrorResponse::TooManyChannels {
                channel: msg[1].clone(),
            }),
            "421" => Some(ErrorResponse::UnknownCommand {
                command: msg[1].clone(),
            }),
            "431" => Some(ErrorResponse::NoNicknameGiven),
            "432" => Some(ErrorResponse::NotRegistered),
            "441" => Some(ErrorResponse::UserNotInChannel {
                nickname: msg[1].clone(),
                channel: msg[2].clone(),
            }),
            "442" => Some(ErrorResponse::NotOnChannel {
                channel: msg[1].clone(),
            }),
            "443" => Some(ErrorResponse::UserOnChannel {
                nickname: msg[1].clone(),
                channel: msg[2].clone(),
            }),
            "444" => Some(ErrorResponse::NoLogin {
                nickname: msg[1].clone(),
            }),
            "461" => Some(ErrorResponse::NeedMoreParams {
                command: msg[1].clone(),
            }),
            "462" => Some(ErrorResponse::AlreadyRegistered {
                nickname: msg[1].clone(),
            }),
            "464" => Some(ErrorResponse::PasswordMismatch),
            "465" => Some(ErrorResponse::YouAreBanned),
            "467" => Some(ErrorResponse::KeySet {
                channel: msg[1].clone(),
            }),
            "471" => Some(ErrorResponse::ChannelIsFull {
                channel: msg[1].clone(),
            }),
            "473" => Some(ErrorResponse::InviteOnlyChannel {
                channel: msg[1].clone(),
            }),
            "474" => Some(ErrorResponse::BannedFromChannel {
                channel: msg[1].clone(),
            }),
            "475" => Some(ErrorResponse::BadChannelKey {
                channel: msg[1].clone(),
            }),
            "481" => Some(ErrorResponse::NoPrivileges),
            "482" => Some(ErrorResponse::ChanOPrivsNeeded {
                channel: msg[1].clone(),
            }),
            "501" => Some(ErrorResponse::UnknownModeFlag),
            "502" => Some(ErrorResponse::UsersDontMatch),
            "472" => Some(ErrorResponse::UnknownMode {
                character: msg[1].chars().next().unwrap(),
            }),
            "433" => Some(ErrorResponse::NickInUse {
                nickname: msg[1].clone(),
            }),
            "999" => Some(ErrorResponse::ClientDisconnected {
                nickname: msg[1].clone(),
            }),
            _ => None,
        }
    }
}
