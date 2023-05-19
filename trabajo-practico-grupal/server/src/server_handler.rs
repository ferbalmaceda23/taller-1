use std::{net::TcpStream, sync::Arc};

use model::{
    message::{Message, MessageType},
    network::Network,
    responses::{replies::CommandResponse, response::Response},
    session::Session,
};

use crate::{
    commands::{
        command_utils::{read_lock_channels, read_lock_clients},
        invite::handle_invite_command,
        join::handle_join_command,
        kick::handle_kick_command,
        list::handle_list_command,
        mode::handle_mode_command,
        names::handle_names_command,
        part::handle_part_command,
        privmsg::handle_privmsg_command,
        server::handle_server_command,
        server_commands_handler::{
            handle_mode_server_reply, handle_server_away_command, handle_server_dcc_command,
            handle_server_list_reply, handle_server_names_reply, handle_server_nick_command,
            handle_server_server_reply, handle_server_who_reply,
        },
        squit::handle_squit_command,
        topic::handle_topic_command,
        who::handle_who_command,
    },
    server_errors::ServerError,
    socket::{read_socket, write_socket},
};

/// Function that handles the server connection.
/// If the credentials are right ir registers the client and
/// proceeds to handle the messages.
/// # Arguments
/// * `arc_socket` - An atomic reference of the new server socket.
/// * `message` - The message that the new server sent.
/// * `session` - The session of the current server.
/// * `network` - The struct that contains the information of the network.
pub fn handle_server(
    arc_socket: Arc<TcpStream>,
    message: Message,
    session: Session,
    network: Network,
) -> Result<(), ServerError> {
    let mut name = None;
    register_server(message, &mut name, arc_socket.clone(), &network)?;
    if let Some(n) = name {
        while let Ok(msg_str) = read_socket(arc_socket.clone()) {
            match Message::serialize(msg_str.to_owned()) {
                Ok(msg) => {
                    match handle_server_message(msg, &n, &session, &network) {
                        Ok(_) => (),
                        Err(e) => println!("Error handling server message {:?}", e),
                    };
                }
                Err(e) => {
                    match Response::serialize(msg_str) {
                        Some(r) => {
                            match handle_server_response(r, &n, &session, &network) {
                                Ok(_) => (),
                                Err(e) => println!("Error handling server response {:?}", e),
                            };
                        }
                        None => println!("Error parsing server msg {:?}", e),
                    };
                }
            };
        }
        disconnect_server(&network, &n)?;
    }
    Ok(())
}

/// Function that deletes a server from the network struct
/// when it disconnects.
/// # Arguments
/// * `network` - The struct that contains the information of the network.
/// * `server_name` - The name of the server that disconnected.
fn disconnect_server(network: &Network, server_name: &String) -> Result<(), ServerError> {
    let mut servers_lock = network.servers.write()?;
    servers_lock.remove(server_name);
    drop(servers_lock);
    Ok(())
}

