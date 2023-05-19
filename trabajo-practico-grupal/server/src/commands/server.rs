use crate::{server_errors::ServerError, socket::write_socket};
use model::{message::Message, network::Network};

/// Function that handles the SERVER command received from another server.
/// It inserts the new server into the network struct and informs the network
/// of the new server.
/// # Arguments
/// * `network` - The network struct that contains all the information about the network.
/// * `name` - The name of the server that is being added to the network.
/// * `message` - The message struct that contains the message received.
pub fn handle_server_command(
    message: Message,
    name: &String,
    network: &Network,
) -> Result<(), ServerError> {
    if message.parameters.len() < 2 {
        return Err(ServerError::InvalidParameters);
    }

    let from = match message.prefix.to_owned() {
        Some(p) => p,
        None => "".to_string(),
    };
    let new_server_name = message.parameters[0].to_owned();
    let hopcount = message.parameters[1].parse::<u8>().unwrap_or(0);

    if new_server_name == *name {
        return Err(ServerError::ServerAlreadyRegistered);
    }

    let mut servers_lock = network.servers.as_ref().write()?;
    if servers_lock.get(&new_server_name).is_some() {
        drop(servers_lock);
        return Err(ServerError::ServerAlreadyRegistered);
    }

    println!("Received from server: {}", from);
    println!("New server entered the IRC Network: {}", new_server_name);

    let server_lock = network.server.as_ref().write()?;

    let mut msg = message;
    msg.parameters[1] = (hopcount + 1).to_string();
    msg.trailing = Some(server_lock.name.to_owned());
    let buff = Message::deserialize(msg.to_owned())?;

    if let Some((father_name, father_socket)) = server_lock.father.to_owned() {
        if father_name != from {
            write_socket(father_socket, &buff)?;
        }
    }

    for (child_name, child_socket) in server_lock.children.clone().into_iter() {
        if *child_name != from {
            write_socket(child_socket, &buff)?;
        }
    }

    servers_lock.insert(new_server_name, hopcount);

    drop(server_lock);
    drop(servers_lock);

    Ok(())
}
