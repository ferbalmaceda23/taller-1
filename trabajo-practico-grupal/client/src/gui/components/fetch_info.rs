use gtk::prelude::*;
use std::sync::mpsc::Sender;

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
    // match tx.send("MODE".to_string()) {
    //     Ok(_) => (),
    //     Err(_) => {
    //         error_modal.show();
    //     }
    // }
}
