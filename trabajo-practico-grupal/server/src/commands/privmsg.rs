use super::{
    command_utils::{lock_sockets, read_lock_channels, read_lock_clients},
    server_commands_handler::handle_server_privmsg_command,
};
use crate::{
    server_errors::ServerError,
    socket::{inform_client, inform_network, write_socket},
};
use model::{
    channelflag::ChannelFlag,
    message::Message,
    network::Network,
    responses::{errors::ErrorResponse, message::MessageResponse, replies::CommandResponse},
    session::Session,
};

/// Function to handle the PRIVMSG command from a client/server
/// # Arguments
/// * `message` - The message received from the client/server
/// * `nickname` - The nickname of the client
/// * `session` - The session of the current server
/// * `network` - The struct that contains the network information
/// * `server_name` - The name of the server
pub fn handle_privmsg_command(
    message: Message,
    nickname: &String,
    session: &Session,
    network: &Network,
    server_name: &String,
) -> Result<(), ServerError> {
    if message.parameters.len() < 2 && message.trailing.is_none()
        || message.parameters.is_empty() && message.trailing.is_some()
    {
        let response = ErrorResponse::NeedMoreParams {
            command: "PRIVMSG".to_string(),
        }
        .to_string();
        inform_client(session, nickname, &response)?;
        return Err(ServerError::InvalidParameters);
    }

    let receivers = message.parameters[0]
        .split(',')
        .map(|a| a.trim())
        .collect::<Vec<_>>();
    for receiver in receivers {
        if receiver == nickname {
            continue;
        }

        if receiver.starts_with('&') {
            msg_to_local_channel(receiver, nickname, session, &message)?;
        } else if receiver.starts_with('#') {
            msg_to_distributed_channel(
                receiver,
                nickname,
                session,
                network,
                &message,
                server_name,
            )?;
        } else {
            msg_to_client(receiver, nickname, session, network, &message, server_name)?;
        }
    }
    Ok(())
}

/// Function that sends a PRIVMSG to a local channel
/// # Arguments
/// * `chan_receiver` - The receiver channel that receives the message
/// * `nickname` - The nickname of the client
/// * `session` - The session of the current server
/// * `message` - The message received from the client
fn msg_to_local_channel(
    chan_receiver: &str,
    nickname: &String,
    session: &Session,
    message: &Message,
) -> Result<(), ServerError> {
    if let Some(channel) = read_lock_channels(session)?.get(chan_receiver) {
        if !channel.users.contains(nickname)
            && channel.modes.contains(&ChannelFlag::NoMessageFromOutside)
        {
            let response = (ErrorResponse::CannotSendToChannel {
                channel: chan_receiver.to_string(),
            })
            .to_string();
            inform_client(session, nickname, &response)?;
            return Err(ServerError::UserNotInChannel);
        }
        if channel.banned_users.contains(nickname) {
            let response = (ErrorResponse::CannotSendToChannel {
                channel: chan_receiver.to_string(),
            })
            .to_string();
            inform_client(session, nickname, &response)?;
            return Err(ServerError::UserNotInChannel);
        }
        if !channel.moderators.contains(nickname)
            && channel.modes.contains(&ChannelFlag::ModeratedChannel)
        {
            let response = (ErrorResponse::CannotSendToChannel {
                channel: chan_receiver.to_string(),
            })
            .to_string();
            inform_client(session, nickname, &response)?;
            return Err(ServerError::ChannelIsModerated);
        }
        for user in channel.users.iter() {
            if let Some(c) = read_lock_clients(session)?.get(user) {
                if c.connected && c.nickname != *nickname {
                    if let Some(socket) = lock_sockets(session)?.get(&c.nickname) {
                        let msg = match message.prefix.to_owned() {
                            Some(prefix) => {
                                prepare_chan_msg(message, &prefix, &chan_receiver.to_string())
                            }
                            None => prepare_chan_msg(message, nickname, &chan_receiver.to_string()),
                        };
                        write_socket(socket.clone(), msg.as_str())?;
                    }
                }
            }
        }
    }
    Ok(())
}

