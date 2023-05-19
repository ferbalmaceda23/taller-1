use std::{
    io::{Read, Write},
    net::TcpStream,
    sync::Arc,
};

use crate::client_errors::ClientError;

const MAX_MSG_SIZE: usize = 510;

/// Function that writes the socket received.
/// # Arguments
/// * `arc_socket` - The socket to write to.
/// * `message` - The message to write.
pub fn write_socket(arc_socket: Arc<TcpStream>, message: &str) -> Result<(), ClientError> {
    let mut msg = message.to_owned().into_bytes();
    msg.resize(MAX_MSG_SIZE, 0);
    arc_socket.as_ref().write_all(&msg)?;
    Ok(())
}

/// Function that reads the socket received. It returs
/// the message read in a String.
/// # Arguments
/// * `arc_socket` - The socket to read from.
pub fn read_socket(arc_socket: Arc<TcpStream>) -> Result<String, ClientError> {
    let mut buff = [0u8; MAX_MSG_SIZE];
    arc_socket.as_ref().read_exact(&mut buff)?;
    let msg = buff.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
    Ok(String::from_utf8_lossy(&msg).to_string())
}
