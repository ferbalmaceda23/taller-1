use std::collections::HashMap;
use std::io::stdin;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::net::TcpStream;
use std::sync::mpsc::SyncSender;
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;

use model::client_errors::ClientError;
use model::dcc::DccMessage;

const MAX_MSG_SIZE: usize = 510;

/// This function is the main loop of the client to run in the terminal.
/// It creates a thread to listen to the server and the other one is used to to listen to the user input.
/// Returns a ClientError in case of error
pub fn client_run(address: &str) -> Result<(), ClientError> {
    let socket = TcpStream::connect(address)?;

    let arc_socket = Arc::new(socket);
    let arc_socket_clone = arc_socket.clone();

    let dcc_connections = HashMap::<String, SyncSender<String>>::new();
    let arc_dcc_connections = Arc::new(RwLock::new(dcc_connections));
    let arc_dcc_connections_clone = arc_dcc_connections.clone();

    let dcc_ongoing_transfers = HashMap::<String, u64>::new();
    let arc_dcc_ongoing_transfers = Arc::new(RwLock::new(dcc_ongoing_transfers));
    let arc_dcc_ongoing_transfers_clone = arc_dcc_ongoing_transfers.clone();

    thread::spawn(move || {
        while read_server_response(
            arc_socket_clone.clone(),
            arc_dcc_connections.clone(),
            arc_dcc_ongoing_transfers.clone(),
        )
        .is_ok()
        {}
    });

    send_client_request(
        arc_socket,
        arc_dcc_connections_clone,
        arc_dcc_ongoing_transfers_clone,
    )?;
    Ok(())
}

/// This function listens from the stdin and sends the request to the server
/// Returns a ClientError if it can't read from stdin or write to the server
/// Returns an Ok if it the client sends a QUIT command
fn send_client_request(
    arc_socket: Arc<TcpStream>,
    _dcc_connections: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
    _dcc_ongoing_transfers: Arc<RwLock<HashMap<String, u64>>>,
) -> Result<(), ClientError> {
    let stdin = stdin();
    let reader = BufReader::new(stdin);
    for line in reader.lines().flatten() {
        let mut buff = line.to_owned().into_bytes();
        buff.resize(MAX_MSG_SIZE, 0);
        arc_socket.as_ref().write_all(&buff)?;
        if line.starts_with("QUIT ") {
            break;
        } else if let Ok(_dcc_msg) = DccMessage::deserialize(line) {
            //manage_dcc_request_from_current_client(line, dcc_connections.clone(), dcc_ongoing_transfers.clone())?;
        }
    }
    Ok(())
}

/// This function reads the response from the server and prints it to the stdout
/// Returns a ClientError if it can't read from the server
/// Returns an Ok(()) if the server closes the connection
fn read_server_response(
    arc_socket: Arc<TcpStream>,
    _dcc_connections: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
    _dcc_ongoing_transfers: Arc<RwLock<HashMap<String, u64>>>,
) -> Result<(), ClientError> {
    let mut buffer = [0u8; MAX_MSG_SIZE];
    match arc_socket.as_ref().read_exact(&mut buffer) {
        Ok(_) => {
            let buffer = buffer
                .into_iter()
                .take_while(|&x| x != 0)
                .collect::<Vec<_>>();
            let line = String::from_utf8_lossy(&buffer).to_string();

            if let Ok(_dcc_msg) = DccMessage::deserialize(line.clone()) {
                //manage_dcc_request_from_client(line, dcc_connections, dcc_ongoing_transfers)?;
            } else {
                println!("[DEBUG] {line}");
            }
        }
        Err(_) => {
            println!("[ERROR] Connection with server finished");
            return Err(ClientError::ConnectionFinished);
        }
    }

    Ok(())
}
