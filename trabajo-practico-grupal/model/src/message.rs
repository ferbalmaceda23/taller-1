static CRLF: &str = "\r\n";
/// Represents the types of messages that can be sent to the server.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum MessageType {
    Pass,
    Nick,
    User,
    Privmsg,
    Join,
    Part,
    Quit,
    Kick,
    Names,
    Topic,
    List,
    Mode,
    Oper,
    Invite,
    Who,
    WhoIs,
    Away,
    Server,
    Squit,
    Dcc,
}

impl MessageType {
    /// Returns the message type according to the command.
    pub fn string_to_message_type(command: String) -> Result<MessageType, MessageError> {
        let message_type = match command.as_str() {
            "PASS" => MessageType::Pass,
            "NICK" => MessageType::Nick,
            "USER" => MessageType::User,
            "PRIVMSG" => MessageType::Privmsg,
            "JOIN" => MessageType::Join,
            "QUIT" => MessageType::Quit,
            "PART" => MessageType::Part,
            "KICK" => MessageType::Kick,
            "NAMES" => MessageType::Names,
            "TOPIC" => MessageType::Topic,
            "LIST" => MessageType::List,
            "MODE" => MessageType::Mode,
            "OPER" => MessageType::Oper,
            "INVITE" => MessageType::Invite,
            "WHO" => MessageType::Who,
            "WHOIS" => MessageType::WhoIs,
            "AWAY" => MessageType::Away,
            "SERVER" => MessageType::Server,
            "SQUIT" => MessageType::Squit,
            "DCC" => MessageType::Dcc,
            _ => return Err(MessageError::InvalidCommand),
        };
        Ok(message_type)
    }

    /// Returns the string representation of the message type.
    pub fn message_type_to_string(command: MessageType) -> Result<String, MessageError> {
        let command_string = match command {
            MessageType::Pass => "PASS".to_string(),
            MessageType::Nick => "NICK".to_string(),
            MessageType::User => "USER".to_string(),
            MessageType::Privmsg => "PRIVMSG".to_string(),
            MessageType::Join => "JOIN".to_string(),
            MessageType::Quit => "QUIT".to_string(),
            MessageType::Part => "PART".to_string(),
            MessageType::Kick => "KICK".to_string(),
            MessageType::Names => "NAMES".to_string(),
            MessageType::Topic => "TOPIC".to_string(),
            MessageType::List => "LIST".to_string(),
            MessageType::Mode => "MODE".to_string(),
            MessageType::Oper => "OPER".to_string(),
            MessageType::Invite => "INVITE".to_string(),
            MessageType::Who => "WHO".to_string(),
            MessageType::WhoIs => "WHOIS".to_string(),
            MessageType::Away => "AWAY".to_string(),
            MessageType::Server => "SERVER".to_string(),
            MessageType::Squit => "SQUIT".to_string(),
            MessageType::Dcc => "DCC".to_string(),
        };
        Ok(command_string)
    }
}

/// Struct that represents a message.
/// The message is composed of a message type, a prefix, a command, a list of parameters and a trailing parameter.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Message {
    pub prefix: Option<String>,
    pub command: MessageType,
    pub parameters: Vec<String>,
    pub trailing: Option<String>,
}

/// Represents the errors that can occur when parsing a message.
#[derive(Debug)]
pub enum MessageError {
    EmptyMessage,
    EmptyCommand,
    InvalidCommand,
}

impl Message {
    /// Creates a new message.
    pub fn new(
        prefix: Option<String>,
        command: MessageType,
        parameters: Vec<String>,
        trailing: Option<String>,
    ) -> Self {
        Self {
            prefix,
            command,
            parameters,
            trailing,
        }
    }

