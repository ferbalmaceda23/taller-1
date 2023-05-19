use std::{
    collections::HashMap,
    net::{Shutdown, TcpListener, TcpStream},
    sync::{
        mpsc::{sync_channel, Receiver, SyncSender},
        Arc, RwLock,
    },
    thread,
    time::Duration,
};

use crate::dcc_commands::close::remove_connection;
use gtk::glib;
use model::{
    client_errors::ClientError,
    dcc::{DccMessage, DccMessageType},
    responses::{
        dcc::DccResponse, errors::ErrorResponse, ongoing_transfer::OngoingTransfer,
        response::Response,
    },
    socket::{read_socket, write_socket},
};

use crate::dcc_commands::{
    chat::incoming_chat_request,
    close::{incoming_close_request, outgoing_close_request},
    resume::{incoming_resume_request, outgoing_resume_request},
    send::{incoming_send_request, outgoing_send_request},
    stop::{incoming_stop_request, outgoing_stop_request},
};

/// This function manages the DCC request from the current client
/// Sends the message to the thread that manages the connection
/// If a dcc connection doesn't exist, it creates a new one
/// If a dcc connection is already active, it sends the message to the active thread
/// Returns a ClientError in case of error
pub fn manage_dcc_request_from_current_client(
    dcc_msg: DccMessage,
    dcc_connections: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
    tx_chats: gtk::glib::Sender<Response>, // comunication with interface
    arc_dcc_interface_communication: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
) -> Result<(), ClientError> {
    let mut dcc_hash_lock = dcc_connections.as_ref().write()?;
    let requested_client = dcc_msg.parameters[0].to_owned();

    match dcc_msg.command {
        DccMessageType::Chat => {
            if let Some(dcc_connection) = dcc_hash_lock.get(&requested_client) {
                // send information to the active thread that manages the connection
                if dcc_connection
                    .send(DccMessage::serialize(dcc_msg)?)
                    .is_err()
                {
                    println!(
                        "[ERROR] Communication with dcc thread failed. Client: {requested_client}"
                    );
                };
                drop(dcc_hash_lock);
            } else {
                //create new p2p connection waiting for the other client to connect
                let (dcc_sender, dcc_receiver) = sync_channel::<String>(0);
                dcc_hash_lock.insert(requested_client.clone(), dcc_sender);
                drop(dcc_hash_lock);
                create_new_dcc_connection(
                    dcc_receiver,
                    dcc_msg,
                    requested_client,
                    dcc_connections,
                    tx_chats,
                    arc_dcc_interface_communication,
                )?;
            }
        }
        _ => {
            if let Some(dcc_connection) = dcc_hash_lock.get(&requested_client) {
                println!("El requested client es {requested_client}");
                // send information to the active thread that manages the connection
                if dcc_connection
                    .send(DccMessage::serialize(dcc_msg)?)
                    .is_err()
                {
                    println!(
                        "[ERROR] Communication with dcc thread failed. Client: {requested_client}"
                    );
                };
                drop(dcc_hash_lock);
            } else {
                println!("[ERROR] No active p2p connection with {}", requested_client);
                drop(dcc_hash_lock);
            }
        }
    }

    Ok(())
}

/// This function manages the DCC request from other clients
/// Sends the message to the thread that manages the connection
/// If a dcc connection doesn't exist, it connects to the requested connection from the other client
/// If a dcc connection is already active, it sends the message to the active thread
/// Returns a ClientError in case of error
pub fn manage_dcc_request_from_client(
    dcc_msg: DccMessage,
    dcc_connections: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
    tx_chats: gtk::glib::Sender<Response>,
    arc_dcc_interface_communication: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
) -> Result<(), ClientError> {
    let mut dcc_hash_lock = dcc_connections.as_ref().write()?;
    let requested_client = dcc_msg.prefix.clone().unwrap_or_else(|| "".to_owned()); // just in the "CHAT" case

    match dcc_msg.command {
        DccMessageType::Chat => {
            if let Some(dcc_connection) = dcc_hash_lock.get(&requested_client) {
                // send information to the active thread that manages the connection
                if dcc_connection.send(DccMessage::serialize(dcc_msg)?).is_ok() {};
                drop(dcc_hash_lock);
            } else {
                // connects to the other client's p2p connection
                let (dcc_sender, dcc_receiver) = sync_channel::<String>(0);
                dcc_hash_lock.insert(requested_client.clone(), dcc_sender);
                drop(dcc_hash_lock);
                connect_to_new_dcc_connection(
                    dcc_receiver,
                    dcc_msg,
                    requested_client,
                    dcc_connections,
                    tx_chats,
                    arc_dcc_interface_communication,
                )?;
            }
        }
        _ => {
            if let Some(dcc_connection) = dcc_hash_lock.get(&requested_client) {
                // send information to the active thread that manages the connection
                if dcc_connection.send(DccMessage::serialize(dcc_msg)?).is_ok() {};
                drop(dcc_hash_lock);
            } else {
                println!("[DEBUG] No active p2p connection with {requested_client}");
                drop(dcc_hash_lock);
            }
        }
    }

    Ok(())
}

