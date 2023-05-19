use std::{collections::HashMap, sync::mpsc::Sender};

use gtk::{
    glib::{self, clone},
    prelude::*,
};

use crate::gui::utils::new_conversation;

/// Struct that contains the widgets of the names list, which is the list of
/// channels and users.
/// # Fields
/// * `tx`: The sender of the channel that sends the messages to the server.
/// * `channels_list `: The list of channels.
/// * `users_list`: The list of users.
/// * `current_chat`: The label containing the name of the current chat.
/// * `clients_hash`: A hashmap that maps the name of the channel to the list of
/// users in that channel.
/// * `stack_conversations`: The stack of conversations.
/// * `send_button`: The button that sends the chat message.
/// * `channel_actions`: The box that containes the channel actions menu.
/// * `stack_channels_info`: The stack that contains the channels info.
pub struct NamesList {
    tx: Sender<String>,
    channels_list: gtk::Box,
    users_list: gtk::Box,
    current_chat: gtk::Label,
    clients_hash: HashMap<String, Vec<String>>,
    stack_conversations: gtk::Stack,
    send_button: gtk::Button,
    channel_actions: gtk::Box,
    stack_channels_info: gtk::Stack,
    dcc_button: gtk::Button,
    chat_button: gtk::Button,
    file_send_button: gtk::FileChooserButton,
    dcc_close_button: gtk::Button,
    message_entry: gtk::Entry,
}

impl NamesList {
    /// Creates a new `NamesList` struct.
    /// # Arguments
    /// * `builder`: The builder of the glade file that builds the application.
    /// * `tx`: The sender of the channel that sends the messages to the server.
    pub fn new(builder: &gtk::Builder, tx: Sender<String>) -> Self {
        let channels_list = builder.object::<gtk::Box>("channels_list").unwrap();
        let users_list = builder.object::<gtk::Box>("users_list").unwrap();
        let current_chat = builder.object::<gtk::Label>("current_chat").unwrap();
        let send_button = builder.object::<gtk::Button>("send_message").unwrap();
        let stack_conversations = builder.object::<gtk::Stack>("conversation_stack").unwrap();
        let channel_actions = builder.object::<gtk::Box>("channel_actions").unwrap();
        let stack_channels_info = builder.object::<gtk::Stack>("stack_channels_info").unwrap();
        let dcc_button = builder.object::<gtk::Button>("dcc_button").unwrap();
        let chat_button = builder.object::<gtk::Button>("chat_button").unwrap();
        let file_send_button = builder
            .object::<gtk::FileChooserButton>("file_chooser_button")
            .unwrap();
        let dcc_close_button = builder.object::<gtk::Button>("close_dcc_button").unwrap();
        let message_entry = builder.object::<gtk::Entry>("chat_input").unwrap();

        NamesList {
            tx,
            channels_list,
            users_list,
            current_chat,
            clients_hash: HashMap::new(),
            stack_conversations,
            send_button,
            channel_actions,
            stack_channels_info,
            dcc_button,
            chat_button,
            file_send_button,
            dcc_close_button,
            message_entry,
        }
    }

    /// Updates the client hash map. This is used to keep track of the users in
    /// each channel.
    pub fn update_clients(&mut self, names: Vec<String>, channel: String) {
        self.clients_hash.insert(channel, names);
    }

    /// Adds all channels and users to the list of channels and users on the left side of the
    /// application, then it connects the signals of the channels and users and clears the client hash map.
    pub fn add_names_to_list(&mut self) {
        self.clean_list();
        let hash_clone = self.clients_hash.clone();
        let mut clients_shown = vec![];
        for (channel, names) in hash_clone {
            for name in names {
                if clients_shown.contains(&name) {
                    continue;
                }
                clients_shown.push(name.clone());

                self.create_user_button(name);
            }
            self.users_list.show_all();
            if channel == "*" {
                continue;
            }
            let button_channel = self.create_channel_button(channel);
            self.channels_list.set_child(Some(&button_channel));
            self.channels_list.show_all();
        }
        self.clients_hash.clear();
    }

