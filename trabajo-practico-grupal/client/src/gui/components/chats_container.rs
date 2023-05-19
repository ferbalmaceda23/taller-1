use std::sync::mpsc::Sender;

use gtk::{
    glib::{self, clone},
    prelude::*,
};

use crate::gui::{
    messages_box::{message_received_box, message_sent_box},
    utils::{adjust_scroll_to_bottom, new_conversation},
};

/// This struct is used to store the widgets that are used to display the chat
/// messages.
/// # Fields
/// * `scrolled_window`: The scrolled window that contains the messages.
/// * `stack_conversations`: The stack of conversations.
///
pub struct ChatsContainer {
    scrolled_window: gtk::ScrolledWindow,
    stack_conversations: gtk::Stack,
    send_button: gtk::Button,
    current_chat: gtk::Label,
    message_entry: gtk::Entry,
    dcc_button: gtk::Button,
    close_dcc_button: gtk::Button,
    chat_button: gtk::Button,
    file_chooser_button: gtk::FileChooserButton,
}

impl ChatsContainer {
    /// Creates a new `ChatsContainer` struct.
    /// # Arguments
    /// * `builder`: The builder of the glade file that builds the application.
    pub fn new(builder: &gtk::Builder) -> Self {
        let scrolled_window = builder
            .object::<gtk::ScrolledWindow>("scrolled_window")
            .unwrap();
        let dcc_button = builder.object::<gtk::Button>("dcc_button").unwrap();
        let chat_button = builder.object::<gtk::Button>("chat_button").unwrap();
        let current_chat = builder.object::<gtk::Label>("current_chat").unwrap();
        let file_chooser_button: gtk::FileChooserButton = builder
            .object::<gtk::FileChooserButton>("file_chooser_button")
            .unwrap();
        let stack_conversations = builder.object::<gtk::Stack>("conversation_stack").unwrap();
        let builder = builder.clone();
        let send_button = builder.object::<gtk::Button>("send_message").unwrap();
        let message_entry = builder.object::<gtk::Entry>("chat_input").unwrap();
        let close_dcc_button = builder.object::<gtk::Button>("close_dcc_button").unwrap();

        ChatsContainer {
            scrolled_window,
            current_chat,
            stack_conversations,
            send_button,
            message_entry,
            dcc_button,
            close_dcc_button,
            chat_button,
            file_chooser_button,
        }
    }

    /// Builds the send button and the message entry.
    /// # Arguments
    /// * `tx`: The sender of the channel that sends the messages to the server.
    /// * `builder`: The builder of the glade file that builds the application.
    pub fn build(&self, builder: &gtk::Builder, tx: Sender<String>) {
        self.active_send_button(builder, tx);
    }

    /// Adds a new message to the corresponding conversation stack, depending on the channel, showing it on the left side of the conversation.
    /// # Arguments
    /// * `sender`: The name of the sender.
    /// * `message`: The message that was received.
    /// * `channel`: The channel that the message was sent to.
    pub fn add_message_channel_received(&self, channel: String, sender: String, message: String) {
        let message_box =
            message_received_box(format!("{}: {}", sender, message), "message_received");
        Self::add_message_to_screen(self, channel, message_box, "Channel conversations");
    }

    /// Adds a new message to the corresponding user stack, showing it on the left side of the conversation.
    /// # Arguments
    /// * `sender`: The name of the sender.
    /// * `message`: The message that was received.
    pub fn add_message_received(&self, sender: String, message: String, stack_name: &str) {
        println!("El message received es {}", message);
        let message_box: gtk::Box = message_received_box(message, "message_received");
        Self::add_message_to_screen(self, sender, message_box, stack_name);
    }

    /// Adds a new message to the corresponding conversation stack, depending on the channel or user,
    ///  showing it on the right side of the conversation.
    /// # Arguments
    /// * `message_box`: The message box that contains the message.
    /// * `sender`:  The name of the sender.
    pub fn add_message_to_screen(&self, sender: String, message_box: gtk::Box, stack_name: &str) {
        println!("El stack name es {}", stack_name);
        let stack_visible_conversations = self
            .stack_conversations
            .child_by_name(stack_name)
            .unwrap()
            .downcast::<gtk::Stack>()
            .unwrap();
        let box_conversation;
        if stack_visible_conversations.child_by_name(&sender).is_none() {
            box_conversation = new_conversation(&sender);
            stack_visible_conversations.add_named(&box_conversation, &sender);
        } else {
            box_conversation = stack_visible_conversations
                .child_by_name(&sender)
                .unwrap()
                .downcast::<gtk::Box>()
                .unwrap();
        }
        box_conversation.add(&message_box);
        box_conversation.show_all();
        stack_visible_conversations.show_all();
        adjust_scroll_to_bottom(&self.scrolled_window);
    }

