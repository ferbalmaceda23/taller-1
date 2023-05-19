use std::{io::Write, sync::mpsc::Receiver};

use model::{persistence::PersistenceType, session::Session};

use crate::server_errors::ServerError;

static CLIENTS_PATH: &str = "server/rsc/clients.txt";
static CHANNELS_PATH: &str = "server/rsc/channels.txt";

/// Function that sendes the action and the data to
/// be done by the database
/// # Arguments
/// * `persistence_type` - The action to be done by the database
/// * `data` - The data to be saved by the database
/// * `session` - The session of the current server.
pub fn inform_database(
    persistence_type: PersistenceType,
    data: String,
    session: &Session,
) -> Result<(), ServerError> {
    let info_to_db = (persistence_type, data);
    session.database_sender.send(info_to_db)?;
    Ok(())
}

/// Function that receives the action and data to be done by the database
/// # Arguments
/// * `rx` - The receiver of the database
pub fn handle_database(rx: Receiver<(PersistenceType, String)>) {
    std::thread::spawn(move || {
        while let Ok((persistence_type, data)) = rx.recv() {
            match handle_persistence(persistence_type, data) {
                Ok(_) => (),
                Err(e) => println!("Error handling persistence: {:?}", e),
            }
        }
    });
}

/// Function that handles the actoin to be done by the database
/// # Arguments
/// * `persistence_type` - The action to be done by the database
/// * `data` - The data to be saved by the database
fn handle_persistence(persystence_type: PersistenceType, data: String) -> Result<(), ServerError> {
    match persystence_type {
        PersistenceType::ClientSave => persist_client(data)?,
        PersistenceType::ClientUpdate(id) => update_client(id, data)?,
        PersistenceType::ClientDelete(id) => delete_client(id)?,
        PersistenceType::ChannelSave => persist_channel(data)?,
        PersistenceType::ChannelUpdate(id) => update_channel(id, data)?,
        PersistenceType::ChannelDelete(id) => delete_channel(id)?,
    }
    Ok(())
}

/// Function that saves a new client to the database
/// # Arguments
/// * `data` - The data to be saved by the database
pub fn persist_client(data: String) -> Result<(), ServerError> {
    let mut file = std::fs::OpenOptions::new()
        .create(false)
        .write(true)
        .append(true)
        .open(CLIENTS_PATH)?;
    file.write_all(data.as_bytes())?;
    file.write_all("\n".as_bytes())?;
    Ok(())
}

/// Function that updates a client in the database
/// # Arguments
/// * `id` - The nickname that identifies the client to be updated
/// * `data` - The data to be saved by the database
pub fn update_client(id: String, data: String) -> Result<(), ServerError> {
    let clients_str = std::fs::read_to_string(CLIENTS_PATH)?;
    let mut clients = clients_str
        .split('\n')
        .map(|x| x.to_string())
        .collect::<Vec<_>>();
    if clients.len() > 1 {
        clients.pop();
    }
    for client in &mut clients {
        if client.split(';').collect::<Vec<_>>()[0] == id {
            *client = data;
            break;
        }
    }
    std::fs::remove_file(CLIENTS_PATH)?;
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(CLIENTS_PATH)?;
    file.write_all(clients.join("\n").as_bytes())?;
    file.write_all("\n".as_bytes())?;
    Ok(())
}

/// Function that deletes a client from the database
/// # Arguments
/// * `id` - The nickname that identifies the client to be deleted
pub fn delete_client(id: String) -> Result<(), ServerError> {
    let clients_str = std::fs::read_to_string(CLIENTS_PATH)?;
    let mut clients = clients_str.split('\n').collect::<Vec<_>>();
    if clients.len() > 1 {
        clients.pop();
    }
    for i in 0..clients.len() {
        if clients[i].split(';').collect::<Vec<_>>()[0] == id {
            clients.remove(i);
            break;
        }
    }
    std::fs::remove_file(CLIENTS_PATH)?;
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(CLIENTS_PATH)?;
    file.write_all(clients.join("\n").as_bytes())?;
    file.write_all("\n".as_bytes())?;
    Ok(())
}

/// Function that saves a new channel to the database
/// # Arguments
/// * `data` - The data to be saved by the database
pub fn persist_channel(data: String) -> Result<(), ServerError> {
    let mut file = std::fs::OpenOptions::new()
        .create(false)
        .write(true)
        .append(true)
        .open(CHANNELS_PATH)?;
    file.write_all(data.as_bytes())?;
    file.write_all("\n".as_bytes())?;
    Ok(())
}

/// Function that updates a channel in the database
/// # Arguments
/// * `id` - The name that identifies the channel to be updated
/// * `data` - The data to be saved by the database
pub fn update_channel(id: String, data: String) -> Result<(), ServerError> {
    let channels_str = std::fs::read_to_string(CHANNELS_PATH)?;
    let mut channels = channels_str
        .split('\n')
        .map(|x| x.to_string())
        .collect::<Vec<_>>();
    if channels.len() > 1 {
        channels.pop();
    }
    for channel in &mut channels {
        if channel.split(';').collect::<Vec<_>>()[0] == id {
            *channel = data;
            break;
        }
    }
    std::fs::remove_file(CHANNELS_PATH)?;
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(CHANNELS_PATH)?;
    file.write_all(channels.join("\n").as_bytes())?;
    file.write_all("\n".as_bytes())?;
    Ok(())
}

/// Function that deletes a channel from the database
/// # Arguments
/// * `id` - The name that identifies the channel to be deleted
pub fn delete_channel(id: String) -> Result<(), ServerError> {
    let channels_str = std::fs::read_to_string(CHANNELS_PATH)?;
    let mut channels = channels_str.split('\n').collect::<Vec<_>>();
    if channels.len() > 1 {
        channels.pop();
    }
    for i in 0..channels.len() {
        if channels[i].split(';').collect::<Vec<_>>()[0] == id {
            channels.remove(i);
            break;
        }
    }
    std::fs::remove_file(CHANNELS_PATH)?;
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(CHANNELS_PATH)?;
    file.write_all(channels.join("\n").as_bytes())?;
    file.write_all("\n".as_bytes())?;
    Ok(())
}
