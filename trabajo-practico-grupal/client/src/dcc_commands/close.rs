use std::{
    collections::HashMap,
    net::Shutdown,
    sync::{mpsc::SyncSender, Arc, RwLock},
};

use gtk::glib;
use model::{
    responses::{dcc::DccResponse, response::Response},
    socket::write_socket,
};

/// Closes a dcc connection, removing it from the current connections hash
pub fn close_request(
    arc_socket: Arc<std::net::TcpStream>,
    dcc_connections: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
    requested_client: String,
) {
    println!("[INFO] Shutding down socket...");
    if arc_socket.as_ref().shutdown(Shutdown::Both).is_ok() {};
    println!("[INFO] Removing connection...");
    remove_connection(dcc_connections, requested_client);
}
/// Manages an incoming dcc close request, which closes the connection with the client who requested it
/// and sends a message to the current client's interface
pub fn incoming_close_request(
    arc_socket: Arc<std::net::TcpStream>,
    dcc_connections: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
    requested_client: String,
    tx_chats: glib::Sender<Response>,
) {
    println!("[INFO] request to close connection received from {requested_client}");
    let response = Response::DccResponse {
        response: DccResponse::CloseConnection {
            sender: requested_client.clone(),
        },
    };
    if tx_chats.send(response).is_ok() {};
    close_request(arc_socket, dcc_connections, requested_client);
}

/// Manages an outgoing dcc close request, which closes the connection with the requested client
/// It sends the close message to the requested client and then closes the connection
pub fn outgoing_close_request(
    arc_socket: Arc<std::net::TcpStream>,
    dcc_connections: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
    requested_client: String,
    message_for_client: String,
) {
    println!("[DEBUG] request from current client to close connection with {requested_client}");
    if write_socket(arc_socket.clone(), &message_for_client).is_ok() {};
    close_request(arc_socket, dcc_connections, requested_client);
}

/// Removes the dcc connection with the client from the current connections hash
pub fn remove_connection(
    dcc_connections: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
    client: String,
) {
    let mut dcc_connections_lock = match dcc_connections.write() {
        Ok(lock) => lock,
        Err(e) => {
            println!("[ERROR] Error removing connection: {e}");
            return;
        }
    };
    dcc_connections_lock.remove(&client);
    drop(dcc_connections_lock);
}

/// Removes every dcc connection from the current connections hash
pub fn close_all_dcc_connections(
    dcc_connections: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
) {
    let mut dcc_hash_lock = match dcc_connections.as_ref().write() {
        Ok(hash) => hash,
        Err(_) => {
            println!("[ERROR] Can't close DCC connections");
            return;
        }
    };
    for (client, dcc_connection) in dcc_hash_lock.iter() {
        if dcc_connection.send(format!("DCC CLOSE {client}")).is_ok() {};
    }

    dcc_hash_lock.clear();
    drop(dcc_hash_lock);
}

#[cfg(test)]
mod dcc_close_test {
    use std::{
        collections::HashMap,
        net::{TcpListener, TcpStream},
        sync::{
            mpsc::{sync_channel, Receiver, Sender, SyncSender},
            Arc, Mutex, RwLock,
        },
        thread,
    };

    //use client::dcc_commands::{chat::incoming_chat_request, transfer::{receive_file, transfer_file, remove_transfer_communication}, close::{incoming_close_request, outgoing_close_request}};
    use gtk::glib;
    use model::{
        dcc::{DccMessage, DccMessageType},
        message::Message,
        network::Network,
        persistence::PersistenceType,
        server::Server,
        session::Session,
    };
    use server::{client_handler::register_client, database::handle_database};

    use crate::dcc_commands::close::{incoming_close_request, outgoing_close_request};

    use super::close_all_dcc_connections;