/// Function that sends a PRIVMSG to a distributed channel
/// # Arguments
/// * `chan_receiver` - The receiver channel that receives the message
/// * `nickname` - The nickname of the client
/// * `session` - The session of the current server
/// * `network` - The struct that contains the network information
/// * `message` - The message received from the client/server
/// * `server_name` - The name of the server
fn msg_to_distributed_channel(
    chan_receiver: &str,
    nickname: &String,
    session: &Session,
    network: &Network,
    message: &Message,
    server_name: &String,
) -> Result<(), ServerError> {
    msg_to_local_channel(chan_receiver, nickname, session, message)?;
    let mut msg = message.clone();
    msg.prefix = Some(nickname.to_string());
    msg.parameters[0] = chan_receiver.to_string();
    let msg = Message::deserialize(msg)?;
    inform_network(network, server_name, &msg)?;

    Ok(())
}

/// Function that sends a PRIVMSG to a client
/// # Arguments
/// * `receiver` - The receiver client that receives the message
/// * `nickname` - The nickname of the client that sended the message
/// * `session` - The session of the current server
/// * `network` - The struct that contains the network information
/// * `message` - The message received from the client/server
/// * `server_name` - The name of the server
fn msg_to_client(
    receiver: &str,
    nickname: &String,
    session: &Session,
    network: &Network,
    message: &Message,
    server_name: &String,
) -> Result<(), ServerError> {
    let local_clients = read_lock_clients(session)?;
    if let Some(c) = local_clients.get(receiver) {
        if let Some(away_msg) = c.away_message.to_owned() {
            if local_clients.get(nickname).is_some() {
                let response = (CommandResponse::Away {
                    nickname: receiver.to_string(),
                    message: away_msg,
                })
                .to_string();
                inform_client(session, nickname, &response)?;
            } else {
                let msg = format!(":{} AWAY {} :{}", nickname, receiver, away_msg);
                let server_lock = network.server.as_ref().read()?;
                let current_server_name = server_lock.name.clone();
                drop(server_lock);
                inform_network(network, &current_server_name, &msg)?;
            }
            return Ok(());
        } else if c.connected {
            if let Some(socket) = lock_sockets(session)?.get(receiver) {
                let msg = match message.prefix.to_owned() {
                    Some(prefix) => prepare_msg(message, &prefix),
                    None => prepare_msg(message, nickname),
                };
                write_socket(socket.clone(), msg.as_str())?;
            }
        }
    } else {
        let mut msg = message.clone();
        msg.prefix = Some(nickname.to_string());
        msg.parameters[0] = receiver.to_string();
        handle_server_privmsg_command(msg, nickname, server_name, session, network)?;
    }
    Ok(())
}

/// Function that parses the message from client to client
/// # Arguments
/// * `message` - The message received from the client
/// * `nickname` - The nickname of the client that sent the message
fn prepare_msg(message: &Message, nick: &String) -> String {
    let msg;
    if message.parameters.len() > 1 {
        if let Some(trailing) = message.trailing.to_owned() {
            let mut params = message.parameters[1..].to_owned();
            params.push(trailing);
            msg = params.join(" ");
        } else {
            let params = message.parameters[1..].to_owned();
            msg = params.join(" ");
        }
    } else if let Some(traling) = message.trailing.to_owned() {
        msg = traling;
    } else {
        msg = "".to_owned();
    }

    MessageResponse::UserPrivMsg {
        sender: nick.to_owned(),
        message: msg,
    }
    .to_string()
}

/// Function that parses the message from client to channel
/// # Arguments
/// * `message` - The message received from the client
/// * `nick` - The nickname of the client that sent the message
/// * `chan` - The channel that receives the message
fn prepare_chan_msg(message: &Message, nick: &String, chan: &String) -> String {
    let msg;
    if message.parameters.len() > 1 {
        if let Some(trailing) = message.trailing.to_owned() {
            let mut params = message.parameters[1..].to_owned();
            params.push(trailing);
            msg = params.join(" ");
        } else {
            let params = message.parameters[1..].to_owned();
            msg = params.join(" ");
        }
    } else if let Some(trailing) = message.trailing.to_owned() {
        msg = trailing;
    } else {
        msg = "".to_owned();
    }

    MessageResponse::ChannelPrivMsg {
        channel: chan.to_owned(),
        sender: nick.to_owned(),
        message: msg,
    }
    .to_string()
}

