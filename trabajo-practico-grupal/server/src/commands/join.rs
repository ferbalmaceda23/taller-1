use super::command_utils::write_lock_channels;
use crate::{
    database::inform_database,
    server_errors::ServerError,
    socket::{inform_client, inform_network},
};
use model::{
    channel::Channel,
    channelflag::ChannelFlag,
    message::Message,
    network::Network,
    persistence::PersistenceType,
    responses::{errors::ErrorResponse, replies::CommandResponse},
    session::Session,
};

/// Handles the join command, which joins a client to a channel. If the channel does not exist, it creates it.
/// # Errors
/// ServerError::InvalidParameters if the message does not have the correct number of parameters.
/// ServerError::ChannelMustStartWithHashOrAmpersand if the channel name is invalid.
/// ServerError::UserAlreadyInChannel if the user is already in the channel.
/// ServerError::ChannelIsFull if the channel has a user limmit and is already full.
/// ServerError::ChannelIsInviteOnly if the channel is invite only and the user is not invited.
/// ServerError::ChannelIsBanned if the user is banned from the channel.
/// ServerError::ChannelIsModerated if the channel is moderated and the user is not a channel operator.
/// ServerError::ChannelIsSecret if the channel is secret and the user is not a channel operator.
/// ServerError::ChannelIsPrivate if the channel is private and the user is not a channel operator.
/// ServerError::IncorrectPassword if the channel is password protected and the password is incorrect.
///
/// For each error a message is sent to the client to infomr them of the error.
/// In case of success, a message is sent to the client to know the updated status of the server.
///
pub fn handle_join_command(
    message: Message,
    nickname: &String,
    session: &Session,
    network: &Network,
    server_name: &String,
) -> Result<(), ServerError> {
    if message.parameters.is_empty() {
        let error_response = (ErrorResponse::NeedMoreParams {
            command: "JOIN".to_string(),
        })
        .to_string();
        inform_client(session, nickname, error_response.as_str())?;
        return Err(ServerError::InvalidParameters);
    }
    if !message.parameters[0].starts_with('&') && !message.parameters[0].starts_with('#') {
        let error_response = (ErrorResponse::NoSuchChannel {
            channel: message.parameters[0].clone(),
        })
        .to_string();
        inform_client(session, nickname, error_response.as_str())?;
        return Err(ServerError::ChannelMustStartWithHashOrAmpersand);
    }
    let channels_name = message.parameters[0].to_owned();
    let channels_name = channels_name
        .split(',')
        .into_iter()
        .map(|a| a.trim())
        .collect::<Vec<_>>();
    let mut channels_lock = write_lock_channels(session)?;
    for name in channels_name {
        match channels_lock.get_mut(name) {
            Some(channel) => {
                if channel.users.contains(nickname) {
                    return Err(ServerError::UserAlreadyInChannel);
                }
                if channel.modes.contains(&ChannelFlag::InviteOnly) {
                    let error_response = (ErrorResponse::InviteOnlyChannel {
                        channel: name.to_string(),
                    })
                    .to_string();
                    inform_client(session, nickname, error_response.as_str())?;
                    return Err(ServerError::ChannelIsInviteOnly);
                }
                if channel.modes.contains(&ChannelFlag::Secret) {
                    return Err(ServerError::ChannelIsSecret);
                }
                if channel.banned_users.contains(nickname) {
                    let error_response = (ErrorResponse::BannedFromChannel {
                        channel: name.to_string(),
                    })
                    .to_string();
                    inform_client(session, nickname, error_response.as_str())?;
                    return Err(ServerError::UserIsBanned);
                }
                if let Some(limit) = channel.limit {
                    if channel.users.len() >= (limit as usize) {
                        let error_response = (ErrorResponse::ChannelIsFull {
                            channel: name.to_string(),
                        })
                        .to_string();
                        inform_client(session, nickname, error_response.as_str())?;
                        return Err(ServerError::ChannelIsFull);
                    }
                }
                if channel.password.is_some() {
                    if message.parameters.len() > 1 {
                        if let Some(password) = &channel.password {
                            if password != &message.parameters[1] {
                                let error_response = (ErrorResponse::BadChannelKey {
                                    channel: name.to_string(),
                                })
                                .to_string();
                                inform_client(session, nickname, error_response.as_str())?;
                                return Err(ServerError::IncorrectPassword);
                            }
                        }
                    } else {
                        let error_response = ErrorResponse::BadChannelKey {
                            channel: name.to_string(),
                        }
                        .to_string();
                        inform_client(session, nickname, error_response.as_str())?;
                        return Err(ServerError::MustInsertPassword);
                    }
                }
                channel.users.push(nickname.to_owned());
                inform_database(
                    PersistenceType::ChannelUpdate(channel.name.to_owned()),
                    channel.to_string(),
                    session,
                )?;

                let response = CommandResponse::Topic {
                    channel: channel.name.to_string(),
                    topic: channel.topic.to_string(),
                };
                inform_client(session, nickname, response.to_string().as_str())?;
                println!("Channel joined: {}", channel.name);
                inform_network_about_join(
                    name,
                    message.to_owned(),
                    nickname,
                    network,
                    server_name,
                )?;
            }
            None => {
                let mut channel =
                    Channel::new(name.to_string(), "".to_string(), vec![nickname.to_owned()]);
                channel.operators.push(nickname.to_owned());
                println!("Channel created: {}", channel.name);
                inform_database(PersistenceType::ChannelSave, channel.to_string(), session)?;
                channels_lock.insert(name.to_string(), channel);
                let response = CommandResponse::Topic {
                    channel: name.to_string(),
                    topic: "".to_string(),
                };
                inform_client(session, nickname, response.to_string().as_str())?;
                inform_network_about_join(
                    name,
                    message.to_owned(),
                    nickname,
                    network,
                    server_name,
                )?;
            }
        }
    }
    drop(channels_lock);
    Ok(())
}

