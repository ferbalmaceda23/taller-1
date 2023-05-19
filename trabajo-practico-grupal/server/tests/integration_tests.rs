#[cfg(test)]
mod integration_test {
    use model::network::Network;
    use model::persistence::PersistenceType;
    use model::session::Session;
    use model::{message::Message, server::Server};
    use server::database::handle_database;
    use server::server_errors::ServerError;
    use server::{client_handler::register_client, message_handler::handle_client_message};
    use std::collections::HashMap;
    use std::sync::mpsc::Sender;
    use std::sync::{Arc, Mutex};
    use std::{
        net::{TcpListener, TcpStream},
        sync::RwLock,
    };

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

    #[test]
    fn test_user_sends_msg_to_other_user() {
        let listener = TcpListener::bind("127.0.0.1:8200".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8200".to_string(),
            name: "test".to_string(),
            operators: vec![],
            father: None,
            children: HashMap::new(),
        }));

        let network = Network {
            server: arc_server,
            servers: arc_servers,
            clients: arc_network_clients,
        };

        let (db_tx, db_rx) = std::sync::mpsc::channel::<(PersistenceType, String)>();
        handle_database(db_rx);
        let session = create_session_for_test(db_tx);
        //let client = create_client_for_test(&session, "127.0.0.1:8200".to_string(), "nickname".to_string());
        //let client2 = create_client_for_test(&session, "127.0.0.1:8200".to_string(), "nickname2".to_string());

        let client_stream_sender = Arc::new(TcpStream::connect("127.0.0.1:8200").unwrap());
        let client_stream_receiver = Arc::new(TcpStream::connect("127.0.0.1:8200").unwrap());

        let message_nick_sender = Message::serialize("NICK sender".to_string()).unwrap();
        let message_nick_receiver = Message::serialize("NICK receiver".to_string()).unwrap();
        let message_user_sender = Message::serialize("USER sender a a a".to_string()).unwrap();
        let message_user_receiver = Message::serialize("USER receiver a a a".to_string()).unwrap();

        let mut nickname: Option<String> = Option::None;
        let mut user_parameters = Option::None;
        let mut password = Option::None;
        // Register receiver
        assert!(register_client(
            message_user_receiver,
            (&mut nickname, &mut user_parameters),
            &mut password,
            client_stream_receiver.clone(),
            &session,
            &network,
            &"test".to_string(),
        )
        .is_ok());
        assert!(register_client(
            message_nick_receiver,
            (&mut nickname, &mut user_parameters),
            &mut password,
            client_stream_receiver.clone(),
            &session,
            &network,
            &"test".to_string(),
        )
        .is_ok());

        nickname = Option::None;
        user_parameters = Option::None;

        // Register sender
        assert!(register_client(
            message_user_sender,
            (&mut nickname, &mut user_parameters),
            &mut password,
            client_stream_sender.clone(),
            &session,
            &network,
            &"test".to_string(),
        )
        .is_ok());
        assert!(register_client(
            message_nick_sender,
            (&mut nickname, &mut user_parameters),
            &mut password,
            client_stream_sender.clone(),
            &session,
            &network,
            &"test".to_string(),
        )
        .is_ok());

        let lock_clients = session.clients.as_ref().write().unwrap();

        assert!(lock_clients.contains_key("sender"));
        assert!(lock_clients.contains_key("receiver"));

        let sender = lock_clients.get("sender").unwrap().clone();

        drop(lock_clients);

        let message = Message::serialize("PRIVMSG receiver Hello".to_string()).unwrap();
        let result = handle_client_message(
            message,
            sender.nickname,
            &session,
            &network,
            &"test".to_string(),
        );

        assert!(result.is_ok());

        drop(client_stream_receiver);
        drop(client_stream_sender);
        drop(listener);
    }

    /*
    #[test]
    fn test_user_leaves_and_rejoins_channel_but_is_no_longer_operator() {
        let listener = TcpListener::bind("127.0.0.1:8201".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8201".to_string(),
            name: "test".to_string(),
            operators: vec![],
            father: None,
            children: HashMap::new(),
        }));

        let network = Network {
            server: arc_server,
            servers: arc_servers,
            clients: arc_network_clients,
        };

        let (db_tx, db_rx) = std::sync::mpsc::channel::<(PersistenceType, String)>();
        handle_database(db_rx);
        let session = create_session_for_test(db_tx);

        let mut nickname: Option<String> = Option::None;
        let mut user_parameters = Option::None;
        let mut password = Option::None;
        let client1_stream = Arc::new(TcpStream::connect("127.0.0.1:8201").unwrap());
        let client2_stream = Arc::new(TcpStream::connect("127.0.0.1:8201").unwrap());

        let message_user1 = Message::serialize("USER operator a a a".to_string()).unwrap();
        let message_user2 = Message::serialize("USER user a a a".to_string()).unwrap();
        let message_nick1 = Message::serialize("NICK operator".to_string()).unwrap();
        let message_nick2 = Message::serialize("NICK user".to_string()).unwrap();
        //let message_join = Message::serialize("JOIN #test".to_string()).unwrap();


        assert!(register_client(
            message_user1,
            &mut nickname,
            &mut user_parameters,
            &mut password,
            client1_stream.clone(),
            &session,
            &network,
            &"test".to_string(),
        )
        .is_ok());
        assert!(register_client(
            message_nick1,
            &mut nickname,
            &mut user_parameters,
            &mut password,
            client1_stream.clone(),
            &session,
            &network,
            &"test".to_string(),

        )
        .is_ok());

        let mut nickname: Option<String> = Option::None;
        let mut user_parameters = Option::None;
        let mut password = Option::None;

        assert!(register_client(
            message_user2,
            &mut nickname,
            &mut user_parameters,
            &mut password,
            client2_stream.clone(),
            &session,
            &network,
            &"test".to_string(),
        )
        .is_ok());
        assert!(register_client(
            message_nick2,
            &mut nickname,
            &mut user_parameters,
            &mut password,
            client2_stream.clone(),
            &session,
            &network,
            &"test".to_string(),

        )
        .is_ok());

        let lock_clients = session.clients.as_ref().write().unwrap();

        assert!(lock_clients.contains_key("operator"));
        assert!(lock_clients.contains_key("user"));

        let operator = lock_clients.get("operator").unwrap().clone();
        let user = lock_clients.get("user").unwrap().clone();

        drop(lock_clients);

        let message_join = Message::serialize("JOIN #test".to_string()).unwrap();
        let result = handle_client_message(
            message_join,
            operator.nickname.clone(),
            &session,
            &network,
            &"test".to_string(),
        );

        assert!(result.is_ok());

        let message_join = Message::serialize("JOIN #test".to_string()).unwrap();
        let result = handle_client_message(
            message_join,
            user.nickname.clone(),
            &session,
            &network,
            &"test".to_string(),
        );

        assert!(result.is_ok());

        let lock_channels = session.channels.as_ref().read().unwrap();
        assert!(lock_channels.contains_key("#test"));
        assert!(lock_channels
        .get("#test")
        .unwrap()
        .users
        .contains(&operator.nickname));

        assert!(lock_channels
            .get("#test")
        .unwrap()
        .operators
        .contains(&operator.nickname));


        assert!(lock_channels
        .get("#test")
        .unwrap()
        .users
        .contains(&user.nickname));

        let message_operator = Message::serialize("MODE #test +o user".to_string()).unwrap();
        let result = handle_client_message(
            message_operator,
            operator.nickname.clone(),
            &session,
            &network,
            &"test".to_string(),
        );
        assert_eq!(true, false);

        assert!(result.is_ok());
        assert!(lock_channels
            .get("#test")
            .unwrap()
            .operators
            .contains(&user.nickname));

        let message_part = Message::serialize("PART #test".to_string()).unwrap();
        let result = handle_client_message(
            message_part,
            user.nickname.clone(),
            &session,
            &network,
            &"test".to_string(),
        );

        assert!(result.is_ok());
        assert!(!lock_channels
            .get("#test")
            .unwrap()
            .users
            .contains(&user.nickname));

        let message_join = Message::serialize("JOIN #test".to_string()).unwrap();
        let result = handle_client_message(
            message_join,
            user.nickname.clone(),
            &session,
            &network,
            &"test".to_string(),
        );

        assert!(result.is_ok());
        assert!(lock_channels
            .get("#test")
            .unwrap()
            .users
            .contains(&user.nickname));

        assert!(!lock_channels
            .get("#test")
            .unwrap()
            .operators
            .contains(&user.nickname));

        drop(client1_stream);
        drop(client2_stream);
        drop(lock_channels);
        drop(listener);
    }*/

    #[test]
    fn test_user_banned_after_message() {
        let listener = TcpListener::bind("127.0.0.1:8201".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8201".to_string(),
            name: "test".to_string(),
            operators: vec![],
            father: None,
            children: HashMap::new(),
        }));

        let network = Network {
            server: arc_server,
            servers: arc_servers,
            clients: arc_network_clients,
        };

        let (db_tx, db_rx) = std::sync::mpsc::channel::<(PersistenceType, String)>();
        handle_database(db_rx);
        let session = create_session_for_test(db_tx);

        let mut nickname: Option<String> = Option::None;
        let mut user_parameters = Option::None;
        let mut password = Option::None;
        let client1_stream = Arc::new(TcpStream::connect("127.0.0.1:8201").unwrap());
        let client2_stream = Arc::new(TcpStream::connect("127.0.0.1:8201").unwrap());

        let message_user1 = Message::serialize("USER user1 a a a".to_string()).unwrap();
        let message_user2 = Message::serialize("USER user2 a a a".to_string()).unwrap();
        let message_nick1 = Message::serialize("NICK user1".to_string()).unwrap();
        let message_nick2 = Message::serialize("NICK user2".to_string()).unwrap();

        assert!(register_client(
            message_user1,
            (&mut nickname, &mut user_parameters),
            &mut password,
            client1_stream.clone(),
            &session,
            &network,
            &"test".to_string(),
        )
        .is_ok());
        assert!(register_client(
            message_nick1,
            (&mut nickname, &mut user_parameters),
            &mut password,
            client1_stream.clone(),
            &session,
            &network,
            &"test".to_string(),
        )
        .is_ok());

        let mut nickname: Option<String> = Option::None;
        let mut user_parameters = Option::None;
        let mut password = Option::None;

        assert!(register_client(
            message_user2,
            (&mut nickname, &mut user_parameters),
            &mut password,
            client2_stream.clone(),
            &session,
            &network,
            &"test".to_string(),
        )
        .is_ok());
        assert!(register_client(
            message_nick2,
            (&mut nickname, &mut user_parameters),
            &mut password,
            client2_stream.clone(),
            &session,
            &network,
            &"test".to_string(),
        )
        .is_ok());

        let lock_clients = session.clients.as_ref().write().unwrap();
        assert!(lock_clients.contains_key("user1"));
        assert!(lock_clients.contains_key("user2"));

        let user1 = lock_clients.get("user1").unwrap().clone();
        let user2 = lock_clients.get("user2").unwrap().clone();

        drop(lock_clients);

        let message_join = Message::serialize("JOIN #test".to_string()).unwrap();
        let result = handle_client_message(
            message_join,
            user1.nickname.clone(),
            &session,
            &network,
            &"test".to_string(),
        );
        assert!(result.is_ok());

        let message_join = Message::serialize("JOIN #test".to_string()).unwrap();
        let result = handle_client_message(
            message_join,
            user2.nickname.clone(),
            &session,
            &network,
            &"test".to_string(),
        );
        assert!(result.is_ok());

        let message_to_chan = Message::serialize("PRIVMSG #test :ban me".to_string()).unwrap();
        let result = handle_client_message(
            message_to_chan,
            user2.nickname.clone(),
            &session,
            &network,
            &"test".to_string(),
        );
        assert!(result.is_ok());

        let message_ban = Message::serialize("MODE #test +b user2".to_string()).unwrap();
        let result = handle_client_message(
            message_ban,
            user1.nickname.clone(),
            &session,
            &network,
            &"test".to_string(),
        );
        assert!(result.is_ok());

        let message_join = Message::serialize("JOIN #test".to_string()).unwrap();
        let result = handle_client_message(
            message_join,
            user2.nickname.clone(),
            &session,
            &network,
            &"test".to_string(),
        );
        assert_eq!(Err(ServerError::UserIsBanned), result);
        drop(listener);
    }

    #[test]
    fn test_users_join_channel_and_send_messages() {
        let listener = TcpListener::bind("127.0.0.1:8202".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8202".to_string(),
            name: "test".to_string(),
            operators: vec![],
            father: None,
            children: HashMap::new(),
        }));

        let network = Network {
            server: arc_server,
            servers: arc_servers,
            clients: arc_network_clients,
        };

        let (db_tx, db_rx) = std::sync::mpsc::channel::<(PersistenceType, String)>();
        handle_database(db_rx);
        let session = create_session_for_test(db_tx);

        let mut nickname: Option<String> = Option::None;
        let mut user_parameters = Option::None;
        let mut password = Option::None;
        let client_stream_sender = Arc::new(TcpStream::connect("127.0.0.1:8202").unwrap());
        let client_stream_receiver = Arc::new(TcpStream::connect("127.0.0.1:8202").unwrap());

        let message_nick_sender = Message::serialize("NICK sender".to_string()).unwrap();
        let message_nick_receiver = Message::serialize("NICK receiver".to_string()).unwrap();
        let message_user_sender = Message::serialize("USER sender a a a".to_string()).unwrap();
        let message_user_receiver = Message::serialize("USER receiver a a a".to_string()).unwrap();
        let message_join_sender = Message::serialize("JOIN #test".to_string()).unwrap();
        let message_join_receiver = Message::serialize("JOIN #test".to_string()).unwrap();
        let message_privmg_from_sender =
            Message::serialize("PRIVMSG #test Hello".to_string()).unwrap();
        let message_privmg_from_receiver =
            Message::serialize("PRIVMSG #test Hello".to_string()).unwrap();

        assert!(register_client(
            message_user_receiver,
            (&mut nickname, &mut user_parameters),
            &mut password,
            client_stream_receiver.clone(),
            &session,
            &network,
            &"test".to_string(),
        )
        .is_ok());
        assert!(register_client(
            message_nick_receiver,
            (&mut nickname, &mut user_parameters),
            &mut password,
            client_stream_receiver.clone(),
            &session,
            &network,
            &"test".to_string(),
        )
        .is_ok());

        nickname = Option::None;
        user_parameters = Option::None;

        assert!(register_client(
            message_user_sender,
            (&mut nickname, &mut user_parameters),
            &mut password,
            client_stream_sender.clone(),
            &session,
            &network,
            &"test".to_string(),
        )
        .is_ok());
        assert!(register_client(
            message_nick_sender,
            (&mut nickname, &mut user_parameters),
            &mut password,
            client_stream_sender.clone(),
            &session,
            &network,
            &"test".to_string(),
        )
        .is_ok());

        let lock_clients = session.clients.as_ref().write().unwrap();

        assert!(lock_clients.contains_key("receiver"));
        assert!(lock_clients.contains_key("sender"));

        let receiver = lock_clients.get("receiver").unwrap().clone();
        let sender = lock_clients.get("sender").unwrap().clone();

        drop(lock_clients);

        let result_join_sender = handle_client_message(
            message_join_sender,
            sender.nickname.clone(),
            &session,
            &network,
            &"test".to_string(),
        );
        let result_join_receiver = handle_client_message(
            message_join_receiver,
            receiver.nickname.clone(),
            &session,
            &network,
            &"test".to_string(),
        );

        assert!(result_join_sender.is_ok());
        assert!(result_join_receiver.is_ok());

        let lock_channels = session.channels.as_ref().read().unwrap();
        assert!(lock_channels.contains_key("#test"));
        assert!(lock_channels
            .get("#test")
            .unwrap()
            .users
            .contains(&sender.nickname));
        assert!(lock_channels
            .get("#test")
            .unwrap()
            .operators
            .contains(&sender.nickname));
        assert!(lock_channels
            .get("#test")
            .unwrap()
            .users
            .contains(&receiver.nickname));
        assert!(!lock_channels
            .get("#test")
            .unwrap()
            .operators
            .contains(&receiver.nickname));

        drop(lock_channels);

        let result_msg_from_sender = handle_client_message(
            message_privmg_from_sender,
            sender.nickname,
            &session,
            &network,
            &"test".to_string(),
        );

        assert!(result_msg_from_sender.is_ok());

        let result_msg_from_receiver = handle_client_message(
            message_privmg_from_receiver,
            receiver.nickname,
            &session,
            &network,
            &"test".to_string(),
        );

        assert!(result_msg_from_receiver.is_ok());

        drop(client_stream_sender);
        drop(client_stream_receiver);
        drop(listener);
    }
}
