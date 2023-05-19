use model::{
    channel::Channel, channelflag::ChannelFlag, message::Message, network::Network,
    persistence::PersistenceType, responses::errors::ErrorResponse,
    responses::replies::CommandResponse, session::Session,
};
use std::{collections::HashMap, sync::RwLockWriteGuard};

use crate::{
    database::inform_database,
    server_errors::ServerError,
    socket::{inform_client, inform_network},
};

use super::command_utils::write_lock_channels;

/// Function that handles the topic command.
/// If one paramater is received in the message, then it returs
/// the topic of the channel.
/// Else it sets the topic of the channel
/// # Arguments
/// * `message` - The message that sent the client.
/// * `nickname` - The nickname of the client that sent the message.
/// * `session` - The session of the user that sent the message.
/// * `network` - The struct that contains information about the network.
pub fn handle_topic_command(
    message: Message,
    nickname: String,
    session: &Session,
    network: &Network,
    server_name: &String,
) -> Result<(), ServerError> {
    let mut channels_lock = write_lock_channels(session)?;

    match message.parameters.len() {
        0 => {
            let error_response = (ErrorResponse::NeedMoreParams {
                command: "TOPIC".to_string(),
            })
            .to_string();
            inform_client(session, &nickname, &error_response)?;
            return Err(ServerError::InvalidParameters);
        }
        1 => {
            if message.trailing.is_some() {
                set_topic(
                    &mut channels_lock,
                    &nickname,
                    message,
                    session,
                    network,
                    server_name,
                )?;
            } else {
                match channels_lock.get(&message.parameters[0]) {
                    Some(channel) => {
                        println!("Topic of {:?} is: {:?}", channel.name, channel.topic);
                        let response = (CommandResponse::Topic {
                            channel: channel.name.clone(),
                            topic: channel.topic.clone(),
                        })
                        .to_string();
                        inform_client(session, &nickname, response.as_str())?;
                    }
                    None => {
                        drop(channels_lock);
                        return Err(ServerError::ChannelNotFound);
                    }
                }
            }
        }
        _ => {
            set_topic(
                &mut channels_lock,
                &nickname,
                message,
                session,
                network,
                server_name,
            )?;
        }
    }
    drop(channels_lock);
    Ok(())
}

/// Function that sets the topic of a channel.
/// # Arguments
/// * `channels_lock` - The lock of the channels of the server.
/// * `nickname` - The nickname of the client thatsent the message.
/// * `message` - The message that sent the client.
/// * `session` - The session of the user that sent the message.
/// * `network` - The struct that contains information about the network.
/// * `server_name` - The name of the current server.
fn set_topic(
    channels_lock: &mut RwLockWriteGuard<HashMap<String, Channel>>,
    nickname: &String,
    message: Message,
    session: &Session,
    network: &Network,
    server_name: &String,
) -> Result<(), ServerError> {
    match channels_lock.get_mut(&message.parameters[0]) {
        Some(channel) => {
            if !channel.users.contains(nickname) {
                let error_response = (ErrorResponse::NotOnChannel {
                    channel: channel.name.clone(),
                })
                .to_string();
                inform_client(session, nickname, &error_response)?;
                return Err(ServerError::NotOnChannel);
            }
            if channel
                .modes
                .contains(&ChannelFlag::TopicSettableOnlyOperators)
                && !channel.operators.contains(nickname)
            {
                let response = (ErrorResponse::ChanOPrivsNeeded {
                    channel: channel.name.clone(),
                })
                .to_string();
                inform_client(session, nickname, response.as_str())?;
                return Err(ServerError::TopicOnlySetableByOperators);
            }
            channel.topic = get_topic(message.to_owned());
            inform_database(
                PersistenceType::ChannelUpdate(channel.name.to_owned()),
                channel.to_string(),
                session,
            )?;
            println!(
                "Topic of {:?} changed to: {:?}",
                channel.name, channel.topic
            );
            let response = (CommandResponse::Topic {
                channel: channel.name.clone(),
                topic: channel.topic.clone(),
            })
            .to_string();
            inform_client(session, nickname, response.as_str())?;
            if channel.name.starts_with('#') {
                let mut msg = message.to_owned();
                msg.prefix = Some(nickname.to_owned());
                let msg = Message::deserialize(msg)?;
                inform_network(network, server_name, &msg)?;
            }
        }
        None => {
            // drop(channels_lock);
            return Err(ServerError::ChannelNotFound);
        }
    }
    Ok(())
}

