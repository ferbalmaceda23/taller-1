use gtk::glib::clone;
use gtk::{glib, prelude::*};
use model::responses::errors::ErrorResponse;
use model::responses::replies::CommandResponse;
use model::responses::response::Response;
use std::sync::mpsc::Sender;

///Struct that represents the connection screen.
/// This screen will be displayed when the client is not connected to the server.
/// It will allow the user to enter the ip and port of the server and connect to it.
/// # Fields
/// * `tx` - The sender that is used to send messages to the server
pub struct ConnectionScreen {
    tx: Sender<String>,
}

impl ConnectionScreen {
    ///Creates a new instance of the connection screen.
    /// # Arguments
    /// * `tx` - The sender that is used to send messages to the server
    pub fn new(tx: Sender<String>) -> Self {
        Self { tx }
    }

    ///Builds the connection screen, it sets the callbacks for the buttons and the receiver for the messages.
    /// # Arguments
    /// * `builder` - The builder used to build the screen
    /// * `rx` - The receiver used to receive messages from the client that where sent by the server
    pub fn build(self, builder: &gtk::Builder, rx: glib::Receiver<Response>) {
        let connect_button = builder.object::<gtk::Button>("connect_button").unwrap();
        let port_entry: gtk::Entry = builder.object("port_entry").unwrap();
        let ip_entry: gtk::Entry = builder.object("ip_entry").unwrap();
        let stack: gtk::Stack = builder.object("stack").unwrap();
        let error_label: gtk::Label = builder.object("error_connection").unwrap();

        let tx_clone = self.tx;
        connect_button.connect_clicked(
            clone!(@weak port_entry, @weak ip_entry, @weak stack, @weak error_label=> move |_| {
                let port = port_entry.text();
                let ip = ip_entry.text();

                if port.is_empty() || ip.is_empty() {
                    error_label.set_text("Please fill all the fields");
                    println!("Please fill out all fields");
                }
                else {
                    let address = format!("{ip}:{port}");
                    match tx_clone.send(address){
                        Ok(_) => {},
                        Err(_) => error_label.set_text("Error sending connection info"),
                    }
                }
            }),
        );

        rx.attach(None, move |message| {
            match message {
                Response::CommandResponse {
                    response: CommandResponse::ConnectionSuccees,
                } => stack.set_visible_child_name("Registration"),

                Response::ErrorResponse {
                    response: ErrorResponse::ErrorWhileConnecting,
                } => stack.set_visible_child_name("Error"),
                _ => (),
            }

            glib::Continue(true)
        });
    }
}
