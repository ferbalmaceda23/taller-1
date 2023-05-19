use std::{
    collections::HashMap,
    net::{TcpListener, TcpStream},
    sync::{
        mpsc::{sync_channel, Receiver, SyncSender},
        Arc, RwLock,
    },
    thread,
    time::Duration,
};

use crate::dcc_commands::transfer::receive_file;
use crate::dcc_commands::transfer::transfer_file;
use crate::{dcc_commands::transfer::remove_transfer_communication, run_interface::check_address};
use gtk::glib;
use model::{
    client_errors::ClientError,
    dcc::DccMessage,
    responses::{dcc::DccResponse, ongoing_transfer::OngoingTransfer, response::Response},
    socket::write_socket,
};

/// Manages a resume request to resume the current file transfer that was previously paused
/// Checks if the address is valid and if the file is being transferred
/// If the file is being transferred, it sends a message to the interface to notify that the transfer is being resumed
/// It checks if the current client has the file, and calls the function to resume the transfer or the receipt of the file
/// It returns a ClientError if there is an error creating the socket
pub fn outgoing_resume_request(
    dcc_msg: DccMessage,
    arc_socket: Arc<TcpStream>,
    tx_chats: glib::Sender<Response>,
    arc_transfers_communication: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
    arc_ongoing_transfers: Arc<RwLock<HashMap<String, OngoingTransfer>>>,
) -> Result<(), ClientError> {
    let requested_client = dcc_msg.parameters[0].to_owned();
    let filename = dcc_msg.parameters[1].to_owned();
    let ip = dcc_msg.parameters[2].to_owned();
    let port = dcc_msg.parameters[3].to_owned();

    if !check_address(ip, port) {
        let response = Response::DccResponse {
            response: DccResponse::ResumeAddressErrorResponse {
                sender: requested_client,
                file_name: filename,
            },
        };
        if tx_chats.send(response).is_ok() {};
        return Err(ClientError::SocketError);
    }

    let arc_ongoing_transfers_lock = match arc_ongoing_transfers.as_ref().read() {
        Ok(lock) => lock,
        Err(_) => {
            println!("[ERROR] Error getting read lock on ongoing_transfers");
            return Err(ClientError::LockError);
        }
    };

    let mut client_has_the_file = false;
    let mut file_path = String::new();
    let file_offset = match arc_ongoing_transfers_lock.get(&filename) {
        Some(ongoing_transfer) => {
            if !ongoing_transfer.file_path.is_empty() {
                client_has_the_file = true;
                file_path = ongoing_transfer.file_path.to_owned();
            }
            ongoing_transfer.file_offset
        }
        None => {
            println!("[ERROR] No ongoing transfer found with key: {filename}");
            return Err(ClientError::NoOngoingTransfer);
        }
    };
    drop(arc_ongoing_transfers_lock);

    let mut dcc_msg_for_client = dcc_msg.clone();
    dcc_msg_for_client
        .parameters
        .append(&mut vec![file_offset.to_string()]);
    write_socket(
        arc_socket.clone(),
        &DccMessage::serialize(dcc_msg_for_client)?,
    )?;

    let (tx_transfer, rx_transfer) = sync_channel(0);
    let mut arc_transfers_communication_lock = match arc_transfers_communication.as_ref().write() {
        Ok(lock) => lock,
        Err(_) => {
            println!("[ERROR] Error getting write lock on transfers_communication");
            return Err(ClientError::LockError);
        }
    };
    arc_transfers_communication_lock.insert(filename.clone(), tx_transfer);
    drop(arc_transfers_communication_lock);

    thread::spawn(move || {
        if client_has_the_file {
            if outgoing_resume_transfer_file(
                (filename, file_path, file_offset),
                dcc_msg,
                arc_socket,
                rx_transfer,
                tx_chats,
                arc_ongoing_transfers,
                arc_transfers_communication,
            )
            .is_ok()
            {};
        } else if outgoing_resume_receive_file(
            (filename, file_offset),
            dcc_msg,
            tx_chats,
            arc_ongoing_transfers,
        )
        .is_ok()
        {
        };
    });

    Ok(())
}

