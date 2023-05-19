use gtk::prelude::*;
use std::sync::mpsc::Sender;

/// Sends a message to the server to fetch the neccessary information for the gui, such as the list of channels and topics and the list of users.
pub fn fetch_information(tx: Sender<String>, error_modal: &gtk::Window) {
    match tx.send("NAMES".to_string()) {
        Ok(_) => (),
        Err(_) => {
            error_modal.show();
        }
    }
    match tx.send("LIST".to_string()) {
        Ok(_) => (),
        Err(_) => {
            error_modal.show();
        }
    }
}

/// Sends the MODE message to the server to change the given mode.
/// # Arguments
/// * `tx` - The channel to send the message to the server.
/// * `mode` - The mode and value to change.
/// * `error_modal` - The modal window that is opened when there is an error.
/// * `channel` - The channel to change the mode in.
pub fn send_mode_message(tx: Sender<String>, error_modal: &gtk::Window, channel: &str, mode: &str) {
    let message = format!("MODE {} {}", channel, mode);
    match tx.send(message) {
        Ok(_) => (),
        Err(_) => {
            error_modal.show();
        }
    }
}
