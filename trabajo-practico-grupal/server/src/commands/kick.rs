use super::command_utils::write_lock_channels;
use crate::{
    database::inform_database,
    server_errors::ServerError,
    socket::{inform_client, inform_network},
};
use model::{
    message::Message,
    network::Network,
    persistence::PersistenceType,
    responses::{errors::ErrorResponse, message::MessageResponse},
    session::Session,
};

/// Handles the kick command, which kicks a client from a channel.
pub fn handle_kick_command(
    message: Message,
    nickname: String,
    session: &Session,
    network: &Network,
    server_name: &String,
) -> Result<(), ServerError> {
    if message.parameters.len() < 2 {
        let response = ErrorResponse::NeedMoreParams {
            command: "KICK".to_string(),
        }
        .to_string();
        inform_client(session, &nickname, &response)?;
        return Err(ServerError::InvalidParameters);
    };
    let mut channels_lock = write_lock_channels(session)?;
    let user_to_kick = message.parameters[1].to_owned();

    match channels_lock.get_mut(&message.parameters[0]) {
        Some(channel) => {
            if !channel.operators.contains(&nickname) {
                let response = (ErrorResponse::ChanOPrivsNeeded {
                    channel: channel.name.clone(),
                })
                .to_string();
                inform_client(session, &nickname, response.as_str())?;
                return Err(ServerError::UserNotOperator);
            }
            let mut user_eliminated = false;
            for (i, user) in channel.users.iter().enumerate() {
                if user == &user_to_kick {
                    channel.users.remove(i);
                    user_eliminated = true;
                    break;
                }
            }
            if channel.name.starts_with('#') && !user_eliminated {
                let network_cliens = network.clients.read()?;
                for (i, (user, _)) in network_cliens.iter().enumerate() {
                    if user == &user_to_kick {
                        channel.users.remove(i);
                        user_eliminated = true;
                        break;
                    }
                }
                drop(network_cliens);
            }
            if !user_eliminated {
                return Err(ServerError::UserNotInChannel);
            }
            inform_database(
                PersistenceType::ChannelUpdate(channel.name.to_owned()),
                channel.to_string(),
                session,
            )?;
            println!("Client {} kicked from {}", &user_to_kick, channel.name);
            let mut msg = format!("{} kicked you from {}", nickname, channel.name);
            if message.parameters.len() > 2 {
                msg = message.parameters[2..].to_owned().join(" ");
            }
            let response = MessageResponse::KickMsg { message: msg }.to_string();
            inform_client(session, &user_to_kick, response.as_str())?;
            if channel.name.starts_with('#') {
                let mut msg = message.clone();
                msg.prefix = Some(nickname);
                let msg = Message::deserialize(msg)?;
                inform_network(network, server_name, &msg)?;
            }
        }
        None => {
            let response = ErrorResponse::NoSuchChannel {
                channel: message.parameters[0].to_owned(),
            }
            .to_string();
            inform_client(session, &nickname, &response)?;
            return Err(ServerError::ChannelNotFound);
        }
    }
    drop(channels_lock);
    Ok(())
}

#[cfg(test)]
mod kick_tests {
    use std::collections::HashMap;
    use std::io::Read;
    use std::net::TcpListener;
    use std::sync::{Arc, RwLock};

    use model::channel::Channel;
    use model::message::MessageType;
    use model::network::Network;
    use model::persistence::PersistenceType;
    use model::responses::errors::ErrorResponse;
    use model::responses::response::Response;
    use model::server::Server;

    use crate::commands::command_utils::{
        create_client_for_test, create_message_for_test, create_session_for_test,
        read_lock_channels, write_lock_channels,
    };
    use crate::commands::kick::handle_kick_command;
    use crate::database::handle_database;
    use crate::server_errors::ServerError;

    #[test]
    fn test_command_kick_client_leaves_the_channel() {
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
        let operator =
            create_client_for_test(&session, address_port.to_string(), "nickname".to_string());
        let user_to_kick = create_client_for_test(
            &session,
            address_port.to_string(),
            "user_to_kick".to_string(),
        );

        let mut channel = Channel::new("#channel_test".to_string(), "".to_string(), vec![]);
        channel.operators.push(operator.nickname.clone());
        channel.users.push(operator.nickname.clone());
        channel.users.push(user_to_kick.nickname.to_string());

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(channel.name.clone(), channel);
        drop(channels_lock);

        let message = create_message_for_test(
            MessageType::Kick,
            vec![
                "#channel_test".to_string(),
                user_to_kick.nickname.to_string(),
            ],
        );
        let result = handle_kick_command(
            message,
            operator.nickname.to_string(),
            &session,
            &network,
            &"test".to_string(),
        );

        let channels_lock = read_lock_channels(&session).unwrap();
        let channel = channels_lock.get("#channel_test").unwrap();
        assert!(!channel.users.contains(&"user_test".to_string()));
        drop(channels_lock);
        drop(listener);
        assert!(result.is_ok());
    }