/// Manages the transfer of a file that was previously paused
/// It creates a thread to resume the transfer and sends a message to the interface to notify that the transfer is being resumed
/// It returns a ClientError if there is an error creating the socket
fn outgoing_resume_transfer_file(
    file_data: (String, String, u64),
    dcc_msg: DccMessage,
    arc_socket: Arc<TcpStream>,
    rx_transfer: Receiver<String>,
    tx_chats: glib::Sender<Response>,
    arc_ongoing_transfers: Arc<RwLock<HashMap<String, OngoingTransfer>>>,
    arc_transfers_communication: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
) -> Result<(), ClientError> {
    let (filename, filepath, file_offset) = file_data;
    let requested_client = dcc_msg.parameters[0].to_owned();
    let ip = dcc_msg.parameters[2].to_owned();
    let port = dcc_msg.parameters[3].to_owned();

    let listener = match TcpListener::bind(format!("{ip}:{port}")) {
        Ok(listener) => listener,
        Err(_) => {
            println!("[ERROR] Error binding to {ip}:{port}");
            let response = Response::DccResponse {
                response: DccResponse::ErrorResponse {
                    description: "Invalid address.".to_owned(),
                },
            };
            if tx_chats.send(response).is_ok() {};
            return Err(ClientError::SocketError);
        }
    };

    let filename_clone = filename.clone();
    thread::spawn(move || {
        if let Ok((transfer_socket, _)) = listener.accept() {
            let arc_transfer_socket = Arc::new(transfer_socket);
            println!("[DEBUG] Resuming transfer on {file_offset} bytes");

            let response = Response::DccResponse {
                response: DccResponse::TransferResumed {
                    sender: requested_client.clone(),
                    file_name: filename_clone.clone(),
                },
            };
            if tx_chats.send(response).is_ok() {};

            match transfer_file(
                (filename_clone, filepath, -1.0, file_offset),
                requested_client,
                arc_transfer_socket,
                rx_transfer,
                tx_chats,
                arc_socket,
                arc_ongoing_transfers,
            ) {
                Ok(_) => {}
                Err(err) => {
                    println!("[ERROR] Error transferring file: {err:?}");
                }
            }
        }
        remove_transfer_communication(arc_transfers_communication, filename);
    });

    Ok(())
}

/// Manages the receipt of a file that was previously paused
/// It creates a thread to resume the receipt and sends a message to the interface to notify that the receipt is being resumed
/// It returns a ClientError if there is an error creating the socket
fn outgoing_resume_receive_file(
    file_data: (String, u64),
    dcc_msg: DccMessage,
    tx_chats: glib::Sender<Response>,
    arc_ongoing_transfers: Arc<RwLock<HashMap<String, OngoingTransfer>>>,
) -> Result<(), ClientError> {
    let (filename, file_offset) = file_data;
    let requested_client = dcc_msg.parameters[0].to_owned();
    let ip = dcc_msg.parameters[2].to_owned();
    let port = dcc_msg.parameters[3].to_owned();

    let listener = match TcpListener::bind(format!("{ip}:{port}")) {
        Ok(listener) => listener,
        Err(_) => {
            println!("[ERROR] Error binding to {ip}:{port}");
            let response = Response::DccResponse {
                response: DccResponse::ErrorResponse {
                    description: "Invalid address.".to_owned(),
                },
            };
            if tx_chats.send(response).is_ok() {};
            return Err(ClientError::SocketError);
        }
    };

    thread::spawn(move || {
        if let Ok((transfer_socket, _)) = listener.accept() {
            let arc_transfer_socket = Arc::new(transfer_socket);
            println!("[DEBUG] Resuming transfer on {file_offset} bytes");

            let response = Response::DccResponse {
                response: DccResponse::TransferResumed {
                    sender: requested_client.clone(),
                    file_name: filename.clone(),
                },
            };
            if tx_chats.send(response).is_ok() {};

            match receive_file(
                (filename, -1.0, file_offset),
                arc_transfer_socket,
                requested_client,
                tx_chats,
                arc_ongoing_transfers,
            ) {
                Ok(_) => {}
                Err(err) => {
                    println!("[ERROR] Error receiving file: {err:?}");
                }
            }
        }
    });

    Ok(())
}

/// Manages an incoming resume request to resume the current file transfer that was previously paused by the other client
/// If the file is being transferred, it sends a message to the interface to notify that the transfer is being resumed
/// It checks if the current client has the file, and calls the function to resume the transfer or the receipt of the file
/// It returns a ClientError if there is an error creating the socket or if the file is not an ongoing transfer
pub fn incoming_resume_request(
    dcc_msg: DccMessage,
    requested_client: String,
    arc_socket: Arc<TcpStream>,
    tx_chats: glib::Sender<Response>,
    arc_transfers_communication: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
    arc_ongoing_transfers: Arc<RwLock<HashMap<String, OngoingTransfer>>>,
) -> Result<(), ClientError> {
    let filename = dcc_msg.parameters[1].to_owned();

    let arc_ongoing_transfers_lock = match arc_ongoing_transfers.as_ref().read() {
        Ok(lock) => lock,
        Err(_) => {
            println!("[ERROR] Error getting read lock on ongoing_transfers");
            return Err(ClientError::LockError);
        }
    };

    let mut client_has_the_file = false;
    let mut file_path = String::new();
    let file_offset = match arc_ongoing_transfers_lock.get(&filename) {
        Some(ongoing_transfer) => {
            if !ongoing_transfer.file_path.is_empty() {
                client_has_the_file = true;
                file_path = ongoing_transfer.file_path.to_owned();
            }
            ongoing_transfer.file_offset
        }
        None => {
            println!("[ERROR] No ongoing transfer found with key: {filename}");
            return Err(ClientError::NoOngoingTransfer);
        }
    };
    drop(arc_ongoing_transfers_lock);

    let (tx_transfer, rx_transfer) = sync_channel(0);
    let mut arc_transfers_communication_lock = match arc_transfers_communication.as_ref().write() {
        Ok(lock) => lock,
        Err(_) => {
            println!("[ERROR] Error getting write lock on transfers_communication");
            return Err(ClientError::LockError);
        }
    };
    arc_transfers_communication_lock.insert(filename.clone(), tx_transfer);
    drop(arc_transfers_communication_lock);

    let mut dcc_msg_aux = dcc_msg;
    dcc_msg_aux.parameters[0] = requested_client;

    thread::spawn(move || {
        if client_has_the_file {
            if incoming_resume_transfer_file(
                (filename, file_path, file_offset),
                dcc_msg_aux,
                arc_socket,
                rx_transfer,
                tx_chats,
                arc_ongoing_transfers,
                arc_transfers_communication,
            )
            .is_ok()
            {};
        } else if incoming_resume_receive_file(
            (filename, file_offset),
            dcc_msg_aux,
            tx_chats,
            arc_ongoing_transfers,
        )
        .is_ok()
        {
        };
    });

    Ok(())
}

