use crate::server_errors::ServerError;
use model::{client::Client, message::Message, network::Network, session::Session};

/// Functions that handles the NICK command sent by a client.
/// If nickname already in use, it checks the password if exists.
/// # Arguments
/// * `message` - The message sent by the client.
/// * `user_parameters` - The username, realname, servername y hostname.
/// * `nickname` - The nickname of the client.
/// * `password` - The password of the client.
/// * `session` - The session of current server.
/// * `network` - The struct that contains information about the network.
pub fn handle_nick_command(
    message: Message,
    nickname: &mut Option<String>,
    user_parameters: &mut Option<Vec<String>>,
    password: &mut Option<String>,
    session: &Session,
    network: &Network,
) -> Result<Option<Client>, ServerError> {
    if message.parameters.len() != 1 {
        return Err(ServerError::InvalidParameters);
    }
    println!("NICK {}", message.parameters[0]);
    *nickname = Option::Some(message.parameters[0].to_owned());
    let nick = message.parameters[0].to_owned();

    match session.clients.read() {
        Ok(clients) => {
            if let Some(c) = clients.get(&nick) {
                if user_parameters.is_some() {
                    *user_parameters = None;
                    *nickname = None;
                    *password = None;
                    return Err(ServerError::NicknameInUse(nick.clone()));
                }
                if let Some(pass) = c.password.to_owned() {
                    if let Some(p) = password.to_owned() {
                        if pass == p {
                            let vec = vec![
                                c.username.to_owned(),
                                c.hostname.to_owned(),
                                c.servername.to_owned(),
                                c.realname.to_owned(),
                            ];
                            if c.connected {
                                return Err(ServerError::NicknameInUse(nick.clone()));
                            }
                            *user_parameters = Option::Some(vec);
                            return Ok(Some(c.to_owned()));
                        } else {
                            *nickname = None;
                            *password = None;
                            return Err(ServerError::InvalidPassword);
                        }
                    } else {
                        *nickname = None;
                        *password = None;
                        return Err(ServerError::InvalidPassword);
                    }
                } else {
                    let vec = vec![
                        c.username.to_owned(),
                        c.hostname.to_owned(),
                        c.servername.to_owned(),
                        c.realname.to_owned(),
                    ];
                    *user_parameters = Option::Some(vec);
                    return Ok(Option::Some(c.to_owned()));
                }
            } else {
                let network_clients = network.clients.read()?;
                if network_clients.get(&nick).is_some() {
                    *user_parameters = None;
                    *nickname = None;
                    *password = None;
                    return Err(ServerError::NicknameInUse(nick.clone()));
                }
                drop(network_clients);
            }
            drop(clients);
        }
        Err(_) => {
            println!("Error reading clients");
            return Err(ServerError::LockError);
        }
    };

    match user_parameters {
        Some(user_params) => {
            let client = Client::from_connection(
                nick,
                user_params[0].to_string(),
                user_params[1].to_string(),
                user_params[2].to_string(),
                user_params[3].to_string(),
                password.to_owned(),
                true,
            );
            Ok(Option::Some(client))
        }
        None => Err(ServerError::ErroneusNickname),
    }
}

#[cfg(test)]
mod nick_tests {

    use std::{
        collections::HashMap,
        net::TcpListener,
        sync::{Arc, RwLock},
    };

    use model::{
        message::MessageType, network::Network, persistence::PersistenceType, server::Server,
    };

    use crate::{
        commands::{
            command_utils::{
                create_client_for_test, create_message_for_test, create_session_for_test,
                read_lock_clients,
            },
            nick::handle_nick_command,
        },
        database::handle_database,
    };

    #[test]
    fn test_command_nick_with_username() {
        let listener = TcpListener::bind("127.0.0.1:8144".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8144".to_string(),
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
        let mut nickname = Option::None;
        let mut user_parameters = Option::Some(vec![
            "username".to_string(),
            "hostname".to_string(),
            "servername".to_string(),
            "realname".to_string(),
        ]);
        let mut password = Option::None;

        let msg = create_message_for_test(MessageType::Nick, vec!["nickname".to_string()]);
        let client = handle_nick_command(
            msg,
            &mut nickname,
            &mut user_parameters,
            &mut password,
            &session,
            &network,
        );

        assert!(client.is_ok());
        let client = client.unwrap().unwrap();
        assert_eq!(client.username, "username".to_string());
        assert_eq!(client.hostname, "hostname".to_string());
        assert_eq!(client.servername, "servername".to_string());
        assert_eq!(client.realname, "realname".to_string());
        assert_eq!(client.nickname, "nickname".to_string());
        drop(listener);
    }

    #[test]
    fn test_nick_command_with_existing_nickname() {
        let listener = TcpListener::bind("127.0.0.1:8145".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8145".to_string(),
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
        let client = create_client_for_test(
            &session,
            "127.0.0.1:8145".to_string(),
            "nickname".to_string(),
        );

        let msg = create_message_for_test(MessageType::Nick, vec!["nickname".to_string()]);
        let mut nickname = Option::None;
        let mut user_parameters = Option::Some(vec![
            "username".to_string(),
            "hostname".to_string(),
            "servername".to_string(),
            "realname".to_string(),
        ]);
        let mut password = Option::None;
        let result = handle_nick_command(
            msg,
            &mut nickname,
            &mut user_parameters,
            &mut password,
            &session,
            &network,
        );
        drop(listener);
        let lock_clients = read_lock_clients(&session).unwrap();
        assert!(lock_clients.contains_key(&client.nickname));
        assert!(result.is_err());
    }

    #[test]
    fn test_command_nick_invalid_parameters() {
        let listener = TcpListener::bind("127.0.0.1:8146".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8146".to_string(),
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
        let msg = create_message_for_test(
            MessageType::Nick,
            vec!["nickname".to_string(), "nickname2".to_string()],
        );
        let mut nickname = Option::None;
        let mut user_parameters = Option::Some(vec![
            "username".to_string(),
            "hostname".to_string(),
            "servername".to_string(),
            "realname".to_string(),
        ]);
        let mut password = Option::None;

        let result = handle_nick_command(
            msg,
            &mut nickname,
            &mut user_parameters,
            &mut password,
            &session,
            &network,
        );

        drop(listener);
        assert!(result.is_err());
    }
}
