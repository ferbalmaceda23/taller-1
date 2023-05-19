use std::{net::TcpStream, sync::Arc};

use model::{
    client::Client,
    message::{Message, MessageType},
    network::Network,
    persistence::PersistenceType,
    responses::{errors::ErrorResponse, replies::CommandResponse},
    session::Session,
};

use crate::{
    database::inform_database,
    message_handler::handle_client_message,
    registration::handle_registration,
    server_errors::ServerError,
    socket::{inform_network, read_socket, write_socket},
};

///Handles the client registration and login, and returns a ServerError in case of failure.
/// If the client is already registered, it handles the client messages.
/// If the client is not registered, it handles the registration.
/// If the client is not registered and the registration fails, it sends an error response to the client.
pub fn handle_client(
    arc_socket: Arc<TcpStream>,
    message: Message,
    session: Session,
    network: Network,
    server_name: &String,
) -> Result<(), ServerError> {
    let mut nickname: Option<String> = Option::None;
    let mut user_parameters = Option::None;
    let mut password = Option::None;

    if register_client(
        message,
        (&mut nickname, &mut user_parameters),
        &mut password,
        arc_socket.clone(),
        &session,
        &network,
        server_name,
    )
    .is_err()
    {
        println!("Error registering client");
    }

    while nickname.is_none() || user_parameters.is_none() {
        let msg = read_socket(arc_socket.clone())?;
        let message = match Message::serialize(msg) {
            Ok(m) => m,
            Err(e) => {
                println!("Error parsing message: {:?}", e);
                continue;
            }
        };

        match register_client(
            message,
            (&mut nickname, &mut user_parameters),
            &mut password,
            arc_socket.clone(),
            &session,
            &network,
            server_name,
        ) {
            Ok(_) => (),
            Err(e) => {
                println!("Error registering client: {:?}", e);
                continue;
            }
        }
    }

    while let Ok(msg) = read_socket(arc_socket.clone()) {
        let msg = match Message::serialize(msg) {
            Ok(m) => m,
            Err(e) => {
                println!("Error parsing message: {:?}", e);
                continue;
            }
        };

        if let Some(nick) = nickname.clone() {
            match handle_client_message(msg, nick, &session, &network, server_name) {
                Ok(_) => (),
                Err(e) => println!("Error handling message: {:?}", e),
            }
        }
    }

    disconnect_client(&nickname, &session);
    Ok(())
}

/// Disconnects the client from the server.
/// # Arguments
/// * `nickname` - The nickname of the client to disconnect.
/// * `session` - The session of the server.
fn disconnect_client(nickname: &Option<String>, session: &Session) {
    if let Some(n) = nickname.to_owned() {
        match session.clients.as_ref().write() {
            Ok(mut clients) => {
                if let Some(c) = clients.get_mut(&n) {
                    c.connected = false;
                    println!("Client {} left the server", n);
                }
                drop(clients);
            }
            Err(_) => println!("Error locking clients"),
        }
    }
}

/// Registers the client if it is not already registered. If an error occurs it sends an error response to the client.
///  # Errors
/// * ServerError::LockError - If the clients cannot be locked.
/// * ServerError::NicknameInUse - If the nickname is already in use.
/// * ServerError::InvalidPassword - If the password is incorrect.
/// * ServerError::ErroneusNickname - If the nickname is invalid.
/// For each error it sends an error response to the client.
pub fn register_client(
    message: Message,
    credentials: (&mut Option<String>, &mut Option<Vec<String>>),
    password: &mut Option<String>,
    client_stream: Arc<TcpStream>,
    session: &Session,
    network: &Network,
    server_name: &String,
) -> Result<(), ServerError> {
    match handle_registration(
        message,
        credentials.0,
        credentials.1,
        password,
        session,
        network,
    ) {
        Ok(some_client) => {
            if let Some(client) = some_client {
                match save_client(session, network, client, client_stream, server_name) {
                    Ok(_) => match session.clients.as_ref().read() {
                        Ok(clients) => {
                            let nicknames = clients.keys().collect::<Vec<_>>();
                            for nick in nicknames.clone() {
                                let msg = Message::new(None, MessageType::Names, vec![], None);
                                match handle_client_message(
                                    msg,
                                    nick.to_string(),
                                    session,
                                    network,
                                    server_name,
                                ) {
                                    Ok(_) => (),
                                    Err(e) => println!("Error handling message: {:?}", e),
                                }
                            }
                            let network_clients = network.clients.read()?;
                            for net_nick in network_clients.keys() {
                                if nicknames.contains(&net_nick) {
                                    continue;
                                }
                                let message = format!(":{} NAMES", net_nick);
                                inform_network(network, server_name, &message)?;
                            }
                            drop(network_clients);
                            drop(clients);
                        }
                        Err(_) => {
                            return Err(ServerError::LockError);
                        }
                    },
                    Err(e) => {
                        println!("Error saving client: {:?}", e);
                        *credentials.0 = None;
                    }
                }
            }
        }
        Err(e) => {
            *credentials.1 = None;
            *credentials.0 = None;
            *password = None;
            match e {
                ServerError::NicknameInUse(nickname) => {
                    let response = ErrorResponse::NickInUse { nickname }.to_string();
                    write_socket(client_stream, response.as_str())?;
                }
                ServerError::InvalidPassword => {
                    let response = ErrorResponse::NotRegistered.to_string();
                    write_socket(client_stream, response.as_str())?;
                }
                ServerError::ErroneusNickname => {
                    let response = ErrorResponse::NotRegistered.to_string();
                    write_socket(client_stream, response.as_str())?;
                }
                _ => return Err(e),
            }
        }
    }
    Ok(())
}

/// Saves the client in the server, and if everything is ok it sends a welcome message to the client and informs the network.
/// # Errors
/// * ServerError::LockError - If the clients cannot be locked.
/// * ServerError::ClientConnected - If a client is already connected with the same nickname.
/// For each error it sends an error response to the client.
fn save_client(
    session: &Session,
    network: &Network,
    client: Client,
    client_stream: Arc<TcpStream>,
    server_name: &String,
) -> Result<(), ServerError> {
    let nick = client.nickname.to_owned();
    match session.clients.as_ref().write() {
        Ok(mut clients) => {
            if let Some(c) = clients.get_mut(&nick) {
                if c.connected {
                    let response = (ErrorResponse::NickInUse {
                        nickname: c.nickname.clone(),
                    })
                    .to_string();
                    write_socket(client_stream, response.as_str())?;
                    return Err(ServerError::ClientConnected);
                }
                c.connected = true;
            } else {
                clients.insert(nick.to_owned(), client.to_owned());
                inform_database(PersistenceType::ClientSave, client.to_string(), session)?;
                let mut network_clients = network.clients.as_ref().write()?;
                network_clients.insert(nick.to_owned(), 0);
                drop(network_clients);
            }
            let response = (CommandResponse::Welcome {
                nickname: client.nickname,
                username: client.username,
                hostname: client.hostname,
            })
            .to_string();
            write_socket(client_stream.clone(), response.as_str())?;
            drop(clients);
        }
        Err(_) => {
            return Err(ServerError::LockError);
        }
    }
    match session.sockets.as_ref().lock() {
        Ok(mut sockets) => {
            sockets.insert(nick.to_owned(), client_stream);
            drop(sockets);
        }
        Err(_) => {
            return Err(ServerError::LockError);
        }
    }

    let msg = format!(":{} NICK {} 1", server_name, nick);
    inform_network(network, server_name, &msg)?;

    Ok(())
}
