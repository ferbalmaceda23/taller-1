use super::transfer::{receive_file, transfer_file};
use crate::{
    dcc_commands::transfer::{remove_interface_communication, remove_transfer_communication},
    run_interface::check_address,
};
use gtk::glib;
use model::{
    client_errors::ClientError,
    dcc::{DccMessage, DccMessageType},
    responses::{dcc::DccResponse, ongoing_transfer::OngoingTransfer, response::Response},
    socket::{read_socket, write_socket},
};
use std::{
    collections::HashMap,
    net::{Shutdown, TcpListener, TcpStream},
    sync::{
        mpsc::{sync_channel, SyncSender},
        Arc, RwLock,
    },
    thread,
    time::Duration,
};

/// Receives a file from the requested client through the dcc connection
/// It creates a new thread to receive the file and then sends a message to the current client's interface
/// to notify that the file is being received and show the progress
/// It also creates a new thread to communicate with the interface, so the user can cancel the transfer
/// It returns a ClientError if there is an error creating the socket or the thread
pub fn incoming_send_request(
    requested_client: String,
    dcc_msg: DccMessage,
    tx_chats: glib::Sender<Response>,
    arc_interface_communication: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
    arc_ongoing_transfers: Arc<RwLock<HashMap<String, OngoingTransfer>>>,
) -> Result<(), ClientError> {
    // set channel to communicate with the interface
    let (tx_interface, rx_interface) = sync_channel(0);
    let mut arc_interface_communication_lock = match arc_interface_communication.as_ref().write() {
        Ok(lock) => lock,
        Err(_) => {
            println!("[ERROR] Error accesing the hash for communication with interface");
            return Err(ClientError::LockError);
        }
    };
    arc_interface_communication_lock.insert(requested_client.clone(), tx_interface);
    drop(arc_interface_communication_lock);

    let tx_chats_clone_1 = tx_chats.clone();
    let file_name = dcc_msg.parameters[1].to_owned();
    let ip = dcc_msg.parameters[2].to_owned();
    let port = dcc_msg.parameters[3].to_owned();
    let file_size = dcc_msg.parameters[4].parse::<f64>().unwrap_or(0.0);

    let response = Response::DccResponse {
        response: DccResponse::TransferRequest {
            sender: requested_client.clone(),
            file_name: file_name.clone(),
            file_size,
        },
    };
    if tx_chats.send(response).is_err() {
        println!("[ERROR] Error sending DCC transfer request to the interface");
        return Err(ClientError::GuiCommunicationError);
    }

    thread::sleep(Duration::from_millis(500));
    let transfer_socket = match TcpStream::connect(format!("{ip}:{port}")) {
        Ok(socket) => socket,
        Err(e) => {
            println!("[ERROR] Error connecting to {ip}:{port}. {e:?}");
            return Err(ClientError::SocketError);
        }
    };

    let arc_transfer_socket = Arc::new(transfer_socket);
    if let Ok(gui_answer) = rx_interface.recv() {
        if let Ok(dcc_answer) = DccMessage::deserialize(gui_answer) {
            if dcc_answer.command == DccMessageType::Accept {
                if write_socket(
                    arc_transfer_socket.clone(),
                    &DccMessage::serialize(dcc_answer).unwrap_or_else(|_| "".to_string()),
                )
                .is_ok()
                {};
            } else {
                if write_socket(
                    arc_transfer_socket.clone(),
                    &DccMessage::serialize(dcc_answer).unwrap_or_else(|_| "".to_string()),
                )
                .is_ok()
                {};
                if arc_transfer_socket
                    .as_ref()
                    .shutdown(Shutdown::Both)
                    .is_ok()
                {};
                return Ok(());
            }
        }
    }

    remove_interface_communication(arc_interface_communication, requested_client.clone());

    thread::spawn(move || {
        match receive_file(
            (file_name, file_size, 0),
            arc_transfer_socket,
            requested_client,
            tx_chats_clone_1,
            arc_ongoing_transfers,
        ) {
            Ok(_) => {}
            Err(e) => println!("[DEBUG] Transfer failed: {e:?}"),
        };
    });

    Ok(())
}

