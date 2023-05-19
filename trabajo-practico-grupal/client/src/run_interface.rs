use gtk::glib::clone;
use gtk::Application;
use gtk::{glib, prelude::*};
use model::client_errors::ClientError;
use model::dcc::{DccMessage, DccMessageType};
use model::responses::dcc::DccResponse;
use model::responses::errors::ErrorResponse;
use model::responses::replies::CommandResponse;
use model::responses::response::Response;
use model::socket::{read_socket, write_socket};
use std::collections::HashMap;
use std::net::{TcpListener, TcpStream};
use std::ops::ControlFlow;
use std::sync::mpsc::Sender;
use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::{Arc, RwLock};
use std::thread;

use crate::dcc_commands::close::close_all_dcc_connections;
use crate::dcc_commands::dcc_management::{
    manage_dcc_request_from_client, manage_dcc_request_from_current_client,
};
use crate::gui::controller::send_to_screen;
use crate::gui::screens::chats_screen::ChatsScreen;
use crate::gui::screens::connection_screen::ConnectionScreen;
use crate::gui::screens::registration_screen::RegistrationScreen;

/// This function creates a new thread that will handle the connection to the server.
/// The main thread will handle the GUI, creating the main application.
/// It will create the channels that will be used to communicate between the client thread and the GUI.
/// It will also create de builder from a glade file that will be used to build the GUI.
/// Returns a ClientError in case of error

pub fn client_run_interface() {
    let (tx_view, rx_cliente): (Sender<String>, Receiver<String>) = std::sync::mpsc::channel();
    let (tx_connection, rx_connection): (
        gtk::glib::Sender<Response>,
        gtk::glib::Receiver<Response>,
    ) = gtk::glib::MainContext::channel(gtk::glib::PRIORITY_DEFAULT);

    let (tx_registration, rx_registration): (
        gtk::glib::Sender<Response>,
        gtk::glib::Receiver<Response>,
    ) = gtk::glib::MainContext::channel(gtk::glib::PRIORITY_DEFAULT);

    let (tx_chats, rx_chats): (gtk::glib::Sender<Response>, gtk::glib::Receiver<Response>) =
        gtk::glib::MainContext::channel(gtk::glib::PRIORITY_DEFAULT);

    let dcc_interface_communication = HashMap::<String, SyncSender<String>>::new();
    let arc_dcc_interface_communication = Arc::new(RwLock::new(dcc_interface_communication));
    let arc_dcc_interface_communication_clone = arc_dcc_interface_communication.clone();

    thread::spawn(move || {
        run_client(
            rx_cliente,
            tx_connection,
            tx_registration,
            tx_chats,
            arc_dcc_interface_communication_clone,
        )
    });

    gtk::init().expect("Failed to initialize GTK.");

    let builder = gtk::Builder::from_file("client/src/gui/irc.glade");
    let app = Application::builder().application_id("irc").build();

    let connection = ConnectionScreen::new(tx_view.clone());
    let register = RegistrationScreen::new(tx_view.clone());
    let chats = ChatsScreen::new(tx_view);
    connection.build(&builder, rx_connection);
    register.build(&builder, rx_registration);
    chats.build(&builder, rx_chats, arc_dcc_interface_communication);

    app.connect_activate(clone!(@weak builder => move |app| {
        let window: gtk::Window = (builder).object("main_window").unwrap();
        window.set_application(Some(app));
        window.show_all();
    }));

    app.run();
}

/// This function will handle the connection to the server.
/// It will receive the address from the GUI thread and will try to connect to the server.
/// If the connection is successful, it will send a ConnectionSuccess message to the GUI thread.
/// If the connection fails, it will send an ErrorWhileConnecting message to the GUI thread.
/// It will be the function responsible for handling the communication between the server and the GUI.
fn run_client(
    rx: Receiver<String>,
    tx_connection: gtk::glib::Sender<Response>,
    tx_registration: gtk::glib::Sender<Response>,
    tx_chats: gtk::glib::Sender<Response>,
    arc_dcc_interface_communication: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
) -> Result<(), ClientError> {
    let socket = match connect_to_server(&rx, &tx_connection) {
        Ok(value) => {
            let response = Response::CommandResponse {
                response: CommandResponse::ConnectionSuccees,
            };
            send_to_screen(
                tx_connection.clone(),
                tx_registration.clone(),
                tx_chats.clone(),
                response,
            );
            value
        }
        Err(_) => {
            let response = Response::ErrorResponse {
                response: ErrorResponse::ErrorWhileConnecting,
            };
            send_to_screen(tx_connection, tx_registration, tx_chats, response);
            return Err(ClientError::ErrorWhileConnecting);
        }
    };

    let arc_socket = Arc::new(socket);
    let arc_socket_reader = arc_socket.clone();

    // channels que se van a comunicar con cada thread correspondiente a una conexion DCC
    let dcc_connections = HashMap::<String, SyncSender<String>>::new();
    let arc_dcc_connections = Arc::new(RwLock::new(dcc_connections));
    let arc_dcc_connections_clone = arc_dcc_connections.clone();

    let tx_chats_clone = tx_chats.clone();
    let arc_dcc_interface_communication_clone = arc_dcc_interface_communication.clone();
    // recibe mensajes de server y se lo manda a la interfaz
    thread::spawn(move || loop {
        if let Ok(msg) = read_socket(arc_socket_reader.clone()) {
            if let ControlFlow::Break(_) = read_from_server(
                msg,
                tx_connection.clone(),
                tx_registration.clone(),
                tx_chats_clone.clone(),
                arc_dcc_connections_clone.clone(),
                arc_dcc_interface_communication_clone.clone(),
            ) {
                continue;
            }
        }
    });

    loop {
        read_from_interface(
            &rx,
            arc_socket.clone(),
            tx_chats.clone(),
            arc_dcc_connections.clone(),
            arc_dcc_interface_communication.clone(),
        )?;
    }
}

