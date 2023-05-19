use gtk::{
    glib::{self, clone},
    prelude::*,
};
use std::{collections::HashMap, sync::mpsc::Sender};

use super::actions::send_mode_message;

/// The user mode window.
/// Creates the window that shows and allows the user to change their user mode.
/// The user mode window is a modal window that is opened when the user clicks on the user mode button.
/// # Fields
/// * `tx` - The channel to send messages to the server.
/// * `mode_user_modal` - The modal window that is opened when the user clicks on the user mode button.
/// * `nick_label`- The label that shows the user's nick.
/// * `invisible_switch` - The switch that allows the user to change their invisible mode.
/// * `wallops_switch` - The switch that allows the user to change their wallops mode.
/// * `server_messages_switch` - The switch that allows the user to change their server messages mode.
pub struct UserMode {
    tx: Sender<String>,
    mode_user_modal: gtk::Window,
    nick_label: gtk::Label,
    invisible_switch: gtk::Switch,
    wallops_switch: gtk::Switch,
    server_messages_switch: gtk::Switch,
}

impl UserMode {
    /// Creates a new user mode window.
    pub fn new(builder: &gtk::Builder, tx: Sender<String>) -> Self {
        let mode_user_modal: gtk::Window = builder.object("mode_user_modal").unwrap();
        let nick_label: gtk::Label = builder.object("user_nick").unwrap();
        let invisible_switch: gtk::Switch = builder.object("invisible_switch").unwrap();
        let wallops_switch: gtk::Switch = builder.object("wallops_switch").unwrap();
        let server_messages_switch: gtk::Switch = builder.object("server_messages_switch").unwrap();

        UserMode {
            tx,
            mode_user_modal,
            nick_label,
            invisible_switch,
            wallops_switch,
            server_messages_switch,
        }
    }

    /// Builds the moder user buttons, giving them the correct functionality. One of them opens the
    /// user mode modal with the current user mode information and the other one sends the new user mode to the server.
    pub fn build(&mut self, builder: &gtk::Builder) {
        self.build_user_mode_button(builder);
        self.build_user_mode_modal_button(builder);
    }

    /// Builds the user mode button, giving it the correct functionality. It opens the user mode modal and calls the
    /// MODE message to the server so it shows the user's current mode.
    fn build_user_mode_button(&mut self, builder: &gtk::Builder) {
        let mode_user_button: gtk::Button = builder.object("mode_user_button").unwrap();
        let error_modal: gtk::Window = builder.object("error_modal").unwrap();
        let mode_user_modal: gtk::Window = builder.object("mode_user_modal").unwrap();

        mode_user_button.connect_clicked(
            clone!(@weak mode_user_modal, @weak error_modal => move |_| {
                mode_user_modal.show();
            }),
        );
    }

    /// Builds the user mode modal button, giving it the correct functionality. It sends the new user mode to the server.
    /// It also closes the user mode modal.
    fn build_user_mode_modal_button(&mut self, builder: &gtk::Builder) {
        let mode_user_modal_button: gtk::Button = builder.object("mode_user_modal_button").unwrap();
        let mode_user_modal: gtk::Window = builder.object("mode_user_modal").unwrap();
        let error_modal: gtk::Window = builder.object("error_modal").unwrap();

        let tx = self.tx.clone();
        mode_user_modal_button.connect_clicked(clone!(@weak mode_user_modal, @weak self.invisible_switch as invisible_switch,
            @weak self.wallops_switch as wallops_switch, @weak self.server_messages_switch as server_messages_switch,
            @weak error_modal, @weak self.nick_label as nick_label => move |_| {
                let nick = nick_label.text().to_string();
                let invisible = if invisible_switch.is_active() {"+i"} else {"-i"};
                let wallops = if wallops_switch.is_active() {"+w"} else {"-w"};
                let server_messages = if server_messages_switch.is_active() {"+s"} else {"-s"};
                send_mode_message(tx.clone(), &error_modal, &nick, invisible);
                send_mode_message(tx.clone(),&error_modal, &nick, wallops);
                send_mode_message(tx.clone(),&error_modal, &nick, server_messages);
                mode_user_modal.hide();
        }));
    }

    /// Updates the user mode modal with the user's current mode, setting the switches to the correct value.
    pub fn update_user_modes(&mut self, modes: HashMap<String, String>) {
        self.set_switch_state(&modes, &self.invisible_switch, "Invisible");
        self.set_switch_state(&modes, &self.wallops_switch, "Wallops");
        self.set_switch_state(&modes, &self.server_messages_switch, "ServerNotice");

        self.mode_user_modal.show();
    }

    /// Sets the switch state to the correct value.
    /// # Arguments
    /// * `modes` - The user's current modes.
    /// * `switch` - The switch to set the state.
    /// * `key` - The mode to check if it is active.
    fn set_switch_state(&self, modes: &HashMap<String, String>, switch: &gtk::Switch, key: &str) {
        let value = match modes.get(key) {
            Some(value) => value == "+",
            None => false,
        };
        switch.set_active(value);
    }
}
