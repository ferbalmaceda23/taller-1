use super::command_utils::{read_lock_channels, read_lock_clients};
use crate::{
    server_errors::ServerError,
    socket::{inform_client, inform_server},
};
use model::{
    channelflag::ChannelFlag,
    message::Message,
    network::Network,
    responses::{errors::ErrorResponse, replies::CommandResponse},
    session::Session,
    userflag::UserFlag,
};
use std::collections::HashSet;

/// Returns a list of all users on the server if there are not any parameters, or a list of all users on the server
/// matching the given parameters.
/// # Errors
/// * `ServerError::InvalidParameters`: If the command is not followed by enough parameters. It will send the client a response with the error ErrorResponse::NeedMoreParams.
/// * `ServerError::ChannelNotFound`: If the channel that was requested does not exist. It will send the client a response with the error ErrorResponse::NoSuchChannel.
pub fn handle_who_command(
    message: Message,
    nickname: String,
    session: &Session,
    network: &Network,
    server_name: Option<String>,
) -> Result<(), ServerError> {
    if message.parameters.len() > 1 {
        let response = (ErrorResponse::NeedMoreParams {
            command: "WHO".to_string(),
        })
        .to_string();
        inform_client(session, &nickname, &response)?;
        return Err(ServerError::InvalidParameters);
    }
    let mut clients_to_display: Vec<String> = vec![];
    if message.parameters.is_empty() {
        let mut visible_users: Vec<String> = Vec::new();
        let channels_lock = read_lock_channels(session)?;
        for channel in channels_lock.values() {
            if channel.users.contains(&nickname) {
                let mut users_aux: Vec<String> = channel.users.clone();
                visible_users.append(&mut users_aux);
            }
        }
        drop(channels_lock);
        let visible_users: HashSet<String> = HashSet::from_iter(visible_users);
        let clients_lock = read_lock_clients(session)?;
        for c in clients_lock.values() {
            if !visible_users.contains(&c.nickname) && !c.modes.contains(&UserFlag::Invisible) {
                clients_to_display.push(c.nickname.clone());
            }
        }
        drop(clients_lock);
    } else if message.parameters[0].starts_with('&') || message.parameters[0].starts_with('#') {
        let channels_lock = read_lock_channels(session)?;
        let channel_name = message.parameters[0].to_string();
        match channels_lock.get(&channel_name) {
            Some(channel) => {
                if channel.users.contains(&nickname)
                    || !channel.modes.contains(&ChannelFlag::Private)
                {
                    println!("Channel: {}", channel.name);
                    clients_to_display = channel.users.clone();
                }
            }
            None => {
                return Err(ServerError::ChannelNotFound);
            }
        }
        drop(channels_lock);
    } else {
        let clients_lock = read_lock_clients(session)?;
        for (n, c) in clients_lock.iter() {
            if *n == message.parameters[0]
                || c.username == message.parameters[0]
                || c.hostname == message.parameters[0]
                || c.servername == message.parameters[0]
                || c.realname == message.parameters[0]
            {
                clients_to_display.push(c.nickname.clone());
            }
        }
        drop(clients_lock);
    }

    let network_clients = network.clients.as_ref().read()?;
    for c in network_clients.keys().clone() {
        if !clients_to_display.contains(c) {
            clients_to_display.push(c.to_owned());
        }
    }
    drop(network_clients);
    println!("Matching users: {:?}", clients_to_display);

    let response = (CommandResponse::WhoReply {
        users: clients_to_display,
    })
    .to_string();
    if let Some(name) = server_name.to_owned() {
        inform_server(network, &name, &response)?;
    } else {
        inform_client(session, &nickname, response.as_str())?;
    }

    let response = CommandResponse::EndOfWho.to_string();
    if let Some(name) = server_name {
        inform_server(network, &name, &response)?;
    } else {
        inform_client(session, &nickname, &response)?;
    }

    Ok(())
}

#[cfg(test)]
mod who_tests {
    use std::collections::HashMap;
    use std::io::Read;
    use std::net::TcpListener;
    use std::sync::{Arc, RwLock};

    use model::channel::Channel;
    use model::message::MessageType;
    use model::network::Network;
    use model::persistence::PersistenceType;
    use model::responses::errors::ErrorResponse;
    use model::responses::replies::CommandResponse;
    use model::responses::response::Response;
    use model::server::Server;

    use crate::commands::command_utils::{
        create_client_for_test, create_message_for_test, create_session_for_test,
        write_lock_channels,
    };
    use crate::commands::who::handle_who_command;
    use crate::database::handle_database;
    use crate::server_errors::ServerError;