/// This function will receive messagges from the GUI and then send them to the server
/// It will return ClientError::ErrorWhileConnectingWithInterface if there a is a problem receiving a message from the GUI.
/// It will also return an error if it can't send the message to the server.
/// It will return Ok(()) if the message was sent successfully.
fn read_from_interface(
    rx: &Receiver<String>,
    arc_socket: Arc<TcpStream>,
    tx_chats: gtk::glib::Sender<Response>,
    dcc_connections: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
    arc_dcc_interface_communication: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
) -> Result<(), ClientError> {
    match rx.recv() {
        Ok(msg) => {
            println!("[DEBUG] Mensaje recibido de la interfaz: {msg}");
            // chequeamos si es dcc
            // si es dcc, nos fijamos si es CHAT
            // si es CHAT creamos un thread que va a manejar el nuevo chat p2p con el server
            // el thread va a tener que recibir los mensajes que lleguen desde acá mediante un channel
            // mismo el thread va a tener que informar a la interfaz de cambios mediante OTRO channel
            if msg.starts_with("QUIT ") {
                close_all_dcc_connections(dcc_connections);
                return Ok(());
            }
            if let Ok(dcc_msg) = DccMessage::deserialize(msg.clone()) {
                if dcc_msg.command == DccMessageType::Chat && dcc_msg.prefix.is_some() {
                    // first DCC CHAT command case
                    let ip = dcc_msg.parameters[1].to_owned();
                    let port = dcc_msg.parameters[2].to_owned();
                    if check_address(ip, port) {
                        write_socket(arc_socket, &msg)?;
                    } else {
                        let sender = dcc_msg.parameters[0].to_owned();
                        let response = Response::DccResponse {
                            response: DccResponse::ChatAddressErrorResponse { sender },
                        };
                        if tx_chats.send(response).is_ok() {};
                        return Ok(());
                    }
                }
                manage_dcc_request_from_current_client(
                    dcc_msg,
                    dcc_connections,
                    tx_chats,
                    arc_dcc_interface_communication,
                )?;
            } else {
                write_socket(arc_socket, &msg)?;
            }
        }
        Err(_) => {
            println!("Error al recibir mensaje");
            return Err(ClientError::ErrorWhileConnectingWithInterface);
        }
    }
    Ok(())
}

/// This function will read from the server, parse the message and send it to the GUI.
/// It will return ControlFlow::Break if an error ocurred while reading from the server or while parsing the message.
/// It will return ControlFlow::Continue if the message was parsed and sent correctly.
fn read_from_server(
    msg: String,
    tx_connection: glib::Sender<Response>,
    tx_registration: glib::Sender<Response>,
    tx_chats: glib::Sender<Response>,
    dcc_connections: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
    arc_dcc_interface_communication: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
) -> ControlFlow<()> {
    if let Ok(dcc_msg) = DccMessage::deserialize(msg.clone()) {
        if manage_dcc_request_from_client(
            dcc_msg,
            dcc_connections,
            tx_chats,
            arc_dcc_interface_communication,
        )
        .is_ok()
        {}
    } else {
        let response = match Response::serialize(msg) {
            Some(value) => value,
            None => {
                println!("[ERROR] Error parsing response");
                return ControlFlow::Break(());
            }
        };
        send_to_screen(tx_connection, tx_registration, tx_chats, response);
    }

    ControlFlow::Continue(())
}

/// This function will make the connection between the server and the client.
/// It will receive the address from the GUI thread and will try to connect to the server.
/// If the connection is successful, it will send a ConnectionSuccess message to the GUI thread.
/// If the connection fails, it will send an ErrorWhileConnecting message to the GUI thread.
/// It will return the TcpStream if the connection was successful.
/// It will return an error if the connection failed.
fn connect_to_server(
    rx: &Receiver<String>,
    tx: &glib::Sender<Response>,
) -> Result<TcpStream, ClientError> {
    let socket: TcpStream = match rx.recv() {
        Ok(address) => {
            println!("Conectándome a {address:?}");
            match TcpStream::connect(&address) {
                Ok(socket) => socket,
                Err(_) => {
                    match tx.send(Response::ErrorResponse {
                        response: ErrorResponse::ErrorWhileConnecting,
                    }) {
                        Ok(_) => {}
                        Err(_) => {
                            println!("Error al enviar mensaje");
                            return Err(ClientError::ErrorWhileConnectingWithInterface);
                        }
                    }
                    return Err(ClientError::ErrorWhileConnecting);
                }
            }
        }
        Err(_) => {
            println!("Error al recibir dirección");
            return Err(ClientError::ErrorWhileConnectingWithInterface);
        }
    };
    Ok(socket)
}

/// Returns true if the ip:port address is valid, false otherwise.
pub fn check_address(ip: String, port: String) -> bool {
    TcpListener::bind(format!("{ip}:{port}")).is_ok()
}