/// Manages an incoming resume request to resume the current transfer that was previously paused by the other client
/// It creates a thread to resume the transfer and sends a message to the interface to notify that the transfer is being resumed
/// It returns a ClientError if there is an error creating the socket
fn incoming_resume_transfer_file(
    file_data: (String, String, u64), // filename, filepath, file_offset
    dcc_msg: DccMessage,
    arc_socket: Arc<TcpStream>,
    rx_transfer: Receiver<String>,
    tx_chats: glib::Sender<Response>,
    arc_ongoing_transfers: Arc<RwLock<HashMap<String, OngoingTransfer>>>,
    arc_transfers_communication: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
) -> Result<(), ClientError> {
    let (filename, filepath, file_offset) = file_data;
    let requested_client = dcc_msg.parameters[0].to_owned();
    let ip = dcc_msg.parameters[2].to_owned();
    let port = dcc_msg.parameters[3].to_owned();

    thread::sleep(Duration::from_millis(500));
    let transfer_socket = match TcpStream::connect(format!("{ip}:{port}")) {
        Ok(socket) => socket,
        Err(_) => {
            println!("[ERROR] Error connecting to {ip}:{port}");
            return Err(ClientError::SocketError);
        }
    };

    let arc_transfer_socket = Arc::new(transfer_socket);

    let filename_clone = filename.clone();
    thread::spawn(move || {
        println!("[DEBUG] Resuming transfer on {file_offset} bytes");

        let response = Response::DccResponse {
            response: DccResponse::TransferResumed {
                sender: requested_client.clone(),
                file_name: filename_clone.clone(),
            },
        };
        if tx_chats.send(response).is_ok() {};

        match transfer_file(
            (filename_clone, filepath, -1.0, file_offset),
            requested_client,
            arc_transfer_socket,
            rx_transfer,
            tx_chats,
            arc_socket,
            arc_ongoing_transfers,
        ) {
            Ok(_) => {}
            Err(err) => {
                println!("[ERROR] Error transferring file: {err:?}");
            }
        }
        remove_transfer_communication(arc_transfers_communication, filename);
    });

    Ok(())
}

/// Manages an incoming resume request to resume the current receipt that was previously paused by the other client
/// It creates a thread to resume the receipt and sends a message to the interface to notify that the receipt is being resumed
/// It returns a ClientError if there is an error creating the socket
fn incoming_resume_receive_file(
    file_data: (String, u64), // filename, file_offset
    dcc_msg: DccMessage,
    tx_chats: glib::Sender<Response>,
    arc_ongoing_transfers: Arc<RwLock<HashMap<String, OngoingTransfer>>>,
) -> Result<(), ClientError> {
    let (filename, file_offset) = file_data;
    let requested_client = dcc_msg.parameters[0].to_owned();
    let ip = dcc_msg.parameters[2].to_owned();
    let port = dcc_msg.parameters[3].to_owned();

    thread::sleep(Duration::from_millis(500));
    let transfer_socket = match TcpStream::connect(format!("{ip}:{port}")) {
        Ok(socket) => socket,
        Err(_) => {
            println!("[ERROR] Error connecting to {ip}:{port}");
            return Err(ClientError::SocketError);
        }
    };

    let arc_transfer_socket = Arc::new(transfer_socket);
    let filename_clone = filename;
    thread::spawn(move || {
        println!("[DEBUG] Resuming transfer on {file_offset} bytes");

        let response = Response::DccResponse {
            response: DccResponse::TransferResumed {
                sender: requested_client.clone(),
                file_name: filename_clone.clone(),
            },
        };
        if tx_chats.send(response).is_ok() {};

        match receive_file(
            (filename_clone, -1.0, file_offset),
            arc_transfer_socket,
            requested_client,
            tx_chats,
            arc_ongoing_transfers,
        ) {
            Ok(_) => {}
            Err(err) => {
                println!("[ERROR] Error receiving file: {err:?}");
            }
        }
    });

    Ok(())
}