/// Function that handles the messages received from the server connected.
/// # Arguments
/// * `message` - The message that the server sent.
/// * `name` - The name of the server that sent the message.
/// * `session` - The session of the current server.
/// * `network` - The struct that contains the information of the network.
fn handle_server_message(
    message: Message,
    name: &String,
    session: &Session,
    network: &Network,
) -> Result<(), ServerError> {
    match message.command {
        MessageType::Server => {
            handle_server_command(message, name, network)?;
        }
        MessageType::Squit => {
            handle_squit_command(message, name, network)?;
        }
        MessageType::Privmsg => {
            let nickname = match message.prefix.to_owned() {
                Some(p) => p,
                None => "".to_owned(),
            };
            handle_privmsg_command(message, &nickname, session, network, name)?;
        }
        MessageType::Nick => {
            handle_server_nick_command(message, name, session, network)?;
        }
        MessageType::Who => {
            handle_who_command(
                message,
                "".to_owned(),
                session,
                network,
                Some(name.to_owned()),
            )?;
        }
        MessageType::List => {
            if let Some(prx) = message.prefix.to_owned() {
                let clients = read_lock_clients(session)?;
                if clients.get(&prx).is_some() {
                    drop(clients);
                    handle_list_command(message, &prx, session, network, None)?;
                } else {
                    drop(clients);
                }
            } else {
                handle_list_command(
                    message,
                    &"".to_string(),
                    session,
                    network,
                    Some(name.to_owned()),
                )?;
            }
        }
        MessageType::Names => {
            if let Some(prx) = message.prefix.to_owned() {
                let clients = read_lock_clients(session)?;
                if clients.get(&prx).is_some() {
                    drop(clients);
                    handle_names_command(message, prx, session, network, None)?;
                } else {
                    drop(clients);
                }
            } else {
                handle_names_command(
                    message,
                    "".to_string(),
                    session,
                    network,
                    Some(name.to_owned()),
                )?;
            }
        }
        MessageType::Join => {
            let nickname = match message.prefix.to_owned() {
                Some(p) => p,
                None => "".to_owned(),
            };
            handle_join_command(message, &nickname, session, network, name)?;
        }
        MessageType::Invite => {
            let nickname = match message.prefix.to_owned() {
                Some(p) => p,
                None => "".to_owned(),
            };
            handle_invite_command(message, nickname, session, network, name)?;
        }
        MessageType::Kick => {
            let nickname = match message.prefix.to_owned() {
                Some(p) => p,
                None => "".to_owned(),
            };
            handle_kick_command(message, nickname, session, network, name)?;
        }
        MessageType::Part => {
            let nickname = match message.prefix.to_owned() {
                Some(p) => p,
                None => "".to_owned(),
            };
            handle_part_command(message, nickname, session, network, name)?;
        }
        MessageType::Topic => {
            let nickname = match message.prefix.to_owned() {
                Some(p) => p,
                None => "".to_owned(),
            };
            handle_topic_command(message, nickname, session, network, name)?;
        }
        MessageType::Mode => {
            let nickname = match message.prefix.to_owned() {
                Some(p) => p,
                None => "".to_owned(),
            };
            handle_mode_command(message, nickname, session, network, name)?;
        }
        MessageType::Away => {
            handle_server_away_command(message, name, session, network)?;
        }
        MessageType::Dcc => {
            handle_server_dcc_command(message, name, session, network)?;
        }
        _ => {}
    }
    Ok(())
}

/// Function that handles the responses received from the server connected.
/// # Arguments
/// * `response` - The response received from the server.
/// * `name` - The name of the server that sent the response.
/// * `session` - The session of the current server.
/// * `network` - The struct that contains the information of the network.
fn handle_server_response(
    response: Response,
    name: &str,
    session: &Session,
    network: &Network,
) -> Result<(), ServerError> {
    match response {
        Response::ErrorResponse { response: _ } => {}
        Response::CommandResponse { response } => match response {
            CommandResponse::WhoReply { users } => {
                handle_server_who_reply(users, name, session, network)?;
            }
            CommandResponse::Names { channel, names } => {
                handle_server_names_reply(channel, names, session, network, name)?;
            }
            CommandResponse::ChannelMode { channel, modes } => {
                handle_mode_server_reply(channel, modes, session, network)?;
            }
            CommandResponse::List { channel, topic } => {
                handle_server_list_reply(channel, topic, session, network, name)?;
            }
            CommandResponse::Server { servers } => {
                handle_server_server_reply(servers, session, network)?;
            }
            _ => {}
        },
        _ => {}
    }
    Ok(())
}

/// Function that registers a new server in the network struct.
/// # Arguments
/// * `message` - The message received from the new server.
/// * `name` - The name of the new server.
/// * `arc_socket` - An atomic reference of the new server socket.
/// * `network` - The struct that contains the information of the network.
fn register_server(
    message: Message,
    name: &mut Option<String>,
    arc_socket: Arc<TcpStream>,
    network: &Network,
) -> Result<(), ServerError> {
    if message.parameters.len() < 2 {
        return Err(ServerError::InvalidParameters);
    }
    let child_name = message.parameters[0].to_owned();
    let hopcount = message.parameters[1].parse::<u8>().unwrap_or(0);
    let info = match message.trailing.to_owned() {
        Some(t) => t,
        None => String::new(),
    };

    let mut servers_lock = network.servers.as_ref().write()?;
    if servers_lock.get(&child_name).is_some() {
        drop(servers_lock);
        return Err(ServerError::ServerAlreadyRegistered);
    }

    let mut server_lock = network.server.as_ref().write()?;
    if server_lock.children.get(&child_name).is_some() {
        return Err(ServerError::ServerAlreadyRegistered);
    } else {
        *name = Some(child_name.to_owned());
        println!("New child server connected: {}", child_name);
        println!("New server info: {}", info);
        server_lock
            .children
            .insert(child_name.to_owned(), arc_socket);
        servers_lock.insert(child_name.to_owned(), hopcount);

        let mut msg = message;
        msg.prefix = Some(server_lock.name.to_owned());
        msg.parameters[1] = (hopcount + 1).to_string();
        let buff = Message::deserialize(msg.to_owned())?;

        if let Some((_father_name, father_socket)) = server_lock.father.to_owned() {
            write_socket(father_socket, &buff)?;
        }
        for (child_server_name, child_socket) in server_lock.children.clone().into_iter() {
            if *child_server_name != child_name {
                write_socket(child_socket, &buff)?;
            } else {
                let response = CommandResponse::Server {
                    servers: servers_lock.clone(),
                }
                .to_string();
                write_socket(child_socket.clone(), &response)?;
                write_socket(child_socket.clone(), "WHO")?;
                write_socket(child_socket.clone(), "NAMES")?;
                write_socket(child_socket, "LIST")?;
            }
        }
    }

    drop(server_lock);
    drop(servers_lock);
    Ok(())
}

