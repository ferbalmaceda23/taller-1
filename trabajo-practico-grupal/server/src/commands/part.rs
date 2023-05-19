use crate::{
    database::inform_database,
    server_errors::ServerError,
    socket::{inform_client, inform_network},
};
use model::{
    message::Message, network::Network, persistence::PersistenceType,
    responses::errors::ErrorResponse, session::Session,
};

use super::command_utils::write_lock_channels;

/// Handles the PART command received from a client/server
/// If channel is empty, it is removed from the database
/// # Arguments
/// * `session` - The session of the current server
/// * `network` - The network the client is connected to
/// * `message` - The message received from the client/server
/// * `nickname` - The nickname of the client
/// * `server_name` - The name of the server
pub fn handle_part_command(
    message: Message,
    nickname: String,
    session: &Session,
    network: &Network,
    server_name: &String,
) -> Result<(), ServerError> {
    if message.parameters.is_empty() {
        let response = ErrorResponse::NeedMoreParams {
            command: "PART".to_string(),
        }
        .to_string();
        inform_client(session, &nickname, &response)?;
        return Err(ServerError::InvalidParameters);
    }
    let channels_name = message.parameters[0]
        .split(',')
        .map(|a| a.trim())
        .collect::<Vec<_>>();

    for channel_name in channels_name {
        let mut channels = write_lock_channels(session)?;
        if let Some(channel) = channels.get_mut(channel_name) {
            let mut user_eliminated = false;
            for (i, user) in channel.users.iter().enumerate() {
                if *user == nickname {
                    channel.users.remove(i);
                    println!("Channel left: {:?}", channel);
                    user_eliminated = true;
                    break;
                }
            }
            if channel_name.starts_with('#') && !user_eliminated {
                let network_cliens = network.clients.read()?;
                for (i, (user, _)) in network_cliens.iter().enumerate() {
                    if *user == nickname {
                        channel.users.remove(i);
                        user_eliminated = true;
                        break;
                    }
                }
                drop(network_cliens);
            }
            if !user_eliminated {
                let response = ErrorResponse::NotOnChannel {
                    channel: channel_name.to_string(),
                }
                .to_string();
                inform_client(session, &nickname, &response)?;
                continue;
            }
            if channel.users.is_empty() {
                inform_database(
                    PersistenceType::ChannelDelete(channel.name.to_owned()),
                    channel.to_string(),
                    session,
                )?;
                channels.remove(channel_name);
            } else {
                inform_database(
                    PersistenceType::ChannelUpdate(channel.name.to_owned()),
                    channel.to_string(),
                    session,
                )?;
            }
            if channel_name.starts_with('#') {
                let mut msg = message.clone();
                msg.prefix = Some(nickname.clone());
                let msg = Message::deserialize(msg)?;
                inform_network(network, server_name, &msg)?;
            }
        } else {
            let response = ErrorResponse::NoSuchChannel {
                channel: channel_name.to_string(),
            }
            .to_string();
            inform_client(session, &nickname, &response)?;
        }
        drop(channels);
    }
    Ok(())
}

#[cfg(test)]
mod part_tests {
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
    use crate::commands::part::handle_part_command;
    use crate::database::handle_database;
    use crate::server_errors::ServerError;

    #[test]
    fn test_command_part_client_leaves_the_channel() {
        let listener = TcpListener::bind("127.0.0.1:8120".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8120".to_string(),
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
            "127.0.0.1:8120".to_string(),
            "nickname".to_string(),
        );
        let client2 = create_client_for_test(
            &session,
            "127.0.0.1:8120".to_string(),
            "nickname2".to_string(),
        );

        let mut channel = Channel::new("#channel_test".to_string(), "".to_string(), vec![]);
        channel.users.push(client.nickname.to_string());
        channel.users.push(client2.nickname.to_string());

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(channel.name.clone(), channel.clone());
        assert!((channel.clone())
            .users
            .contains(&(client.nickname.to_string())));
        drop(channels_lock);

        let message = create_message_for_test(MessageType::Part, vec![channel.name.to_string()]);

        let result = handle_part_command(
            message,
            client.nickname.to_string(),
            &session,
            &network,
            &"test".to_string(),
        );
        assert!(result.is_ok());

        let channels_lock = read_lock_channels(&session).unwrap();
        let channel_final = channels_lock.get(&"#channel_test".to_string()).unwrap();
        assert!(!channel_final.users.contains(&client.nickname.to_string()));
        drop(channels_lock);
        drop(listener);
    }

    #[test]
    fn test_command_part_with_one_client_deletes_the_channel() {
        let listener = TcpListener::bind("127.0.0.1:8121".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8121".to_string(),
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
            "127.0.0.1:8121".to_string(),
            "nickname".to_string(),
        );

        let mut channel = Channel::new("#channel_test".to_string(), "".to_string(), vec![]);
        channel.users.push(client.nickname.to_string());

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(channel.name.clone(), channel.clone());
        assert!((channel.clone())
            .users
            .contains(&(client.nickname.to_string())));
        drop(channels_lock);

        let message = create_message_for_test(MessageType::Part, vec![channel.name.to_string()]);
        let result = handle_part_command(
            message,
            client.nickname,
            &session,
            &network,
            &"test".to_string(),
        );
        assert!(result.is_ok());

        let channels_lock = read_lock_channels(&session).unwrap();
        assert!(!channels_lock.contains_key(&channel.name.to_string()));
        drop(channels_lock);
        drop(listener);
    }

    #[test]
    pub fn test_command_part_invalid_channel_name() {
        let listener = TcpListener::bind("127.0.0.1:8121".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8121".to_string(),
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
            "127.0.0.1:8121".to_string(),
            "nickname".to_string(),
        );

        let message =
            create_message_for_test(MessageType::Part, vec!["invalid_channel".to_string()]);

        let result = handle_part_command(
            message,
            client.nickname,
            &session,
            &network,
            &"test".to_string(),
        );
        assert!(result.is_ok());
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
    }

    #[test]
    pub fn test_command_part_invalid_parameters() {
        let listener = TcpListener::bind("127.0.0.1:8122".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8122".to_string(),
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
            "127.0.0.1:8122".to_string(),
            "nickname".to_string(),
        );

        let message = create_message_for_test(MessageType::Part, vec![]);
        let result = handle_part_command(
            message,
            client.nickname,
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
            Response::ErrorResponse {
                response: err_response,
            } => match err_response {
                ErrorResponse::NeedMoreParams { command } => {
                    assert_eq!(command, "PART".to_string());
                }
                _ => {
                    assert!(false);
                }
            },
            _ => {
                assert!(false);
            }
        }

        assert_eq!(Err(ServerError::InvalidParameters), result);
    }

    #[test]
    fn test_command_part_not_on_channel() {
        let listener = TcpListener::bind("127.0.0.1:8123".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8123".to_string(),
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
            "127.0.0.1:8123".to_string(),
            "nickname".to_string(),
        );
        let client2 = create_client_for_test(
            &session,
            "127.0.0.1:8123".to_string(),
            "nickname2".to_string(),
        );

        let mut channel = Channel::new("#channel_test".to_string(), "".to_string(), vec![]);
        channel.users.push(client2.nickname.to_string());

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(channel.name.clone(), channel.clone());
        drop(channels_lock);

        let message = create_message_for_test(MessageType::Part, vec![channel.name.to_string()]);
        let result = handle_part_command(
            message,
            client.nickname,
            &session,
            &network,
            &"test".to_string(),
        );
        assert!(result.is_ok());

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
    }
}
