use std::{
    collections::HashMap,
    net::{TcpListener, TcpStream},
    sync::{mpsc::Sender, Arc, Mutex},
};

use model::{
    client::Client,
    message::{Message, MessageType},
    network::Network,
    persistence::PersistenceType,
    session::Session,
};

use crate::{
    message_handler::handle_client_message, server_errors::ServerError, socket::inform_network,
};

/// Returns a channel hashmap write lock for the session in case of success, and a ServerError::LockError in case of
/// failure.
pub fn write_lock_channels(
    session: &Session,
) -> Result<
    std::sync::RwLockWriteGuard<std::collections::HashMap<String, model::channel::Channel>>,
    ServerError,
> {
    let channels_lock = match session.channels.as_ref().write() {
        Ok(c) => c,
        Err(_) => {
            println!("Error locking channels");
            return Err(ServerError::LockError);
        }
    };
    Ok(channels_lock)
}

/// Returns a channel hashmap read lock for the session in case of success, and a ServerError::LockError in case of
/// failure.
pub fn read_lock_channels(
    session: &Session,
) -> Result<
    std::sync::RwLockReadGuard<std::collections::HashMap<String, model::channel::Channel>>,
    ServerError,
> {
    let channels_lock = match session.channels.as_ref().read() {
        Ok(c) => c,
        Err(_) => {
            println!("Error locking channels");
            return Err(ServerError::LockError);
        }
    };
    Ok(channels_lock)
}

/// Returns a client hashmap read lock for the session in case of success, and a ServerError::LockError in case of
/// failure.
pub fn read_lock_clients(
    session: &Session,
) -> Result<
    std::sync::RwLockReadGuard<std::collections::HashMap<String, model::client::Client>>,
    ServerError,
> {
    let clients_lock = match session.clients.as_ref().read() {
        Ok(c) => c,
        Err(_) => {
            println!("Error locking clients");
            return Err(ServerError::LockError);
        }
    };
    Ok(clients_lock)
}

/// Returns a client hashmap write lock for the session in case of success, and a ServerError::LockError in case of
/// failure
pub fn write_lock_clients(
    session: &Session,
) -> Result<
    std::sync::RwLockWriteGuard<std::collections::HashMap<String, model::client::Client>>,
    ServerError,
> {
    let clients_lock = match session.clients.as_ref().write() {
        Ok(c) => c,
        Err(_) => {
            println!("Error locking clients");
            return Err(ServerError::LockError);
        }
    };
    Ok(clients_lock)
}

/// Returns a mutex lock of the session sockets in case of success, and a ServerError::LockError in case of failure
pub fn lock_sockets(
    session: &Session,
) -> Result<std::sync::MutexGuard<std::collections::HashMap<String, Arc<TcpStream>>>, ServerError> {
    let sockets_lock = match session.sockets.as_ref().lock() {
        Ok(s) => s,
        Err(_) => {
            println!("Error locking sockets");
            return Err(ServerError::LockError);
        }
    };
    Ok(sockets_lock)
}

pub fn create_client_for_test(session: &Session, addr: String, nickname: String) -> Client {
    let client = Client::from_connection(
        nickname,
        "username".to_string(),
        "hostname".to_string(),
        "servername".to_string(),
        "realname".to_string(),
        None,
        true,
    );

    session
        .clients
        .write()
        .unwrap()
        .insert(client.clone().nickname, client.clone());

    let client_stream = Arc::new(TcpStream::connect(addr).unwrap());
    let mut sockets = match lock_sockets(session) {
        Ok(sockets) => sockets,
        Err(_) => panic!("Could not lock sockets"),
    };
    sockets.insert(client.nickname.clone(), client_stream.clone());
    drop(sockets);
    drop(client_stream);

    client
}

pub fn create_session_for_test(tx: Sender<(PersistenceType, String)>) -> Session {
    let clients = Arc::new(std::sync::RwLock::new(std::collections::HashMap::new()));
    let sockets = Arc::new(Mutex::new(HashMap::new()));
    let channels = Arc::new(std::sync::RwLock::new(HashMap::new()));
    Session {
        clients,
        sockets,
        channels,
        database_sender: tx,
    }
}

pub fn create_server_for_test(addr: String) -> TcpListener {
    TcpListener::bind(addr).unwrap()
}

pub fn create_message_for_test(command: MessageType, parameters: Vec<String>) -> Message {
    Message::new(
        Some("".to_string()),
        command,
        parameters,
        Some("".to_string()),
    )
}

pub fn fetch_info(
    session: &Session,
    network: &Network,
    server_name: &String,
) -> Result<(), ServerError> {
    let clients_lock = read_lock_clients(session)?;
    let nicknames = clients_lock.keys().collect::<Vec<_>>();
    for nick in nicknames.clone() {
        let msg_names = Message::new(None, MessageType::Names, vec![], None);
        let msg_list = Message::new(None, MessageType::List, vec![], None);
        match handle_client_message(msg_names, nick.to_string(), session, network, server_name) {
            Ok(_) => (),
            Err(e) => println!("Error handling message: {:?}", e),
        }
        match handle_client_message(msg_list, nick.to_string(), session, network, server_name) {
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
        let message = format!(":{} LIST", net_nick);
        inform_network(network, server_name, &message)?;
    }
    drop(network_clients);
    drop(clients_lock);
    Ok(())
}
