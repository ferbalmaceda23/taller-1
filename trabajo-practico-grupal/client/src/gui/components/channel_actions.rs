use gtk::{
    glib::{self, clone},
    prelude::*,
};
use std::sync::mpsc::Sender;

use super::actions::fetch_information;

/// Contains the channel actions.
/// The channel actions are the buttons that are shown when the user clicks on the channel name.
/// # Fields
/// * `tx` - The channel to send messages to the server.
/// * `channel_box_actions` - The box that contains the channel actions buttons
/// * `error_modal` - The modal that shows an error message.
pub struct ChannelActions {
    tx: Sender<String>,
    error_modal: gtk::Window,
    channel_box_actions: gtk::Box,
}

impl ChannelActions {
    /// Creates a new channel actions.
    /// # Arguments
    /// * `builder` - The builder that contains the widgets and builds the application.
    /// * `tx` - The channel to send messages to the server.
    pub fn new(tx: Sender<String>, builder: &gtk::Builder) -> Self {
        let error_modal = builder.object("error_modal").unwrap();
        let channel_box_actions = builder.object::<gtk::Box>("channel_actions").unwrap();

        ChannelActions {
            tx,
            error_modal,
            channel_box_actions,
        }
    }

    /// Gives all the channel button actions the correct functionality.
    /// The channel actions are JOIN, PART, INVITE, TOPIC and MODE.
    pub fn build(&self, builder: &gtk::Builder) {
        Self::active_join_button(self, builder);
        Self::active_invite_button(self, builder);
        Self::active_topic_button(self, builder);
        Self::active_part_button(self, builder);
        Self::active_mode_button(self, builder);
        self.channel_box_actions.set_visible(false);
    }

    /// Gives the JOIN button the correct functionality.
    /// When the user clicks on the JOIN button, it opens a modal window where the user can enter the channel name and the password if needed.
    fn active_join_button(&self, builder: &gtk::Builder) {
        let join_button = builder.object::<gtk::Button>("join_button").unwrap();
        let join_modal_button = builder.object::<gtk::Button>("join_modal_button").unwrap();
        let join_modal = builder.object::<gtk::Window>("join_modal").unwrap();
        let join_channel_entry = builder.object::<gtk::Entry>("join_channel_entry").unwrap();
        let pass_channel_entry = builder.object::<gtk::Entry>("pass_channel_entry").unwrap();
        let error_join = builder.object::<gtk::Label>("error_join").unwrap();
        let tx_clone = self.tx.clone();

        join_modal.connect_delete_event(move |_win, _| _win.hide_on_delete());

        join_button.connect_clicked(clone!( @weak join_modal=> move |_| {
            join_modal.show();
        }));

        join_modal_button.connect_clicked(
            clone!(@weak join_modal, @weak join_channel_entry, @weak pass_channel_entry, @weak error_join, @weak self.error_modal as error_modal=> move |_| {
                let channel = join_channel_entry.text();
                let pass = pass_channel_entry.text();
                if !channel.starts_with('#') && !channel.starts_with('&') {
                    error_join.set_text("Channel name must start with # or &");
                    join_channel_entry.set_text("");
                    pass_channel_entry.set_text("");
                }
                else{
                    let message = format!("JOIN {} {}", channel, pass);
                    match tx_clone.send(message){
                        Ok(_) => {
                            fetch_information(tx_clone.clone(), &error_modal);
                            error_join.set_text("");
                            join_channel_entry.set_text("");
                            pass_channel_entry.set_text("");
                            join_modal.hide();
                        },
                        Err(_) => error_join.set_text("Error sending message"),
                    }
                }
            })
        );
    }

    /// Gives the INVITE button the correct functionality.
    /// When the user clicks on the INVITE button, it opens a modal window where the user can enter the user name.
    /// The user name is the user that will be invited to the channel selected.
    fn active_invite_button(&self, builder: &gtk::Builder) {
        let invite_button = builder.object::<gtk::Button>("invite_button").unwrap();
        let invite_modal_button = builder
            .object::<gtk::Button>("invite_modal_button")
            .unwrap();
        let invite_modal = builder.object::<gtk::Window>("invite_modal").unwrap();
        let invite_nick_entry = builder.object::<gtk::Entry>("invite_modal_entry").unwrap();
        let tx_clone = self.tx.clone();
        let error_invite = builder.object::<gtk::Label>("error_invite").unwrap();
        let current_chat = builder.object::<gtk::Label>("current_chat").unwrap();

        invite_modal.connect_delete_event(move |_win, _| _win.hide_on_delete());

        invite_button.connect_clicked(clone!( @weak invite_modal=> move |_| {
            invite_modal.show();
        }));

        invite_modal_button.connect_clicked(
            clone!(@weak invite_modal, @weak invite_nick_entry, @weak current_chat, @weak self.error_modal as error_modal => move |_| {
                let nick = invite_nick_entry.text();
                let channel = current_chat.text();
                let message = format!("INVITE {} {}", nick, channel);
                match tx_clone.send(message) {
                    Ok(_) => {
                        fetch_information(tx_clone.clone(), &error_modal);
                        invite_modal.hide();
                    },
                    Err(e) => {
                        error_invite.set_text(&e.to_string());
                    }
                }
                invite_modal.hide();
            })
        );
    }

