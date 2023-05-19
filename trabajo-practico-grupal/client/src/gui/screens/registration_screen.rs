use gtk::glib::clone;
use gtk::{glib, prelude::*};
use model::responses::errors::ErrorResponse;
use model::responses::replies::CommandResponse;
use model::responses::response::Response;
use std::sync::mpsc::Sender;

use crate::gui::components::actions::fetch_information;
///Struct that is used to build the registration screen.
/// This screen will be displayed when the client is connected and will be used to register a new user or to login an existing user.
/// # Fields
/// * `tx` - The sender that is used to send messages to the server
pub struct RegistrationScreen {
    tx: Sender<String>,
}

impl RegistrationScreen {
    ///Creates the registration screen struct
    /// # Arguments
    /// * `tx` - The sender that is used to send messages to the server
    pub fn new(tx: Sender<String>) -> Self {
        Self { tx }
    }
    ///Builds the registration and login screens
    /// # Arguments
    /// * `builder` - The builder used to build the screen
    /// * `rx` - The receiver used to receive messages from the client
    pub fn build(self, builder: &gtk::Builder, rx: glib::Receiver<Response>) {
        Self::active_register_section(builder, self.tx.clone());
        Self::active_login_section(builder, self.tx.clone());

        let error_login: gtk::Label = builder.object("error_login").unwrap();
        let error_registration: gtk::Label = builder.object("error_registration").unwrap();
        let user_nick: gtk::Label = builder.object("user_nick").unwrap();
        let error_modal: gtk::Window = builder.object("error_modal").unwrap();

        let stack = builder.object::<gtk::Stack>("stack").unwrap();

        rx.attach(None, move |message| {
            println!("message: {message}");
            match message {
                Response::ErrorResponse { response } => match response {
                    ErrorResponse::NoNicknameGiven => {
                        error_registration.set_text("No nickname given");
                        error_login.set_text("No nickname given");
                    }
                    ErrorResponse::AlreadyRegistered { nickname } => {
                        error_registration
                            .set_text(&format!("{nickname} is already registered, please login"));
                    }
                    ErrorResponse::NickInUse { nickname } => {
                        error_registration.set_text(&format!(
                            "Nickname {nickname} is already in use, please pick another one"
                        ));
                        error_login.set_text(&format!("{nickname} is already connected"));
                    }
                    ErrorResponse::NotRegistered => {
                        error_login.set_text("Invalid credentials");
                    }
                    _ => println!("Error de registracion: {response}"),
                },
                Response::CommandResponse {
                    response:
                        CommandResponse::Welcome {
                            nickname,
                            username: _,
                            hostname: _,
                        },
                } => {
                    user_nick.set_text(&nickname);
                    fetch_information(self.tx.clone(), &error_modal);
                    stack.set_visible_child_name("Chats room");
                }
                _ => (),
            }

            glib::Continue(true)
        });
    }

    /// Sends the registration or login information to the server
    /// # Arguments
    /// * `tx` - The sender used to send messages to the client which then sends it to the server
    /// * `error_label` - The label used to display errors to the user
    /// * `message` - The message to be sent to the server
    fn send_message(message: String, error_label: &gtk::Label, tx: Sender<String>) {
        match tx.send(message) {
            Ok(_) => (),
            Err(_) => error_label.set_text("Something went wrong, please try again later"),
        }
    }

    /// Builds the registration section, giving it the functionality to send the registration information to the server
    /// # Arguments
    /// * `builder` - The builder used to build the screen
    /// * `tx` - The sender used to send messages to the client which then sends it to the server
    fn active_register_section(builder: &gtk::Builder, tx: Sender<String>) {
        let register_button = builder.object::<gtk::Button>("register_button").unwrap();

        let server_entry: gtk::Entry = builder.object("servername_entry").unwrap();
        let nick_entry: gtk::Entry = builder.object("nick_entry").unwrap();
        let username_entry: gtk::Entry = builder.object("user_entry").unwrap();
        let realname_entry: gtk::Entry = builder.object("realname_entry").unwrap();
        let hostname_entry: gtk::Entry = builder.object("hostname_entry").unwrap();
        let password_entry: gtk::Entry = builder.object("pass_entry").unwrap();
        let error_registration: gtk::Label = builder.object("error_registration").unwrap();
        let error_login: gtk::Label = builder.object("error_login").unwrap();
        let go_to_login_button = builder.object::<gtk::Button>("go_to_login").unwrap();
        let stack_registration: gtk::Stack = builder.object("stack_registration").unwrap();

        register_button.connect_clicked(
            clone!(@weak nick_entry, @weak username_entry, @weak realname_entry, @weak hostname_entry, @weak error_registration, @weak password_entry=> move |_| {
                let nick = nick_entry.text();
                let password = password_entry.text();
                let username = username_entry.text();
                let realname = realname_entry.text();
                let hostname = hostname_entry.text();
                let servername = server_entry.text();
                if nick.is_empty() || username.is_empty() || realname.is_empty() || hostname.is_empty() || servername.is_empty() {
                    error_registration.set_text("Please fill out all fields");
                }
                else {
                    let pass = format!("PASS {password}");
                    let nick_msg = format!("NICK {nick}" );
                    let user_msg = format!("USER {username} {hostname} {servername} :{realname}");
                    Self::send_message(pass, &error_registration, tx.clone());
                    Self::send_message(user_msg, &error_registration, tx.clone());
                    Self::send_message(nick_msg, &error_registration, tx.clone());
                }
            })
        );

        go_to_login_button.connect_clicked(
            clone!(@weak stack_registration, @weak error_login => move |_| {
                error_login.set_text("");
                stack_registration.set_visible_child_name("Login");
            }),
        );
    }

    /// Builds the login section, giving it the functionality to send the login information to the server
    /// # Arguments
    /// * `builder` - The builder used to build the screen
    /// * `tx` - The sender used to send messages to the client which then sends it to the server
    fn active_login_section(builder: &gtk::Builder, tx: Sender<String>) {
        let login_button = builder.object::<gtk::Button>("login_button").unwrap();

        let nick_entry: gtk::Entry = builder.object("nick_entry_login").unwrap();
        let password_entry: gtk::Entry = builder.object("pass_entry_login").unwrap();
        let error_login: gtk::Label = builder.object("error_login").unwrap();
        let error_registration: gtk::Label = builder.object("error_registration").unwrap();
        let stack_registration: gtk::Stack = builder.object("stack_registration").unwrap();

        login_button.connect_clicked(
            clone!(@weak nick_entry, @weak password_entry, @weak error_login => move |_| {
                let nick = nick_entry.text();
                let password = password_entry.text();
                if nick.is_empty() {
                    error_login.set_text("Nick is required");
                }
                  else {
                    let pass = format!("PASS {password}");
                    let nick_msg = format!("NICK {nick}" );
                    Self::send_message(pass, &error_login, tx.clone());
                    Self::send_message(nick_msg, &error_login, tx.clone());
                }
            }
            ),
        );

        let go_to_register_button = builder.object::<gtk::Button>("go_to_register").unwrap();
        go_to_register_button.connect_clicked(
            clone!(@weak stack_registration, @weak error_registration => move |_| {
                error_registration.set_text("");
                stack_registration.set_visible_child_name("Registration");
            }),
        );
    }
}