/// Creates a new dcc connection between the current client and the requested_client.
/// Sends a dcc connection request to the requested_client, who must accept the request to establish the connection.
/// It also handles the incoming and outgoing dcc messages for the current client who created the connection.
/// Returns a ClientError in case of error
pub fn create_new_dcc_connection(
    dcc_receiver: Receiver<String>,
    dcc_msg: DccMessage,
    requested_client: String,
    dcc_connections: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
    tx_chats: gtk::glib::Sender<Response>,
    arc_dcc_interface_communication: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
) -> Result<(), ClientError> {
    // to communicate with the transfers' thread in progress
    let transfers_communication = HashMap::<String, SyncSender<String>>::new();
    let arc_transfers_communication = Arc::new(RwLock::new(transfers_communication));
    let arc_transfers_communication_clone = arc_transfers_communication.clone();

    // to keep track of unfinished transfers
    let ongoing_transfers = HashMap::<String, OngoingTransfer>::new();
    let arc_ongoing_transfers = Arc::new(RwLock::new(ongoing_transfers));
    let arc_ongoing_transfers_clone = arc_ongoing_transfers.clone();

    if dcc_msg.parameters.len() < 3 {
        println!("[ERROR] first DCC CHAT command needs 3 parameters: client, ip, port.");
        remove_connection(dcc_connections, requested_client);
        return Ok(());
    }
    let ip = dcc_msg.parameters[1].to_owned();
    let port = dcc_msg.parameters[2].to_owned();

    let requested_client_clone = requested_client.clone();
    let tx_chats_clone = tx_chats.clone();

    thread::spawn(move || {
        let listener = match TcpListener::bind(format!("{ip}:{port}")) {
            Ok(listener) => listener,
            Err(_) => {
                println!("[ERROR] Error creating the listener on {ip}:{port}");
                remove_connection(dcc_connections.clone(), requested_client.clone());
                let response = Response::DccResponse {
                    response: DccResponse::ErrorResponse {
                        description: "Invalid address.".to_owned(),
                    },
                };
                if tx_chats.send(response).is_ok() {};
                return;
            }
        };

        println!("[DEBUG] Waiting for connection with {requested_client}");
        let response = Response::DccResponse {
            response: DccResponse::Pending {
                sender: requested_client.clone(),
            },
        };
        if tx_chats.send(response).is_ok() {};

        if let Ok((dcc_socket, _)) = listener.accept() {
            let arc_socket = Arc::new(dcc_socket);
            let arc_socket_clone = arc_socket.clone();
            let arc_socket_clone_1 = arc_socket.clone();

            if let Ok(msg) = read_socket(arc_socket_clone.clone()) {
                if let Ok(first_dcc_msg) = DccMessage::deserialize(msg) {
                    match first_dcc_msg.command {
                        DccMessageType::Accept => {
                            println!("[INFO] Client {requested_client} accepted the connection.");
                            let response = Response::DccResponse {
                                response: DccResponse::Accepted {
                                    sender: requested_client.clone(),
                                },
                            };
                            if tx_chats.send(response).is_ok() {};
                        }
                        DccMessageType::Close => {
                            let response;
                            if first_dcc_msg.parameters.len() > 1 {
                                let description = first_dcc_msg.parameters[1].to_owned();
                                if description == "NotConnected" {
                                    response = Response::ErrorResponse {
                                        response: ErrorResponse::ClientDisconnected {
                                            nickname: requested_client.clone(),
                                        },
                                    };
                                } else {
                                    println!("[ERROR] Invalid response received.");
                                    if arc_socket.as_ref().shutdown(Shutdown::Both).is_ok() {};
                                    remove_connection(dcc_connections.clone(), requested_client);
                                    return;
                                }
                            } else {
                                response = Response::DccResponse {
                                    response: DccResponse::Rejected {
                                        sender: requested_client.clone(),
                                    },
                                };
                            }
                            if tx_chats.send(response).is_ok() {};
                            if arc_socket.as_ref().shutdown(Shutdown::Both).is_ok() {};
                            remove_connection(dcc_connections.clone(), requested_client);
                            return;
                        }
                        _ => {
                            println!(
                                "[ERROR] Wrong DCC command received: {:?}",
                                first_dcc_msg.command
                            );
                        }
                    }
                }
            }

            let dcc_connections_clone = dcc_connections.clone();
            let requested_client_clone_1 = requested_client.clone();
            let tx_chats_clone_1 = tx_chats.clone();

            thread::spawn(move || {
                while let Ok(message_from_client) = read_socket(arc_socket_clone.clone()) {
                    let dcc_msg = match DccMessage::deserialize(message_from_client.clone()) {
                        Ok(m) => m,
                        Err(e) => {
                            println!("[ERROR] Invalid DCC message received: {e:?}");
                            continue;
                        }
                    };
                    match dcc_msg.command {
                        DccMessageType::Chat => {
                            incoming_chat_request(
                                requested_client.clone(),
                                dcc_msg.clone(),
                                tx_chats.clone(),
                            );
                        }
                        DccMessageType::Close => {
                            incoming_close_request(
                                arc_socket_clone.clone(),
                                dcc_connections.clone(),
                                requested_client.clone(),
                                tx_chats_clone,
                            );
                            return;
                        }
                        DccMessageType::Send => {
                            match incoming_send_request(
                                requested_client_clone.clone(),
                                dcc_msg.clone(),
                                tx_chats_clone.clone(),
                                arc_dcc_interface_communication.clone(),
                                arc_ongoing_transfers.clone(),
                            ) {
                                Ok(_) => {}
                                Err(e) => println!("[ERROR] Transfer failed: {e:?}"),
                            };
                        }
                        DccMessageType::Resume => {
                            match incoming_resume_request(
                                dcc_msg.clone(),
                                requested_client_clone.clone(),
                                arc_socket.clone(),
                                tx_chats_clone.clone(),
                                arc_transfers_communication.clone(),
                                arc_ongoing_transfers.clone(),
                            ) {
                                Ok(_) => {}
                                Err(e) => println!("[ERROR] Transfer resume failed: {e:?}"),
                            }
                        }
                        DccMessageType::Stop => {
                            match incoming_stop_request(
                                dcc_msg.clone(),
                                arc_transfers_communication.clone(),
                                arc_ongoing_transfers.clone(),
                                requested_client_clone.clone(),
                            ) {
                                Ok(_) => {}
                                Err(e) => println!("[ERROR] Failed to stop transfer: {e:?}"),
                            };
                        }
                        _ => {
                            todo!();
                        }
                    }
                }
            });

            while let Ok(message_for_client) = dcc_receiver.recv() {
                let dcc_msg = match DccMessage::deserialize(message_for_client.clone()) {
                    Ok(m) => m,
                    Err(e) => {
                        println!("[ERROR] Invalid command: {e:?}");
                        continue;
                    }
                };

                match dcc_msg.command {
                    DccMessageType::Send => {
                        if dcc_msg.parameters.len() < 5 {
                            println!("[ERROR] DCC SEND command needs 5 parameters");
                            continue;
                        }

                        match outgoing_send_request(
                            requested_client_clone_1.clone(),
                            arc_socket_clone_1.clone(),
                            dcc_msg.clone(),
                            tx_chats_clone_1.clone(),
                            arc_transfers_communication_clone.clone(),
                            arc_ongoing_transfers_clone.clone(),
                        ) {
                            Ok(_) => {}
                            Err(e) => {
                                println!("[ERROR] Error sending file: {e:?}");
                                continue;
                            }
                        }
                    }
                    DccMessageType::Close => {
                        outgoing_close_request(
                            arc_socket_clone_1,
                            dcc_connections_clone,
                            requested_client_clone_1,
                            message_for_client,
                        );
                        return;
                    }
                    DccMessageType::Chat => {
                        if write_socket(
                            arc_socket_clone_1.clone(),
                            &DccMessage::serialize(dcc_msg).unwrap(),
                        )
                        .is_ok()
                        {}
                    }
                    DccMessageType::Stop => {
                        match outgoing_stop_request(
                            dcc_msg.clone(),
                            arc_socket_clone_1.clone(),
                            arc_transfers_communication_clone.clone(),
                            arc_ongoing_transfers_clone.clone(),
                        ) {
                            Ok(_) => {}
                            Err(e) => {
                                println!("[ERROR] Error stopping file transfer: {e:?}");
                                continue;
                            }
                        };
                    }
                    DccMessageType::Resume => {
                        match outgoing_resume_request(
                            dcc_msg.clone(),
                            arc_socket_clone_1.clone(),
                            tx_chats_clone_1.clone(),
                            arc_transfers_communication_clone.clone(),
                            arc_ongoing_transfers_clone.clone(),
                        ) {
                            Ok(_) => {}
                            Err(e) => {
                                println!("[ERROR] Error resuming file transfer: {e:?}");
                                continue;
                            }
                        };
                    }
                    _ => {
                        todo!();
                    }
                }
            }
        }
    });

    Ok(())
}

