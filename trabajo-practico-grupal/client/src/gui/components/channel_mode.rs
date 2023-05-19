use gtk::{
    glib::{self, clone},
    prelude::*,
};
use std::{collections::HashMap, sync::mpsc::Sender};

use super::actions::send_mode_message;

/// The channel mode window.
/// Creates the window that shows and allows the user to change the channel mode.
/// # Fields
/// * `tx` - The channel to send messages to the server.
/// * `mode_channel_label` - The label that shows the channel name.
/// * `channel_mode_box` - The box that contains the channel mode buttons.
/// * `current_chat` - The current chat that the user is in.
/// * `private_switch` - The switch that allows the user to change the channel mode to private.
/// * `secret_switch` - The switch that allows the user to change the channel mode to secret.
/// * `moderated_switch` - The switch that allows the user to change the channel mode to moderated.
/// * `invite_only_switch` - The switch that allows the user to change the channel mode to invite only.
/// * `topic_settable_switch` - The switch that allows the user to change the channel mode to no external messages.
/// * `no_messages_switch` - The switch that allows the user to change the channel mode to no external messages.
/// * `limmit_label` - The label that shows the current limit of the channel.
/// * `mode_hash` - A hash map that contains the current channel mode.
/// * `confirm_button` - The button that sends the new channel mode to the server.
/// * `error_modal` - The modal window that is opened when there is an error.
/// * `mode_channel_modal` - The modal window that is opened when the user clicks on the channel mode button.
pub struct ChannelMode {
    tx: Sender<String>,
    channel_mode_box: gtk::Box,
    mode_channel_label: gtk::Label,
    current_chat: gtk::Label,
    private_switch: gtk::Switch,
    secret_switch: gtk::Switch,
    invite_only_switch: gtk::Switch,
    topic_settable_switch: gtk::Switch,
    no_messages_switch: gtk::Switch,
    moderated_switch: gtk::Switch,
    limmit_label: gtk::Label,
    mode_hash: HashMap<String, String>,
    confirm_button: gtk::Button,
    error_modal: gtk::Window,
    mode_channel_modal: gtk::Window,
}

impl ChannelMode {
    /// Creates a new channel mode window to show the channel mode and let the users see it and edit it.
    /// # Arguments
    /// * `builder` - The builder that contains the widgets.
    /// * `tx` - The channel to send messages to the server.
    pub fn new(builder: &gtk::Builder, tx: Sender<String>) -> Self {
        let mode_channel_label: gtk::Label = builder.object("mode_channel_label").unwrap();
        let channel_mode_box: gtk::Box = builder.object("channel_mode_box").unwrap();
        let private_switch: gtk::Switch = builder.object("private_switch").unwrap();
        let secret_switch: gtk::Switch = builder.object("secret_switch").unwrap();
        let invite_only_switch: gtk::Switch = builder.object("invite_only_switch").unwrap();
        let topic_settable_switch: gtk::Switch = builder.object("topic_settable_switch").unwrap();
        let moderated_switch: gtk::Switch = builder.object("moderated_switch").unwrap();
        let no_messages_switch: gtk::Switch =
            builder.object("no_messages_from_outside_switch").unwrap();
        let confirm_button: gtk::Button = builder.object("mode_modal_channel_button").unwrap();
        let error_modal: gtk::Window = builder.object("error_modal").unwrap();
        let limmit_label: gtk::Label = builder.object("user_channel_limit").unwrap();
        let current_chat: gtk::Label = builder.object("current_chat").unwrap();
        let mode_channel_modal: gtk::Window = builder.object("mode_channel_modal").unwrap();

        ChannelMode {
            tx,
            channel_mode_box,
            mode_channel_label,
            current_chat,
            private_switch,
            secret_switch,
            invite_only_switch,
            topic_settable_switch,
            moderated_switch,
            no_messages_switch,
            limmit_label,
            mode_hash: HashMap::new(),
            confirm_button,
            error_modal,
            mode_channel_modal,
        }
    }