    /// Gives functionality to the send button, sending the message to the server and adding the message on the screen if its on chat mode.
    /// If it is in DCC mode it sends the message to the corresponding client.
    /// If the file sender button has a file, it opens the modal that asks the ip and port to send it.
    /// # Arguments
    /// * `tx`: The sender of the channel that sends the messages to the server.
    /// * `builder`: The builder of the glade file that builds the application.
    pub fn active_send_button(&self, builder: &gtk::Builder, tx: Sender<String>) {
        let send_button = builder.object::<gtk::Button>("send_message").unwrap();
        let message_entry: gtk::Entry = builder.object::<gtk::Entry>("chat_input").unwrap();
        let receiver_label: gtk::Label = builder.object::<gtk::Label>("current_chat").unwrap();
        let stack_conversations = builder.object::<gtk::Stack>("conversation_stack").unwrap();
        let file_chooser_button: gtk::FileChooserButton = builder
            .object::<gtk::FileChooserButton>("file_chooser_button")
            .unwrap();

        let ip_port_dcc_modal_file = builder
            .object::<gtk::Window>("ip_port_dcc_modal_file")
            .unwrap();

        message_entry.connect_activate(clone!(@weak send_button => move |_| {
            send_button.emit_clicked();
        }));

        send_button.connect_clicked(
            clone!(@weak message_entry, @weak ip_port_dcc_modal_file, @weak builder, @weak file_chooser_button,  @weak self.scrolled_window as scrolled_window, @weak stack_conversations, @weak receiver_label => move |_| {
                if stack_conversations.child_by_name("Loadings").unwrap() != stack_conversations.visible_child().unwrap() && stack_conversations.child_by_name("No conversation").unwrap() != stack_conversations.visible_child().unwrap() {
                    adjust_scroll_to_bottom(&scrolled_window);
                    let message = message_entry.text();
                    if !message.is_empty() {
                        let style = if stack_conversations.child_by_name("DCC conversations").unwrap() == stack_conversations.visible_child().unwrap() {
                            match tx.send(format!("DCC CHAT {} {}", receiver_label.text(), message)){
                                Ok(_) => {
                                    "command_sent"
                                },
                                Err(_) => {
                                    "error_message"
                                }
                            }
                        } else {
                            match tx.send(format!("PRIVMSG {} {}", receiver_label.text(), message)){
                                Ok(_) => {
                                    "command_sent"
                                },
                                Err(_) => {
                                    "error_message"
                                }
                            }
                        };

                        let message_box: gtk::Box = message_sent_box(message.to_string(), style);
                        message_box.set_widget_name(&message);
                        message_entry.set_text("");
                        let stack_visible = stack_conversations.visible_child().unwrap().downcast::<gtk::Stack>().unwrap();
                        let chats_view = stack_visible.visible_child().unwrap().downcast::<gtk::Box>().unwrap();
                        chats_view.add(&message_box);
                        chats_view.show_all();
                    }
                }
                if file_chooser_button.file().is_some() && stack_conversations.child_by_name("DCC conversations").unwrap() == stack_conversations.visible_child().unwrap() {
                    ip_port_dcc_modal_file.show();
                }
            }
        )
        );

        send_button.set_sensitive(false);
    }

    pub fn set_no_channel_selected_screen(&self) {
        self.close_dcc_button.set_visible(false);
        self.file_chooser_button.set_visible(false);
        self.dcc_button.set_visible(false);
        self.chat_button.set_visible(false);
        self.send_button.set_sensitive(false);
        self.message_entry.set_sensitive(false);
        self.stack_conversations
            .set_visible_child_name("No conversation");
        self.current_chat.set_text("");
    }

    /// Removes the last message sent by the user, it is used in the case of an error.
    /// # Arguments
    /// * `sender`: The name of the sender.
    pub fn remove_last_message(&self, sender: String) {
        let visisble_stack = self
            .stack_conversations
            .visible_child()
            .unwrap()
            .downcast::<gtk::Stack>()
            .unwrap();
        let box_conversation = visisble_stack
            .child_by_name(&sender)
            .unwrap()
            .downcast::<gtk::Box>()
            .unwrap();
        let children = box_conversation.children();
        let last_message = children.last().unwrap();
        box_conversation.remove(last_message);
        box_conversation.show_all();
    }
}
