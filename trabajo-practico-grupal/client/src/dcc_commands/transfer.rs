use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    net::{Shutdown, TcpStream},
    path::Path,
    sync::{
        mpsc::{Receiver, SyncSender},
        Arc, RwLock,
    },
    thread,
    time::Duration,
};

use gtk::glib;
use model::{
    client_errors::ClientError,
    dcc::{DccMessage, DccMessageType},
    responses::{dcc::DccResponse, ongoing_transfer::OngoingTransfer, response::Response},
    socket::write_socket,
};

/// Receives data from the socket and writes it to the file
/// It also sends the progress of the transfer to the interface
/// The file_data contains the file name, the file size and the file offset
/// It returns a ClientError if there is an error reading from the socket or writing to the file
pub fn receive_file(
    // file_data.0 = file_name, file_data.1 = file_size, file_data.2 = file_offset
    file_data: (String, f64, u64),
    arc_transfer_socket: Arc<TcpStream>,
    requested_client: String,
    tx_chats: glib::Sender<Response>,
    arc_ongoing_transfers: Arc<RwLock<HashMap<String, OngoingTransfer>>>,
) -> Result<(), ClientError> {
    let file_name = file_data.0;
    let mut file_size = file_data.1;
    let file_offset = file_data.2;

    let transfer_folder = Path::new("./client/files_to_receive");
    if !transfer_folder.exists() && std::fs::create_dir(transfer_folder).is_err() {
        println!("[ERROR] Error creating transfer folder");
        return Ok(());
    }

    let mut file;
    let path_to_save = format!("./client/files_to_receive/{file_name}");
    if file_offset == 0 {
        file = match File::create(path_to_save) {
            Ok(file) => file,
            Err(e) => {
                println!("[ERROR] Error opening file: {e:?}");
                return Ok(());
            }
        };
    } else {
        file = match OpenOptions::new()
            .read(false)
            .create(false)
            .append(true)
            .open(path_to_save)
        {
            Ok(file) => file,
            Err(e) => {
                println!("[ERROR] Error opening file: {e:?}");
                return Ok(());
            }
        };
    }

    if file_size == -1.0 {
        file_size = get_file_size(arc_ongoing_transfers.clone(), file_name.clone())?;
    }
    println!("[DEBUG] File size: {file_size}");

    if file.seek(SeekFrom::Start(file_offset)).is_err() {
        println!("[ERROR] Error seeking file with offset: {file_offset}");
    }

    let mut file_bytes_read = file_offset;
    loop {
        let mut buffer = [0; 1024];

        match arc_transfer_socket.as_ref().read(&mut buffer) {
            Ok(0) => {
                if file_bytes_read < file_size as u64 {
                    println!("[DEBUG] Socket returned 0 so transfer stopped.");
                    println!("[DEBUG] Stopped on {file_bytes_read} bytes");
                    update_ongoing_transfer(
                        arc_ongoing_transfers.clone(),
                        file_bytes_read,
                        file_size,
                        file_name.clone(),
                        "".to_string(),
                    );
                    let response = Response::DccResponse {
                        response: DccResponse::TransferPaused {
                            sender: requested_client,
                            file_name: file_name.clone(),
                        },
                    };
                    if tx_chats.send(response).is_ok() {};
                }
                break;
            }
            Ok(bytes_read) => match file.write(&buffer[0..bytes_read]) {
                Ok(bytes_written) => {
                    if bytes_written != bytes_read {
                        println!("[ERROR] Error writing to file");
                        break;
                    } else {
                        file_bytes_read += bytes_read as u64;
                        send_progress(
                            file_bytes_read as f64,
                            file_size,
                            &requested_client,
                            &file_name,
                            tx_chats.clone(),
                        )?;
                    }
                }
                Err(e) => {
                    println!("[ERROR] Error writing to file: {e:?}");
                }
            },
            Err(e) => {
                println!("[ERROR] Error reading from socket: {e:?}");
                return Ok(());
            }
        }

        update_ongoing_transfer(
            arc_ongoing_transfers.clone(),
            file_bytes_read,
            file_size,
            file_name.clone(),
            "".to_string(),
        );
    }

    if file_bytes_read == file_size as u64 {
        println!("[INFO] Transfer complete, deleting ongoing transfer");
        remove_ongoing_transfer(arc_ongoing_transfers, file_name);
    }

    Ok(())
}

