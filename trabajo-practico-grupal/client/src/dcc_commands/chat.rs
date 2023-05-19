use gtk::glib;
use model::{
    dcc::DccMessage,
    responses::{dcc::DccResponse, response::Response},
};

/// Manages an incoming dcc chat message from another client, sending it to the interface
pub fn incoming_chat_request(
    requested_client: String,
    dcc_msg: DccMessage,
    tx_chats: glib::Sender<Response>,
) {
    let response = Response::DccResponse {
        response: DccResponse::ChatMessage {
            sender: requested_client,
            message: dcc_msg.parameters[1..].join(" "),
        },
    };
    if tx_chats.send(response).is_ok() {};
}

#[cfg(test)]
mod dcc_chat_test {
    use std::{
        collections::HashMap,
        net::{TcpListener, TcpStream},
        sync::{mpsc::Sender, Arc, Mutex, RwLock},
    };

    //use client::dcc_commands::{chat::incoming_chat_request, transfer::{receive_file, transfer_file, remove_transfer_communication}, close::{incoming_close_request, outgoing_close_request}};
    use gtk::glib;
    use model::{
        dcc::{DccMessage, DccMessageType},
        message::Message,
        network::Network,
        persistence::PersistenceType,
        responses::{dcc::DccResponse, response::Response},
        server::Server,
        session::Session,
    };
    use server::{client_handler::register_client, database::handle_database};

    use super::incoming_chat_request;

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

    // tests
    #[test]
    fn test_dcc_chat() {
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

        let (tx_chats, rx_chats) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        //let dcc_connections: Arc<RwLock<HashMap<String, SyncSender<String>>>> = Arc::new(RwLock::new(HashMap::new()));
        //let arc_dcc_interface_communication: Arc<RwLock<HashMap<String, SyncSender<String>>>> = Arc::new(RwLock::new(HashMap::new()));

        let dcc_msg = DccMessage {
            prefix: None,
            command: DccMessageType::Chat,
            parameters: ["receiver".to_string(), "hello".to_string()].to_vec(),
        };

        let msg = DccMessage::serialize(dcc_msg.clone()).unwrap();
        let first_dcc_msg = DccMessage::deserialize(msg).unwrap();
        incoming_chat_request("sender".to_string(), first_dcc_msg, tx_chats);

        rx_chats.attach(None, move |action| {
            match action {
                Response::DccResponse { response } => match response {
                    DccResponse::ChatMessage { sender, message } => {
                        assert_eq!(sender, "sender".to_string());
                        assert_eq!(message, "hello".to_string());
                    }
                    _ => assert!(false),
                },
                _ => {}
            }
            glib::Continue(false)
        });
    }

    #[test]
    fn test_dcc_chat_multiple_messages() {
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

        let (tx_chats, rx_chats) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        //let dcc_connections: Arc<RwLock<HashMap<String, SyncSender<String>>>> = Arc::new(RwLock::new(HashMap::new()));
        //let arc_dcc_interface_communication: Arc<RwLock<HashMap<String, SyncSender<String>>>> = Arc::new(RwLock::new(HashMap::new()));

        let dcc_msg1 = DccMessage {
            prefix: None,
            command: DccMessageType::Chat,
            parameters: ["receiver".to_string(), "hello".to_string()].to_vec(),
        };

        let msg = DccMessage::serialize(dcc_msg1.clone()).unwrap();
        let first_dcc_msg = DccMessage::deserialize(msg).unwrap();
        incoming_chat_request("sender".to_string(), first_dcc_msg, tx_chats);

        rx_chats.attach(None, move |action| {
            match action {
                Response::DccResponse { response } => match response {
                    DccResponse::ChatMessage { sender, message } => {
                        assert_eq!(sender, "sender".to_string());
                        assert_eq!(message, "hello".to_string());
                    }
                    _ => assert!(false),
                },
                _ => {}
            }
            glib::Continue(false)
        });

        let (tx_chats, rx_chats) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        // Second message
        let dcc_msg2 = DccMessage {
            prefix: None,
            command: DccMessageType::Chat,
            parameters: ["sender".to_string(), "hey, how are you?".to_string()].to_vec(),
        };

        let msg = DccMessage::serialize(dcc_msg2.clone()).unwrap();
        let second_dcc_msg = DccMessage::deserialize(msg).unwrap();
        incoming_chat_request("receiver".to_string(), second_dcc_msg, tx_chats);

        rx_chats.attach(None, move |action| {
            match action {
                Response::DccResponse { response } => match response {
                    DccResponse::ChatMessage { sender, message } => {
                        assert_eq!(sender, "receiver".to_string());
                        assert_eq!(message, "hey, how are you?".to_string());
                    }
                    _ => assert!(false),
                },
                _ => {}
            }
            glib::Continue(false)
        });
    }
}
