use model::{network::Network, session::Session};
use std::{
    io::{Read, Write},
    net::TcpStream,
    sync::Arc,
};

use crate::commands::command_utils::lock_sockets;
use crate::server_errors::ServerError;

//static CRLF: &str = "\r\n";
const MAX_MSG_SIZE: usize = 510;

/// Function that writes the socket received.
/// # Arguments
/// * `arc_socket` - The socket to write to.
/// * `message` - The message to write.
pub fn write_socket(arc_socket: Arc<TcpStream>, message: &str) -> Result<(), ServerError> {
    let mut msg = message.to_owned().into_bytes();
    msg.resize(MAX_MSG_SIZE, 0);
    arc_socket.as_ref().write_all(&msg)?;
    Ok(())
}

/// Function that reads the socket received. It returs
/// the message read in a String.
/// # Arguments
/// * `arc_socket` - The socket to read from.
pub fn read_socket(arc_socket: Arc<TcpStream>) -> Result<String, ServerError> {
    let mut buff = [0u8; MAX_MSG_SIZE];
    arc_socket.as_ref().read_exact(&mut buff)?;
    let msg = buff.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
    Ok(String::from_utf8_lossy(&msg).to_string())
}

/// Function that sends a message to the client socket in session.
/// # Arguments
/// * `session` - The session to send the message to.
/// * `nickname` - The nickname of the client.
/// * `message` - The message to send.
pub fn inform_client(
    session: &Session,
    nickname: &String,
    message: &str,
) -> Result<(), ServerError> {
    let sockets = lock_sockets(session)?;
    if let Some(socket) = sockets.get(nickname) {
        write_socket(socket.clone(), message)?
    }
    Ok(())
}

/// Function that sends a message to a server.
/// # Arguments
/// * `network` - The network to send the message to.
/// * `servername` - The server to send the message to.
/// * `message` - The message to send.
pub fn inform_server(
    network: &Network,
    servername: &String,
    message: &str,
) -> Result<(), ServerError> {
    let server_lock = network.server.as_ref().write()?;
    if let Some((father_name, father_socket)) = server_lock.father.to_owned() {
        if father_name == *servername {
            write_socket(father_socket, message)?;
            drop(server_lock);
            return Ok(());
        }
    }
    for (child_name, child_socket) in server_lock.children.clone() {
        if child_name == *servername {
            write_socket(child_socket, message)?;
            break;
        }
    }
    drop(server_lock);
    Ok(())
}

/// Function that sends a message to the servers in the network.
/// # Arguments
/// * `network` - The network to send the message to.
/// * `server_name` - The server to exclude from the message.
/// * `message` - The message to send.
pub fn inform_network(
    network: &Network,
    server_name: &String,
    message: &str,
) -> Result<(), ServerError> {
    let server_lock = network.server.as_ref().write()?;
    if let Some((father_name, father_socket)) = server_lock.father.to_owned() {
        if father_name != *server_name {
            write_socket(father_socket, message)?;
        }
    }

    for (child_name, child_socket) in server_lock.children.clone() {
        if child_name != *server_name {
            write_socket(child_socket.clone(), message)?;
        }
    }
    drop(server_lock);
    Ok(())
}
