use crate::server_errors::ServerError;
use model::{message::Message, network::Network};
use std::net::Shutdown;

/// Function that ends a connection with a server.
/// # Arguments
/// * `network` - The network struct that contains the connection.
/// * `message` - The message that sent the server that wants to end the connection.
/// * `name` - The name of the server wants to end the connection.
pub fn handle_squit_command(
    message: Message,
    name: &String,
    network: &Network,
) -> Result<(), ServerError> {
    let mut server_lock = network.server.as_ref().write()?;
    let mut servers_lock = network.servers.as_ref().write()?;
    if let Some(child_socket) = server_lock.children.get(name) {
        child_socket.as_ref().shutdown(Shutdown::Both)?;
        server_lock.children.remove(name);
        servers_lock.remove(name);
        let trailing = match message.trailing {
            Some(t) => t,
            None => "".to_string(),
        };
        println!("Child server {} disconnected: {}", name, trailing);
    }
    drop(server_lock);
    drop(servers_lock);
    Ok(())
}