    /// Builds the change limmit and the cange channel password buttons.
    pub fn build(&mut self, builder: &gtk::Builder) {
        self.build_change_limit(builder);
        self.build_change_password(builder);
        self.active_confirm_button();
    }

    /// Builds the change limmit button. Opens a modal window when the user clicks on it and sends the new limit to the server.
    /// # Arguments
    /// * `builder` - The builder that contains the widgets.
    pub fn build_change_limit(&mut self, builder: &gtk::Builder) {
        let change_limmit_modal: gtk::Window = builder.object("change_limmit_modal").unwrap();
        let change_limmit_button: gtk::Button = builder.object("channel_limmit_button").unwrap();
        let change_limmit_entry: gtk::Entry = builder.object("channel_limmit_entry").unwrap();
        let current_chat: gtk::Label = builder.object("current_chat").unwrap();
        let change_limmit_confirm_button: gtk::Button =
            builder.object("change_limmit_confirm_button").unwrap();
        let error_modal: gtk::Window = builder.object("error_modal").unwrap();
        let error_limmit_label: gtk::Label = builder.object("error_limmit_label").unwrap();

        change_limmit_modal.connect_delete_event(move |_win, _| _win.hide_on_delete());

        change_limmit_button.connect_clicked(clone!(@weak change_limmit_modal => move |_| {
            change_limmit_modal.show();
        }));

        let tx_clone = self.tx.clone();
        change_limmit_confirm_button.connect_clicked(
            clone!(@weak change_limmit_modal, @weak change_limmit_entry, @weak error_modal, @weak current_chat,
                 @weak error_limmit_label => move |_| {
                let channel = current_chat.text().to_string();
                let limmit = change_limmit_entry.text().to_string().parse::<i32>().unwrap_or(-1);
                println!("LIMMIT: {}", limmit);
                if limmit < 0 {
                    error_limmit_label.set_text("Limmit must be a positive number");
                }
                else {
                    send_mode_message(tx_clone.clone(), &error_modal, &channel, &format!("+l {}", limmit));
                    error_limmit_label.set_text("");
                    change_limmit_modal.hide();
                }
            }
        )
        );
    }

    /// Builds the change password button. Opens a modal window when the user clicks on it and sends the new password to the server.
    /// # Arguments
    /// * `builder` - The builder that contains the widgets.
    fn build_change_password(&self, builder: &gtk::Builder) {
        let channel_pass_modal: gtk::Window = builder.object("channel_pass_modal").unwrap();
        let channel_pass_entry: gtk::Entry = builder.object("channel_pass_entry").unwrap();
        let channel_pass_button: gtk::Button = builder.object("channel_pass_button").unwrap();
        let change_channel_pass_button: gtk::Button =
            builder.object("change_channel_pass_button").unwrap();
        let error_modal: gtk::Window = builder.object("error_modal").unwrap();
        let current_chat: gtk::Label = builder.object("current_chat").unwrap();

        let tx = self.tx.clone();
        change_channel_pass_button.connect_clicked(clone!(@weak channel_pass_modal => move |_| {
            channel_pass_modal.show();
        }));

        channel_pass_button.connect_clicked(
            clone!(@weak channel_pass_modal, @weak channel_pass_entry, @weak error_modal=> move |_| {
                let channel = current_chat.text().to_string();
                let new_pass = channel_pass_entry.text().to_string();
                send_mode_message(tx.clone(), &error_modal, &channel, &format!("+k {}", new_pass));
                channel_pass_modal.hide();
                channel_pass_entry.set_text("");
            })
        );
    }