/// Sends a file to the requested client through the dcc connection
/// It creates a new thread to send the file and then sends a message to the current client's interface
/// to notify that the file is being sent and show the progress
/// It also creates a new thread to communicate with the interface, so the user can cancel the transfer
/// It returns a ClientError if there is an error creating the socket or the thread
pub fn outgoing_send_request(
    requested_client: String,
    arc_socket: Arc<TcpStream>,
    dcc_msg: DccMessage,
    tx_chats: glib::Sender<Response>,
    arc_transfers_communication: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
    arc_ongoing_transfers: Arc<RwLock<HashMap<String, OngoingTransfer>>>,
) -> Result<(), ClientError> {
    let filepath = dcc_msg.parameters[1].to_owned();
    let filename = dcc_msg.parameters[1]
        .split('/')
        .last()
        .unwrap_or("")
        .to_owned();
    let ip = dcc_msg.parameters[2].to_owned();
    let port = dcc_msg.parameters[3].to_owned();
    let file_size = dcc_msg.parameters[4].parse::<f64>().unwrap_or(0.0);

    if check_ongoing_transfer(
        arc_ongoing_transfers.clone(),
        requested_client.clone(),
        filename.clone(),
        tx_chats.clone(),
    ) {
        println!("[ERROR] Ongoing transfer with same file name");
        return Err(ClientError::OngoingTransfer);
    }

    if !check_address(ip.clone(), port.clone()) {
        let response = Response::DccResponse {
            response: DccResponse::SendAddressErrorResponse {
                sender: requested_client,
                file_name: filename,
            },
        };
        if tx_chats.send(response).is_ok() {};
        return Err(ClientError::SocketError);
    }

    let mut dcc_msg_aux = dcc_msg;
    dcc_msg_aux.parameters[1] = filename.clone();
    let dcc_msg_str = DccMessage::serialize(dcc_msg_aux).unwrap_or_else(|_| "".to_string());

    if write_socket(arc_socket.clone(), &dcc_msg_str).is_ok() {}

    let listener = match TcpListener::bind(format!("{ip}:{port}")) {
        Ok(listener) => listener,
        Err(_) => {
            println!("[ERROR] Error creating the listener on {ip}:{port}");
            let response = Response::DccResponse {
                response: DccResponse::ErrorResponse {
                    description: "Invalid address.".to_owned(),
                },
            };
            if tx_chats.send(response).is_ok() {};
            return Err(ClientError::SocketError);
        }
    };

    if let Ok((transfer_socket, _)) = listener.accept() {
        let arc_transfer_socket = Arc::new(transfer_socket);

        if let Ok(answer) = read_socket(arc_transfer_socket.clone()) {
            let dcc_answer = DccMessage::deserialize(answer)?;
            match dcc_answer.command {
                DccMessageType::Accept => {
                    println!("[INFO] Transfer accepted");
                }
                DccMessageType::Close => {
                    println!("[INFO] Transfer declined");
                    let response = Response::DccResponse {
                        response: DccResponse::TransferDeclined {
                            sender: requested_client,
                            file_name: filename,
                        },
                    };
                    if tx_chats.send(response).is_err() {
                        println!("[ERROR] Error sending response to interface");
                    }
                    if arc_transfer_socket.shutdown(Shutdown::Both).is_err() {
                        println!("[ERROR] Error shutting down the socket");
                    }
                    drop(listener);
                    return Ok(());
                }
                _ => {
                    println!(
                        "[INFO] Unexpected command received: {:?}",
                        dcc_answer.command
                    );
                    return Err(ClientError::InvalidCommand);
                }
            }
        }

        let (tx_transfer, rx_transfer) = sync_channel(0);
        let mut transfers_communication_lock = match arc_transfers_communication.as_ref().write() {
            Ok(lock) => lock,
            Err(_) => {
                println!("[ERROR] Error getting write lock on transfers_communication");
                return Err(ClientError::LockError);
            }
        };

        transfers_communication_lock.insert(filename.clone(), tx_transfer);
        drop(transfers_communication_lock);

        let filename_clone = filename.clone();
        thread::spawn(move || {
            match transfer_file(
                (filename_clone, filepath, file_size, 0),
                requested_client,
                arc_transfer_socket,
                rx_transfer,
                tx_chats,
                arc_socket.clone(),
                arc_ongoing_transfers,
            ) {
                Ok(_) => {}
                Err(e) => {
                    println!("[ERROR] Transfer failed: {e:?}");
                }
            }
            remove_transfer_communication(arc_transfers_communication, filename);
        });
    }

    Ok(())
}

fn check_ongoing_transfer(
    arc_ongoing_transfers: Arc<RwLock<HashMap<String, OngoingTransfer>>>,
    requested_client: String,
    filename: String,
    tx_chats: glib::Sender<Response>,
) -> bool {
    let ongoing_transfers_lock = match arc_ongoing_transfers.as_ref().read() {
        Ok(lock) => lock,
        Err(e) => {
            println!("[DEBUG] Error accesing the ongoing transfers hash: {e:?}");
            return false;
        }
    };
    if ongoing_transfers_lock.get(&filename).is_some() {
        let response = Response::DccResponse {
            response: DccResponse::OngoingTransfer {
                sender: requested_client,
                file_name: filename,
            },
        };
        if tx_chats.send(response).is_ok() {};
        return true;
    }
    drop(ongoing_transfers_lock);
    false
}