    /// Gives the TOPIC button the correct functionality.
    /// When the user clicks on the TOPIC button, it opens a modal window where the user can enter the topic.
    /// The topic will be set to the selected channel
    fn active_topic_button(&self, builder: &gtk::Builder) {
        let topic_button = builder.object::<gtk::Button>("topic_button").unwrap();
        let topic_modal_button = builder.object::<gtk::Button>("topic_modal_button").unwrap();
        let topic_modal = builder.object::<gtk::Window>("topic_modal").unwrap();
        let topic_entry = builder.object::<gtk::Entry>("topic_modal_entry").unwrap();
        let current_chat = builder.object::<gtk::Label>("current_chat").unwrap();
        let tx_clone = self.tx.clone();

        topic_modal.connect_delete_event(move |_win, _| _win.hide_on_delete());

        topic_button.connect_clicked(clone!( @weak topic_modal=> move |_| {
            topic_modal.show();
        }));

        topic_modal_button.connect_clicked(
            clone!(@weak topic_modal, @weak topic_entry, @weak current_chat, @weak self.error_modal as error_modal=> move |_| {
                let topic = topic_entry.text();
                let channel = current_chat.text();

                let message = format!("TOPIC {} :{}", channel, topic);
                match tx_clone.send(message){
                    Ok(_) => {
                        topic_modal.hide();
                        fetch_information(tx_clone.clone(), &error_modal)
                    },
                    Err(e) => {
                        println!("{}", e);
                    }
                }
            })
        );
    }

    /// Gives the PART button the correct functionality.
    /// When the user clicks on the PART button, it sends the server a PART message, it will make the user leave the selected channel.
    fn active_part_button(&self, builder: &gtk::Builder) {
        let part_button = builder.object::<gtk::Button>("part_button").unwrap();
        let current_chat = builder.object::<gtk::Label>("current_chat").unwrap();
        let stack_channels_info: gtk::Stack = builder.object("stack_channels_info").unwrap();
        let channel_box_actions = builder.object::<gtk::Box>("channel_actions").unwrap();
        let tx_clone = self.tx.clone();

        part_button.connect_clicked(
            clone!(@weak current_chat, @weak self.error_modal as error_modal, @weak stack_channels_info, @weak channel_box_actions => move |_| {
                let channel = current_chat.text();
                let message = format!("PART {}", channel);
                if tx_clone.send(message).is_ok()  {
                        stack_channels_info.set_visible_child_name("empty_channel");
                        channel_box_actions.set_visible(false);
                        fetch_information(tx_clone.clone(), &error_modal);
                }
            }),
        );
    }

    /// Gives the MODE button the correct functionality.
    /// When the user clicks on the MODE button, it opens a modal window where the user can see and change the channel mode.
    /// The mode will be set to the selected channel.
    fn active_mode_button(&self, builder: &gtk::Builder) {
        let channel_mode_button = builder
            .object::<gtk::Button>("channel_mode_button")
            .unwrap();
        let current_chat = builder.object::<gtk::Label>("current_chat").unwrap();
        let mode_modal = builder.object::<gtk::Window>("mode_channel_modal").unwrap();

        mode_modal.connect_delete_event(move |_win, _| _win.hide_on_delete());

        let tx_clone = self.tx.clone();
        channel_mode_button.connect_clicked(clone!(@weak mode_modal=> move |_| {
                let channel = current_chat.text();
                match tx_clone.send(format!("MODE {}", channel)){
                    Ok(_) => {
                        mode_modal.show();
                    },
                    Err(_) => {
                        mode_modal.show();
                    }
                }
        }));
    }

    /// Hides the channel action buttons when a channel is not selected.
    pub fn hide(&self) {
        self.channel_box_actions.set_visible(false);
    }

    /// Shows the channel action buttons when a channel is selected.
    pub fn show(&self) {
        self.channel_box_actions.set_visible(true);
    }
}