    fn create_session_for_test(tx: Sender<(PersistenceType, String)>) -> Session {
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

    fn register_client_for_test(
        nick: String,
        stream: Arc<TcpStream>,
        session: &Session,
        network: &Network,
    ) {
        let message_user = Message::serialize(format!("USER {} a a a", nick).to_string()).unwrap();
        let message_nick = Message::serialize(format!("NICK {}", nick).to_string()).unwrap();
        let mut nickname: Option<String> = Option::None;
        let mut user_parameters = Option::None;
        let mut password = Option::None;
        // Register receiver
        register_client(
            message_user,
            (&mut nickname, &mut user_parameters),
            &mut password,
            stream.clone(),
            &session,
            &network,
            &"test".to_string(),
        )
        .unwrap();

        register_client(
            message_nick,
            (&mut nickname, &mut user_parameters),
            &mut password,
            stream.clone(),
            &session,
            &network,
            &"test".to_string(),
        )
        .unwrap();
    }

    #[test]
    fn test_dcc_close() {
        let listener = TcpListener::bind("127.0.0.1:0".to_string()).unwrap();
        let port = listener.local_addr().unwrap().port();
        let address_port = format!("127.0.0.1:{}", port.to_string());
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "127.0.0.1".to_string(),
            port: port.to_string(),
            name: "test".to_string(),
            operators: vec![],
            father: None,
            children: HashMap::new(),
        }));
        let (db_tx, db_rx) = std::sync::mpsc::channel::<(PersistenceType, String)>();
        handle_database(db_rx);
        let session = create_session_for_test(db_tx);

        let network = Network {
            server: arc_server,
            servers: arc_servers,
            clients: arc_network_clients,
        };

        let client_stream_sender = Arc::new(TcpStream::connect(address_port.clone()).unwrap());
        let client_stream_receiver = Arc::new(TcpStream::connect(address_port.clone()).unwrap());
        register_client_for_test(
            "sender".to_string(),
            client_stream_sender.clone(),
            &session,
            &network,
        );
        register_client_for_test(
            "receiver".to_string(),
            client_stream_receiver.clone(),
            &session,
            &network,
        );

        let (tx_chats, _rx_chats) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        // Create channel for tx and rx
        let (tx1, _rx1) = sync_channel(0);
        let (tx2, _rx2) = sync_channel(0);
        let dcc_connections_1 = Arc::new(RwLock::new(HashMap::new()));
        dcc_connections_1
            .write()
            .unwrap()
            .insert("receiver".to_string(), tx2);
        let dcc_connections_2 = Arc::new(RwLock::new(HashMap::new()));
        dcc_connections_2
            .write()
            .unwrap()
            .insert("sender".to_string(), tx1);

        // Create close message from sender to receiver
        let dcc_msg_close = DccMessage {
            prefix: None,
            command: DccMessageType::Close,
            parameters: vec!["receiver".to_string()],
        };

        outgoing_close_request(
            client_stream_receiver.clone(),
            dcc_connections_1.clone(),
            "receiver".to_string(),
            DccMessage::serialize(dcc_msg_close).unwrap(),
        );

        incoming_close_request(
            client_stream_sender.clone(),
            dcc_connections_2.clone(),
            "sender".to_string(),
            tx_chats.clone(),
        );

        assert!(dcc_connections_1.read().unwrap().is_empty());
        assert!(dcc_connections_2.read().unwrap().is_empty());
    }

    #[test]
    fn test_dcc_close_all() {
        let listener = TcpListener::bind("127.0.0.1:0".to_string()).unwrap();
        let port = listener.local_addr().unwrap().port();
        let address_port = format!("127.0.0.1:{}", port.to_string());
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "127.0.0.1".to_string(),
            port: port.to_string(),
            name: "test".to_string(),
            operators: vec![],
            father: None,
            children: HashMap::new(),
        }));
        let (db_tx, db_rx) = std::sync::mpsc::channel::<(PersistenceType, String)>();
        handle_database(db_rx);
        let session = create_session_for_test(db_tx);

        let network = Network {
            server: arc_server,
            servers: arc_servers,
            clients: arc_network_clients,
        };

        let client_stream_sender = Arc::new(TcpStream::connect(address_port.clone()).unwrap());
        let client_stream_receiver1 = Arc::new(TcpStream::connect(address_port.clone()).unwrap());
        let client_stream_receiver2 = Arc::new(TcpStream::connect(address_port.clone()).unwrap());
        let client_stream_receiver3 = Arc::new(TcpStream::connect(address_port.clone()).unwrap());
        register_client_for_test(
            "sender".to_string(),
            client_stream_sender.clone(),
            &session,
            &network,
        );
        register_client_for_test(
            "receiver1".to_string(),
            client_stream_receiver1.clone(),
            &session,
            &network,
        );
        register_client_for_test(
            "receiver2".to_string(),
            client_stream_receiver2.clone(),
            &session,
            &network,
        );
        register_client_for_test(
            "receiver3".to_string(),
            client_stream_receiver3.clone(),
            &session,
            &network,
        );

        // Fill dcc connections hash with sender's dcc connections
        let (tx1, rx1): (SyncSender<String>, Receiver<String>) = sync_channel(0);
        let (tx11, _rx11): (SyncSender<String>, Receiver<String>) = sync_channel(0);

        let dcc_connections_sender = Arc::new(RwLock::new(HashMap::new()));
        dcc_connections_sender
            .write()
            .unwrap()
            .insert("receiver1".to_string(), tx1);

        let dcc_connections_receiver1 = Arc::new(RwLock::new(HashMap::new()));
        dcc_connections_receiver1
            .write()
            .unwrap()
            .insert("sender".to_string(), tx11);

        // thread to close dcc connections
        let dcc_connections_sender_clone = dcc_connections_sender.clone();
        thread::spawn(move || {
            close_all_dcc_connections(dcc_connections_sender.clone());
        });

        assert!(rx1.recv().unwrap().contains("CLOSE"));
        assert!(dcc_connections_sender_clone.read().unwrap().is_empty());
    }
}
