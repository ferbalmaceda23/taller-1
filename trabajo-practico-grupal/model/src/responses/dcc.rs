use std::fmt::Display;
///Enum that represents the different types of DCC responses the client can send to the interface
#[derive(Debug)]
pub enum DccResponse {
    Pending {
        sender: String,
    },
    Accepted {
        sender: String,
    },
    Rejected {
        sender: String,
    },
    ChatRequest {
        sender: String,
    },
    ChatMessage {
        sender: String,
        message: String,
    },
    TransferProgress {
        sender: String,
        file_name: String,
        progress: f64,
    },
    CloseConnection {
        sender: String,
    },
    TransferRequest {
        sender: String,
        file_name: String,
        file_size: f64,
    },
    TransferDeclined {
        sender: String,
        file_name: String,
    },
    TransferPaused {
        sender: String,
        file_name: String,
    },
    TransferResumed {
        sender: String,
        file_name: String,
    },
    ErrorResponse {
        description: String,
    },
    ResumeAddressErrorResponse {
        sender: String,
        file_name: String,
    },
    SendAddressErrorResponse {
        sender: String,
        file_name: String,
    },
    ChatAddressErrorResponse {
        sender: String,
    },
    OngoingTransfer {
        sender: String,
        file_name: String,
    },
}

impl Display for DccResponse {
    ///Formats the DccResponse into a string
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let r = match self {
            DccResponse::Accepted { sender } => {
                format!("200 {}", sender)
            }
            DccResponse::Pending { sender } => {
                format!("201 {}", sender)
            }
            DccResponse::Rejected { sender } => {
                format!("202 {}", sender)
            }
            DccResponse::ChatRequest { sender } => {
                format!("203 {}", sender)
            }
            DccResponse::ChatMessage { sender, message } => {
                format!("204 {} {}", sender, message)
            }
            DccResponse::CloseConnection { sender } => {
                format!("205 {}", sender)
            }
            DccResponse::TransferProgress {
                sender,
                file_name,
                progress,
            } => format!("206 {} {} {}", sender, file_name, progress),
            DccResponse::TransferRequest {
                sender,
                file_name,
                file_size,
            } => {
                format!("207 {} {} {}", sender, file_name, file_size)
            }
            DccResponse::TransferDeclined { sender, file_name } => {
                format!("208 {} {}", sender, file_name)
            }
            DccResponse::TransferPaused { sender, file_name } => {
                format!("209 {} {}", sender, file_name)
            }
            DccResponse::ErrorResponse { description } => {
                format!("210 {}", description)
            }
            DccResponse::TransferResumed { sender, file_name } => {
                format!("211 {} {}", sender, file_name)
            }
            DccResponse::ResumeAddressErrorResponse { sender, file_name } => {
                format!("212 {} {}", sender, file_name)
            }
            DccResponse::SendAddressErrorResponse { sender, file_name } => {
                format!("213 {} {}", sender, file_name)
            }
            DccResponse::ChatAddressErrorResponse { sender } => {
                format!("214 {}", sender)
            }
            DccResponse::OngoingTransfer { sender, file_name } => {
                format!("215 {} {}", sender, file_name)
            }
        };
        write!(f, "{}", r)
    }
}

impl DccResponse {
    ///Creates a new instance of DccResponse from a string, it will parse the string and return the correct enum variant.
    /// It is the opposite of the `Display` trait.
    /// It will return none if the string is not a valid message response.
    pub fn serialize(response: String) -> Option<DccResponse> {
        let msg = response
            .split_whitespace()
            .map(|x| x.to_string())
            .collect::<Vec<_>>();
        let command = msg[0].as_str();
        match command {
            "200" => Some(DccResponse::Accepted {
                sender: msg[1].clone(),
            }),
            "201" => Some(DccResponse::Pending {
                sender: msg[1].clone(),
            }),
            "202" => Some(DccResponse::Rejected {
                sender: msg[1].clone(),
            }),
            "203" => Some(DccResponse::ChatRequest {
                sender: msg[1].clone(),
            }),
            "204" => Some(DccResponse::ChatMessage {
                sender: msg[1].clone(),
                message: msg[2..].join(" "),
            }),
            "205" => Some(DccResponse::CloseConnection {
                sender: msg[1].clone(),
            }),
            "206" => Some(DccResponse::TransferProgress {
                sender: msg[1].clone(),
                file_name: msg[2].clone(),
                progress: msg[3].parse::<f64>().unwrap_or(0.0),
            }),
            "207" => Some(DccResponse::TransferRequest {
                sender: msg[1].clone(),
                file_name: msg[2].clone(),
                file_size: msg[3].parse::<f64>().unwrap_or(0.0),
            }),
            "208" => Some(DccResponse::TransferDeclined {
                sender: msg[1].clone(),
                file_name: msg[2].clone(),
            }),
            "209" => Some(DccResponse::TransferPaused {
                sender: msg[1].clone(),
                file_name: msg[2].clone(),
            }),
            "210" => Some(DccResponse::ErrorResponse {
                description: msg[1..].join(" "),
            }),
            "211" => Some(DccResponse::TransferResumed {
                sender: msg[1].clone(),
                file_name: msg[2].clone(),
            }),
            "212" => Some(DccResponse::ResumeAddressErrorResponse {
                sender: msg[1].clone(),
                file_name: msg[2].clone(),
            }),
            "213" => Some(DccResponse::SendAddressErrorResponse {
                sender: msg[1].clone(),
                file_name: msg[2].clone(),
            }),
            "214" => Some(DccResponse::ChatAddressErrorResponse {
                sender: msg[1].clone(),
            }),
            "215" => Some(DccResponse::OngoingTransfer {
                sender: msg[1].clone(),
                file_name: msg[2].clone(),
            }),
            _ => None,
        }
    }
}
