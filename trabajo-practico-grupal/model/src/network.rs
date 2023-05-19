use crate::server::Server;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

/// Struct that holds the information about the servers in the network,
/// clients in the network and the current server runnning.
/// /// # Fields
/// * `server`: The current server running.
/// * `servers`: The servers in the network.
/// * `clients`: The clients in the network.
#[derive(Debug, Clone)]
pub struct Network {
    pub server: Arc<RwLock<Server>>,
    pub servers: Arc<RwLock<HashMap<String, u8>>>,
    pub clients: Arc<RwLock<HashMap<String, u8>>>,
}
