use super::{
    command_utils::{read_lock_channels, read_lock_clients},
    mode::get_channel_modes_hash,
};
use crate::{
    server_errors::ServerError,
    socket::{inform_client, inform_server},
};
use model::{
    channel::Channel, channelflag::ChannelFlag, message::Message, network::Network,
    responses::replies::CommandResponse, session::Session,
};
use std::collections::HashSet;

/// Function that handles the NAMES command received from a client/server.
/// # Arguments
/// * `message` - The message received from the client.
/// * `nickname` - The nickname of the client.
/// * `session` - The session of the client that sent the command.
/// * `network` - The network that the client is connected to.
/// * `server_name` - The name of the server.
pub fn handle_names_command(
    message: Message,
    nickname: String,
    session: &Session,
    network: &Network,
    server_name: Option<String>,
) -> Result<(), ServerError> {
    let channels_lock = read_lock_channels(session)?;
    let mut channel_users: String = String::new();
    let mut response;
    if message.parameters.is_empty() {
        let mut visible_users: Vec<String> = Vec::new();
        let mut not_visible_users: Vec<String> = Vec::new();
        for channel in channels_lock.values() {
            if channel.users.contains(&nickname) || !channel.modes.contains(&ChannelFlag::Secret) {
                response = (CommandResponse::Names {
                    channel: channel.name.clone(),
                    names: channel.users.clone(),
                })
                .to_string();
                if let Some(name) = server_name.to_owned() {
                    inform_server_about_channel(network, &name, channel, &response)?;
                } else {
                    inform_client(session, &nickname, &response.to_string())?;
                }

                let chans_users_str = format!(
                    "{},{:?};",
                    channel.name.to_owned(),
                    channel.users.to_owned()
                );
                channel_users.push_str(&chans_users_str);
                let mut users_aux: Vec<String> = channel.users.clone();
                visible_users.append(&mut users_aux);
            }
        }
        let users: HashSet<String> = HashSet::from_iter(visible_users);
        let clients_lock = read_lock_clients(session)?;
        for client in clients_lock.values() {
            if !users.contains(&client.nickname) {
                not_visible_users.push(client.nickname.clone());
            }
        }
        let network_clients = network.clients.as_ref().read()?;
        for nick in network_clients.keys().clone() {
            if !users.contains(nick) {
                not_visible_users.push(nick.clone());
            }
        }
        drop(network_clients);

        drop(clients_lock);
        response = (CommandResponse::Names {
            channel: "*".to_string(),
            names: not_visible_users,
        })
        .to_string();
        inform_client(session, &nickname, response.as_str())?;
        response = CommandResponse::EndNames.to_string();
        inform_client(session, &nickname, response.as_str())?;
    } else {
        let channels_name = message.parameters[0]
            .split(',')
            .into_iter()
            .map(|a| a.trim())
            .collect::<Vec<_>>();
        for name in channels_name {
            if let Some(channel) = channels_lock.get(name) {
                if channel.users.contains(&nickname)
                    || !channel.modes.contains(&ChannelFlag::Secret)
                {
                    response = (CommandResponse::Names {
                        channel: channel.name.clone(),
                        names: channel.users.clone(),
                    })
                    .to_string();
                    inform_client(session, &nickname, response.as_str())?;
                    let chans_users_str = format!(
                        "{},{:?};",
                        channel.name.to_owned(),
                        channel.users.to_owned()
                    );
                    channel_users.push_str(&chans_users_str);
                }
            }
        }
        response = CommandResponse::EndNames.to_string();
        inform_client(session, &nickname, response.as_str())?;
    }
    drop(channels_lock);
    Ok(())
}

fn inform_server_about_channel(
    network: &Network,
    server_name: &String,
    channel: &Channel,
    response: &str,
) -> Result<(), ServerError> {
    if channel.name.starts_with('#') {
        inform_server(network, server_name, response)?;
        let mode_response = CommandResponse::ChannelMode {
            channel: channel.name.to_owned(),
            modes: get_channel_modes_hash(channel),
        }
        .to_string();
        inform_server(network, server_name, &mode_response)?;
    }

    Ok(())
}

#[cfg(test)]
mod names_tests {
    use crate::{
        commands::command_utils::{
            create_client_for_test, create_message_for_test, create_session_for_test,
            write_lock_channels,
        },
        database::handle_database,
    };

    use super::*;
    use model::{
        channel::Channel, message::MessageType, persistence::PersistenceType,
        responses::response::Response, server::Server,
    };
    use std::{
        collections::HashMap,
        io::Read,
        net::TcpListener,
        sync::{Arc, RwLock},
    };

