use std::{collections::HashMap, net::TcpStream, sync::Arc, thread, time::Duration};

use model::{
    channel::Channel,
    channelflag::ChannelFlag,
    dcc::{DccMessage, DccMessageType},
    message::Message,
    network::Network,
    responses::{errors::ErrorResponse, replies::CommandResponse},
    session::Session,
};

use crate::{
    server_errors::ServerError,
    socket::{inform_client, inform_network, write_socket},
};

use super::command_utils::{read_lock_clients, write_lock_channels};

/// Function that handles the command `NICK` received from a connected server.
/// # Arguments
/// * `message` - The message struct that contains the message received from the server.
/// * `name` - The name of the server that sent the message.
/// * `_session` - The session of the current server.
/// * `network` - The struct that contains information about the network.
pub fn handle_server_nick_command(
    message: Message,
    name: &String,
    _session: &Session,
    network: &Network,
) -> Result<(), ServerError> {
    let nickname = message.parameters[0].to_owned();
    let hopcount = message.parameters[1].parse::<u8>().unwrap_or(0);
    println!("New client connected to network: {}", nickname);
    // add new client to network info
    let mut clients_lock = network.clients.as_ref().write()?;
    clients_lock.insert(nickname, hopcount);
    drop(clients_lock);

    // inform father and children about new client
    let mut msg = message;
    msg.prefix = Some(name.to_owned());
    msg.parameters[1] = (hopcount + 1).to_string();
    let msg = Message::deserialize(msg)?;

    inform_network(network, name, &msg)?;
    Ok(())
}

/// Function that handles the command `PRIVMSG` received from a server.
/// # Arguments
/// * `message` - The message struct that contains the message received from the server.
/// * `server_name` - The name of the server that sent the message.
/// * `session` - The session of the current server.
/// * `network` - The struct that contains information about the network.
pub fn handle_server_privmsg_command(
    message: Message,
    nickname: &String,
    server_name: &String,
    session: &Session,
    network: &Network,
) -> Result<(), ServerError> {
    let receiver = message.parameters[0].to_owned();

    let clients = network.clients.as_ref().read()?;
    if clients.get(&receiver).is_some() {
        let server_lock = network.server.as_ref().write()?;
        if let Some((father_name, father_socket)) = server_lock.father.to_owned() {
            if *server_name != father_name {
                let buff = Message::deserialize(message.to_owned())?;
                write_socket(father_socket, &buff)?;
            }
        }
        for (child_server_name, child_socket) in server_lock.children.clone().into_iter() {
            if *server_name != child_server_name {
                let buff = Message::deserialize(message.to_owned())?;
                write_socket(child_socket, &buff)?;
            }
        }
    } else {
        let response = ErrorResponse::NoSuchNick { nickname: receiver }.to_string();
        inform_client(session, nickname, &response)?;
    }

    Ok(())
}

/// Function that handles the command `AWAY` received from a server.
/// # Arguments
/// * `message` - The message struct that contains the message received from the server.
/// * `name` - The name of the server that sent the message.
/// * `session` - The session of the current server.
/// * `network` - The struct that contains information about the network.
pub fn handle_server_away_command(
    message: Message,
    name: &String,
    session: &Session,
    network: &Network,
) -> Result<(), ServerError> {
    let sender = match message.prefix.to_owned() {
        Some(p) => p,
        None => "".to_owned(),
    };

    let receiver = message.parameters[0].to_owned();

    let away_msg = match message.trailing.to_owned() {
        Some(t) => t,
        None => "".to_owned(),
    };

    let local_clients = read_lock_clients(session)?;
    if local_clients.get(&sender).is_some() {
        let response = CommandResponse::Away {
            nickname: receiver,
            message: away_msg,
        }
        .to_string();
        inform_client(session, &sender, &response)?;
    } else {
        let msg = Message::deserialize(message)?;
        inform_network(network, name, &msg)?;
    }
    drop(local_clients);

    Ok(())
}

