#[derive(Debug)]
pub enum DccMessageError {
    InvalidMessage,
    InvalidCommand,
    CannotParsePrefix,
    NoParameters,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum DccMessageType {
    Chat,
    Send,
    Accept,
    Close,
    Resume,
    Msg,
    Stop,
    Invalid,
}
impl DccMessageType {
    pub fn string_to_dcc_message_type(msg: String) -> Result<DccMessageType, DccMessageError> {
        match msg.to_uppercase().as_str() {
            "CHAT" => Ok(DccMessageType::Chat),
            "SEND" => Ok(DccMessageType::Send),
            "ACCEPT" => Ok(DccMessageType::Accept),
            "RESUME" => Ok(DccMessageType::Resume),
            "MSG" => Ok(DccMessageType::Msg),
            "CLOSE" => Ok(DccMessageType::Close),
            "STOP" => Ok(DccMessageType::Stop),
            _ => Err(DccMessageError::InvalidCommand),
        }
    }

    pub fn dcc_message_type_to_string(msg: DccMessageType) -> Result<String, DccMessageError> {
        match msg {
            DccMessageType::Chat => Ok("CHAT".to_string()),
            DccMessageType::Send => Ok("SEND".to_string()),
            DccMessageType::Accept => Ok("ACCEPT".to_string()),
            DccMessageType::Resume => Ok("RESUME".to_string()),
            DccMessageType::Msg => Ok("MSG".to_string()),
            DccMessageType::Close => Ok("CLOSE".to_string()),
            DccMessageType::Stop => Ok("STOP".to_string()),
            _ => Err(DccMessageError::InvalidCommand),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DccMessage {
    pub prefix: Option<String>,
    pub command: DccMessageType,
    pub parameters: Vec<String>,
}
impl DccMessage {
    pub fn serialize(message: DccMessage) -> Result<String, DccMessageError> {
        let prefix = match message.prefix {
            Some(prefix) => format!(":{} ", prefix),
            None => "".to_string(),
        };

        let command = DccMessageType::dcc_message_type_to_string(message.command)?;

        let parameters = message.parameters.join(" ");

        Ok(format!("{}DCC {} {}", prefix, command, parameters))
    }

    pub fn deserialize(msg: String) -> Result<DccMessage, DccMessageError> {
        let msg = msg
            .split_whitespace()
            .into_iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

        if msg.is_empty() {
            return Err(DccMessageError::InvalidMessage);
        }

        let prefix = if msg[0].starts_with(':') {
            match msg[0].strip_prefix(':') {
                Some(p) => {
                    if msg[1] != "DCC" {
                        return Err(DccMessageError::InvalidMessage);
                    } else {
                        Some(p.to_string())
                    }
                }
                None => return Err(DccMessageError::CannotParsePrefix),
            }
        } else if msg[0] != "DCC" {
            return Err(DccMessageError::InvalidMessage);
        } else {
            None
        };

        let command = if prefix.is_some() {
            if msg.len() < 4 {
                return Err(DccMessageError::NoParameters);
            } else {
                DccMessageType::string_to_dcc_message_type(msg[2].to_owned())?
            }
        } else if msg.len() < 3 {
            return Err(DccMessageError::NoParameters);
        } else {
            DccMessageType::string_to_dcc_message_type(msg[1].to_owned())?
        };

        let parameters = if prefix.is_some() {
            msg[3..].to_vec()
        } else {
            msg[2..].to_vec()
        };

        Ok(DccMessage {
            prefix,
            command,
            parameters,
        })
    }
}