/// Functions that parses the topic checking the
/// fields of the message received
/// # Arguments
/// * `message` - The message that sern the client.
fn get_topic(message: Message) -> String {
    let mut topic = "".to_string();
    if message.parameters.len() > 1 {
        if let Some(trailing) = message.trailing {
            let params = message.parameters[1..].join(" ");
            topic = format!("{} {}", params, trailing);
        } else {
            topic = message.parameters[1..].join(" ");
        }
    } else if let Some(trailing) = message.trailing {
        topic = trailing;
    }
    topic
}

#[cfg(test)]
mod topic_tests {
    use std::collections::HashMap;
    use std::io::Read;
    use std::net::TcpListener;
    use std::sync::{Arc, RwLock};
    use std::vec;

    use crate::commands::command_utils::{
        create_client_for_test, create_message_for_test, create_session_for_test,
        read_lock_channels, write_lock_channels,
    };
    use crate::commands::topic::handle_topic_command;
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
    pub fn test_command_topic_changes_channel_topic() {
        let listener = TcpListener::bind("127.0.0.1:8140".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8140".to_string(),
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
            create_client_for_test(&session, "127.0.0.1:8140".to_string(), "client".to_string());

        let mut channel = Channel::new("#channel_test".to_string(), "".to_string(), vec![]);
        channel.users.push(client.nickname.clone());
        channel.operators.push(client.nickname.clone());

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(channel.name.clone(), channel.clone());
        drop(channels_lock);

        let msg = create_message_for_test(
            MessageType::Topic,
            vec!["#channel_test".to_string(), "new_topic".to_string()],
        );
        let result = handle_topic_command(
            msg,
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
        let channels_lock = read_lock_channels(&session).unwrap();
        assert_eq!(
            channels_lock.get(&channel.name).unwrap().topic,
            "new_topic ".to_string()
        );
        drop(channels_lock);
        assert!(result.is_ok());
    }

    #[test]
    fn test_command_topic_only_settable_by_operators() {
        let listener = TcpListener::bind("127.0.0.1:8141".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8141".to_string(),
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
            create_client_for_test(&session, "127.0.0.1:8141".to_string(), "client".to_string());

        let mut channel = Channel::new("#channel_test".to_string(), "".to_string(), vec![]);
        channel.users.push(client.nickname.clone());
        channel.modes.push(ChannelFlag::TopicSettableOnlyOperators);

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(channel.name.clone(), channel.clone());
        drop(channels_lock);

        let msg = create_message_for_test(
            MessageType::Topic,
            vec!["#channel_test".to_string(), "new_topic".to_string()],
        );
        let result = handle_topic_command(
            msg,
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
                response: cmd_response,
            } => match cmd_response {
                ErrorResponse::ChanOPrivsNeeded { channel: c } => {
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
        assert_eq!(
            channels_lock.get(&channel.name).unwrap().topic,
            "".to_string()
        );
        drop(channels_lock);
        assert_eq!(Err(ServerError::TopicOnlySetableByOperators), result);
    }

    #[test]
    pub fn test_command_topic_cant_change_topic_of_non_existing_channel() {
        let listener = TcpListener::bind("127.0.0.1:8142".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8142".to_string(),
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
            create_client_for_test(&session, "127.0.0.1:8142".to_string(), "client".to_string());

        let message = create_message_for_test(
            MessageType::Topic,
            vec!["#non_existing_channel".to_string(), "new_topic".to_string()],
        );
        let result = handle_topic_command(
            message,
            client.nickname,
            &session,
            &network,
            &"test".to_string(),
        );
        drop(listener);
        assert_eq!(Err(ServerError::ChannelNotFound), result);
    }

    #[test]
    pub fn test_command_topic_invalid_parameters() {
        let listener = TcpListener::bind("127.0.0.1:8143".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8143".to_string(),
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
            create_client_for_test(&session, "127.0.0.1:8143".to_string(), "client".to_string());

        let message = create_message_for_test(MessageType::Topic, vec![]);

        let result = handle_topic_command(
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
                response: cmd_response,
            } => match cmd_response {
                ErrorResponse::NeedMoreParams { command } => {
                    assert_eq!(command, "TOPIC".to_string());
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
}
