use std::{
    collections::HashMap,
    net::TcpStream,
    sync::{Arc, Mutex, RwLock},
};

use crate::{channel::Channel, client::Client, persistence::PersistenceType};

/// Struct that holds the information of the server session
/// # Fields
/// * `clients`: A hashmap that contains the clients of the server.
/// * `channels`: A hashmap that contains the channels of the server.
/// * `sockets`: A hashmap that contains the sockets of the clients.
/// * `database_sender`: The sender of the server that informs the database about changes of channels and clients.
#[derive(Debug, Clone)]
pub struct Session {
    pub clients: Arc<RwLock<HashMap<String, Client>>>,
    pub sockets: Arc<Mutex<HashMap<String, Arc<TcpStream>>>>,
    pub channels: Arc<RwLock<HashMap<String, Channel>>>,
    pub database_sender: std::sync::mpsc::Sender<(PersistenceType, String)>,
}
