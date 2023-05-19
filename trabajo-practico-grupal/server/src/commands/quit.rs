use crate::server_errors::ServerError;
use model::{message::Message, session::Session};
use std::net::Shutdown;

use super::command_utils::lock_sockets;

/// Handles the quit command, closing the connection with the client.
/// # Arguments
/// * `message` - The message received from the client
/// * `nickname` - The nickname of the client
/// * `session` - The session of the server
///
/// Returns a Result with a ServerError if an error occurs.
///
pub fn handle_quit_command(
    message: Message,
    nickname: String,
    session: &Session,
) -> Result<(), ServerError> {
    if message.parameters.is_empty() {
        println!("QUIT {}", nickname);
    } else {
        println!("QUIT {}", message.parameters[0]);
    }

    if let Some(socket) = lock_sockets(session)?.get(&nickname) {
        socket.shutdown(Shutdown::Both)?;
    }
    Ok(())
}
