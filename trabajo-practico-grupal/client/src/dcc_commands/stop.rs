use std::{
    collections::HashMap,
    net::TcpStream,
    sync::{mpsc::SyncSender, Arc, RwLock},
};

use model::{
    client_errors::ClientError, dcc::DccMessage, responses::ongoing_transfer::OngoingTransfer,
    socket::write_socket,
};

/// Manages a stop request from the current client in order to stop the current file transfer
/// If the current client has the file, it sends the stop message through the transfer sender to the transfer thread
/// Otherwise, it sends a stop request to the client's socket
/// Returns a ClientError if the lock on the ongoing transfers hash can't be acquired
pub fn outgoing_stop_request(
    dcc_msg: DccMessage,
    arc_socket: Arc<TcpStream>,
    arc_transfers_communication: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
    arc_ongoing_transfers: Arc<RwLock<HashMap<String, OngoingTransfer>>>,
) -> Result<(), ClientError> {
    let filename = dcc_msg.parameters[1].to_owned();

    let ongoing_transfers_lock = match arc_ongoing_transfers.as_ref().read() {
        Ok(lock) => lock,
        Err(_) => {
            println!("[ERROR] Error getting write lock on ongoing_transfers");
            return Err(ClientError::LockError);
        }
    };
    let mut client_has_the_file = false;
    if let Some(ongoing_transfer) = ongoing_transfers_lock.get(&filename) {
        if !ongoing_transfer.file_path.is_empty() {
            client_has_the_file = true;
        }
    }
    drop(ongoing_transfers_lock);

    if client_has_the_file {
        let arc_transfers_communication_lock = match arc_transfers_communication.as_ref().read() {
            Ok(lock) => lock,
            Err(_) => {
                println!("[ERROR] Error getting read lock on transfers_communication");
                return Err(ClientError::LockError);
            }
        };

        if let Some(tx_transfer) = arc_transfers_communication_lock.get(&filename) {
            match tx_transfer.send(DccMessage::serialize(dcc_msg)?) {
                Ok(()) => {}
                Err(e) => println!("[ERROR] Error sending message to transfer thread: {e}"),
            };
        }
    } else {
        // send stop request through socket
        write_socket(arc_socket, &DccMessage::serialize(dcc_msg)?)?;
    }
    Ok(())
}

/// Manages an incoming stop request from another client
/// If the current client has the file, it sends the stop message through the transfer sender to the transfer thread
/// Returns a ClientError if the lock on the ongoing transfers hash can't be acquired
pub fn incoming_stop_request(
    dcc_msg: DccMessage,
    arc_transfers_communication: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
    arc_ongoing_transfers: Arc<RwLock<HashMap<String, OngoingTransfer>>>,
    requested_client: String,
) -> Result<(), ClientError> {
    let filename = dcc_msg.parameters[1].to_owned();

    let ongoing_transfers_lock = match arc_ongoing_transfers.as_ref().read() {
        Ok(lock) => lock,
        Err(_) => {
            println!("[ERROR] Error getting write lock on ongoing_transfers");
            return Err(ClientError::LockError);
        }
    };
    let mut client_has_the_file = false;
    if let Some(ongoing_transfer) = ongoing_transfers_lock.get(&filename) {
        if !ongoing_transfer.file_path.is_empty() {
            client_has_the_file = true;
        }
    }
    drop(ongoing_transfers_lock);

    if client_has_the_file {
        let arc_transfers_communication_lock = match arc_transfers_communication.as_ref().read() {
            Ok(lock) => lock,
            Err(_) => {
                println!("[ERROR] Error getting read lock on transfers_communication");
                return Err(ClientError::LockError);
            }
        };

        let mut dcc_msg_aux = dcc_msg;
        dcc_msg_aux.parameters[0] = requested_client;
        if let Some(tx_transfer) = arc_transfers_communication_lock.get(&filename) {
            match tx_transfer.send(DccMessage::serialize(dcc_msg_aux)?) {
                Ok(()) => {}
                Err(e) => println!("[ERROR] Error sending message to transfer thread: {e}"),
            };
        }
        drop(arc_transfers_communication_lock);
    }

    Ok(())
}