/// Sends the file to the requested client through the transfer socket
/// It also sends the progress of the transfer to the interface
/// The file_data contains the file name, the file path, the file size and the file offset
/// It returns a ClientError if there is an error reading from the file or writing to the socket
pub fn transfer_file(
    // (filedata.0 = filename, filedata.1 = filepath, filedata.2 = filesize, filedata.3 = offset)
    file_data: (String, String, f64, u64),
    requested_client: String,
    arc_transfer_socket: Arc<TcpStream>,
    rx_transfer: Receiver<String>,
    tx_chats: glib::Sender<Response>,
    arc_socket: Arc<TcpStream>,
    arc_ongoing_transfers: Arc<RwLock<HashMap<String, OngoingTransfer>>>,
) -> Result<(), ClientError> {
    let file_name = file_data.0;
    let file_path = file_data.1;
    let mut file_size = file_data.2;
    let file_offset = file_data.3;

    let mut file = match File::open(file_path.clone()) {
        Ok(file) => file,
        Err(e) => {
            println!("[ERROR] Error opening file: {e:?}");
            return Err(ClientError::FileError);
        }
    };

    if file_size == -1.0 {
        file_size = get_file_size(arc_ongoing_transfers.clone(), file_name.clone())?;
    }

    println!("[DEBUG] File size: {file_size}");

    if file.seek(SeekFrom::Start(file_offset)).is_err() {
        println!("[ERROR] Error seeking file with offset: {file_offset}");
    }

    let mut bytes_read = file_offset;
    loop {
        let mut buffer = [0; 1024];

        if let Ok(msg) = rx_transfer.try_recv() {
            if let Ok(dcc_msg) = DccMessage::deserialize(msg.clone()) {
                match dcc_msg.command {
                    DccMessageType::Stop => {
                        println!("[INFO] Stopping transfer");
                        if dcc_msg.parameters.len() < 3 {
                            let message_for_client =
                                format!("DCC STOP {requested_client} {file_name} {bytes_read}");
                            write_socket(arc_socket, &message_for_client)?;
                            update_ongoing_transfer(
                                arc_ongoing_transfers,
                                bytes_read,
                                file_size,
                                file_name.clone(),
                                file_path,
                            );
                        } else {
                            update_ongoing_transfer(
                                arc_ongoing_transfers,
                                dcc_msg.parameters[2].parse::<u64>().unwrap_or(0),
                                file_size,
                                file_name.clone(),
                                file_path,
                            );
                        }
                        let response = Response::DccResponse {
                            response: DccResponse::TransferPaused {
                                sender: requested_client,
                                file_name,
                            },
                        };
                        if tx_chats.send(response).is_ok() {};
                        println!("[DEBUG] Stopped on {bytes_read} bytes");
                        arc_transfer_socket.as_ref().shutdown(Shutdown::Both)?;
                        return Ok(());
                    }
                    _ => {
                        println!(
                            "[ERROR] Unknown DCC command received within transfer: {:?}",
                            dcc_msg.command
                        );
                    }
                }
            }
        }

        let read = match file.read(&mut buffer) {
            Ok(bytes_read) => bytes_read,
            Err(e) => {
                println!("[ERROR] Error reading from file: {e:?}");
                break;
            }
        };

        // just to test progress bar in GUI
        thread::sleep(Duration::from_millis(1000));

        match arc_transfer_socket.as_ref().write(&buffer[0..read]) {
            Ok(0) => {
                // transfer failed or ended
                break;
            }
            Ok(bytes_written) => {
                if bytes_written != read {
                    println!("[ERROR] Error writing to socket");
                    break;
                } else {
                    bytes_read += read as u64;

                    send_progress(
                        bytes_read as f64,
                        file_size,
                        &requested_client,
                        &file_name,
                        tx_chats.clone(),
                    )?;
                }
            }
            Err(e) => {
                println!("[ERROR] Error writing to socket: {e:?}");
                break;
            }
        };

        update_ongoing_transfer(
            arc_ongoing_transfers.clone(),
            bytes_read,
            file_size,
            file_name.clone(),
            file_path.clone(),
        );
    }

    if bytes_read == file_size as u64 {
        println!("[INFO] Transfer complete, deleting ongoing transfer..");
        remove_ongoing_transfer(arc_ongoing_transfers, file_name);
    }

    Ok(())
}

