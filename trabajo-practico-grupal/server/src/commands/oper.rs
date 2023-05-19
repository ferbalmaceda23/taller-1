use std::io::{BufRead, BufReader};

use crate::{server_errors::ServerError, socket::inform_client};
use model::{
    message::Message,
    network::Network,
    responses::{errors::ErrorResponse, replies::CommandResponse},
    session::Session,
};

/// Handles the `OPER` command.
/// It sets an operator flag for a client.
/// # Arguments
/// * `session` - The session of the client.
/// * `network` - The network the client is connected to.
/// * `message` - The message sent by the client.
/// * `nickname` - The nickname of the client.
pub fn handle_oper_command(
    message: Message,
    nickname: String,
    session: &Session,
    network: &Network,
) -> Result<(), ServerError> {
    if message.parameters.len() != 2 {
        let response = ErrorResponse::NeedMoreParams {
            command: "OPER".to_string(),
        }
        .to_string();
        inform_client(session, &nickname, &response)?;
        return Err(ServerError::InvalidParameters);
    }

    let pass = message.parameters[1].to_owned();
    let nick = message.parameters[0].to_owned();

    match std::fs::File::open("server/src/server_opers.txt") {
        Ok(file) => {
            let mut found: bool = false;
            let buff = BufReader::new(file);
            for line in buff.lines() {
                let line = match line {
                    Ok(line) => line,
                    Err(_) => return Err(ServerError::CannotReadFromFile),
                };
                let credentials = line.split(';').into_iter().collect::<Vec<&str>>();
                if credentials[0] == nick && credentials[1] == pass {
                    let mut server_lock = match network.server.as_ref().write() {
                        Ok(server_lock) => server_lock,
                        Err(_) => return Err(ServerError::LockError),
                    };
                    if !server_lock.operators.contains(&nickname) {
                        server_lock.operators.push(nickname.clone());
                    }
                    let response = CommandResponse::YouAreOperator.to_string();
                    inform_client(session, &nickname, response.as_str())?;
                    println!("Operator added: {:?}", server_lock);
                    drop(server_lock);
                    found = true;
                    break;
                }
            }
            if !found {
                let response = ErrorResponse::PasswordMismatch.to_string();
                inform_client(session, &nickname, response.as_str())?;
                return Err(ServerError::InvalidCredentials);
            }
        }
        Err(_) => {
            println!("Error opening file");
            return Err(ServerError::CannotReadFromFile);
        }
    }

    Ok(())
}

#[cfg(test)]
mod oper_tests {
    use std::collections::HashMap;
    use std::io::Read;
    use std::net::TcpListener;
    use std::sync::{Arc, RwLock};
    use std::vec;

    use crate::commands::command_utils::{
        create_client_for_test, create_message_for_test, create_session_for_test,
    };
    use crate::commands::oper::handle_oper_command;
    use crate::database::handle_database;
    use crate::server_errors::ServerError;
    use model::message::MessageType;
    use model::network::Network;
    use model::persistence::PersistenceType;
    use model::responses::errors::ErrorResponse;
    use model::responses::response::Response;
    use model::server::Server;

    #[test]
    pub fn test_command_oper_invalid_parameters() {
        let listener = TcpListener::bind("127.0.0.1:8118".to_string()).unwrap();
        let arc_network_clients = Arc::new(RwLock::new(HashMap::new()));
        let arc_servers = Arc::new(RwLock::new(HashMap::<String, u8>::new()));
        let arc_server = Arc::new(RwLock::new(Server {
            ip: "0.0.0.0".to_string(),
            port: "8118".to_string(),
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
            "127.0.0.1:8118".to_string(),
            "nickname".to_string(),
        );
        let message = create_message_for_test(MessageType::Oper, vec![]);

        let result = handle_oper_command(message, client.nickname.to_string(), &session, &network);
        let (mut reader, _addr) = listener.accept().unwrap();
        drop(listener);
        let mut buf = vec![0u8; 510];
        reader.read(&mut buf).unwrap();
        let msg = buf.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
        let response = String::from_utf8(msg).unwrap();

        let response = Response::serialize(response).unwrap();
        match response {
            Response::ErrorResponse {
                response: err_response,
            } => match err_response {
                ErrorResponse::NeedMoreParams { command } => {
                    assert_eq!(command, "OPER".to_string());
                }
                _ => {
                    assert!(false);
                }
            },
            _ => {
                assert!(false);
            }
        }

        assert_eq!(Err(ServerError::InvalidParameters), result);
    }
}