    #[test]
    fn test_who_command_invalid_parameters() {
        let listener = TcpListener::bind("127.0.0.1:8132".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8132".to_string(),
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
            create_client_for_test(&session, "127.0.0.1:8132".to_string(), "sender".to_string());

        let message = create_message_for_test(
            MessageType::Who,
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
        );

        let result = handle_who_command(message, client.nickname, &session, &network, None);

        let (mut reader, _addr) = listener.accept().unwrap();
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
                    assert_eq!(command, "WHO".to_string());
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
        assert_eq!(Err(ServerError::InvalidParameters), result);
    }

    #[test]
    fn test_who_command_no_parameters() {
        let listener = TcpListener::bind("127.0.0.1:8133".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8133".to_string(),
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
            create_client_for_test(&session, "127.0.0.1:8133".to_string(), "client".to_string());
        let client2 = create_client_for_test(
            &session,
            "127.0.0.1:8133".to_string(),
            "client2".to_string(),
        );
        let client3 = create_client_for_test(
            &session,
            "127.0.0.1:8133".to_string(),
            "client3".to_string(),
        );

        let mut channel = Channel::new("#channel_test".to_string(), "".to_string(), vec![]);
        channel.users.push(client2.nickname.clone());
        channel.users.push(client.nickname.clone());
        channel.operators.push(client2.nickname.clone());

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(channel.name.clone(), channel.clone());
        drop(channels_lock);

        let message = create_message_for_test(MessageType::Who, vec![]);

        let result = handle_who_command(message, client.nickname, &session, &network, None);
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
                CommandResponse::WhoReply { users } => {
                    assert_eq!(users, [client3.nickname.to_string()]);
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
                CommandResponse::EndOfWho => {
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
    fn test_who_command_existing_channel() {
        let listener = TcpListener::bind("127.0.0.1:8134".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8134".to_string(),
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
            create_client_for_test(&session, "127.0.0.1:8134".to_string(), "client".to_string());
        let client2 = create_client_for_test(
            &session,
            "127.0.0.1:8134".to_string(),
            "client2".to_string(),
        );
        let client3 = create_client_for_test(
            &session,
            "127.0.0.1:8134".to_string(),
            "client3".to_string(),
        );

        let mut channel = Channel::new("#channel_test".to_string(), "".to_string(), vec![]);
        channel.users.push(client2.nickname.clone());
        channel.operators.push(client2.nickname.clone());
        channel.users.push(client3.nickname.clone());
        channel.users.push(client.nickname.clone());

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(channel.name.clone(), channel.clone());
        drop(channels_lock);

        let message = create_message_for_test(MessageType::Who, vec![channel.name.to_string()]);

        let result = handle_who_command(
            message,
            client.nickname.to_string(),
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
                CommandResponse::WhoReply { users } => {
                    assert!(users.contains(&client.nickname.to_string()));
                    assert!(users.contains(&client2.nickname.to_string()));
                    assert!(users.contains(&client3.nickname.to_string()));
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
                CommandResponse::EndOfWho => {
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
    fn test_who_command_client_nickname() {
        let listener = TcpListener::bind("127.0.0.1:8135".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8135".to_string(),
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
            create_client_for_test(&session, "127.0.0.1:8135".to_string(), "client".to_string());
        let client2 = create_client_for_test(
            &session,
            "127.0.0.1:8135".to_string(),
            "client2".to_string(),
        );

        let message = create_message_for_test(MessageType::Who, vec![client2.nickname.to_string()]);

        let result = handle_who_command(
            message,
            client.nickname.to_string(),
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
                CommandResponse::WhoReply { users } => {
                    assert_eq!(users, vec![client2.nickname.to_string()]);
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
                CommandResponse::EndOfWho => {
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
    fn test_who_command_client_username() {
        let listener = TcpListener::bind("127.0.0.1:8136".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8136".to_string(),
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
            create_client_for_test(&session, "127.0.0.1:8136".to_string(), "client".to_string());

        let message = create_message_for_test(MessageType::Who, vec![client.username.to_string()]);

        let result = handle_who_command(
            message,
            client.nickname.to_string(),
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
                CommandResponse::WhoReply { users } => {
                    assert_eq!(users, vec![client.nickname.to_string()]);
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
                CommandResponse::EndOfWho => {
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
    fn test_who_command_client_hostname() {
        let listener = TcpListener::bind("127.0.0.1:8137".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8137".to_string(),
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
            create_client_for_test(&session, "127.0.0.1:8137".to_string(), "client".to_string());

        let message = create_message_for_test(MessageType::Who, vec![client.hostname.to_string()]);

        let result = handle_who_command(
            message,
            client.nickname.to_string(),
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
                CommandResponse::WhoReply { users } => {
                    assert_eq!(users, vec![client.nickname.to_string()]);
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
                CommandResponse::EndOfWho => {
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
    fn test_who_command_client_servername() {
        let listener = TcpListener::bind("127.0.0.1:8138".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8138".to_string(),
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
            create_client_for_test(&session, "127.0.0.1:8138".to_string(), "client".to_string());

        let message =
            create_message_for_test(MessageType::Who, vec![client.servername.to_string()]);

        let result = handle_who_command(
            message,
            client.nickname.to_string(),
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
                CommandResponse::WhoReply { users } => {
                    assert_eq!(users, vec![client.nickname.to_string()]);
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
                CommandResponse::EndOfWho => {
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
    fn test_who_command_client_realname() {
        let listener = TcpListener::bind("127.0.0.1:8139".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8139".to_string(),
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
            create_client_for_test(&session, "127.0.0.1:8139".to_string(), "client".to_string());

        let message = create_message_for_test(MessageType::Who, vec![client.realname.to_string()]);

        let result = handle_who_command(
            message,
            client.nickname.to_string(),
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
                CommandResponse::WhoReply { users } => {
                    assert_eq!(users, vec![client.nickname.to_string()]);
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
                CommandResponse::EndOfWho => {
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