/// Sends the progress of the transfer to the interface
pub fn send_progress(
    bytes_transfered: f64,
    file_size: f64,
    requested_client: &str,
    filename: &str,
    tx_chats: glib::Sender<Response>,
) -> Result<(), ClientError> {
    let progress = bytes_transfered / file_size;
    println!("[INFO] Transfer progress : {:.2}%", progress * 100.00);
    let response = Response::DccResponse {
        response: DccResponse::TransferProgress {
            sender: requested_client.to_owned(),
            file_name: filename.to_owned(),
            progress,
        },
    };
    if tx_chats.send(response).is_ok() {};
    Ok(())
}

/// Updates the ongoing transfers hash with the the file and its file data
pub fn update_ongoing_transfer(
    arc_ongoing_transfers: Arc<RwLock<HashMap<String, OngoingTransfer>>>,
    bytes_read: u64,
    file_size: f64,
    file_name: String,
    file_path: String,
) {
    let mut arc_ongoing_transfers_lock = match arc_ongoing_transfers.as_ref().write() {
        Ok(arc_ongoing_transfers) => arc_ongoing_transfers,
        Err(e) => {
            println!("[ERROR] Error locking ongoing transfers: {e:?}");
            return;
        }
    };

    let ongoing_transfer = OngoingTransfer {
        file_offset: bytes_read,
        file_size,
        file_path,
    };

    arc_ongoing_transfers_lock.insert(file_name, ongoing_transfer);
    drop(arc_ongoing_transfers_lock);
}

/// Removes the ongoing transfer from the hash
pub fn remove_ongoing_transfer(
    arc_ongoing_transfers: Arc<RwLock<HashMap<String, OngoingTransfer>>>,
    file_name: String,
) {
    let mut arc_ongoing_transfers_lock = match arc_ongoing_transfers.as_ref().write() {
        Ok(lock) => lock,
        Err(_) => {
            println!("[ERROR] Error accesing the hash for ongoing transfers");
            return;
        }
    };
    arc_ongoing_transfers_lock.remove(&file_name);
    drop(arc_ongoing_transfers_lock);
}

/// Removes the communication channel with the requested client from the interface communication hash
pub fn remove_interface_communication(
    arc_interface_communication: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
    requested_client: String,
) {
    let mut arc_interface_communication_lock = match arc_interface_communication.as_ref().write() {
        Ok(lock) => lock,
        Err(_) => {
            println!("[ERROR] Error accesing the hash for communication with interface");
            return;
        }
    };

    arc_interface_communication_lock.remove(&requested_client);
    drop(arc_interface_communication_lock);
}

/// Removes the transfer channel with the requested client from the transfer communication hash
pub fn remove_transfer_communication(
    arc_transfers_communication: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
    file_name: String,
) {
    let mut arc_transfers_communication_lock = match arc_transfers_communication.as_ref().write() {
        Ok(lock) => lock,
        Err(_) => {
            println!("[ERROR] Error accesing the hash for communication with interface");
            return;
        }
    };

    arc_transfers_communication_lock.remove(&file_name);
    drop(arc_transfers_communication_lock);
}

/// Gets the current file size of the file being transfered from the ongoing transfers hash
/// In case the file is not being transfered, it returns 0
/// In case of error, it returns a ClientError
fn get_file_size(
    arc_ongoing_transfers: Arc<RwLock<HashMap<String, OngoingTransfer>>>,
    file_name: String,
) -> Result<f64, ClientError> {
    let arc_going_transfers_lock = match arc_ongoing_transfers.as_ref().read() {
        Ok(lock) => lock,
        Err(_) => {
            println!("[ERROR] Error accesing the hash for ongoing transfers");
            return Err(ClientError::LockError);
        }
    };

    if let Some(ongoing_transfer) = arc_going_transfers_lock.get(&file_name) {
        Ok(ongoing_transfer.file_size)
    } else {
        Ok(0f64)
    }
}

