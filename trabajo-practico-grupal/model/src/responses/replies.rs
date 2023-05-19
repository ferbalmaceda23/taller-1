use std::collections::HashMap;
use std::fmt::Display;
use std::fmt::Write as _;

/// Enum that represents the different types of responses that the server can send according to the irc protocol.
#[derive(Debug)]
pub enum CommandResponse {
    ConnectionSuccees,
    Topic {
        channel: String,
        topic: String,
    },
    NoTopic {
        channel: String,
    },
    ChannelMode {
        channel: String,
        modes: HashMap<String, String>,
    },
    BanList {
        channel: String,
        ban_list: Vec<String>,
    },
    EndBanList,
    UserMode {
        user: String,
        modes: HashMap<String, String>,
    },
    Names {
        channel: String,
        names: Vec<String>,
    },
    EndNames,
    ListStart,
    List {
        channel: String,

        topic: String,
    },
    ListEnd,
    Inviting {
        channel: String,
        nickname: String,
    },
    Away {
        nickname: String,
        message: String,
    },
    UnAway,
    NowAway,
    Welcome {
        nickname: String,
        username: String,
        hostname: String,
    },
    WhoIsUser {
        nickname: String,
        username: String,
        hostname: String,
        servername: String,
        realname: String,
    },
    WhoIsServer {
        nickname: String,
        servername: String,
        serverinfo: String,
    },
    WhoIsChannels {
        nickname: String,
        channels: HashMap<String, String>,
    },
    YouAreOperator,
    WhoReply {
        users: Vec<String>,
    },
    EndOfWho,
    EndOfWhoIs,
    Server {
        servers: HashMap<String, u8>,
    },
}

impl Display for CommandResponse {
    /// Formats the response to a string that can be sent to the server or received from the server.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let r = match self {
            CommandResponse::Welcome {
                nickname,
                username,
                hostname,
            } => format!(
                "001 {} {} {} :Welcome to the Internet Rust Network {}",
                nickname, username, hostname, nickname
            ),
            CommandResponse::ConnectionSuccees => "000 Connection successful".to_string(),
            CommandResponse::Away { nickname, message } => format!("301 {} :{}", nickname, message),
            CommandResponse::UnAway => "305 :You are no longer marked as being away".to_string(),
            CommandResponse::NowAway => "306 :You have been marked as being away".to_string(),
            CommandResponse::WhoIsUser {
                nickname,
                username,
                hostname,
                servername,
                realname,
            } => format!(
                "311 {} {} {} {} * :{}",
                nickname, username, hostname, servername, realname
            ),
            CommandResponse::WhoIsServer {
                nickname,
                servername,
                serverinfo,
            } => format!("312 {} {} :{}", nickname, servername, serverinfo),
            CommandResponse::WhoIsChannels { nickname, channels } => {
                let mut channels_str = String::new();
                for (channel, flags) in channels {
                    let _ = write!(channels_str, ":{}{}", flags, channel);
                }
                format!("319 {} {}", nickname, channels_str)
            }
            CommandResponse::ListStart => "321 Channel :Users Name".to_string(),
            CommandResponse::List { channel, topic } => format!("322 {} {}", channel, topic),
            CommandResponse::ListEnd => "323 :End of /LIST".to_string(),
            CommandResponse::ChannelMode { channel, modes } => {
                let mut modes_str = String::new();
                for (key, value) in modes {
                    let _ = write!(modes_str, " {};{}", key, value);
                }
                format!("324 {}{}", channel, modes_str)
            }
            CommandResponse::NoTopic { channel } => format!("331 {} :No topic is set", channel),
            CommandResponse::Topic { channel, topic } => format!("332 {} {}", channel, topic),
            CommandResponse::Inviting { channel, nickname } => {
                format!("341 {} {}", channel, nickname)
            }
            CommandResponse::Names { channel, names } => {
                let mut names_str = String::new();
                for name in names {
                    let _ = write!(names_str, "{} ", name);
                }
                format!("353 {} {}", channel, names_str)
            }
            CommandResponse::EndNames => "366 :End of /NAMES list.".to_string(),
            CommandResponse::BanList { channel, ban_list } => {
                let mut ban_list_str = String::new();
                for user in ban_list {
                    let _ = write!(ban_list_str, "{} ", user);
                }
                format!("367 {} :{}", channel, ban_list_str)
            }
            CommandResponse::EndBanList => "368 :End of channel ban list".to_string(),
            CommandResponse::UserMode { user, modes } => {
                let mut modes_str = String::new();
                for (key, value) in modes {
                    let _ = write!(modes_str, " {};{}", key, value);
                }
                format!("221 {} {}", user, modes_str)
            }
            CommandResponse::WhoReply { users } => {
                let mut users_str = vec![];
                for user in users {
                    users_str.push(user.to_string());
                }
                format!("352 {}", users_str.join(" "))
            }
            CommandResponse::YouAreOperator => "381 :You are now an IRC operator".to_string(),
            CommandResponse::EndOfWho => "315 :End of /WHO list".to_string(),
            CommandResponse::EndOfWhoIs => "318 :End of /WHOIS list".to_string(),
            CommandResponse::Server { servers } => {
                let mut servers_str = vec![];
                for (server, hops) in servers {
                    servers_str.push(format!("{},{}", server, hops));
                }
                format!("370 {}", servers_str.join(";"))
            }
        };
        write!(f, "{}", r)
    }
}

