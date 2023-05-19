use std::{collections::HashMap, net::TcpStream, sync::Arc};

/// Struct that holds the information of the server
/// # Fields
/// * `name`: The name of the server.
/// * `ip`: The ip of the server.
/// * `port`: The port of the server.
/// * `operators`: The operators of the server.
/// * `father`: The father of the server.
/// * `children`: The children of the server.
#[derive(Debug, Clone)]
pub struct Server {
    pub name: String,
    pub ip: String,
    pub port: String,
    pub operators: Vec<String>,
    pub father: Option<(String, Arc<TcpStream>)>,
    pub children: HashMap<String, Arc<TcpStream>>,
}
impl Server {
    /// Creates the new server.
    /// # Arguments
    /// * `name`: The name of the server.
    /// * `ip`: The ip of the server.
    /// * `port`: The port of the server.
    pub fn new_main_server(ip: String, port: String, name: String) -> Server {
        Server {
            name,
            ip,
            port,
            operators: Vec::new(),
            father: None,
            children: HashMap::new(),
        }
    }

    /// Creates a new child server.
    /// # Arguments
    /// * `name`: The name of the server.
    /// * `ip`: The ip of the server.
    /// * `port`: The port of the server.
    /// * `father`: The father of the server.
    pub fn new_child_server(
        ip: String,
        port: String,
        name: String,
        father: Option<(String, Arc<TcpStream>)>,
    ) -> Server {
        Server {
            name,
            ip,
            port,
            operators: vec![],
            father,
            children: HashMap::new(),
        }
    }
}
