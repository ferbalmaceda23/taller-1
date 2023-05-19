use std::{net::TcpStream, sync::Arc, thread, time::Duration};

use model::{
    dcc::{DccMessage, DccMessageType},
    message::Message,
    network::Network,
    session::Session,
};

use crate::{
    server_errors::ServerError,
    socket::{inform_client, inform_network, write_socket},
};

use super::command_utils::read_lock_clients;

// DCC command structure
// :client_who_request DCC command_type client_requested client_ip client_port

pub fn handle_dcc_command(
    message: Message,
    _nickname: String,
    session: &Session,
    network: &Network,
    server_name: &String,
) -> Result<(), ServerError> {
    println!("[DEBUG] DCC CHAT REQUEST");

    let dcc_message = DccMessage::deserialize(Message::deserialize(message)?)?;
    let local_clients = read_lock_clients(session)?;

    // match dcc_message.command {
    //     DccMessageType::Chat => {
    //         let client_requested = dcc_message.parameters[0].to_owned();
    //         if let Some(client) = local_clients.get(&client_requested) {
    //             if client.connected {
    //                 inform_client(session, &client_requested, &DccMessage::serialize(dcc_message)?)?;
    //             } else {
    //                 let response = format!("DCC CLOSE {} NotConnected", client_requested);
    //                 let ip = dcc_message.parameters[1].to_owned();
    //                 let port = dcc_message.parameters[2].to_owned();
    //                 thread::sleep(Duration::from_millis(500));
    //                 let arc_dcc_socket = Arc::new(TcpStream::connect(format!("{}:{}", ip, port))?);
    //                 write_socket(arc_dcc_socket, &response)?;
    //             }
    //         } else {
    //             inform_network(network, server_name, &DccMessage::serialize(dcc_message)?)?;
    //         }
    //     }
    //     _ => {}
    // }

    if dcc_message.command == DccMessageType::Chat {
        let client_requested = dcc_message.parameters[0].to_owned();
        if let Some(client) = local_clients.get(&client_requested) {
            if client.connected {
                inform_client(
                    session,
                    &client_requested,
                    &DccMessage::serialize(dcc_message)?,
                )?;
            } else {
                let response = format!("DCC CLOSE {} NotConnected", client_requested);
                let ip = dcc_message.parameters[1].to_owned();
                let port = dcc_message.parameters[2].to_owned();
                thread::sleep(Duration::from_millis(500));
                let arc_dcc_socket = Arc::new(TcpStream::connect(format!("{}:{}", ip, port))?);
                write_socket(arc_dcc_socket, &response)?;
            }
        } else {
            inform_network(network, server_name, &DccMessage::serialize(dcc_message)?)?;
        }
    }

    drop(local_clients);

    Ok(())
}