    /// Serializes the received string into a Message.
    /// Returns a message error if the message is invalid.
    pub fn serialize(msg: String) -> Result<Message, MessageError> {
        fn load_parameters(
            msg: std::str::SplitWhitespace,
            parameters: &mut Vec<String>,
        ) -> Option<String> {
            let mut trailing = vec![];
            for param in msg {
                if param.starts_with(':') || !trailing.is_empty() {
                    match param.strip_prefix(':') {
                        Some(t) => trailing.push(t.to_string()),
                        None => trailing.push(param.to_string()),
                    }
                } else {
                    parameters.push(param.to_string());
                }
            }
            if !trailing.is_empty() {
                return Some(trailing.join(" "));
            }
            None
        }

        let mut msg = msg.split_whitespace();
        let mut prefix = None;
        let command;
        let mut parameters = vec![];
        let first_token = match msg.next() {
            Some(token) => token,
            None => return Err(MessageError::EmptyMessage),
        };
        if first_token.starts_with(':') {
            prefix = first_token.strip_prefix(':').map(|p| p.to_owned());
            match msg.next() {
                Some(token) => command = MessageType::string_to_message_type(token.to_owned())?,
                None => return Err(MessageError::EmptyCommand),
            };
        } else {
            command = MessageType::string_to_message_type(first_token.to_owned())?
        }
        let trailing = load_parameters(msg, &mut parameters);

        let message = Message {
            prefix,
            command,
            parameters,
            trailing,
        };
        Ok(message)
    }

    pub fn deserialize(message: Message) -> Result<String, MessageError> {
        let mut msg = String::new();
        if let Some(prefix) = message.prefix.to_owned() {
            let prx = format!(":{} ", prefix);
            msg.push_str(&prx);
        }
        let message_type = match MessageType::message_type_to_string(message.command) {
            Ok(msg) => msg,
            Err(e) => return Err(e),
        };
        msg.push_str(&message_type);
        for param in message.parameters {
            let param = format!(" {}", param);
            msg.push_str(&param);
        }
        if let Some(trailing) = message.trailing {
            let trail = format!(" :{}", trailing);
            msg.push_str(&trail);
        }
        msg.push_str(CRLF);
        Ok(msg)
    }
}

//tests a serialize y des
/*
#[cfg(test)]
mod nick_tests {
    use crate::message::{Message, MessageType};

    #[test]
    fn test_serialize_message() {
        let string_message = "PRIVMSG user hi other".to_string();

        let expected = Message::new(
            "".to_string(),
            MessageType::Privmsg,
            vec!["user".to_string(), "hi".to_string(), "other".to_string()],
            "".to_string(),
        );

        assert_eq!(Message::serialize(string_message).unwrap(), expected);
    }

    #[test]
    fn test_serialize_message_with_trailing() {
        let string_message = "JOIN #channel :a channel".to_string();

        let expected = Message::new(
            "".to_string(),
            MessageType::Join,
            vec!["#channel".to_string()],
            "a channel".to_string(),
        );

        assert_eq!(Message::serialize(string_message).unwrap(), expected);
    }

    #[test]
    fn test_serialize_message_with_prefix() {
        let string_message = ":user JOIN #channel a_channel".to_string();
        let expected = Message::new(
            "user".to_string(),
            MessageType::Join,
            vec!["#channel".to_string(), "a_channel".to_string()],
            "".to_string(),
        );
        assert_eq!(Message::serialize(string_message).unwrap(), expected);
    }

    #[test]
    fn test_deserialize_message() {
        let message = Message::new(
            "".to_string(),
            MessageType::Privmsg,
            vec!["user".to_string(), "hi".to_string(), "other".to_string()],
            "".to_string(),
        );
        let expected = "PRIVMSG user hi other".to_string();
        assert_eq!(Message::deserialize(message).unwrap(), expected);
    }

    #[test]
    fn test_deserialize_message_with_trailing() {
        let message = Message::new(
            "".to_string(),
            MessageType::Join,
            vec!["#channel".to_string()],
            "a channel".to_string(),
        );
        let expected = "JOIN #channel :a channel";
        assert_eq!(Message::deserialize(message).unwrap(), expected);
    }

    #[test]
    fn test_deserialize_message_with_prefix() {
        let expected = ":user JOIN #channel a_channel";
        let message = Message::new(
            "user".to_string(),
            MessageType::Join,
            vec!["#channel".to_string(), "a_channel".to_string()],
            "".to_string(),
        );
        assert_eq!(Message::deserialize(message).unwrap(), expected);
    }
}
*/