    /// Sets the current channel mode and updates the GUI, setting the switches with the correct values and the limmit label.
    /// # Arguments
    /// * `modes` - The current channel mode.
    /// * `channel` - The channel name.
    pub fn update_channel_modes(&mut self, channel: String, modes: HashMap<String, String>) {
        self.mode_channel_label
            .set_text(&format!("Change configuration for {}", channel));
        self.mode_hash = modes.clone();
        self.set_switch_state(&modes, &self.private_switch, "Private");
        self.set_switch_state(&modes, &self.secret_switch, "Secret");
        self.set_switch_state(&modes, &self.invite_only_switch, "InviteOnly");
        self.set_switch_state(
            &modes,
            &self.topic_settable_switch,
            "TopicSettableOnlyOperators",
        );
        self.set_switch_state(&modes, &self.moderated_switch, "ModeratedChannel");
        self.set_switch_state(&modes, &self.no_messages_switch, "NoMessageFromOutside");
        self.limmit_label
            .set_text(modes.get("UserLimit").unwrap_or(&"-".to_string()));

        self.channel_mode_box.show_all();
    }

    /// Sets the state of a switch, corresponding to the mode.
    /// # Arguments
    /// * `modes` - The current channel mode.
    /// * `switch` - The switch to set.
    /// * `key` - The mode to check.
    fn set_switch_state(&self, modes: &HashMap<String, String>, switch: &gtk::Switch, key: &str) {
        let value = match modes.get(key) {
            Some(value) => value == "+",
            None => false,
        };
        switch.set_active(value);
    }

    /// Builds the confirm button. Sends the new mode to the server when the user clicks on it.
    fn active_confirm_button(&self) {
        let tx = self.tx.clone();
        self.confirm_button.connect_clicked(
            clone!(@weak self.private_switch as private_switch, @weak self.secret_switch as secret_switch, @weak self.invite_only_switch as invite_only_switch,
            @weak self.topic_settable_switch as topic_settable_switch, @weak self.mode_channel_modal as mode_modal,
             @weak self.no_messages_switch as no_messages_switch, @weak self.error_modal as error_modal,
             @weak self.moderated_switch as moderated_switch, @weak self.current_chat as current_chat => move |_| {
                let channel = current_chat.text();
                let private = if private_switch.is_active() {"+p"} else {"-p"};
                let secret = if secret_switch.is_active() {"+s"} else {"-s"};
                let invite_only = if invite_only_switch.is_active() {"+i"} else {"-i"};
                let topic_settable_only_operators = if topic_settable_switch.is_active() {"+t"} else {"-t"};
                let no_message_from_outside = if no_messages_switch.is_active() {"+n"} else {"-n"};
                let moderated = if moderated_switch.is_active() {"+m"} else {"-m"};
                send_mode_message(tx.clone(), &error_modal, &channel, private);
                send_mode_message(tx.clone(),&error_modal, &channel, secret);
                send_mode_message(tx.clone(),&error_modal, &channel, invite_only);
                send_mode_message(tx.clone(),&error_modal, &channel, topic_settable_only_operators);
                send_mode_message(tx.clone(),&error_modal, &channel, moderated);
                send_mode_message(tx.clone(),&error_modal, &channel, no_message_from_outside);
                match tx.send(format!("MODE {}", channel)){
                    Ok(_) => mode_modal.hide(),
                    Err(_) => {

                        error_modal.show();
                    }
                }
            })
        );
    }

    /// Returns the opers list.
    pub fn get_opers(&self) -> Vec<String> {
        let mut opers_vec = Vec::new();
        let default = String::new();
        let operators = self.mode_hash.get("ChannelOperator").unwrap_or(&default);
        let opers = operators.split(',');
        for oper in opers {
            opers_vec.push(oper.to_string());
        }
        opers_vec
    }

    /// Returns the moderators list.
    pub fn get_moderators(&self) -> Vec<String> {
        let mut moderators_vec = Vec::new();
        let default = String::new();
        let moderators = self
            .mode_hash
            .get("SpeakInModeratedChannel")
            .unwrap_or(&default);
        let mods = moderators.split(',');
        for moderator in mods {
            moderators_vec.push(moderator.to_string());
        }
        moderators_vec
    }
}