pub fn handle_server_dcc_command(
    message: Message,
    name: &String,
    session: &Session,
    network: &Network,
) -> Result<(), ServerError> {
    let dcc_message = DccMessage::deserialize(Message::deserialize(message)?)?;
    let requested_client = dcc_message.parameters[0].to_owned();
    let local_clients = read_lock_clients(session)?;

    // match dcc_message.command {
    //     DccMessageType::Chat => {
    //         if let Some(client) = local_clients.get(&requested_client) {
    //             if client.connected {
    //                 inform_client(session, &requested_client, &DccMessage::serialize(dcc_message)?)?;
    //             } else {
    //                 let response = format!("DCC CLOSE {} NotConnected", requested_client);
    //                 let ip = dcc_message.parameters[1].to_owned();
    //                 let port = dcc_message.parameters[2].to_owned();
    //                 thread::sleep(Duration::from_millis(500));
    //                 let arc_dcc_socket = Arc::new(TcpStream::connect(format!("{}:{}", ip, port))?);
    //                 write_socket(arc_dcc_socket, &response)?;
    //             }
    //         } else {
    //             inform_network(network, name, &DccMessage::serialize(dcc_message)?)?;
    //         }
    //     }
    //     _ => {}
    // }

    if dcc_message.command == DccMessageType::Chat {
        if let Some(client) = local_clients.get(&requested_client) {
            if client.connected {
                inform_client(
                    session,
                    &requested_client,
                    &DccMessage::serialize(dcc_message)?,
                )?;
            } else {
                let response = format!("DCC CLOSE {} NotConnected", requested_client);
                let ip = dcc_message.parameters[1].to_owned();
                let port = dcc_message.parameters[2].to_owned();
                thread::sleep(Duration::from_millis(500));
                let arc_dcc_socket = Arc::new(TcpStream::connect(format!("{}:{}", ip, port))?);
                write_socket(arc_dcc_socket, &response)?;
            }
        } else {
            inform_network(network, name, &DccMessage::serialize(dcc_message)?)?;
        }
    }

    drop(local_clients);

    Ok(())
}

/// Function that handles the reply of the command `WHO` received from a server.
/// # Arguments
/// * `users` - The users to be added to the current server.
/// * `_name` - The name of the current server.
/// * `session` - The session of the current server.
/// * `network` - The struct that contains information about the network.
pub fn handle_server_who_reply(
    users: Vec<String>,
    server_name: &str,
    session: &Session,
    network: &Network,
) -> Result<(), ServerError> {
    let local_clients = read_lock_clients(session)?;
    let mut network_clients = network.clients.as_ref().write()?;

    for user in users.clone() {
        if !local_clients.contains_key(&user) {
            println!("New network client: {}", user);
            network_clients.insert(user, 1);
        }
    }

    drop(local_clients);
    drop(network_clients);

    let response = CommandResponse::WhoReply { users }.to_string();
    inform_network(network, &server_name.to_string(), &response)?;
    Ok(())
}

/// Function that handles the reply of the command `NAMES` received from a server.
/// # Arguments
/// * `channel_name` - The name of the channel to be added.
/// * `channel_users` - The users of the channel to be added.
/// * `session` - The session of the current server.
/// * `_network` - The struct that contains information about the network.
pub fn handle_server_names_reply(
    channel_name: String,
    channel_users: Vec<String>,
    session: &Session,
    network: &Network,
    server_name: &str,
) -> Result<(), ServerError> {
    let mut channels = write_lock_channels(session)?;
    if !channels.contains_key(&channel_name) {
        let channel = Channel {
            name: channel_name.to_owned(),
            users: channel_users.to_owned(),
            topic: "".to_string(),
            operators: vec![],
            modes: vec![],
            banned_users: vec![],
            password: None,
            limit: None,
            moderators: vec![],
        };
        println!("New distributed channel: {}", channel_name);
        channels.insert(channel_name.to_owned(), channel);
    }
    drop(channels);

    let response = CommandResponse::Names {
        channel: channel_name,
        names: channel_users,
    }
    .to_string();
    inform_network(network, &server_name.to_string(), &response)?;

    Ok(())
}

/// Function that handles the reply of the command `LIST` received from a server.
/// # Arguments
/// * `channel` - The name of the channel to be updated.
/// * `channel_topic` - The topic of the channel to be setted.
/// * `session` - The session of the current server.
/// * `_network` - The struct that contains information about the network.
pub fn handle_server_list_reply(
    channel_name: String,
    channel_topic: String,
    session: &Session,
    network: &Network,
    server_name: &str,
) -> Result<(), ServerError> {
    let mut channels = write_lock_channels(session)?;
    if let Some(mut channel) = channels.get_mut(&channel_name) {
        channel.topic = channel_topic.to_owned();
    }
    drop(channels);

    let response = CommandResponse::List {
        channel: channel_name,
        topic: channel_topic,
    }
    .to_string();
    inform_network(network, &server_name.to_string(), &response)?;
    Ok(())
}