    #[test]
    fn test_names_command_multiple_cases() {
        let listener = TcpListener::bind("127.0.0.1:8114".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8114".to_string(),
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
        let client = create_client_for_test(
            &session,
            "127.0.0.1:8114".to_string(),
            "nickname".to_string(),
        );
        let client2 = create_client_for_test(
            &session,
            "127.0.0.1:8114".to_string(),
            "nickname2".to_string(),
        );
        let client3 = create_client_for_test(
            &session,
            "127.0.0.1:8114".to_string(),
            "nickname3".to_string(),
        );
        let client4 = create_client_for_test(
            &session,
            "127.0.0.1:8114".to_string(),
            "nickname4".to_string(),
        );

        let mut channel = Channel::new("&channel_test".to_string(), "".to_string(), vec![]);
        channel.users.push(client.nickname.to_string());
        channel.users.push(client2.nickname.to_string());

        let mut channel2 = Channel::new("&channel_test2".to_string(), "".to_string(), vec![]);
        channel2.users.push(client2.nickname.to_string());
        channel2.users.push(client3.nickname.to_string());

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(channel.name.clone(), channel.clone());
        channels_lock.insert(channel2.name.clone(), channel2.clone());
        drop(channels_lock);

        let message = create_message_for_test(MessageType::Names, vec![]);

        let mut expected_responses = HashMap::<String, Vec<String>>::new();
        expected_responses.insert(channel.name.clone(), channel.users.clone());
        expected_responses.insert(channel2.name.clone(), channel2.users.clone());
        expected_responses.insert("*".to_string(), vec![client4.nickname.to_string()]);

        let result = handle_names_command(
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
                CommandResponse::Names { channel: c, names } => {
                    assert!(expected_responses.contains_key(&c));
                    assert_eq!(&names, expected_responses.get(&c).unwrap());
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
                CommandResponse::Names { channel: c, names } => {
                    assert!(expected_responses.contains_key(&c));
                    assert_eq!(&names, expected_responses.get(&c).unwrap());
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
                CommandResponse::Names { channel: c, names } => {
                    assert!(expected_responses.contains_key(&c));
                    assert_eq!(&names, expected_responses.get(&c).unwrap());
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
                CommandResponse::EndNames => {
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

        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_names_command_no_channel() {
        let listener = TcpListener::bind("127.0.0.1:8115".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8115".to_string(),
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
        let client = create_client_for_test(
            &session,
            "127.0.0.1:8115".to_string(),
            "nickname".to_string(),
        );
        let client2 = create_client_for_test(
            &session,
            "127.0.0.1:8115".to_string(),
            "nickname2".to_string(),
        );

        let message = create_message_for_test(MessageType::Names, vec![]);

        let result = handle_names_command(
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
                CommandResponse::Names { channel: c, names } => {
                    assert_eq!(c, "*".to_string());
                    assert!(names.contains(&client.nickname.to_string()));
                    assert!(names.contains(&client2.nickname.to_string()));
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
                CommandResponse::EndNames => {
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

        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_names_command_one_channel() {
        let listener = TcpListener::bind("127.0.0.1:8116".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8116".to_string(),
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
        let client = create_client_for_test(
            &session,
            "127.0.0.1:8116".to_string(),
            "nickname".to_string(),
        );
        let client2 = create_client_for_test(
            &session,
            "127.0.0.1:8116".to_string(),
            "nickname2".to_string(),
        );

        let mut channel = Channel::new("&channel_test".to_string(), "".to_string(), vec![]);
        channel.users.push(client.nickname.to_string());
        channel.users.push(client2.nickname.to_string());

        let mut channel2 = Channel::new("&channel_test2".to_string(), "".to_string(), vec![]);
        channel2.users.push(client2.nickname.to_string());

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(channel.name.clone(), channel.clone());
        channels_lock.insert(channel2.name.clone(), channel2.clone());
        drop(channels_lock);

        let message =
            create_message_for_test(MessageType::Names, vec!["&channel_test2".to_string()]);

        let result = handle_names_command(
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
                CommandResponse::Names { channel: c, names } => {
                    assert_eq!(c, channel2.name.to_string());
                    assert_eq!(names, channel2.users);
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
                CommandResponse::EndNames => {
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

        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_names_command_secret_channel() {
        let listener = TcpListener::bind("127.0.0.1:8117".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8117".to_string(),
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
        let client = create_client_for_test(
            &session,
            "127.0.0.1:8117".to_string(),
            "nickname".to_string(),
        );
        let client2 = create_client_for_test(
            &session,
            "127.0.0.1:8117".to_string(),
            "nickname2".to_string(),
        );

        let mut channel = Channel::new("&channel_test".to_string(), "".to_string(), vec![]);
        channel.users.push(client.nickname.to_string());
        channel.users.push(client2.nickname.to_string());

        let mut channel2 = Channel::new("&channel_test2".to_string(), "".to_string(), vec![]);
        channel2.users.push(client2.nickname.to_string());
        channel2.modes.push(ChannelFlag::Secret);

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(channel.name.clone(), channel.clone());
        channels_lock.insert(channel2.name.clone(), channel2.clone());
        drop(channels_lock);

        let message = create_message_for_test(
            MessageType::Names,
            vec!["&channel_test,&channel_test2".to_string()],
        );

        let result = handle_names_command(
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
                CommandResponse::Names { channel: c, names } => {
                    assert_eq!(c, channel.name.to_string());
                    assert_eq!(names, channel.users);
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
                CommandResponse::EndNames => {
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

        assert!(result.is_ok());
    }
}