//tests
#[cfg(test)]
mod privmsg_tests {
    use model::channel::Channel;
    use model::channelflag::ChannelFlag;
    use model::message::MessageType;
    use model::network::Network;
    use model::persistence::PersistenceType;
    use model::responses::errors::ErrorResponse;
    use model::responses::response::Response;
    use model::server::Server;
    use std::collections::HashMap;
    use std::io::Read;
    use std::net::TcpListener;
    use std::sync::{Arc, RwLock};

    use crate::commands::command_utils::{
        create_client_for_test, create_message_for_test, create_session_for_test,
        write_lock_channels,
    };
    use crate::commands::privmsg::handle_privmsg_command;
    use crate::database::handle_database;
    use crate::server_errors::ServerError;

    #[test]
    fn test_privmsg_to_user() {
        let listener = TcpListener::bind("127.0.0.1:8124".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8124".to_string(),
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
            create_client_for_test(&session, "127.0.0.1:8124".to_string(), "sender".to_string());
        let mut client2 = create_client_for_test(
            &session,
            "127.0.0.1:8124".to_string(),
            "receiver".to_string(),
        );
        client2.connected = true;

        //let mut channel = Channel::new("#channel_test".to_string(), "".to_string(), vec![]);
        //channel.users.push(client.nickname.clone());

        let mut clients = HashMap::new();
        clients.insert(client.nickname.clone(), client.clone());
        clients.insert(client2.nickname.clone(), client2.clone());
        let message = create_message_for_test(
            MessageType::Privmsg,
            vec!["receiver".to_string(), "hello".to_string()],
        );

        let result = handle_privmsg_command(
            message,
            &client.nickname,
            &session,
            &network,
            &"test".to_string(),
        );

        assert!(result.is_ok());

        drop(listener);
    }

    #[test]
    fn test_privmsg_to_channel() {
        let listener = TcpListener::bind("127.0.0.1:8125".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8125".to_string(),
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
            create_client_for_test(&session, "127.0.0.1:8125".to_string(), "sender".to_string());
        let mut client2 = create_client_for_test(
            &session,
            "127.0.0.1:8125".to_string(),
            "receiver".to_string(),
        );
        client2.connected = true;

        let mut channel = Channel::new("#channel_test".to_string(), "".to_string(), vec![]);
        channel.users.push(client.nickname.clone());
        channel.users.push(client2.nickname.clone());

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(channel.name.clone(), channel.clone());
        drop(channels_lock);

        let message = create_message_for_test(
            MessageType::Privmsg,
            vec![channel.name.to_string(), "hello".to_string()],
        );

        let result = handle_privmsg_command(
            message,
            &client.nickname,
            &session,
            &network,
            &"test".to_string(),
        );
        assert!(result.is_ok());

        drop(listener);
    }

    #[test]
    fn test_privmsg_invlaid_parameters() {
        let listener = TcpListener::bind("127.0.0.1:8126".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8126".to_string(),
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
            create_client_for_test(&session, "127.0.0.1:8126".to_string(), "sender".to_string());
        let mut client2 = create_client_for_test(
            &session,
            "127.0.0.1:8126".to_string(),
            "receiver".to_string(),
        );
        client2.connected = true;

        let message = create_message_for_test(MessageType::Privmsg, vec![]);

        let result = handle_privmsg_command(
            message,
            &client.nickname,
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
                    assert_eq!(command, "PRIVMSG".to_string());
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
    fn test_privmsg_invlaid_parameters_no_trailing() {
        let listener = TcpListener::bind("127.0.0.1:8127".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8127".to_string(),
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
            create_client_for_test(&session, "127.0.0.1:8127".to_string(), "sender".to_string());
        let mut client2 = create_client_for_test(
            &session,
            "127.0.0.1:8127".to_string(),
            "receiver".to_string(),
        );
        client2.connected = true;

        let mut message = create_message_for_test(MessageType::Privmsg, vec![]);
        message.trailing = None;

        let result = handle_privmsg_command(
            message,
            &client.nickname,
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
                    assert_eq!(command, "PRIVMSG".to_string());
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
    fn test_privmsg_send_message_to_client_and_channel() {
        let listener = TcpListener::bind("127.0.0.1:8128".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8128".to_string(),
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
            create_client_for_test(&session, "127.0.0.1:8128".to_string(), "sender".to_string());
        let mut client2 = create_client_for_test(
            &session,
            "127.0.0.1:8128".to_string(),
            "receiver".to_string(),
        );
        let mut client3 = create_client_for_test(
            &session,
            "127.0.0.1:8128".to_string(),
            "receiver".to_string(),
        );
        client2.connected = true;
        client3.connected = true;

        let mut channel = Channel::new("#channel_test".to_string(), "".to_string(), vec![]);
        channel.users.push(client.nickname.clone());
        channel.users.push(client2.nickname.clone());

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(channel.name.clone(), channel.clone());
        drop(channels_lock);

        let message = create_message_for_test(
            MessageType::Privmsg,
            vec![
                format!("{},{}", channel.name.clone(), client3.nickname.clone()),
                "message".to_string(),
            ],
        );

        let result = handle_privmsg_command(
            message,
            &client.nickname,
            &session,
            &network,
            &"test".to_string(),
        );
        assert!(result.is_ok());

        drop(listener);
    }

    #[test]
    fn test_privmsg_cant_send_message_to_moderated_channel() {
        let listener = TcpListener::bind("127.0.0.1:8129".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8129".to_string(),
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
            create_client_for_test(&session, "127.0.0.1:8129".to_string(), "sender".to_string());
        let mut client2 = create_client_for_test(
            &session,
            "127.0.0.1:8129".to_string(),
            "receiver".to_string(),
        );
        client2.connected = true;

        let mut channel = Channel::new("#channel_test".to_string(), "".to_string(), vec![]);
        channel.users.push(client.nickname.clone());
        channel.users.push(client2.nickname.clone());
        channel.moderators.push(client2.nickname.clone());
        channel.operators.push(client2.nickname.clone());
        channel.modes.push(ChannelFlag::ModeratedChannel);

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(channel.name.clone(), channel.clone());
        drop(channels_lock);

        let message = create_message_for_test(
            MessageType::Privmsg,
            vec![channel.name.to_string(), "hello".to_string()],
        );

        let result = handle_privmsg_command(
            message,
            &client.nickname,
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
                ErrorResponse::CannotSendToChannel { channel: c } => {
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

        drop(listener);
        assert_eq!(Err(ServerError::ChannelIsModerated), result);
    }

    #[test]
    fn test_privmsg_cant_send_message_to_no_message_from_outside_channel() {
        let listener = TcpListener::bind("127.0.0.1:8130".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8130".to_string(),
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
            create_client_for_test(&session, "127.0.0.1:8130".to_string(), "sender".to_string());
        let mut client2 = create_client_for_test(
            &session,
            "127.0.0.1:8130".to_string(),
            "receiver".to_string(),
        );
        client2.connected = true;

        let mut channel = Channel::new("#channel_test".to_string(), "".to_string(), vec![]);
        channel.users.push(client2.nickname.clone());
        channel.operators.push(client2.nickname.clone());
        channel.modes.push(ChannelFlag::NoMessageFromOutside);

        let mut channels_lock = write_lock_channels(&session).unwrap();
        channels_lock.insert(channel.name.clone(), channel.clone());
        drop(channels_lock);

        let message = create_message_for_test(
            MessageType::Privmsg,
            vec![channel.name.to_string(), "hello".to_string()],
        );

        let result = handle_privmsg_command(
            message,
            &client.nickname,
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
                ErrorResponse::CannotSendToChannel { channel: c } => {
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

        drop(listener);
        assert_eq!(Err(ServerError::UserNotInChannel), result);
    }
}
