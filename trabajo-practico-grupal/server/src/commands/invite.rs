use model::{
    channelflag::ChannelFlag,
    message::Message,
    network::Network,
    persistence::PersistenceType,
    responses::{errors::ErrorResponse, message::MessageResponse, replies::CommandResponse},
    session::Session,
};

use crate::{
    database::inform_database,
    server_errors::ServerError,
    socket::{inform_client, inform_network},
};

use super::command_utils::{read_lock_clients, write_lock_channels};

/// Handles the invite message, which invites a client to a channel.
/// #Errors
/// ServerError::InvalidParameters if the message has no parameters or the parameters are invalid.
/// ServerError::NoSuchChannel if the channel does not exist.
/// Server Error::ChannelMustStartWithHashOrAmpersand if the channel does not start with a # or &.
/// ServerError::UserAlreadyInChannel if the user is already in the channel.
/// ServerError::ChannelIsFull if the channel is full.
/// ServerError::NoSuchUser if the user does not exist.
/// ServerError::ClientNotFound if the client is not found.
/// ServerError::UserIsBanned if the user is banned from the channel.
pub fn handle_invite_command(
    message: Message,
    nickname: String,
    session: &Session,
    network: &Network,
    server_name: &String,
) -> Result<(), ServerError> {
    if message.parameters.is_empty() || message.parameters.len() < 2 {
        return Err(ServerError::InvalidParameters);
    }
    if !message.parameters[1].starts_with('&') && !message.parameters[1].starts_with('#') {
        let error_response = ErrorResponse::NoSuchChannel {
            channel: message.parameters[1].clone(),
        }
        .to_string();
        inform_client(session, &nickname, error_response.as_str())?;
        return Err(ServerError::ChannelMustStartWithHashOrAmpersand);
    }

    let user_to_invite = message.parameters[0].to_owned();
    let channel_name = message.parameters[1].to_owned();

    match write_lock_channels(session)?.get_mut(&channel_name) {
        Some(channel) => {
            if channel.modes.contains(&ChannelFlag::InviteOnly)
                && !channel.operators.contains(&nickname)
            {
                let response = ErrorResponse::ChanOPrivsNeeded {
                    channel: channel_name.clone(),
                };
                inform_client(session, &nickname, response.to_string().as_str())?;
                return Err(ServerError::ChannelIsInviteOnly);
            }
            if channel.banned_users.contains(&user_to_invite) {
                let response = (ErrorResponse::BannedFromChannel {
                    channel: channel.name.to_string(),
                })
                .to_string();
                inform_client(session, &nickname, response.as_str())?;
                return Err(ServerError::UserIsBanned);
            }
            if !channel.users.contains(&nickname) {
                let response = (ErrorResponse::NotOnChannel {
                    channel: channel.name.to_string(),
                })
                .to_string();
                inform_client(session, &nickname, response.as_str())?;
                return Err(ServerError::ClientNotOnChannel);
            }
            if channel.users.contains(&user_to_invite) {
                let response = (ErrorResponse::UserOnChannel {
                    channel: channel.name.to_string(),
                    nickname: user_to_invite,
                })
                .to_string();
                inform_client(session, &nickname, response.as_str())?;
                return Err(ServerError::UserAlreadyInChannel);
            }
            if let Some(limit) = channel.limit {
                if channel.users.len() >= limit as usize {
                    let response = (ErrorResponse::ChannelIsFull {
                        channel: channel.name.to_string(),
                    })
                    .to_string();
                    inform_client(session, &nickname, response.as_str())?;
                    return Err(ServerError::ChannelIsFull);
                }
            }
            match read_lock_clients(session)?.get(&user_to_invite) {
                Some(c) => {
                    if let Some(away_msg) = c.away_message.to_owned() {
                        let response = CommandResponse::Away {
                            nickname: user_to_invite.to_string(),
                            message: away_msg.to_string(),
                        }
                        .to_string();
                        inform_client(session, &nickname, response.as_str())?;
                        println!("{} is away: {}", user_to_invite, away_msg);
                        return Ok(());
                    }
                    channel.users.push(user_to_invite.to_owned());
                    inform_database(
                        PersistenceType::ChannelUpdate(channel.name.to_owned()),
                        channel.to_string(),
                        session,
                    )?;
                    let response = CommandResponse::Inviting {
                        channel: channel_name.to_owned(),
                        nickname: user_to_invite.to_owned(),
                    }
                    .to_string();
                    let msg = format!("{} has invited you to {}", nickname, channel_name);
                    inform_client(session, &nickname, response.as_str())?;
                    let response = MessageResponse::InviteMsg { message: msg }.to_string();
                    inform_client(session, &user_to_invite, response.as_str())?;
                    println!("{} joined {}", user_to_invite, channel_name);
                }

                None => {
                    if channel.name.starts_with('#') {
                        let network_clients = network.clients.read()?;
                        if network_clients.get(&user_to_invite).is_some() {
                            channel.users.push(user_to_invite.to_owned());
                            inform_database(
                                PersistenceType::ChannelUpdate(channel.name.to_owned()),
                                channel.to_string(),
                                session,
                            )?;
                            let response = CommandResponse::Inviting {
                                channel: channel_name.to_owned(),
                                nickname: nickname.to_owned(),
                            }
                            .to_string();
                            let msg = format!("{} has invited you to {}", nickname, channel_name);
                            inform_client(session, &nickname, response.as_str())?;
                            let response = MessageResponse::InviteMsg { message: msg }.to_string();
                            inform_client(session, &user_to_invite, response.as_str())?;
                            println!("{} joined {}", user_to_invite, channel_name);

                            let mut msg = message;
                            msg.prefix = Some(nickname);
                            let msg = Message::deserialize(msg)?;
                            inform_network(network, server_name, &msg)?;
                        }
                    } else {
                        let response = ErrorResponse::NoSuchNick {
                            nickname: user_to_invite.to_owned(),
                        };
                        inform_client(session, &nickname, response.to_string().as_str())?;
                        return Err(ServerError::ClientNotFound);
                    }
                }
            }
        }
        None => {
            let response = ErrorResponse::NoSuchChannel {
                channel: channel_name.to_owned(),
            };
            inform_client(session, &nickname, response.to_string().as_str())?;
            return Err(ServerError::ChannelNotFound);
        }
    }

    Ok(())
}