fn inform_network_about_join(
    channel_name: &str,
    message: Message,
    nickname: &String,
    network: &Network,
    server_name: &String,
) -> Result<(), ServerError> {
    if channel_name.starts_with('#') {
        let mut msg = message;
        msg.prefix = Some(nickname.to_owned());
        msg.parameters[0] = channel_name.to_string();
        let msg = Message::deserialize(msg)?;
        inform_network(network, server_name, &msg)?;
    }

    Ok(())
}

#[cfg(test)]
mod join_tests {
    use std::collections::HashMap;
    use std::io::Read;
    use std::net::TcpListener;
    use std::sync::{Arc, RwLock};
    use std::vec;

    use crate::commands::command_utils::{
        create_client_for_test, create_message_for_test, create_session_for_test,
        read_lock_channels, write_lock_channels,
    };
    use crate::commands::join::handle_join_command;
    use crate::database::handle_database;
    use crate::server_errors::ServerError;
    use model::channel::Channel;
    use model::channelflag::ChannelFlag;
    use model::message::MessageType;
    use model::network::Network;
    use model::persistence::PersistenceType;
    use model::responses::errors::ErrorResponse;
    use model::responses::replies::CommandResponse;
    use model::responses::response::Response;
    use model::server::Server;

    #[test]
    fn test_command_join_existing_channel_doesnt_make_user_operator() {
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

        let mut channel = Channel::new("#channel_test".to_string(), "".to_string(), vec![]);
        channel.users.push("existing_nickname".to_string());
        channel.operators.push("existing_nickname".to_string());

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(channel.name.clone(), channel.clone());

        assert_eq!(channels_lock.contains_key("#channel_test"), true);
        let channels_lock_clone = channels_lock.clone();
        let channel = channels_lock_clone
            .get(&"#channel_test".to_string())
            .unwrap();
        assert_eq!(
            channel.users.contains(&("existing_nickname".to_string())),
            true
        );
        assert_eq!(channel.users.contains(&("nickname".to_string())), false);

        drop(channels_lock);

        let message = create_message_for_test(MessageType::Join, vec![channel.name.to_string()]);

        let result = handle_join_command(
            message,
            &client.nickname.to_string(),
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

        let channels_lock = read_lock_channels(&session).unwrap();

        assert_eq!(channels_lock.contains_key("#channel_test"), true);
        let channel = channels_lock.get(&"#channel_test".to_string()).unwrap();
        assert!(channel.users.contains(&("nickname".to_string())));
        assert!(!channel.operators.contains(&("nickname".to_string())));
        assert!(channel
            .operators
            .contains(&("existing_nickname".to_string())));

        let response = Response::serialize(response).unwrap();
        match response {
            Response::CommandResponse {
                response: cmd_response,
            } => match cmd_response {
                CommandResponse::Topic { channel: c, topic } => {
                    assert_eq!(c, channel.name.to_string());
                    assert_eq!(topic, channel.topic.to_string());
                }
                _ => {
                    assert!(false);
                }
            },
            _ => {
                assert!(false);
            }
        }

        drop(channels_lock);
        assert!(result.is_ok());
    }

    #[test]
    fn test_command_join_with_no_existing_channel_creates_a_new_one() {
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

        let channels_lock = write_lock_channels(&session).unwrap();
        assert!(!channels_lock.contains_key("#channel_test"));

        drop(channels_lock);

        let message = create_message_for_test(MessageType::Join, vec!["#channel_test".to_string()]);

        let result = handle_join_command(
            message,
            &client.nickname.to_string(),
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

        let channels_lock = read_lock_channels(&session).unwrap();
        assert_eq!(channels_lock.contains_key("#channel_test"), true);
        let channel = channels_lock.get(&"#channel_test".to_string()).unwrap();
        assert_eq!(channel.users.contains(&("nickname".to_string())), true);

        let response = Response::serialize(response).unwrap();
        match response {
            Response::CommandResponse {
                response: cmd_response,
            } => match cmd_response {
                CommandResponse::Topic { channel: c, topic } => {
                    assert_eq!(c, channel.name.to_string());
                    assert_eq!(topic, "".to_string());
                }
                _ => {
                    assert!(false);
                }
            },
            _ => {
                assert!(false);
            }
        }

        drop(channels_lock);
        assert!(result.is_ok());
    }

    #[test]
    fn test_command_join_user_is_operator_when_creating_a_channel() {
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

        let channels_lock = write_lock_channels(&session).unwrap();
        assert!(!channels_lock.contains_key("#channel_test"));
        drop(channels_lock);

        let message = create_message_for_test(MessageType::Join, vec!["#channel_test".to_string()]);

        let result = handle_join_command(
            message,
            &client.nickname.to_string(),
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

        let channels_lock = read_lock_channels(&session).unwrap();
        assert_eq!(channels_lock.contains_key("#channel_test"), true);
        let channel = channels_lock.get(&"#channel_test".to_string()).unwrap();
        assert!(channel.users.contains(&("nickname".to_string())));
        assert!(channel.operators.contains(&("nickname".to_string())));

        let response = Response::serialize(response).unwrap();
        match response {
            Response::CommandResponse {
                response: cmd_response,
            } => match cmd_response {
                CommandResponse::Topic { channel: c, topic } => {
                    assert_eq!(c, channel.name.to_string());
                    assert_eq!(topic, "".to_string());
                }
                _ => {
                    assert!(false);
                }
            },
            _ => {
                assert!(false);
            }
        }
        drop(channels_lock);
        assert!(result.is_ok());
    }

    #[test]
    fn test_command_join_invalid_parameters() {
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

        let channels_lock = write_lock_channels(&session).unwrap();
        assert!(!channels_lock.contains_key("#channel_test"));
        drop(channels_lock);

        let message = create_message_for_test(MessageType::Join, vec![]);

        let channels_lock = read_lock_channels(&session).unwrap();
        assert!(!channels_lock.contains_key("#channel_test"));
        drop(channels_lock);

        let result = handle_join_command(
            message,
            &client.nickname.to_string(),
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
                ErrorResponse::NeedMoreParams { command } => {
                    assert_eq!(command, "JOIN".to_string());
                }
                _ => {
                    assert!(false);
                }
            },
            _ => {
                assert!(false);
            }
        }
        assert_eq!(result, Err(ServerError::InvalidParameters));
    }

    #[test]
    fn test_command_join_invalid_channel_name() {
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

        let message =
            create_message_for_test(MessageType::Join, vec!["invalid_channel".to_string()]);

        let result = handle_join_command(
            message,
            &client.nickname.to_string(),
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

        let channels_lock = read_lock_channels(&session).unwrap();
        assert!(!channels_lock.contains_key("invalid_channel"));

        drop(channels_lock);
        assert_eq!(
            Err(ServerError::ChannelMustStartWithHashOrAmpersand),
            result
        );
    }

    #[test]
    fn test_command_join_user_cant_join_twice() {
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

        let channels_lock = read_lock_channels(&session).unwrap();
        assert!(!channels_lock.contains_key("#channel_test"));
        drop(channels_lock);

        let message = create_message_for_test(MessageType::Join, vec!["#channel_test".to_string()]);

        let result = handle_join_command(
            message.clone(),
            &client.nickname.to_string(),
            &session,
            &network,
            &"test".to_string(),
        );
        let (mut reader, _addr) = listener.accept().unwrap();
        drop(listener);
        let mut buf = vec![0u8; 510];
        reader.read(&mut buf).unwrap();

        assert!(result.is_ok());
        assert_eq!(
            Err(ServerError::UserAlreadyInChannel),
            handle_join_command(
                message,
                &client.nickname.to_string(),
                &session,
                &network,
                &"test".to_string()
            )
        );
    }

    #[test]
    fn test_command_join_cant_join_secret_channel() {
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

        let mut channels_write_lock = write_lock_channels(&session).unwrap();
        channels_write_lock.insert(
            "#secret".to_string(),
            Channel {
                name: "#secret".to_string(),
                users: vec![],
                operators: vec![],
                topic: "".to_string(),
                modes: vec![ChannelFlag::Secret],
                banned_users: vec![],
                password: None,
                limit: None,
                moderators: vec![],
            },
        );
        drop(channels_write_lock);

        let message = create_message_for_test(MessageType::Join, vec!["#secret".to_string()]);

        let result = handle_join_command(
            message,
            &client.nickname.to_string(),
            &session,
            &network,
            &"test".to_string(),
        );
        drop(listener);

        let channels_lock = read_lock_channels(&session).unwrap();
        let channel = channels_lock.get("#secret").unwrap();
        assert!(!channel.users.contains(&client.nickname.to_string()));
        drop(channels_lock);
        assert_eq!(Err(ServerError::ChannelIsSecret), result);
    }

    #[test]
    fn test_command_join_cant_join_invite_only_channel() {
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

        let message = create_message_for_test(MessageType::Join, vec!["#invite".to_string()]);

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(
            "#invite".to_string(),
            Channel {
                name: "#invite".to_string(),
                users: vec![],
                operators: vec![],
                topic: "".to_string(),
                modes: vec![ChannelFlag::InviteOnly],
                banned_users: vec![],
                password: None,
                limit: None,
                moderators: vec![],
            },
        );
        drop(channels_lock);

        let result = handle_join_command(
            message,
            &client.nickname.to_string(),
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
                ErrorResponse::InviteOnlyChannel { channel: c } => {
                    assert_eq!(c, "#invite".to_string());
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
        let channel = channels_lock.get("#invite").unwrap();
        assert!(!channel.users.contains(&client.nickname.to_string()));
        drop(channels_lock);
        assert_eq!(Err(ServerError::ChannelIsInviteOnly), result);
    }

    #[test]
    fn test_command_join_cant_join_full_channel() {
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

        let message = create_message_for_test(MessageType::Join, vec!["#full".to_string()]);

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(
            "#full".to_string(),
            Channel {
                name: "#full".to_string(),
                users: vec!["user1".to_string()],
                operators: vec![],
                topic: "".to_string(),
                modes: vec![],
                banned_users: vec![],
                password: None,
                limit: Some(1),
                moderators: vec![],
            },
        );
        drop(channels_lock);

        let result = handle_join_command(
            message,
            &client.nickname.to_string(),
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
                ErrorResponse::ChannelIsFull { channel: c } => {
                    assert_eq!(c, "#full".to_string());
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
        let channel = channels_lock.get("#full").unwrap();
        assert!(!channel.users.contains(&client.nickname.to_string()));
        drop(channels_lock);
        assert_eq!(Err(ServerError::ChannelIsFull), result);
    }

    #[test]
    fn test_command_join_banned_user_cant_join() {
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

        let message = create_message_for_test(MessageType::Join, vec!["#banned".to_string()]);

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(
            "#banned".to_string(),
            Channel {
                name: "#banned".to_string(),
                users: vec!["user1".to_string()],
                operators: vec!["user1".to_string()],
                topic: "".to_string(),
                modes: vec![],
                banned_users: vec![client.nickname.to_string()],
                password: None,
                limit: None,
                moderators: vec![],
            },
        );
        drop(channels_lock);

        let result = handle_join_command(
            message,
            &client.nickname.to_string(),
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
                    assert_eq!(c, "#banned".to_string());
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
        let channel = channels_lock.get("#banned").unwrap();
        assert!(!channel.users.contains(&client.nickname.to_string()));
        drop(channels_lock);
        assert_eq!(Err(ServerError::UserIsBanned), result);
    }

    #[test]
    fn test_command_join_no_password_provided() {
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

        let message = create_message_for_test(MessageType::Join, vec!["#pass".to_string()]);

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(
            "#pass".to_string(),
            Channel {
                name: "#pass".to_string(),
                users: vec!["user1".to_string()],
                operators: vec!["user1".to_string()],
                topic: "".to_string(),
                modes: vec![],
                banned_users: vec![],
                password: Some("123".to_string()),
                limit: None,
                moderators: vec![],
            },
        );
        drop(channels_lock);

        let result = handle_join_command(
            message,
            &client.nickname.to_string(),
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
                ErrorResponse::BadChannelKey { channel: c } => {
                    assert_eq!(c, "#pass".to_string());
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
        let channel = channels_lock.get("#pass").unwrap();
        assert!(!channel.users.contains(&client.nickname.to_string()));
        drop(channels_lock);
        assert_eq!(Err(ServerError::MustInsertPassword), result);
    }

    #[test]
    pub fn test_command_join_invalid_password_provided() {
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

        let message = create_message_for_test(
            MessageType::Join,
            vec!["#pass".to_string(), "321".to_string()],
        );

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(
            "#pass".to_string(),
            Channel {
                name: "#pass".to_string(),
                users: vec!["user1".to_string()],
                operators: vec!["user1".to_string()],
                topic: "".to_string(),
                modes: vec![],
                banned_users: vec![],
                password: Some("123".to_string()),
                limit: None,
                moderators: vec![],
            },
        );
        drop(channels_lock);

        let result = handle_join_command(
            message,
            &client.nickname.to_string(),
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
                ErrorResponse::BadChannelKey { channel: c } => {
                    assert_eq!(c, "#pass".to_string());
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
        let channel = channels_lock.get("#pass").unwrap();
        assert!(!channel.users.contains(&client.nickname.to_string()));
        drop(channels_lock);
        assert_eq!(Err(ServerError::IncorrectPassword), result);
    }

    #[test]
    pub fn test_command_join_valid_password_provided() {
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

        let message = create_message_for_test(
            MessageType::Join,
            vec!["#pass".to_string(), "123".to_string()],
        );

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(
            "#pass".to_string(),
            Channel {
                name: "#pass".to_string(),
                users: vec!["user1".to_string()],
                operators: vec!["user1".to_string()],
                topic: "".to_string(),
                modes: vec![],
                banned_users: vec![],
                password: Some("123".to_string()),
                limit: None,
                moderators: vec![],
            },
        );
        drop(channels_lock);

        let result = handle_join_command(
            message,
            &client.nickname.to_string(),
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
        let channels_lock = read_lock_channels(&session).unwrap();
        let channel = channels_lock.get("#pass").unwrap();
        match response {
            Response::CommandResponse {
                response: cmd_response,
            } => match cmd_response {
                CommandResponse::Topic { channel: c, topic } => {
                    assert_eq!(c, channel.name.to_string());
                    assert_eq!(topic, channel.topic.to_string());
                }
                _ => {
                    assert!(false);
                }
            },
            _ => {
                assert!(false);
            }
        }

        assert!(channel.users.contains(&client.nickname.to_string()));
        drop(channels_lock);
        assert!(result.is_ok());
    }
}