/// Receives a dcc connection request from the interface, from the client who requested the connection.
/// If the connection is accepted, the response is sent back to the client who requested it, and the connection establishes.
/// It also handles the incoming and outgoing dcc messages for the current client who received the connection request.
pub fn connect_to_new_dcc_connection(
    dcc_receiver: Receiver<String>,
    dcc_msg: DccMessage,
    requested_client: String,
    dcc_connections: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
    tx_chats: glib::Sender<Response>,
    arc_dcc_interface_communication: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
) -> Result<(), ClientError> {
    if dcc_msg.parameters.len() < 3 {
        println!("[ERROR] DCC needs more parameters");
        remove_connection(dcc_connections, requested_client);
        return Ok(());
    }

    let transfers_communication = HashMap::<String, SyncSender<String>>::new();
    let arc_transfers_communication = Arc::new(RwLock::new(transfers_communication));
    let arc_transfers_communication_clone = arc_transfers_communication.clone();

    // to keep track of unfinished transfers
    // (file_name, (offset, size, path))
    let ongoing_transfers = HashMap::<String, OngoingTransfer>::new();
    let arc_ongoing_transfers = Arc::new(RwLock::new(ongoing_transfers));
    let arc_ongoing_transfers_clone = arc_ongoing_transfers.clone();

    let ip = dcc_msg.parameters[1].to_owned();
    let port = dcc_msg.parameters[2].to_owned();

    let tx_chats_clone = tx_chats.clone();
    let tx_chats_clone_1 = tx_chats.clone();

    thread::spawn(move || {
        let response = Response::DccResponse {
            response: DccResponse::ChatRequest {
                sender: requested_client.clone(),
            },
        };
        if tx_chats.send(response).is_ok() {};

        // wait for the other client to bind
        thread::sleep(Duration::from_millis(500));
        let socket = match TcpStream::connect(format!("{ip}:{port}")) {
            Ok(socket) => socket,
            Err(e) => {
                println!("[ERROR] Error connecting to {ip}:{port}. {e:?}");
                remove_connection(dcc_connections.clone(), requested_client);
                return;
            }
        };

        let arc_socket = Arc::new(socket);
        let arc_socket_clone = arc_socket.clone();

        if let Ok(answer) = dcc_receiver.recv() {
            let msg = match DccMessage::deserialize(answer) {
                Ok(m) => m,
                Err(e) => {
                    println!("[ERROR] Error deserializing DCC message: {e:?}");
                    return;
                }
            };
            match msg.command {
                DccMessageType::Accept => {
                    println!("[INFO] DCC connection with {requested_client} accepted");
                    if write_socket(
                        arc_socket_clone.clone(),
                        &format!("DCC ACCEPT {requested_client}"),
                    )
                    .is_ok()
                    {};
                }
                DccMessageType::Close => {
                    if write_socket(
                        arc_socket_clone.clone(),
                        &format!("DCC CLOSE {requested_client}"),
                    )
                    .is_ok()
                    {};
                    if arc_socket_clone.as_ref().shutdown(Shutdown::Both).is_ok() {};
                    remove_connection(dcc_connections.clone(), requested_client);
                    return;
                }
                _ => {
                    println!("[ERROR] Invalid DCC command received");
                    return;
                }
            }
        }

        let dcc_connections_clone = dcc_connections.clone();
        let requested_client_clone_1 = requested_client.clone();
        let arc_socket_clone_2 = arc_socket.clone();
        let tx_chats_clone_2 = tx_chats_clone.clone();
        thread::spawn(move || {
            let arc_socket_clone_1 = arc_socket_clone.clone();

            while let Ok(message_from_client) = read_socket(arc_socket_clone.clone()) {
                let requested_client_clone = requested_client.clone();

                let dcc_msg = match DccMessage::deserialize(message_from_client.clone()) {
                    Ok(m) => m,
                    Err(e) => {
                        println!("[ERROR] Error parsing DCC message: {e:?}");
                        continue;
                    }
                };

                match dcc_msg.command {
                    DccMessageType::Chat => {
                        incoming_chat_request(
                            requested_client.clone(),
                            dcc_msg.clone(),
                            tx_chats.clone(),
                        );
                    }
                    DccMessageType::Send => {
                        if dcc_msg.parameters.len() < 4 {
                            println!("[ERROR] Invalid DCC SEND command. Not enough parameters.");
                            continue;
                        }
                        match incoming_send_request(
                            requested_client_clone.clone(),
                            dcc_msg.clone(),
                            tx_chats_clone.clone(),
                            arc_dcc_interface_communication.clone(),
                            arc_ongoing_transfers.clone(),
                        ) {
                            Ok(_) => {}
                            Err(e) => println!("[ERROR] Error sending file: {e:?}"),
                        };
                    }
                    DccMessageType::Close => {
                        incoming_close_request(
                            arc_socket.clone(),
                            dcc_connections.clone(),
                            requested_client.clone(),
                            tx_chats,
                        );
                        return;
                    }
                    DccMessageType::Stop => {
                        match incoming_stop_request(
                            dcc_msg.clone(),
                            arc_transfers_communication.clone(),
                            arc_ongoing_transfers.clone(),
                            requested_client_clone.clone(),
                        ) {
                            Ok(_) => {}
                            Err(e) => println!("[ERROR] Error stopping transfer: {e:?}"),
                        };
                    }
                    DccMessageType::Resume => {
                        match incoming_resume_request(
                            dcc_msg.clone(),
                            requested_client_clone.clone(),
                            arc_socket_clone_1.clone(),
                            tx_chats_clone_1.clone(),
                            arc_transfers_communication.clone(),
                            arc_ongoing_transfers.clone(),
                        ) {
                            Ok(_) => {}
                            Err(e) => println!("[ERROR] Error resuming transfer: {e:?}"),
                        };
                    }
                    _ => {
                        todo!();
                    }
                }
            }
        });

        while let Ok(message_for_client) = dcc_receiver.recv() {
            let arc_socket_clone_3 = arc_socket_clone_2.clone();
            if let Ok(dcc_msg_for_client) = DccMessage::deserialize(message_for_client.clone()) {
                match dcc_msg_for_client.command {
                    DccMessageType::Chat => {
                        if write_socket(arc_socket_clone_2.clone(), &message_for_client).is_ok() {};
                    }
                    DccMessageType::Send => {
                        if dcc_msg_for_client.parameters.len() < 4 {
                            println!("[ERROR] Invalid DCC SEND command. Not enough parameters.");
                            continue;
                        }
                        match outgoing_send_request(
                            requested_client_clone_1.clone(),
                            arc_socket_clone_2.clone(),
                            dcc_msg_for_client.clone(),
                            tx_chats_clone_2.clone(),
                            arc_transfers_communication_clone.clone(),
                            arc_ongoing_transfers_clone.clone(),
                        ) {
                            Ok(_) => {}
                            Err(e) => println!("[ERROR] Transfer failed: {e:?}"),
                        };
                    }
                    DccMessageType::Close => {
                        outgoing_close_request(
                            arc_socket_clone_2,
                            dcc_connections_clone,
                            requested_client_clone_1,
                            message_for_client,
                        );
                        return;
                    }
                    DccMessageType::Stop => {
                        match outgoing_stop_request(
                            dcc_msg_for_client.clone(),
                            arc_socket_clone_2.clone(),
                            arc_transfers_communication_clone.clone(),
                            arc_ongoing_transfers_clone.clone(),
                        ) {
                            Ok(_) => {}
                            Err(e) => println!("[ERROR] Error stopping transfer: {e:?}"),
                        };
                    }
                    DccMessageType::Resume => {
                        match outgoing_resume_request(
                            dcc_msg_for_client.clone(),
                            arc_socket_clone_3.clone(),
                            tx_chats_clone_2.clone(),
                            arc_transfers_communication_clone.clone(),
                            arc_ongoing_transfers_clone.clone(),
                        ) {
                            Ok(_) => {}
                            Err(e) => {
                                println!("[ERROR] Error resuming file transfer: {e:?}");
                                continue;
                            }
                        };
                    }
                    _ => {
                        todo!();
                    }
                }
            }
        }
    });

    Ok(())
}