#[cfg(test)]
mod invite_tests {
    use std::{
        collections::HashMap,
        io::Read,
        net::TcpListener,
        sync::{Arc, RwLock},
        vec,
    };

    use crate::{
        commands::command_utils::{
            create_client_for_test, create_message_for_test, create_session_for_test,
            read_lock_channels, write_lock_channels, write_lock_clients,
        },
        database::handle_database,
        server_errors::ServerError,
    };

    use super::handle_invite_command;
    use model::{
        channel::Channel,
        channelflag::ChannelFlag,
        message::MessageType,
        network::Network,
        persistence::PersistenceType,
        responses::{
            errors::ErrorResponse, message::MessageResponse, replies::CommandResponse,
            response::Response,
        },
        server::Server,
    };

    #[test]
    fn test_invite_command_invite_only_channel_without_operator() {
        let listener = TcpListener::bind("127.0.0.1:8079".to_string()).unwrap();
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8079".to_string(),
            name: "test".to_string(),
            operators: vec![],
            father: None,
            children: HashMap::new(),
        }));

        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));

        let network = Network {
            server: arc_server,
            servers: arc_servers,
            clients: arc_network_clients,
        };

        let (db_tx, db_rx) = std::sync::mpsc::channel::<(PersistenceType, String)>();
        handle_database(db_rx);
        let session = create_session_for_test(db_tx);

        let client = create_client_for_test(
            &session,
            "127.0.0.1:8079".to_string(),
            "nickname".to_string(),
        );
        let client2 = create_client_for_test(
            &session,
            "127.0.0.1:8079".to_string(),
            "nickname2".to_string(),
        );
        //persist_client(client.to_string());
        //persist_client(client2.to_string());

        let mut channel = Channel::new("#channel_test".to_string(), "".to_string(), vec![]);
        channel.modes = vec![ChannelFlag::InviteOnly];
        channel.users = vec![client.nickname.clone()];
        session
            .channels
            .write()
            .unwrap()
            .insert(channel.clone().name, channel.clone());

        let message = create_message_for_test(
            MessageType::Invite,
            vec![client2.nickname.clone(), channel.clone().name],
        );

        let result = handle_invite_command(
            message,
            client.nickname.clone(),
            &session,
            &network,
            &"test".to_string(),
        );

        let (mut reader, _addr) = listener.accept().unwrap();
        drop(listener);
        let mut buf = vec![0u8; 510];
        reader.read(&mut buf).unwrap();
        let msg = buf.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
        let response = String::from_utf8(msg).unwrap();

        //delete_client(client.nickname.to_string()).unwrap();
        //delete_client(client2.nickname.to_string()).unwrap();

        let response = Response::serialize(response).unwrap();
        match response {
            Response::ErrorResponse {
                response: error_response,
            } => match error_response {
                ErrorResponse::ChanOPrivsNeeded { channel: c } => {
                    assert_eq!(c, channel.name);
                }
                _ => {
                    assert!(false);
                }
            },
            _ => {
                assert!(false);
            }
        }

        assert_eq!(Err(ServerError::ChannelIsInviteOnly), result);
    }

    #[test]
    fn test_invite_command_invite_only_channel_with_operator() {
        let listener = TcpListener::bind("127.0.0.1:0".to_string()).unwrap();
        let port = listener.local_addr().unwrap().port();
        let address_port = format!("127.0.0.1:{}", port.to_string());
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: port.to_string(),
            name: "test".to_string(),
            operators: vec![],
            father: None,
            children: HashMap::new(),
        }));

        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));

        let network = Network {
            server: arc_server,
            servers: arc_servers,
            clients: arc_network_clients,
        };

        let (db_tx, db_rx) = std::sync::mpsc::channel::<(PersistenceType, String)>();
        handle_database(db_rx);
        let session = create_session_for_test(db_tx);
        let client =
            create_client_for_test(&session, address_port.to_string(), "nickname".to_string());
        let client2 =
            create_client_for_test(&session, address_port.to_string(), "nickname2".to_string());

        let mut channel = Channel::new("#channel_test".to_string(), "".to_string(), vec![]);
        channel.modes = vec![ChannelFlag::InviteOnly];
        channel.users = vec![client.nickname.clone()];
        channel.operators = vec![client.nickname.clone()];
        session
            .channels
            .write()
            .unwrap()
            .insert(channel.clone().name, channel.clone());

        let message = create_message_for_test(
            MessageType::Invite,
            vec![client2.nickname.to_string(), channel.name.to_string()],
        );

        let result = handle_invite_command(
            message.clone(),
            client.nickname.to_string(),
            &session,
            &network,
            &"test".to_string(),
        );
        let (mut reader, _addr) = listener.accept().unwrap();
        let mut buf = vec![0u8; 510];
        reader.read(&mut buf).unwrap();
        let msg = buf.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
        let response = String::from_utf8(msg).unwrap();

        let response = Response::serialize(response).unwrap();
        match response {
            Response::CommandResponse {
                response: cmd_response,
            } => match cmd_response {
                CommandResponse::Inviting {
                    channel: c,
                    nickname: nick,
                } => {
                    assert_eq!(c, channel.name.to_string());
                    assert_eq!(nick, client2.nickname.to_string());
                }
                _ => {
                    assert!(false);
                }
            },
            _ => {
                assert!(false);
            }
        }

        let (mut reader, _addr) = listener.accept().unwrap();
        drop(listener);
        let mut buf = vec![0u8; 510];
        reader.read(&mut buf).unwrap();
        let msg = buf.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
        let response = String::from_utf8(msg).unwrap();

        let response = Response::serialize(response).unwrap();
        match response {
            Response::MessageResponse {
                response: cmd_response,
            } => match cmd_response {
                MessageResponse::InviteMsg { message } => {
                    let expected = format!(
                        "{} has invited you to {}",
                        client.nickname.to_string(),
                        channel.name.to_string()
                    );
                    assert_eq!(expected, message);
                }
                _ => {
                    assert!(false);
                }
            },
            _ => {
                assert!(false);
            }
        }

        let channels_lock = read_lock_channels(&session).unwrap();
        assert_eq!(channels_lock.contains_key("#channel_test"), true);
        let channel = channels_lock.get(&"#channel_test".to_string()).unwrap();
        assert!(channel.users.contains(&client2.nickname.to_string()));
        assert!(channel.users.contains(&client.nickname.to_string()));
        assert!(!channel.operators.contains(&client2.nickname.to_string()));
        assert!(channel.operators.contains(&client.nickname.to_string()));
        assert!(result.is_ok());
    }

    #[test]
    fn test_invite_command_client_not_found() {
        let listener = TcpListener::bind("127.0.0.1:0".to_string()).unwrap();
        let port = listener.local_addr().unwrap().port();
        let address_port = format!("127.0.0.1:{}", port.to_string());
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: port.to_string(),
            name: "test".to_string(),
            operators: vec![],
            father: None,
            children: HashMap::new(),
        }));

        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));

        let network = Network {
            server: arc_server,
            servers: arc_servers,
            clients: arc_network_clients,
        };

        let (db_tx, db_rx) = std::sync::mpsc::channel::<(PersistenceType, String)>();
        handle_database(db_rx);
        let session = create_session_for_test(db_tx);
        let client =
            create_client_for_test(&session, address_port.to_string(), "nickname".to_string());

        let mut channel = Channel::new("&channel_test".to_string(), "".to_string(), vec![]);
        channel.users = vec![client.nickname.clone()];
        session
            .channels
            .write()
            .unwrap()
            .insert(channel.name.to_string(), channel.clone());

        let message = create_message_for_test(
            MessageType::Invite,
            vec![
                "non_existing_nickname".to_string(),
                channel.name.to_string(),
            ],
        );

        let result = handle_invite_command(
            message,
            client.nickname.to_string(),
            &session,
            &network,
            &"test".to_string(),
        );
        assert_eq!(Err(ServerError::ClientNotFound), result);

        let (mut reader, _addr) = listener.accept().unwrap();
        drop(listener);
        let mut buf = vec![0u8; 510];
        reader.read(&mut buf).unwrap();
        let msg = buf.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
        let response = String::from_utf8(msg).unwrap();

        let response = Response::serialize(response).unwrap();
        match response {
            Response::ErrorResponse {
                response: cmd_response,
            } => match cmd_response {
                ErrorResponse::NoSuchNick { nickname: nick } => {
                    assert_eq!(nick, "non_existing_nickname".to_string());
                }
                _ => {
                    assert!(false);
                }
            },
            _ => {
                assert!(false);
            }
        }
        let channels_lock = read_lock_channels(&session).unwrap();
        assert_eq!(channels_lock.contains_key("&channel_test"), true);
        let channel = channels_lock.get(&"&channel_test".to_string()).unwrap();
        assert!(!channel.users.contains(&"#nickname".to_string()));
        assert!(channel.users.contains(&client.nickname.to_string()));
        assert!(!channel.operators.contains(&"#nickname".to_string()));
        drop(channels_lock);
    }

    #[test]
    fn test_invite_command_client_not_on_channel() {
        let listener = TcpListener::bind("127.0.0.1:0".to_string()).unwrap();
        let port = listener.local_addr().unwrap().port();
        let address_port = format!("127.0.0.1:{}", port.to_string());
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: port.to_string(),
            name: "test".to_string(),
            operators: vec![],
            father: None,
            children: HashMap::new(),
        }));

        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));

        let network = Network {
            server: arc_server,
            servers: arc_servers,
            clients: arc_network_clients,
        };

        let (db_tx, db_rx) = std::sync::mpsc::channel::<(PersistenceType, String)>();
        handle_database(db_rx);
        let session = create_session_for_test(db_tx);
        let client =
            create_client_for_test(&session, address_port.to_string(), "nickname".to_string());
        let client2 =
            create_client_for_test(&session, address_port.to_string(), "nickname2".to_string());

        let channel = Channel::new("#channel_test".to_string(), "".to_string(), vec![]);
        session
            .channels
            .write()
            .unwrap()
            .insert(channel.name.to_string(), channel.clone());

        let message = create_message_for_test(
            MessageType::Invite,
            vec![client2.nickname.to_string(), channel.name.to_string()],
        );

        let result = handle_invite_command(
            message,
            client.nickname.to_string(),
            &session,
            &network,
            &"test".to_string(),
        );
        let (mut reader, _addr) = listener.accept().unwrap();
        drop(listener);
        let mut buf = vec![0u8; 510];
        reader.read(&mut buf).unwrap();
        let msg = buf.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
        let response = String::from_utf8(msg).unwrap();

        let response = Response::serialize(response).unwrap();
        match response {
            Response::ErrorResponse {
                response: cmd_response,
            } => match cmd_response {
                ErrorResponse::NotOnChannel { channel: c } => {
                    assert_eq!(c, channel.name.to_string());
                }
                _ => {
                    assert!(false);
                }
            },
            _ => {
                assert!(false);
            }
        }
        let channels_lock = read_lock_channels(&session).unwrap();
        assert_eq!(channels_lock.contains_key("#channel_test"), true);
        let channel = channels_lock.get(&"#channel_test".to_string()).unwrap();
        assert!(!channel.users.contains(&client2.nickname.to_string()));
        assert!(!channel.users.contains(&client.nickname.to_string()));
        assert!(!channel.operators.contains(&client2.nickname.to_string()));
        assert!(!channel.operators.contains(&client.nickname.to_string()));
        drop(channels_lock);
        assert_eq!(Err(ServerError::ClientNotOnChannel), result);
    }

    #[test]
    fn test_invite_command_channel_not_found() {
        let listener = TcpListener::bind("127.0.0.1:0".to_string()).unwrap();
        let port = listener.local_addr().unwrap().port();
        let address_port = format!("127.0.0.1:{}", port.to_string());
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: port.to_string(),
            name: "test".to_string(),
            operators: vec![],
            father: None,
            children: HashMap::new(),
        }));

        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));

        let network = Network {
            server: arc_server,
            servers: arc_servers,
            clients: arc_network_clients,
        };

        let (db_tx, db_rx) = std::sync::mpsc::channel::<(PersistenceType, String)>();
        handle_database(db_rx);
        let session = create_session_for_test(db_tx);
        let client =
            create_client_for_test(&session, address_port.to_string(), "nickname".to_string());
        let client2 =
            create_client_for_test(&session, address_port.to_string(), "nickname2".to_string());

        let message = create_message_for_test(
            MessageType::Invite,
            vec![client2.nickname, "#non_existing_channel".to_string()],
        );

        let result = handle_invite_command(
            message,
            client.nickname.to_string(),
            &session,
            &network,
            &"test".to_string(),
        );
        let (mut reader, _addr) = listener.accept().unwrap();
        drop(listener);
        let mut buf = vec![0u8; 510];
        reader.read(&mut buf).unwrap();
        let msg = buf.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
        let response = String::from_utf8(msg).unwrap();

        let response = Response::serialize(response).unwrap();
        match response {
            Response::ErrorResponse {
                response: cmd_response,
            } => match cmd_response {
                ErrorResponse::NoSuchChannel { channel: c } => {
                    assert_eq!(c, "#non_existing_channel".to_string());
                }
                _ => {
                    assert!(false);
                }
            },
            _ => {
                assert!(false);
            }
        }

        assert_eq!(Err(ServerError::ChannelNotFound), result);
    }

    #[test]
    fn test_invite_command_not_invite_only() {
        let listener = TcpListener::bind("127.0.0.1:0".to_string()).unwrap();
        let port = listener.local_addr().unwrap().port();
        let address_port = format!("127.0.0.1:{}", port.to_string());
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: port.to_string(),
            name: "test".to_string(),
            operators: vec![],
            father: None,
            children: HashMap::new(),
        }));

        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));

        let network = Network {
            server: arc_server,
            servers: arc_servers,
            clients: arc_network_clients,
        };

        let (db_tx, db_rx) = std::sync::mpsc::channel::<(PersistenceType, String)>();
        handle_database(db_rx);
        let session = create_session_for_test(db_tx);
        let client =
            create_client_for_test(&session, address_port.to_string(), "nickname".to_string());
        let client2 =
            create_client_for_test(&session, address_port.to_string(), "nickname2".to_string());

        let mut channel = Channel::new("#channel_test".to_string(), "".to_string(), vec![]);
        channel.users = vec![client.nickname.to_string()];
        session
            .channels
            .write()
            .unwrap()
            .insert(channel.name.to_string(), channel.clone());

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(channel.name.clone(), channel.clone());
        drop(channels_lock);

        let message = create_message_for_test(
            MessageType::Invite,
            vec![client2.nickname.to_string(), channel.name.to_string()],
        );

        let result = handle_invite_command(
            message,
            client.nickname.to_string(),
            &session,
            &network,
            &"test".to_string(),
        );
        let (mut reader, _addr) = listener.accept().unwrap();
        let mut buf = vec![0u8; 510];
        reader.read(&mut buf).unwrap();
        let msg = buf.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
        let response = String::from_utf8(msg).unwrap();

        let response = Response::serialize(response).unwrap();
        match response {
            Response::CommandResponse {
                response: cmd_response,
            } => match cmd_response {
                CommandResponse::Inviting {
                    channel: c,
                    nickname: nick,
                } => {
                    assert_eq!(c, channel.name.to_string());
                    assert_eq!(nick, client2.nickname.to_string());
                }
                _ => {
                    assert!(false);
                }
            },
            _ => {
                assert!(false);
            }
        }

        let (mut reader, _addr) = listener.accept().unwrap();
        drop(listener);
        let mut buf = vec![0u8; 510];
        reader.read(&mut buf).unwrap();
        let msg = buf.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
        let response = String::from_utf8(msg).unwrap();

        let response = Response::serialize(response).unwrap();
        match response {
            Response::MessageResponse {
                response: cmd_response,
            } => match cmd_response {
                MessageResponse::InviteMsg { message } => {
                    let expected = format!(
                        "{} has invited you to {}",
                        client.nickname.to_string(),
                        channel.name.to_string()
                    );
                    assert_eq!(expected, message);
                }
                _ => {
                    assert!(false);
                }
            },
            _ => {
                assert!(false);
            }
        }
        let channels_lock = read_lock_channels(&session).unwrap();
        assert_eq!(channels_lock.contains_key("#channel_test"), true);
        let channel = channels_lock.get(&"#channel_test".to_string()).unwrap();
        assert!(channel.users.contains(&client2.nickname.to_string()));
        assert!(channel.users.contains(&client.nickname.to_string()));
        assert!(!channel.operators.contains(&client2.nickname.to_string()));
        assert!(!channel.operators.contains(&client.nickname.to_string()));
        drop(channels_lock);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invite_command_invalid_channel_name() {
        let listener = TcpListener::bind("127.0.0.1:0".to_string()).unwrap();
        let port = listener.local_addr().unwrap().port();
        let address_port = format!("127.0.0.1:{}", port.to_string());
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: port.to_string(),
            name: "test".to_string(),
            operators: vec![],
            father: None,
            children: HashMap::new(),
        }));

        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));

        let network = Network {
            server: arc_server,
            servers: arc_servers,
            clients: arc_network_clients,
        };

        let (db_tx, db_rx) = std::sync::mpsc::channel::<(PersistenceType, String)>();
        handle_database(db_rx);
        let session = create_session_for_test(db_tx);
        let client =
            create_client_for_test(&session, address_port.to_string(), "nickname".to_string());
        let client2 =
            create_client_for_test(&session, address_port.to_string(), "nickname2".to_string());

        let message = create_message_for_test(
            MessageType::Invite,
            vec![client2.nickname.to_string(), "invalid_channel".to_string()],
        );
        let result = handle_invite_command(
            message,
            client.nickname,
            &session,
            &network,
            &"test".to_string(),
        );
        let (mut reader, _addr) = listener.accept().unwrap();
        drop(listener);
        let mut buf = vec![0u8; 510];
        reader.read(&mut buf).unwrap();
        let msg = buf.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
        let response = String::from_utf8(msg).unwrap();

        let response = Response::serialize(response).unwrap();
        match response {
            Response::ErrorResponse {
                response: err_response,
            } => match err_response {
                ErrorResponse::NoSuchChannel { channel: c } => {
                    assert_eq!(c, "invalid_channel".to_string());
                }
                _ => {
                    assert!(false);
                }
            },
            _ => {
                assert!(false);
            }
        }

        assert_eq!(
            Err(ServerError::ChannelMustStartWithHashOrAmpersand),
            result
        );
    }

    #[test]
    fn test_invite_command_banned_user() {
        let listener = TcpListener::bind("127.0.0.1:0".to_string()).unwrap();
        let port = listener.local_addr().unwrap().port();
        let address_port = format!("127.0.0.1:{}", port.to_string());
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: port.to_string(),
            name: "test".to_string(),
            operators: vec![],
            father: None,
            children: HashMap::new(),
        }));

        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));

        let network = Network {
            server: arc_server,
            servers: arc_servers,
            clients: arc_network_clients,
        };

        let (db_tx, db_rx) = std::sync::mpsc::channel::<(PersistenceType, String)>();
        handle_database(db_rx);
        let session = create_session_for_test(db_tx);
        let client =
            create_client_for_test(&session, address_port.to_string(), "nickname".to_string());
        let client2 =
            create_client_for_test(&session, address_port.to_string(), "nickname2".to_string());

        let mut channel = Channel::new("#channel_test".to_string(), "".to_string(), vec![]);
        channel.users = vec![client.nickname.to_string()];
        channel.banned_users.push(client2.nickname.to_string());
        session
            .channels
            .write()
            .unwrap()
            .insert(channel.name.to_string(), channel.clone());

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(channel.name.clone(), channel.clone());
        drop(channels_lock);

        let message = create_message_for_test(
            MessageType::Invite,
            vec![client2.nickname.to_string(), channel.name.to_string()],
        );

        let result = handle_invite_command(
            message,
            client.nickname.to_string(),
            &session,
            &network,
            &"test".to_string(),
        );
        let (mut reader, _addr) = listener.accept().unwrap();
        drop(listener);
        let mut buf = vec![0u8; 510];
        reader.read(&mut buf).unwrap();
        let msg = buf.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
        let response = String::from_utf8(msg).unwrap();

        let response = Response::serialize(response).unwrap();
        match response {
            Response::ErrorResponse {
                response: err_response,
            } => match err_response {
                ErrorResponse::BannedFromChannel { channel: c } => {
                    assert_eq!(c, channel.name.to_string());
                }
                _ => {
                    assert!(false);
                }
            },
            _ => {
                assert!(false);
            }
        }

        let channels_lock = read_lock_channels(&session).unwrap();
        assert_eq!(channels_lock.contains_key("#channel_test"), true);
        let channel = channels_lock.get(&"#channel_test".to_string()).unwrap();
        assert!(!channel.users.contains(&client2.nickname.to_string()));
        assert!(channel.users.contains(&client.nickname.to_string()));
        assert!(!channel.operators.contains(&client2.nickname.to_string()));
        assert!(channel.banned_users.contains(&client2.nickname.to_string()));
        drop(channels_lock);
        assert_eq!(Err(ServerError::UserIsBanned), result);
    }

    #[test]
    fn test_invite_command_user_already_in_channel() {
        let listener = TcpListener::bind("127.0.0.1:0".to_string()).unwrap();
        let port = listener.local_addr().unwrap().port();
        let address_port = format!("127.0.0.1:{}", port.to_string());
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: port.to_string(),
            name: "test".to_string(),
            operators: vec![],
            father: None,
            children: HashMap::new(),
        }));

        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));

        let network = Network {
            server: arc_server,
            servers: arc_servers,
            clients: arc_network_clients,
        };

        let (db_tx, db_rx) = std::sync::mpsc::channel::<(PersistenceType, String)>();
        handle_database(db_rx);
        let session = create_session_for_test(db_tx);
        let client =
            create_client_for_test(&session, address_port.to_string(), "nickname".to_string());
        let client2 =
            create_client_for_test(&session, address_port.to_string(), "nickname2".to_string());

        let mut channel = Channel::new("#channel_test".to_string(), "".to_string(), vec![]);
        channel.users = vec![client.nickname.to_string(), client2.nickname.to_string()];
        session
            .channels
            .write()
            .unwrap()
            .insert(channel.name.to_string(), channel.clone());

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(channel.name.clone(), channel.clone());
        drop(channels_lock);

        let message = create_message_for_test(
            MessageType::Invite,
            vec![client2.nickname.to_string(), channel.name.to_string()],
        );

        let result = handle_invite_command(
            message,
            client.nickname.to_string(),
            &session,
            &network,
            &"test".to_string(),
        );
        let (mut reader, _addr) = listener.accept().unwrap();
        drop(listener);
        let mut buf = vec![0u8; 510];
        reader.read(&mut buf).unwrap();
        let msg = buf.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
        let response = String::from_utf8(msg).unwrap();

        let response = Response::serialize(response).unwrap();
        match response {
            Response::ErrorResponse {
                response: cmd_response,
            } => match cmd_response {
                ErrorResponse::UserOnChannel {
                    nickname: n,
                    channel: c,
                } => {
                    assert_eq!(c, channel.name.to_string());
                    assert_eq!(n, client2.nickname.to_string());
                }
                _ => {
                    assert!(false);
                }
            },
            _ => {
                assert!(false);
            }
        }
        let channels_lock = read_lock_channels(&session).unwrap();
        assert_eq!(channels_lock.contains_key("#channel_test"), true);
        let channel = channels_lock.get(&"#channel_test".to_string()).unwrap();
        assert!(channel.users.contains(&client2.nickname.to_string()));
        assert!(channel.users.contains(&client.nickname.to_string()));
        drop(channels_lock);
        assert_eq!(Err(ServerError::UserAlreadyInChannel), result);
    }

    #[test]
    fn test_invite_command_channel_is_full() {
        let listener = TcpListener::bind("127.0.0.1:0".to_string()).unwrap();
        let port = listener.local_addr().unwrap().port();
        let address_port = format!("127.0.0.1:{}", port.to_string());
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: port.to_string(),
            name: "test".to_string(),
            operators: vec![],
            father: None,
            children: HashMap::new(),
        }));

        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));

        let network = Network {
            server: arc_server,
            servers: arc_servers,
            clients: arc_network_clients,
        };

        let (db_tx, db_rx) = std::sync::mpsc::channel::<(PersistenceType, String)>();
        handle_database(db_rx);
        let session = create_session_for_test(db_tx);
        let client =
            create_client_for_test(&session, address_port.to_string(), "nickname".to_string());
        let client2 =
            create_client_for_test(&session, address_port.to_string(), "nickname2".to_string());

        let mut channel = Channel::new("#channel_test".to_string(), "".to_string(), vec![]);
        channel.users = vec![client.nickname.to_string()];
        channel.limit = Some(1);
        session
            .channels
            .write()
            .unwrap()
            .insert(channel.name.to_string(), channel.clone());

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(channel.name.clone(), channel.clone());
        drop(channels_lock);

        let message = create_message_for_test(
            MessageType::Invite,
            vec![client2.nickname.to_string(), channel.name.to_string()],
        );

        let result = handle_invite_command(
            message,
            client.nickname.to_string(),
            &session,
            &network,
            &"test".to_string(),
        );
        let (mut reader, _addr) = listener.accept().unwrap();
        drop(listener);
        let mut buf = vec![0u8; 510];
        reader.read(&mut buf).unwrap();
        let msg = buf.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
        let response = String::from_utf8(msg).unwrap();

        let response = Response::serialize(response).unwrap();
        match response {
            Response::ErrorResponse {
                response: cmd_response,
            } => match cmd_response {
                ErrorResponse::ChannelIsFull { channel: c } => {
                    assert_eq!(c, channel.name.to_string());
                }
                _ => {
                    assert!(false);
                }
            },
            _ => {
                assert!(false);
            }
        }

        let channels_lock = read_lock_channels(&session).unwrap();
        assert_eq!(channels_lock.contains_key("#channel_test"), true);
        let channel = channels_lock.get(&"#channel_test".to_string()).unwrap();
        assert!(!channel.users.contains(&client2.nickname.to_string()));
        assert!(channel.users.contains(&client.nickname.to_string()));
        assert!(!channel.operators.contains(&client2.nickname.to_string()));
        assert_eq!(channel.limit, Some(1));
        drop(channels_lock);
        assert_eq!(Err(ServerError::ChannelIsFull), result);
    }

    #[test]
    fn test_invite_command_away_user() {
        let listener = TcpListener::bind("127.0.0.1:0".to_string()).unwrap();
        let port = listener.local_addr().unwrap().port();
        let address_port = format!("127.0.0.1:{}", port.to_string());
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: port.to_string(),
            name: "test".to_string(),
            operators: vec![],
            father: None,
            children: HashMap::new(),
        }));

        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));

        let network = Network {
            server: arc_server,
            servers: arc_servers,
            clients: arc_network_clients,
        };

        let (db_tx, db_rx) = std::sync::mpsc::channel::<(PersistenceType, String)>();
        handle_database(db_rx);
        let session = create_session_for_test(db_tx);
        let client =
            create_client_for_test(&session, address_port.to_string(), "nickname".to_string());
        let client2 =
            create_client_for_test(&session, address_port.to_string(), "nickname2".to_string());
        if let Some(client) = write_lock_clients(&session)
            .unwrap()
            .get_mut(&client2.nickname.to_string())
        {
            client.away_message = Some("away".to_string());
        }

        let mut channel = Channel::new("#channel_test".to_string(), "".to_string(), vec![]);
        channel.users = vec![client.nickname.to_string()];
        session
            .channels
            .write()
            .unwrap()
            .insert(channel.name.to_string(), channel.clone());

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(channel.name.clone(), channel.clone());
        drop(channels_lock);

        let message = create_message_for_test(
            MessageType::Invite,
            vec![client2.nickname.to_string(), channel.name.to_string()],
        );

        let result = handle_invite_command(
            message,
            client.nickname.to_string(),
            &session,
            &network,
            &"test".to_string(),
        );
        let (mut reader, _addr) = listener.accept().unwrap();
        drop(listener);
        let mut buf = vec![0u8; 510];
        reader.read(&mut buf).unwrap();
        let msg = buf.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
        let response = String::from_utf8(msg).unwrap();

        let response = Response::serialize(response).unwrap();
        match response {
            Response::CommandResponse {
                response: cmd_response,
            } => {
                match cmd_response {
                    CommandResponse::Away {
                        nickname: n,
                        message: _m,
                    } => {
                        assert_eq!(n, client2.nickname.to_string());
                        //assert_eq!(m, client2.away_message.unwrap());
                    }
                    _ => {
                        assert!(false);
                    }
                }
            }
            _ => {
                assert!(false);
            }
        }

        assert!(result.is_ok());

        let channels_lock = read_lock_channels(&session).unwrap();
        assert_eq!(channels_lock.contains_key("#channel_test"), true);
        let channel = channels_lock.get(&"#channel_test".to_string()).unwrap();
        assert!(!channel.users.contains(&client2.nickname.to_string()));
        assert!(channel.users.contains(&client.nickname.to_string()));
        assert!(!channel.operators.contains(&client2.nickname.to_string()));
        drop(channels_lock);
    }
}