/// Function reads from stdin (console) and sends information
/// to the father server, if exists. If it receives "INFO" from
/// stdin, it prints the information of the server in console.
/// # Arguments
/// * `father_socket` - An option argument that may have the socket of the father.
/// * `session` - The session of the current server.
/// * `network` - The struct that contains the information of the network.
pub fn read_from_stdin(
    father_socket: Option<Arc<TcpStream>>,
    session: &Session,
    network: &Network,
) {
    let session_clone = session.clone();
    let network_clone = network.clone();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        let mut first_command = true;
        if father_socket.is_none() {
            first_command = false;
        }
        let mut fetched = false;
        loop {
            if father_socket.is_some() && !first_command && !fetched {
                if let Some(socket) = father_socket.to_owned() {
                    fetched = true;
                    if write_socket(socket.clone(), "WHO").is_ok() {};
                    if write_socket(socket.clone(), "NAMES").is_ok() {};
                    if write_socket(socket.clone(), "LIST").is_ok() {};
                }
            }
            let mut buff = String::new();
            match stdin.read_line(&mut buff) {
                Ok(_) => {
                    if buff.starts_with("INFO") {
                        print_server_info(&session_clone, &network_clone);
                    } else if first_command && buff.starts_with("SERVER") {
                        if buff.split(' ').count() < 3 {
                            continue;
                        } else {
                            first_command = false;
                        }
                        if let Some(socket) = father_socket.to_owned() {
                            if write_socket(socket.clone(), &buff).is_ok() {}
                        }
                    } else if first_command && !buff.starts_with("SERVER") {
                        println!("You must register to the server with 'SERVER' command");
                    } else if let Some(socket) = father_socket.to_owned() {
                        if write_socket(socket.clone(), &buff).is_ok() {}
                    }
                }
                Err(_) => {
                    println!("Error reading from stdin");
                    break;
                }
            }
        }
    });
}

/// Function thar prints the actual server
/// information in console.
/// # Arguments
/// * `session` - The session of the current server.
/// * `network` - The struct that contains the information of the network.
fn print_server_info(session: &Session, network: &Network) {
    if let Ok(local_clients) = read_lock_clients(session) {
        println!("Local clients:");
        println!("{:?}", local_clients);
    }

    if let Ok(channels) = read_lock_channels(session) {
        println!("Channels:");
        println!("{:?}", channels);
    }

    if let Ok(clients) = network.clients.as_ref().read() {
        println!("Clients:");
        println!("{:?}", clients);
    }

    if let Ok(servers) = network.servers.as_ref().read() {
        println!("Servers:");
        println!("{:?}", servers);
    }
}

/// Function handles the communication if the main server with its father.
/// # Arguments
/// * `session` - The session of the current server.
/// * `network` - The struct that contains the information of the network.
/// * `father_name` - The name of the father server.
/// * `father_socket` - An atomic reference of the father server socket.
pub fn handle_father_comunication(
    session: Session,
    network: Network,
    father_name: String,
    father_socket: Arc<TcpStream>,
) -> Result<(), ServerError> {
    read_from_stdin(Some(father_socket.clone()), &session, &network);
    std::thread::spawn(move || {
        while let Ok(msg) = read_socket(father_socket.clone()) {
            match Message::serialize(msg.to_owned()) {
                Ok(m) => {
                    match handle_server_message(m, &father_name, &session, &network) {
                        Ok(_) => (),
                        Err(e) => println!("Error handling server message {:?}", e),
                    };
                }
                Err(e) => match Response::serialize(msg) {
                    Some(r) => {
                        match handle_server_response(r, &father_name, &session, &network) {
                            Ok(_) => (),
                            Err(e) => println!("Error handling server response {:?}", e),
                        };
                    }
                    None => println!("Error parsing message {:?}", e),
                },
            };
        }
    });
    Ok(())
}
