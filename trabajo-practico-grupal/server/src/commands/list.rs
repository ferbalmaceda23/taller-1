use super::command_utils::write_lock_channels;
use crate::{
    server_errors::ServerError,
    socket::{inform_client, inform_server},
};
use model::{
    channelflag::ChannelFlag, message::Message, network::Network,
    responses::replies::CommandResponse, session::Session,
};

/// Function that handles the list command.
/// If no parameters are received in the message, then it returns
/// the list of all channels.
/// Else it returns the list of channels that match the parameters.
///
/// #Errors
/// ServerError::InvalidParameters - If the parameters are invalid.
///
pub fn handle_list_command(
    message: Message,
    nickname: &String,
    session: &Session,
    network: &Network,
    server_name: Option<String>,
) -> Result<(), ServerError> {
    if message.parameters.len() > 1 {
        return Err(ServerError::InvalidParameters);
    }
    let channels_lock = write_lock_channels(session)?;
    let response = CommandResponse::ListStart.to_string();
    if let Some(name) = server_name.to_owned() {
        inform_server(network, &name, &response)?;
    } else {
        inform_client(session, nickname, &response)?;
    }
    if message.parameters.len() == 1 {
        let channels_name = message.parameters[0]
            .split(',')
            .into_iter()
            .map(|a| a.trim())
            .collect::<Vec<_>>();
        for name in channels_name {
            if let Some(channel) = channels_lock.get(name) {
                save_channels_and_topics(
                    channel,
                    nickname,
                    session,
                    network,
                    server_name.to_owned(),
                )?;
            }
        }
    } else if message.parameters.is_empty() {
        for channel in channels_lock.values() {
            save_channels_and_topics(channel, nickname, session, network, server_name.to_owned())?;
        }
    }

    drop(channels_lock);
    let response = CommandResponse::ListEnd.to_string();
    if let Some(name) = server_name {
        inform_server(network, &name, &response)?;
    } else {
        inform_client(session, nickname, &response)?;
    }
    Ok(())
}

fn save_channels_and_topics(
    channel: &model::channel::Channel,
    nickname: &String,
    session: &Session,
    network: &Network,
    server_name: Option<String>,
) -> Result<(), ServerError> {
    if channel.modes.contains(&ChannelFlag::Private)
        && !channel.modes.contains(&ChannelFlag::Secret)
        && !channel.users.contains(nickname)
    {
        println!("Channel: {:?} is Private", channel.name);
    } else {
        let response = CommandResponse::List {
            channel: channel.name.clone(),
            topic: channel.topic.clone(),
        }
        .to_string();
        if let Some(name) = server_name {
            inform_server(network, &name, &response)?;
        } else {
            inform_client(session, nickname, &response)?;
        }
        println!("Channel: {:?} topic is: {:?}", channel.name, channel.topic);
    }
    Ok(())
}

#[cfg(test)]
mod list_tests {
    use std::collections::HashMap;
    use std::io::Read;
    use std::net::TcpListener;
    use std::sync::{Arc, RwLock};

    use model::channel::Channel;
    use model::channelflag::ChannelFlag;
    use model::message::MessageType;
    use model::network::Network;
    use model::persistence::PersistenceType;
    use model::responses::replies::CommandResponse;
    use model::responses::response::Response;
    use model::server::Server;

    use crate::commands::command_utils::{
        create_client_for_test, create_message_for_test, create_session_for_test,
        write_lock_channels,
    };

    use crate::commands::list::handle_list_command;
    use crate::database::handle_database;
    use crate::server_errors::ServerError;