/// Function that handles the reply of the command `MODE` received from a server.
/// # Arguments
/// * `channel_name` - The name of the channel to be updated.
/// * `channel_modes` - The modes of the channel to be setted.
/// * `session` - The session of the current server.
/// * `_network` - The struct that contains information about the network.
pub fn handle_mode_server_reply(
    channel_name: String,
    channel_modes: HashMap<String, String>,
    session: &Session,
    _network: &Network,
) -> Result<(), ServerError> {
    let mut channels = write_lock_channels(session)?;
    if let Some(channel) = channels.get_mut(&channel_name) {
        set_modes(channel, channel_modes);
    }
    drop(channels);
    Ok(())
}

/// Fuction that sets the modes of a channel.
/// # Arguments
/// * `channel` - The channel to be updated.
/// * `modes` - The modes of the channel to be setted.
fn set_modes(channel: &mut Channel, modes: HashMap<String, String>) {
    for flag in ChannelFlag::iter() {
        match flag {
            ChannelFlag::Private => {
                if let Some(mode) = modes.get(&flag.to_string()) {
                    if *mode == "+" {
                        channel.modes.push(flag);
                    }
                }
            }
            ChannelFlag::Secret => {
                if let Some(mode) = modes.get(&flag.to_string()) {
                    if *mode == "+" {
                        channel.modes.push(flag);
                    }
                }
            }
            ChannelFlag::InviteOnly => {
                if let Some(mode) = modes.get(&flag.to_string()) {
                    if *mode == "+" {
                        channel.modes.push(flag);
                    }
                }
            }
            ChannelFlag::ModeratedChannel => {
                if let Some(mode) = modes.get(&flag.to_string()) {
                    if *mode == "+" {
                        channel.modes.push(flag);
                    }
                }
            }
            ChannelFlag::NoMessageFromOutside => {
                if let Some(mode) = modes.get(&flag.to_string()) {
                    if *mode == "+" {
                        channel.modes.push(flag);
                    }
                }
            }
            ChannelFlag::TopicSettableOnlyOperators => {
                if let Some(mode) = modes.get(&flag.to_string()) {
                    if *mode == "+" {
                        channel.modes.push(flag);
                    }
                }
            }
            ChannelFlag::ChannelOperator => {
                if let Some(operators) = modes.get(&flag.to_string()) {
                    let opers = operators
                        .split(',')
                        .filter_map(|s| {
                            if !s.is_empty() {
                                Some(s.to_string())
                            } else {
                                None
                            }
                        })
                        .collect();
                    channel.operators = opers;
                }
            }
            ChannelFlag::Ban => {
                if let Some(banned_users) = modes.get(&flag.to_string()) {
                    let bans = banned_users
                        .split(',')
                        .filter_map(|s| {
                            if !s.is_empty() {
                                Some(s.to_string())
                            } else {
                                None
                            }
                        })
                        .collect();
                    channel.banned_users = bans;
                }
            }
            ChannelFlag::ChannelKey => {
                if let Some(password) = modes.get(&flag.to_string()) {
                    if !password.is_empty() {
                        channel.password = Some(password.to_string());
                    } else {
                        channel.password = None
                    }
                }
            }
            ChannelFlag::UserLimit => {
                if let Some(limit) = modes.get(&flag.to_string()) {
                    if let Ok(l) = limit.parse::<i32>() {
                        channel.limit = Some(l)
                    }
                }
            }
            ChannelFlag::SpeakInModeratedChannel => {
                if let Some(moderators) = modes.get(&flag.to_string()) {
                    let mods = moderators
                        .split(',')
                        .filter_map(|s| {
                            if !s.is_empty() {
                                Some(s.to_string())
                            } else {
                                None
                            }
                        })
                        .collect();
                    channel.moderators = mods;
                }
            }
            ChannelFlag::Other => {}
        }
    }
}

/// Function that handles the reply of the command `SERVER` received from a server.
/// # Arguments
/// * `hash_name` - HashMap with information about the servers of the network.
/// * `_session` - The session of the current server.
/// * `network` - The struct that contains information about the network.
pub fn handle_server_server_reply(
    hash_servers: HashMap<String, u8>,
    _session: &Session,
    network: &Network,
) -> Result<(), ServerError> {
    let mut servers = network.servers.as_ref().write()?;
    let server_lock = network.server.as_ref().read()?;
    for (server, hops) in hash_servers {
        if !servers.contains_key(&server) && server != *server_lock.name {
            servers.insert(server, hops + 1);
        }
    }
    drop(servers);
    drop(server_lock);
    Ok(())
}