    /// Creates a button for a user, then it connects the signal of the button.
    /// When the user button is clicked it creates a new conversation with that user or it shows the conversation
    /// if it already exists.
    fn create_user_button(&mut self, name: String) {
        let button = gtk::Button::with_label(&name);
        button.connect_clicked(
            clone!(@weak self.chat_button as chat_button,@weak self.file_send_button as file_send_button, @weak self.stack_channels_info as stack_channels_info, @weak self.dcc_button as dcc_button, @weak self.send_button as send_button,  @weak self.current_chat as current, @weak self.channel_actions as channel_actions, @weak self.stack_conversations as stack_conversations, @weak self.dcc_close_button as dcc_close_button, @weak self.message_entry as message_entry => move |_| {
                current.set_label(&name);
                dcc_close_button.set_visible(false);
                dcc_button.set_visible(true);
                chat_button.set_visible(false);
                file_send_button.set_visible(false);
                message_entry.set_sensitive(true);

                let stack_user_conversations = stack_conversations
                .child_by_name("User conversations")
                .unwrap()
                .downcast::<gtk::Stack>()
                .unwrap();

                if stack_user_conversations.child_by_name(&name).is_none() {
                    let box_conversation = new_conversation(&name);
                    stack_user_conversations.add_named(&box_conversation, &name);
                }
                send_button.set_sensitive(true);
                stack_user_conversations.set_visible_child_name(&name.to_string());
                stack_user_conversations.show_all();

                stack_conversations.set_visible_child_name("User conversations");
                stack_conversations.show_all();

                channel_actions.set_visible(false);
                stack_channels_info.set_visible_child_name("empty_channel");
            })
        );
        self.users_list.set_child(Some(&button));
    }

    /// Creates a button for a channel, then it connects the signal of the button.
    /// When the channel button is clicked it creates a new conversation with that channel or it shows the conversation
    /// if it already exists. It also sets the channel actions menu visible.
    fn create_channel_button(&mut self, channel: String) -> gtk::Button {
        let button_channel = gtk::Button::with_label(&channel);
        let tx_clone = self.tx.clone();
        button_channel.connect_clicked(
            clone!(@weak self.chat_button as chat_button, @weak self.file_send_button as file_send_button, @weak self.channel_actions as channel_actions, @weak self.dcc_button as dcc_button, @weak self.send_button as send_button, @weak self.stack_conversations as stack_conversations, @weak self.current_chat as current, @weak self.stack_channels_info as stack_channels_info, @weak self.dcc_close_button as dcc_close_button => move |_| {
                current.set_label(&channel);
                dcc_close_button.set_visible(false);
                dcc_button.set_visible(false);
                chat_button.set_visible(false);
                file_send_button.set_visible(false);

                let stack_channel_conversations = stack_conversations
                .child_by_name("Channel conversations")
                .unwrap()
                .downcast::<gtk::Stack>()
                .unwrap();

                if stack_channel_conversations.child_by_name(&channel).is_none() {
                    let box_conversation = new_conversation(&channel);
                    stack_channel_conversations.add_named(&box_conversation, &channel);
                }
                stack_channel_conversations.set_visible(true);
                if tx_clone.send(format!("MODE {}", channel)).is_ok() {}
                stack_channel_conversations.set_visible_child_name(&channel.to_string());
                stack_channel_conversations.show_all();
                channel_actions.set_visible(true);
                stack_conversations.set_visible_child_name("Channel conversations");
                stack_conversations.show_all();
                send_button.set_sensitive(true);
                stack_channels_info.set_visible_child_name(&channel.to_string());
                stack_channels_info.show_all();
            })
        );
        button_channel
    }

    /// Removes all channels and users from the list of channels and users on the left side of the
    /// application.
    fn clean_list(&mut self) {
        self.users_list.foreach(|widget| {
            self.users_list.remove(widget);
        });
        self.channels_list.foreach(|widget| {
            self.channels_list.remove(widget);
        });
    }
}