    #[test]
    fn test_list_command_invalid_parameters() {
        let listener = TcpListener::bind("127.0.0.1:0".to_string()).unwrap();
        let port = listener.local_addr().unwrap().port();
        let address_port = format!("127.0.0.1:{}", port.to_string());
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: port.to_string(),
            name: "test".to_string(),
            operators: vec![],
            father: None,
            children: HashMap::new(),
        }));

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
            MessageType::List,
            vec!["&channel1".to_string(), "&channel2".to_string()],
        );
        let result = handle_list_command(message, &client.nickname, &session, &network, None);
        drop(listener);
        assert_eq!(Err(ServerError::InvalidParameters), result);
    }

    #[test]
    fn test_list_command_no_parameters() {
        let listener = TcpListener::bind("127.0.0.1:0".to_string()).unwrap();
        let port = listener.local_addr().unwrap().port();
        let address_port = format!("127.0.0.1:{}", port.to_string());
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: port.to_string(),
            name: "test".to_string(),
            operators: vec![],
            father: None,
            children: HashMap::new(),
        }));

        let network = Network {
            server: arc_server,
            servers: arc_servers,
            clients: arc_network_clients,
        };

        let (db_tx, db_rx) = std::sync::mpsc::channel::<(PersistenceType, String)>();
        handle_database(db_rx);
        let session = create_session_for_test(db_tx);
        let client1 =
            create_client_for_test(&session, address_port.to_string(), "nickname".to_string());
        let client2 =
            create_client_for_test(&session, address_port.to_string(), "nickname2".to_string());

        let mut channel1 = Channel::new("#channel_test1".to_string(), "topic1".to_string(), vec![]);
        channel1.users.push(client1.nickname.to_string());
        channel1.operators.push(client1.nickname.to_string());
        channel1.users.push(client2.nickname.to_string());
        channel1.operators.push(client2.nickname.to_string());

        let mut channel2 = Channel::new("#channel_test2".to_string(), "topic2".to_string(), vec![]);
        channel2.users.push(client1.nickname.to_string());
        channel2.users.push(client2.nickname.to_string());
        channel2.operators.push(client2.nickname.to_string());

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(channel1.name.clone(), channel1.clone());
        channels_lock.insert(channel2.name.clone(), channel2.clone());
        drop(channels_lock);

        let message = create_message_for_test(MessageType::List, vec![]);

        let result =
            handle_list_command(message, &"nickname".to_string(), &session, &network, None);
        let (mut reader, _addr) = listener.accept().unwrap();
        let mut buf = vec![0u8; 510];
        reader.read(&mut buf).unwrap();
        let msg = buf.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
        let response = String::from_utf8(msg).unwrap();

        let mut channel1_listed = false;
        let mut channel2_listed = false;

        let response = Response::serialize(response).unwrap();
        match response {
            Response::CommandResponse {
                response: cmd_response,
            } => match cmd_response {
                CommandResponse::ListStart => {
                    assert!(true);
                }
                _ => {
                    assert!(false);
                }
            },
            _ => {
                assert!(false);
            }
        }

        let mut buf = vec![0u8; 510];
        reader.read(&mut buf).unwrap();
        let msg = buf.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
        let response = String::from_utf8(msg).unwrap();

        let response = Response::serialize(response).unwrap();
        match response {
            Response::CommandResponse {
                response: cmd_response,
            } => match cmd_response {
                CommandResponse::List { channel: c, topic } => {
                    if c == channel1.name && topic == channel1.topic {
                        channel1_listed = true;
                    } else if c == channel2.name && topic == channel2.topic {
                        channel2_listed = true;
                    } else {
                        assert!(false);
                    }
                }
                _ => {
                    assert!(false);
                }
            },
            _ => {
                assert!(false);
            }
        }

        let mut buf = vec![0u8; 510];
        reader.read(&mut buf).unwrap();
        let msg = buf.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
        let response = String::from_utf8(msg).unwrap();

        let response = Response::serialize(response).unwrap();
        match response {
            Response::CommandResponse {
                response: cmd_response,
            } => match cmd_response {
                CommandResponse::List { channel: c, topic } => {
                    if channel1_listed {
                        assert_eq!(c, channel2.name);
                        assert_eq!(topic, channel2.topic);
                    } else if channel2_listed {
                        assert_eq!(c, channel1.name);
                        assert_eq!(topic, channel1.topic);
                    } else {
                        assert!(false);
                    }
                }
                _ => {
                    assert!(false);
                }
            },
            _ => {
                assert!(false);
            }
        }

        let mut buf = vec![0u8; 510];
        reader.read(&mut buf).unwrap();
        let msg = buf.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
        let response = String::from_utf8(msg).unwrap();

        let response = Response::serialize(response).unwrap();
        match response {
            Response::CommandResponse {
                response: cmd_response,
            } => match cmd_response {
                CommandResponse::ListEnd => {
                    assert!(true);
                }
                _ => {
                    assert!(false);
                }
            },
            _ => {
                assert!(false);
            }
        }
        drop(listener);
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_command_one_parameter() {
        let listener = TcpListener::bind("127.0.0.1:0".to_string()).unwrap();
        let port = listener.local_addr().unwrap().port();
        let address_port = format!("127.0.0.1:{}", port.to_string());
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: port.to_string(),
            name: "test".to_string(),
            operators: vec![],
            father: None,
            children: HashMap::new(),
        }));

        let network = Network {
            server: arc_server,
            servers: arc_servers,
            clients: arc_network_clients,
        };

        let (db_tx, db_rx) = std::sync::mpsc::channel::<(PersistenceType, String)>();
        handle_database(db_rx);
        let session = create_session_for_test(db_tx);
        let client1 =
            create_client_for_test(&session, address_port.to_string(), "nickname".to_string());

        let mut channel = Channel::new(
            "#channel_test".to_string(),
            "test_topic".to_string(),
            vec![],
        );
        channel.users.push(client1.nickname.to_string());
        channel.operators.push(client1.nickname.to_string());

        let mut channel2 = Channel::new("#channel_test2".to_string(), "".to_string(), vec![]);
        channel2.users.push(client1.nickname.to_string());

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(channel.name.clone(), channel.clone());
        channels_lock.insert(channel2.name.clone(), channel2.clone());
        drop(channels_lock);

        let message = create_message_for_test(MessageType::List, vec![channel.name.to_string()]);

        let result =
            handle_list_command(message, &"nickname".to_string(), &session, &network, None);
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
                CommandResponse::ListStart => {
                    assert!(true);
                }
                _ => {
                    assert!(false);
                }
            },
            _ => {
                assert!(false);
            }
        }

        let mut buf = vec![0u8; 510];
        reader.read(&mut buf).unwrap();
        let msg = buf.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
        let response = String::from_utf8(msg).unwrap();

        let response = Response::serialize(response).unwrap();
        match response {
            Response::CommandResponse {
                response: cmd_response,
            } => match cmd_response {
                CommandResponse::List { channel: c, topic } => {
                    assert_eq!(c, channel.name);
                    assert_eq!(topic, channel.topic);
                }
                _ => {
                    assert!(false);
                }
            },
            _ => {
                assert!(false);
            }
        }

        let mut buf = vec![0u8; 510];
        reader.read(&mut buf).unwrap();
        let msg = buf.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
        let response = String::from_utf8(msg).unwrap();

        let response = Response::serialize(response).unwrap();
        match response {
            Response::CommandResponse {
                response: cmd_response,
            } => match cmd_response {
                CommandResponse::ListEnd => {
                    assert!(true);
                }
                _ => {
                    assert!(false);
                }
            },
            _ => {
                assert!(false);
            }
        }

        drop(listener);
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_command_multiple_channels() {
        let listener = TcpListener::bind("127.0.0.1:8112".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8112".to_string(),
            name: "test".to_string(),
            operators: vec![],
            father: None,
            children: HashMap::new(),
        }));

        let network = Network {
            server: arc_server,
            servers: arc_servers,
            clients: arc_network_clients,
        };

        let (db_tx, db_rx) = std::sync::mpsc::channel::<(PersistenceType, String)>();
        handle_database(db_rx);
        let session = create_session_for_test(db_tx);
        let client1 = create_client_for_test(
            &session,
            "127.0.0.1:8112".to_string(),
            "nickname".to_string(),
        );
        let client2 = create_client_for_test(
            &session,
            "127.0.0.1:8112".to_string(),
            "nickname2".to_string(),
        );

        let mut channel1 = Channel::new("#channel_test1".to_string(), "topic1".to_string(), vec![]);
        channel1.users.push(client1.nickname.to_string());
        channel1.operators.push(client1.nickname.to_string());
        channel1.users.push(client2.nickname.to_string());
        channel1.operators.push(client2.nickname.to_string());

        let mut channel2 = Channel::new("#channel_test2".to_string(), "topic2".to_string(), vec![]);
        channel2.users.push(client1.nickname.to_string());
        channel2.users.push(client2.nickname.to_string());
        channel2.operators.push(client2.nickname.to_string());

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(channel1.name.clone(), channel1.clone());
        channels_lock.insert(channel2.name.clone(), channel2.clone());
        drop(channels_lock);

        let message = create_message_for_test(
            MessageType::List,
            vec!["#channel_test1,#channel_test2".to_string()],
        );

        let result =
            handle_list_command(message, &"nickname".to_string(), &session, &network, None);
        let (mut reader, _addr) = listener.accept().unwrap();
        let mut buf = vec![0u8; 510];
        reader.read(&mut buf).unwrap();
        let msg = buf.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
        let response = String::from_utf8(msg).unwrap();

        let mut channel1_listed = false;
        let mut channel2_listed = false;

        let response = Response::serialize(response).unwrap();
        match response {
            Response::CommandResponse {
                response: cmd_response,
            } => match cmd_response {
                CommandResponse::ListStart => {
                    assert!(true);
                }
                _ => {
                    assert!(false);
                }
            },
            _ => {
                assert!(false);
            }
        }

        let mut buf = vec![0u8; 510];
        reader.read(&mut buf).unwrap();

        let msg = buf.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
        let response = String::from_utf8(msg).unwrap();

        let response = Response::serialize(response).unwrap();
        match response {
            Response::CommandResponse {
                response: cmd_response,
            } => match cmd_response {
                CommandResponse::List { channel: c, topic } => {
                    if c == channel1.name && topic == channel1.topic {
                        channel1_listed = true;
                    } else if c == channel2.name && topic == channel2.topic {
                        channel2_listed = true;
                    } else {
                        assert!(false);
                    }
                }
                _ => {
                    assert!(false);
                }
            },
            _ => {
                assert!(false);
            }
        }

        let mut buf = vec![0u8; 510];
        reader.read(&mut buf).unwrap();
        let msg = buf.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
        let response = String::from_utf8(msg).unwrap();

        let response = Response::serialize(response).unwrap();
        match response {
            Response::CommandResponse {
                response: cmd_response,
            } => match cmd_response {
                CommandResponse::List { channel: c, topic } => {
                    if channel1_listed {
                        assert_eq!(c, channel2.name);
                        assert_eq!(topic, channel2.topic);
                    } else if channel2_listed {
                        assert_eq!(c, channel1.name);
                        assert_eq!(topic, channel1.topic);
                    } else {
                        assert!(false);
                    }
                }
                _ => {
                    assert!(false);
                }
            },
            _ => {
                assert!(false);
            }
        }

        let mut buf = vec![0u8; 510];
        reader.read(&mut buf).unwrap();
        let msg = buf.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
        let response = String::from_utf8(msg).unwrap();

        let response = Response::serialize(response).unwrap();
        match response {
            Response::CommandResponse {
                response: cmd_response,
            } => match cmd_response {
                CommandResponse::ListEnd => {
                    assert!(true);
                }
                _ => {
                    assert!(false);
                }
            },
            _ => {
                assert!(false);
            }
        }
        drop(listener);
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_command_private_channel() {
        let listener = TcpListener::bind("127.0.0.1:8113".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8113".to_string(),
            name: "test".to_string(),
            operators: vec![],
            father: None,
            children: HashMap::new(),
        }));

        let network = Network {
            server: arc_server,
            servers: arc_servers,
            clients: arc_network_clients,
        };

        let (db_tx, db_rx) = std::sync::mpsc::channel::<(PersistenceType, String)>();
        handle_database(db_rx);
        let session = create_session_for_test(db_tx);
        let client1 = create_client_for_test(
            &session,
            "127.0.0.1:8113".to_string(),
            "nickname".to_string(),
        );

        let mut channel = Channel::new(
            "#channel_test".to_string(),
            "test_topic".to_string(),
            vec![],
        );
        channel.modes.push(ChannelFlag::Private);

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(channel.name.clone(), channel.clone());
        drop(channels_lock);

        let message = create_message_for_test(MessageType::List, vec![channel.name.to_string()]);

        let result = handle_list_command(
            message,
            &client1.nickname.to_string(),
            &session,
            &network,
            None,
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
                CommandResponse::ListStart => {
                    assert!(true);
                }
                _ => {
                    assert!(false);
                }
            },
            _ => {
                assert!(false);
            }
        }

        let mut buf = vec![0u8; 510];
        reader.read(&mut buf).unwrap();
        let msg = buf.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
        let response = String::from_utf8(msg).unwrap();

        let response = Response::serialize(response).unwrap();
        match response {
            Response::CommandResponse {
                response: cmd_response,
            } => match cmd_response {
                CommandResponse::ListEnd => {
                    assert!(true);
                }
                _ => {
                    assert!(false);
                }
            },
            _ => {
                assert!(false);
            }
        }

        drop(listener);
        assert!(result.is_ok());
    }
}