/*
#[cfg(test)]
mod dcc_tests {
    use std::{fs::{self, File}, sync::{mpsc::{Sender, SyncSender, sync_channel}, Arc, Mutex, RwLock}, collections::HashMap, net::{TcpStream, TcpListener}, path::Path, thread, time::Duration};

    //use crate::{chat::incoming_chat_request, transfer::{receive_file, transfer_file, remove_transfer_communication}, close::{incoming_close_request, outgoing_close_request}};
    use gtk::glib;
    use model::{persistence::PersistenceType, session::Session, network::Network, message::Message, server::Server, dcc::{DccMessage, DccMessageType}, responses::{dcc::DccResponse, response::Response}};
    use server::{client_handler::register_client, database::handle_database};

    use crate::dcc_commands::transfer::{receive_file, transfer_file, remove_transfer_communication};


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

    fn register_client_for_test(nick: String, stream: Arc<TcpStream>, session: &Session, network: &Network) {


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
        ).unwrap();

        register_client(
            message_nick,
            (&mut nickname, &mut user_parameters),
            &mut password,
            stream.clone(),
            &session,
            &network,
            &"test".to_string(),
        ).unwrap();
    }



    #[test]
    fn test_dcc_send() {
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
        register_client_for_test("sender".to_string(), client_stream_sender.clone(), &session, &network);
        register_client_for_test("receiver".to_string(), client_stream_receiver.clone(), &session, &network);

        let (tx_chats, _rx_chats) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

        let dcc_msg = DccMessage {
            prefix: None,
            command: DccMessageType::Send,
            parameters: ["receiver".to_string(), "test_file.txt".to_string(), "127.0.0.1".to_string(), "0".to_string(), "89390".to_string()].to_vec(),
        };

        let arc_ongoing_transfers: Arc<RwLock<HashMap<String, (u64, f64, String)>>> = Arc::new(RwLock::new(HashMap::new()));

        let arc_transfers_communication: Arc<RwLock<HashMap<String, SyncSender<String>>>> = Arc::new(RwLock::new(HashMap::new()));
        let (tx_transfer, rx_transfer) = sync_channel(0);
        let filepath = "./test_file.txt";
        assert!(Path::new(filepath).exists());
        let filename = dcc_msg.parameters[1].split("/").last().unwrap_or("").to_owned();
        let file_data_transfer = (filename.clone(), filepath.to_string(), "89390".parse::<f64>().unwrap_or(0.0), 0);
        let file_data_receive = (filepath.to_string(), "89390".parse::<f64>().unwrap_or(0.0), 0);

        let mut transfers_communication_lock = arc_transfers_communication.as_ref().write().unwrap();
        transfers_communication_lock.insert(filename.to_string(), tx_transfer);
        drop(transfers_communication_lock);


        let transfer_socket = TcpStream::connect(format!("{}:{}", "127.0.0.1", port)).unwrap();
        let arc_transfer_socket = Arc::new(transfer_socket);

        let arc_transfer_socket_clone = arc_transfer_socket.clone();
        let tx_chats_clone = tx_chats.clone();
        let arc_ongoing_transfers_clone = arc_ongoing_transfers.clone();
        // create thread to transfer the file
        thread::spawn(move || {
            // file_data_transfer: (filedata.0 = filename, filedata.1 = filepath, filedata.2 = filesize, filedata.3 = offset)
            transfer_file(
                file_data_transfer,
                "sender".to_string(),
                arc_transfer_socket_clone.clone(),
                rx_transfer,
                tx_chats_clone.clone(),
                client_stream_sender.clone(),
                arc_ongoing_transfers_clone.clone(),
            ).unwrap();
        });

        // sleep 1 seconmd
        thread::sleep(Duration::from_secs(1));
        // file_data.0 = file_name, file_data.1 = file_size, file_data.2 = file_offset
        receive_file(
            file_data_receive,
            arc_transfer_socket.clone(),
            "receiver".to_string(),
            tx_chats.clone(),
            arc_ongoing_transfers.clone(),
            client_stream_receiver.clone()
        ).unwrap();

        let mut file = File::create(format!("./{}", filename)).unwrap();
        assert!(Path::new("./test_file.txt").exists());


        assert!(Path::new("./client/files_to_receive/test_file.txt").exists());
        assert!(fs::metadata("./client/files_to_receive/test_file.txt").unwrap().len() == 89390);

        // Delete the test file
        remove_transfer_communication(arc_transfers_communication, filename);
        fs::remove_file("./client/files_to_receive/test_file.txt").unwrap();
    }




}*/