    #[test]
    fn test_command_kick_invalid_parameters() {
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
        let operator =
            create_client_for_test(&session, address_port.to_string(), "nickname".to_string());

        let mut channel = Channel::new("#channel_test".to_string(), "".to_string(), vec![]);
        channel.operators.push(operator.nickname.clone());
        channel.users.push(operator.nickname.clone());

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(channel.name.clone(), channel);
        drop(channels_lock);

        let message = create_message_for_test(MessageType::Kick, vec!["#channel_test".to_string()]);

        let result = handle_kick_command(
            message,
            operator.nickname.to_string(),
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
                ErrorResponse::NeedMoreParams { command: c } => {
                    assert_eq!(c, "KICK".to_string());
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
        let channel = channels_lock.get("#channel_test").unwrap();
        assert!(channel.users.contains(&operator.nickname.to_string()));
        drop(channels_lock);
        assert_eq!(Err(ServerError::InvalidParameters), result);
    }

    #[test]
    fn test_command_kick_user_not_operator() {
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
        let not_operator =
            create_client_for_test(&session, address_port.to_string(), "nickname".to_string());
        let user_to_kick = create_client_for_test(
            &session,
            address_port.to_string(),
            "user_to_kick".to_string(),
        );

        let mut channel = Channel::new("#channel_test".to_string(), "".to_string(), vec![]);
        channel.users.push(not_operator.nickname.clone());
        channel.users.push(user_to_kick.nickname.clone());

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(channel.name.clone(), channel);
        drop(channels_lock);

        let message = create_message_for_test(
            MessageType::Kick,
            vec![
                "#channel_test".to_string(),
                user_to_kick.nickname.to_string(),
            ],
        );

        let result = handle_kick_command(
            message,
            not_operator.nickname.to_string(),
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
                ErrorResponse::ChanOPrivsNeeded { channel: c } => {
                    assert_eq!(c, "#channel_test".to_string());
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
        let channel = channels_lock.get("#channel_test").unwrap();
        assert!(channel.users.contains(&not_operator.nickname.to_string()));
        assert!(!channel
            .operators
            .contains(&not_operator.nickname.to_string()));
        assert!(channel.users.contains(&user_to_kick.nickname.to_string()));
        drop(channels_lock);
        assert_eq!(Err(ServerError::UserNotOperator), result);
    }

    #[test]
    fn test_command_kick_user_not_in_channel() {
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
        let operator =
            create_client_for_test(&session, address_port.to_string(), "nickname".to_string());
        let user_to_kick = create_client_for_test(
            &session,
            address_port.to_string(),
            "user_to_kick".to_string(),
        );

        let mut channel = Channel::new("#channel_test".to_string(), "".to_string(), vec![]);
        channel.users.push(operator.nickname.clone());
        channel.operators.push(operator.nickname.clone());

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(channel.name.clone(), channel);
        drop(channels_lock);

        let message = create_message_for_test(
            MessageType::Kick,
            vec![
                "#channel_test".to_string(),
                user_to_kick.nickname.to_string(),
            ],
        );

        let result = handle_kick_command(
            message,
            operator.nickname.to_string(),
            &session,
            &network,
            &"test".to_string(),
        );
        drop(listener);
        let channels_lock = read_lock_channels(&session).unwrap();
        let channel = channels_lock.get("#channel_test").unwrap();
        assert!(channel.users.contains(&operator.nickname.to_string()));
        assert!(!channel.users.contains(&user_to_kick.nickname.to_string()));
        drop(channels_lock);
        assert_eq!(Err(ServerError::UserNotInChannel), result);
    }

    #[test]
    fn test_command_kick_channel_not_found() {
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
        let operator =
            create_client_for_test(&session, address_port.to_string(), "nickname".to_string());
        let user_to_kick = create_client_for_test(
            &session,
            address_port.to_string(),
            "user_to_kick".to_string(),
        );

        let message = create_message_for_test(
            MessageType::Kick,
            vec![
                "#non_existing_channel_test".to_string(),
                user_to_kick.nickname.to_string(),
            ],
        );

        let channels_lock = read_lock_channels(&session).unwrap();
        let channel = channels_lock.get("#non_existing_channel_test");
        assert!(channel.is_none());
        drop(channels_lock);

        let result = handle_kick_command(
            message,
            operator.nickname.to_string(),
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
                    assert_eq!(c, "#non_existing_channel_test".to_string());
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
        let channel = channels_lock.get("#non_existing_channel_test");
        assert!(channel.is_none());
        drop(channels_lock);
        assert_eq!(Err(ServerError::ChannelNotFound), result);
    }
}