impl CommandResponse {
    /// Parses a string into a CommandResponse.
    /// This is the inverse of the Display trait.
    /// Returns none if the string is not a valid response.
    pub fn serialize(response: String) -> Option<CommandResponse> {
        let mut msg = response
            .split_whitespace()
            .map(|x| x.to_string())
            .collect::<Vec<_>>();
        let command = msg[0].as_str();
        match command {
            "000" => Some(CommandResponse::ConnectionSuccees),
            "001" => Some(CommandResponse::Welcome {
                nickname: msg[1].to_owned(),
                username: msg[2].to_owned(),
                hostname: msg[3].to_owned(),
            }),
            "301" => {
                msg[2] = match msg[2].strip_prefix(':') {
                    Some(p) => p.to_owned(),
                    None => "".to_owned(),
                };
                Some(CommandResponse::Away {
                    nickname: msg[1].to_owned(),
                    message: msg[2..].to_owned().join(" "),
                })
            }
            "305" => Some(CommandResponse::UnAway),
            "306" => Some(CommandResponse::NowAway),
            "311" => Some(CommandResponse::WhoIsUser {
                nickname: msg[1].to_owned(),
                username: msg[2].to_owned(),
                hostname: msg[3].to_owned(),
                servername: msg[4].to_owned(),
                realname: msg[5].to_owned(),
            }),
            "312" => Some(CommandResponse::WhoIsServer {
                nickname: msg[1].to_owned(),
                servername: msg[2].to_owned(),
                serverinfo: msg[3].to_owned(),
            }),
            "315" => Some(CommandResponse::EndOfWho),
            "318" => Some(CommandResponse::EndOfWhoIs),
            "319" => {
                let nickname = msg[1].to_owned();
                let mut channels = HashMap::new();
                for channel in msg[2..].iter() {
                    let channel = channel.strip_prefix(':').unwrap();
                    let flags = channel
                        .chars()
                        .take_while(|x| (x == &'@' || x == &'&'))
                        .collect::<String>();
                    let channel = match channel.strip_prefix(&flags) {
                        Some(p) => p.to_owned(),
                        None => "".to_owned(),
                    };
                    channels.insert(channel.to_owned(), flags);
                }
                Some(CommandResponse::WhoIsChannels { nickname, channels })
            }
            "321" => Some(CommandResponse::ListStart),
            "322" => Some(CommandResponse::List {
                channel: msg[1].to_owned(),
                topic: msg[2..].to_owned().join(" "),
            }),

            "323" => Some(CommandResponse::ListEnd),
            "324" => {
                let channel = msg[1].to_owned();
                let mut modes = HashMap::new();
                for mode in msg[2..].iter() {
                    let mut mode = mode.split(';');
                    let key = match mode.next() {
                        Some(x) => x.to_owned(),
                        None => {
                            continue;
                        }
                    };
                    let value = match mode.next() {
                        Some(x) => x.to_owned(),
                        None => {
                            continue;
                        }
                    };
                    modes.insert(key, value);
                }
                Some(CommandResponse::ChannelMode { channel, modes })
            }
            "332" => {
                let mut topic = "".to_string();
                if msg.len() > 2 {
                    msg[2] = match msg[2].strip_prefix(':') {
                        Some(p) => p.to_owned(),
                        None => "".to_owned(),
                    };
                    topic = msg[2..].to_owned().join(" ");
                }
                Some(CommandResponse::Topic {
                    channel: msg[1].to_owned(),
                    topic,
                })
            }
            "341" => Some(CommandResponse::Inviting {
                channel: msg[1].to_owned(),
                nickname: msg[2].to_owned(),
            }),
            "352" => Some(CommandResponse::WhoReply {
                users: msg[1..].to_owned().to_vec(),
            }),
            "353" => Some(CommandResponse::Names {
                channel: msg[1].to_owned(),
                names: msg[2..].to_owned().to_vec(),
            }),
            "366" => Some(CommandResponse::EndNames),
            "367" => {
                msg[2] = match msg[2].strip_prefix(':') {
                    Some(p) => p.to_owned(),
                    None => "".to_owned(),
                };
                Some(CommandResponse::BanList {
                    channel: msg[1].to_owned(),
                    ban_list: msg[2..].to_owned().to_vec(),
                })
            }
            "368" => Some(CommandResponse::EndBanList),
            "370" => {
                let mut servers = HashMap::new();
                let servers_str = msg[1]
                    .to_owned()
                    .split(';')
                    .map(|x| x.to_owned())
                    .collect::<Vec<_>>();
                for server in servers_str {
                    let mut s = server.split(',');
                    let server = match s.next() {
                        Some(x) => x.to_owned(),
                        None => "".to_owned(),
                    };
                    let hops = match s.next() {
                        Some(x) => x.parse::<u8>().unwrap_or(0),
                        None => 0,
                    };
                    servers.insert(server, hops);
                }
                Some(CommandResponse::Server { servers })
            }
            "381" => Some(CommandResponse::YouAreOperator),
            _ => None,
        }
    }
}
