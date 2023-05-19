use gtk::{
    gio,
    glib::{self, clone},
    prelude::*,
};
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::mpsc::SyncSender;
use std::sync::Arc;
use std::sync::RwLock;

use crate::gui::{
    components::file_message::add_file_message_box,
    utils::{adjust_scroll_to_bottom, new_conversation},
};

/// Contains the DCC feature and builds the modals and buttons realted to this functionality
/// # Fields
/// * `builder` - The builder that contains the widgets.
/// * `tx` - The channel to send messages to the client.
/// * `chat_button` - The button to switch from DCC conversation to normal conversation.
/// * `close_dcc_button` - The button to close the DCC conversation and delete the DCC chat.
/// * `stack_conversations` - The stack that contains the DCC conversation, loadings and the normal conversations.
/// * `current_chat` - The label that contains the name of the current chat.
/// * `dcc_button` - The button to open the DCC modal to start a new DCC connection.
/// * `message_entry` - The entry to write the message to send to the DCC or normal chat.
/// * `file_chooser_button` - The button to choose the file to send to the DCC chat.
/// * `send_button` - The button to send the message or the file to the DCC or normal chat.
pub struct DCCFeature {
    builder: gtk::Builder,
    tx: Sender<String>,
    chat_button: gtk::Button,
    close_dcc_button: gtk::Button,
    stack_conversations: gtk::Stack,
    current_chat: gtk::Label,
    dcc_button: gtk::Button,
    message_entry: gtk::Entry,
    file_chooser_button: gtk::FileChooserButton,
    send_button: gtk::Button,
    communication_hash: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
}

impl DCCFeature {
    /// Creates a new DCCFeature.
    /// # Arguments
    /// * `builder` - The builder that contains the widgets.
    /// * `tx` - The channel to send messages to the client.
    pub fn new(
        builder: &gtk::Builder,
        tx: Sender<String>,
        communication_hash: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
    ) -> Self {
        let close_dcc_button = builder.object::<gtk::Button>("close_dcc_button").unwrap();
        let chat_button = builder.object::<gtk::Button>("chat_button").unwrap();
        let stack_conversations = builder.object::<gtk::Stack>("conversation_stack").unwrap();
        let current_chat: gtk::Label = builder.object::<gtk::Label>("current_chat").unwrap();
        let dcc_button = builder.object::<gtk::Button>("dcc_button").unwrap();
        let message_entry = builder.object::<gtk::Entry>("chat_input").unwrap();
        let send_button = builder.object::<gtk::Button>("send_message").unwrap();
        let file_chooser_button = builder
            .object::<gtk::FileChooserButton>("file_chooser_button")
            .unwrap();
        let builder = builder.clone();

        DCCFeature {
            builder,
            tx,
            chat_button,
            close_dcc_button,
            stack_conversations,
            current_chat,
            dcc_button,
            message_entry,
            send_button,
            file_chooser_button,
            communication_hash,
        }
    }

    /// Builds the DCC feature widgets, activating the buttons and modals related to this functionality.
    /// # Arguments
    /// * `builder` - The builder that contains the widgets.
    /// * `tx` - The channel to send messages to the client.
    pub fn build(&self, builder: &gtk::Builder, tx: Sender<String>) {
        self.active_dcc_button(builder, tx.clone());
        self.active_dcc_close_button(builder, tx.clone());
        self.active_chat_button(builder);
        self.build_dcc_confirmation_modal(builder, tx.clone());
        self.build_dcc_file_send_modal(builder, tx.clone());
        self.build_dcc_confirmation_file_modal(builder, tx.clone());
        self.build_dcc_ip_port_resume_modal(builder, tx);
    }

    /// Actives the DCC button, giving it the functionality to open the DCC modal.
    /// It also gives functionality to the DCC modal that asks the user for the ip and port to make the connection.
    /// # Arguments
    /// * `builder` - The builder that contains the widgets.
    /// * `tx` - The channel to send messages to the client.
    pub fn active_dcc_button(&self, builder: &gtk::Builder, tx: Sender<String>) {
        let dcc_button = builder.object::<gtk::Button>("dcc_button").unwrap();
        let connect_dcc_button = builder.object::<gtk::Button>("connect_dcc_button").unwrap();
        let receiver_label: gtk::Label = builder.object::<gtk::Label>("current_chat").unwrap();
        let ip_port_dcc_modal = builder.object::<gtk::Window>("ip_port_dcc_modal").unwrap();
        let ip_dcc_entry = builder.object::<gtk::Entry>("ip_dcc_entry").unwrap();
        let port_dcc_entry = builder.object::<gtk::Entry>("port_dcc_entry").unwrap();
        let loading_screens = builder.object::<gtk::Stack>("loading_stack").unwrap();
        let error_ip_port_dcc = builder.object::<gtk::Label>("error_ip_port_dcc").unwrap();
        let dcc_stack = builder.object::<gtk::Stack>("dcc_conversations").unwrap();
        let user_nick = builder.object::<gtk::Label>("user_nick").unwrap();
        let error_modal = builder.object::<gtk::Window>("error_modal").unwrap();
        let error_label = builder.object::<gtk::Label>("error_label").unwrap();
        let file_chooser_button = builder
            .object::<gtk::FileChooserButton>("file_chooser_button")
            .unwrap();

        ip_port_dcc_modal.connect_delete_event(move |_win, _| _win.hide_on_delete());

        dcc_button.connect_clicked(
            clone!(@weak builder, @weak error_modal, @weak error_label, @weak self.current_chat as current_chat, @weak user_nick, @weak self.close_dcc_button as dcc_close_button, @weak self.chat_button as chat_button, @weak self.message_entry as message_entry, @weak self.dcc_button as dcc_button, @weak loading_screens, @weak file_chooser_button, @weak receiver_label, @weak ip_port_dcc_modal, @weak dcc_stack, @weak self.stack_conversations as stack_conversations => move |_| {
                let current_chat_text = current_chat.text();
                let user_nick_text = user_nick.text();
                if current_chat_text == user_nick_text{
                    error_label.set_text("You can't send a DCC to yourself");
                    error_modal.show();
                }else   if dcc_stack.child_by_name(&receiver_label.text()).is_none() && loading_screens.child_by_name(&receiver_label.text()).is_none() {
                    ip_port_dcc_modal.show();
                    }
                    else if loading_screens.child_by_name(&receiver_label.text()).is_some(){
                        loading_screens.set_visible_child_name(&receiver_label.text());
                        loading_screens.show_all();
                        stack_conversations.set_visible_child_name("Loadings");
                    }
                    else{
                        println!("Ya existe la conversacion con {}", receiver_label.text());
                        dcc_stack
                        .child_by_name(&receiver_label.text())
                        .unwrap()
                        .downcast::<gtk::Box>()
                        .unwrap().show_all();
                        dcc_stack.show_all();
                        dcc_stack.set_visible_child_name(&receiver_label.text());
                        stack_conversations.set_visible_child_name("DCC conversations");
                        file_chooser_button.set_visible(true);
                        chat_button.set_visible(true);
                        dcc_button.set_visible(false);
                        dcc_close_button.set_visible(true);
                    }
                }
        )
        );

        connect_dcc_button.connect_clicked(
            clone!(@weak builder, @weak port_dcc_entry, @weak self.stack_conversations as stack_conversations, @weak self.close_dcc_button as dcc_close_button, @weak ip_dcc_entry, @weak dcc_stack, @weak user_nick, @weak receiver_label => move |_| {
                let ip = ip_dcc_entry.text();
                let port = port_dcc_entry.text();
                if ip.is_empty() || port.is_empty() {
                    error_ip_port_dcc.set_text("Please fill out all fields")
                }
                else{
                    let loading_stack = stack_conversations
                        .child_by_name("Loadings")
                        .unwrap()
                        .downcast::<gtk::Stack>()
                        .unwrap();
                    let sender = receiver_label.text();
                    if loading_stack.child_by_name(&sender).is_none() {
                        println!("[DEBUG] create_loading_screen");
                        println!("[DEBUG] {}", sender);
                        let loading_screen = gtk::Box::new(gtk::Orientation::Vertical, 0);
                        let label = gtk::Label::new(
                            Some(&format!("Waiting for {} to accept connection", sender))
                        );
                        let loading_icon = gtk::Spinner::new();
                        loading_icon.set_size_request(50, 50);
                        loading_icon.start();
                        loading_screen.add(&label);
                        loading_screen.add(&loading_icon);
                        loading_screen.show_all();
                        loading_stack.add_named(&loading_screen, &sender);
                    }
                    loading_stack.set_visible_child_name(&sender);
                    loading_stack.show_all();
                    stack_conversations.set_visible_child_name("Loadings");
                                let message = format!(":{} DCC CHAT {} {} {}", user_nick.text(), receiver_label.text(), ip, port);
                                println!("{}", message);
                                    if tx.send(message).is_ok(){
                                            ip_dcc_entry.set_text("");
                                            port_dcc_entry.set_text("");
                                            ip_port_dcc_modal.hide();
                                    }
                            }
            })
        );
    }

    /// Closes the dcc connection with the sender and removes the conversation box from the stack.
    /// # Arguments
    /// * `sender`: The name of the sender who we need to close the connection with.
    pub fn end_dcc_connection(&self, sender: String) {
        let tx_clone = self.tx.clone();
        if tx_clone.send(format!("DCC CLOSE {}", sender)).is_ok() {}
        self.remove_stack_box(sender, "DCC conversations");
    }

    /// Actives the close DCC button, giving it the functionality to close the DCC connection and delete the DCC
    /// conversation box with the current chat opened.
    /// # Arguments
    /// * `builder` - The builder that contains the widgets.
    /// * `tx` - The channel to send messages to the client.
    pub fn active_dcc_close_button(&self, builder: &gtk::Builder, tx: Sender<String>) {
        let current_chat = builder.object::<gtk::Label>("current_chat").unwrap();
        let close_dcc_chat_button = builder.object::<gtk::Button>("close_dcc_button").unwrap();

        let tx_clone = tx;
        close_dcc_chat_button.connect_clicked(
            clone!(@weak self.stack_conversations as stack_conversations, @weak current_chat, @weak self.send_button as send_button, @weak self.message_entry as message_entry, @weak self.file_chooser_button as file_chooser_button, @weak self.chat_button as chat_button, @weak close_dcc_chat_button => move |_| {
                if tx_clone.send(format!("DCC CLOSE {}", current_chat.text())).is_ok() && stack_conversations.visible_child_name().unwrap() == "DCC conversations" {
                    let stack = stack_conversations
                    .child_by_name("DCC conversations")
                    .unwrap()
                    .downcast::<gtk::Stack>()
                    .unwrap();
                    let deleted_box = stack.child_by_name(&current_chat.text()).unwrap().downcast::<gtk::Box>().unwrap();
                    stack.remove(&deleted_box);
                    stack_conversations.set_visible_child_name("No conversation");
                    file_chooser_button.set_visible(false);
                    chat_button.set_visible(false);
                    close_dcc_chat_button.set_visible(false);
                }
            })
        );
    }

    /// Activates the chat button, giving it the functionality to open the normal conversation box with the current chat opened
    /// and close the DCC conversation box.
    pub fn active_chat_button(&self, builder: &gtk::Builder) {
        let chat_button = builder.object::<gtk::Button>("chat_button").unwrap();
        let stack_conversations = builder.object::<gtk::Stack>("conversation_stack").unwrap();
        let receiver_label: gtk::Label = builder.object::<gtk::Label>("current_chat").unwrap();
        let dcc_button = builder.object::<gtk::Button>("dcc_button").unwrap();
        let user_conversations = builder.object::<gtk::Stack>("user_conversations").unwrap();
        let dcc_close_button = builder.object::<gtk::Button>("close_dcc_button").unwrap();
        let file_chooser_button = builder
            .object::<gtk::FileChooserButton>("file_chooser_button")
            .unwrap();
        chat_button.connect_clicked(
            clone!(@weak chat_button, @weak dcc_close_button, @weak user_conversations, @weak file_chooser_button, @weak dcc_button, @weak receiver_label, @weak stack_conversations => move |_| {
                if user_conversations.child_by_name(&receiver_label.text()).is_none() {
                    let new_box = new_conversation(&receiver_label.text().to_string());
                    user_conversations.add_named(&new_box, &receiver_label.text());
                }
                user_conversations.set_visible_child_name(&receiver_label.text());
                stack_conversations.set_visible_child_name("User conversations");
                chat_button.set_visible(false);
                dcc_button.set_visible(true);
                file_chooser_button.set_visible(false);
                dcc_close_button.set_visible(false);
            }
        )
        );
    }

    /// Builds the dcc confirmation modal, giving it the functionality to accept or decline a DCC connection.
    /// If the connection is accepted, it will open a new DCC conversation box.
    /// If the connection is declined, it will send a message to the client to close the connection.
    /// # Arguments
    /// * `builder` - The builder that contains the widgets.
    /// * `tx` - The channel to send messages to the client.
    pub fn build_dcc_confirmation_modal(&self, builder: &gtk::Builder, tx: Sender<String>) {
        let dcc_confirmation_dialog = builder.object::<gtk::Window>("dcc_confirmation").unwrap();
        let decline_dcc_button = builder.object::<gtk::Button>("decline_dcc_button").unwrap();
        let accept_dcc_button = builder.object::<gtk::Button>("accept_dcc_button").unwrap();
        let dcc_sender_label = builder.object::<gtk::Label>("dcc_sender_label").unwrap();
        let current_chat = builder.object::<gtk::Label>("current_chat").unwrap();
        let scrolled_window = builder
            .object::<gtk::ScrolledWindow>("scrolled_window")
            .unwrap();

        dcc_confirmation_dialog.connect_delete_event(move |_win, _| _win.hide_on_delete());

        let tx_clone = tx.clone();

        accept_dcc_button.connect_clicked(clone!(
                @weak dcc_confirmation_dialog,
                @weak scrolled_window,
                @weak self.dcc_button as dcc_button,
                @weak self.file_chooser_button as file_chooser,
                @weak self.chat_button as chat_button,
                @weak self.send_button as send_button,
                @weak self.message_entry as message_entry,
                @weak self.stack_conversations as stack_conversations,
                @weak dcc_sender_label,
                @weak self.close_dcc_button as dcc_close_button,
                @weak current_chat => move |_| {
                    let sender = dcc_sender_label.text().to_string();

            match tx_clone.send(format!("DCC ACCEPT {}", sender)){
                Ok(_) => {
                    dcc_confirmation_dialog.close();

                    send_button.set_sensitive(true);
                    message_entry.set_sensitive(true);
                    current_chat.set_text(&sender);
                    let dcc_stack = stack_conversations
                        .child_by_name("DCC conversations")
                        .unwrap()
                        .downcast::<gtk::Stack>()
                        .unwrap();
                    let box_conversation = new_conversation(&sender);
                    dcc_stack.add_named(&box_conversation, &sender);
                    dcc_stack.set_visible_child_name(&sender);
                    chat_button.set_visible(true);
                    file_chooser.set_visible(true);
                    dcc_button.set_visible(false);
                    dcc_close_button.set_visible(true);
                    stack_conversations.set_visible_child_name("DCC conversations");
                    adjust_scroll_to_bottom(&scrolled_window);
                },
                Err(_) => println!("Error sending accept"),
            }
        }));

        decline_dcc_button.connect_clicked(
            clone!( @weak dcc_confirmation_dialog, @weak dcc_sender_label=> move |_| {
                    let sender = dcc_sender_label.text().to_string();
                   let message = format!("DCC CLOSE {}", sender);
                   match tx.send(message){
                       Ok(_) => {
                        dcc_confirmation_dialog.close();
                       },
                       Err(_) => println!("Error sending decline"),
                   }

            }),
        );
    }

    /// Builds the dcc file send modal, giving it the functionality to send a file to another client.
    /// It will send the file to the client set in the modal sender label, and it has the ip and port entries so that the user
    /// can insert the ip and port on which the file will be sent.
    fn build_dcc_file_send_modal(&self, builder: &gtk::Builder, tx: Sender<String>) {
        let ip_port_dcc_modal_file = builder
            .object::<gtk::Window>("ip_port_dcc_modal_file")
            .unwrap();
        let ip_dcc_entry_file = builder.object::<gtk::Entry>("ip_dcc_entry_file").unwrap();
        let port_dcc_entry_file = builder.object::<gtk::Entry>("port_dcc_entry_file").unwrap();
        let send_dcc_file_button = builder
            .object::<gtk::Button>("send_dcc_file_button")
            .unwrap();
        let error_label = builder.object::<gtk::Label>("error_ip_port_dcc1").unwrap();

        ip_port_dcc_modal_file.connect_delete_event(move |_win, _| _win.hide_on_delete());

        send_dcc_file_button.connect_clicked(
            clone!(@weak builder, @weak ip_port_dcc_modal_file, @weak error_label, @weak self.file_chooser_button as file_chooser_button, @weak self.stack_conversations as stack_conversations, @weak self.current_chat as current_chat, @weak ip_dcc_entry_file, @weak port_dcc_entry_file => move |_| {
                let ip = ip_dcc_entry_file.text().to_string();
                let port = port_dcc_entry_file.text().to_string();

                if ip.is_empty() || port.is_empty() {
                    error_label.set_text("Please fill all the fields");
                }
                else if let Some(file) = file_chooser_button.file() {
                    let file_info = match file.query_info("standard::*", gio::FileQueryInfoFlags::NONE, gio::Cancellable::NONE) {
                        Ok(file_info) => file_info,
                        Err(_) => return,
                    };
                    println!("File info: {:?}", file_info);
                   let file_path=  match file.path() {
                        Some(path) => {
                            path.to_string_lossy().to_string()
                        },
                        None => return,
                    };
                    let receiver = current_chat.text();
                    let file_size = file_info.size();
                    let file_name = file_info.name().to_string_lossy().to_string();
                    match tx.send(format!("DCC SEND {} {} {} {} {}", receiver, file_path, ip, port, file_size)) {
                        Ok(_) => {
                            let message_box: gtk::Box = add_file_message_box(&builder, tx.clone(), receiver.to_string(), file_name.clone(), file_size.to_string(), "command_sent", format!("{}-{}", file_name, receiver));
                            let stack_visible = stack_conversations.visible_child().unwrap().downcast::<gtk::Stack>().unwrap();
                            let chats_view = stack_visible.visible_child().unwrap().downcast::<gtk::Box>().unwrap();
                            chats_view.add(&message_box);
                            chats_view.show_all();
                            file_chooser_button.unselect_all();
                            ip_port_dcc_modal_file.close();
                        },
                        Err(_) => {
                            error_label.set_text("Error sending DCC SEND command");
                        }
                    }
                }
            })
        );
    }

    /// Builds the dcc file transfer confirmation modal, giving it the functionality to accept or decline the file
    /// trfrom another client.
    fn build_dcc_confirmation_file_modal(&self, builder: &gtk::Builder, tx: Sender<String>) {
        let dcc_confirmation_file_modal = builder
            .object::<gtk::Window>("dcc_confirmation_file")
            .unwrap();
        let decline_file_transfer_button = builder
            .object::<gtk::Button>("decline_file_transfer_button")
            .unwrap();
        let accept_file_transfer_button = builder
            .object::<gtk::Button>("accept_file_transfer_button")
            .unwrap();
        let file_name_label = builder.object::<gtk::Label>("file_name_label").unwrap();
        let file_sender_label = builder.object::<gtk::Label>("file_sender_label").unwrap();
        let file_size_label = builder.object::<gtk::Label>("file_size_label").unwrap();
        dcc_confirmation_file_modal.connect_delete_event(move |_win, _| _win.hide_on_delete());

        accept_file_transfer_button.connect_clicked(
            clone!( @weak self.builder as builder, @weak dcc_confirmation_file_modal, @weak file_sender_label, @weak file_name_label, @weak self.stack_conversations as stack_conversations, @weak file_size_label, @weak self.communication_hash as communication_hash => move |_| {
                let sender = file_sender_label.text().to_string();
                let file_name = file_name_label.text().to_string();
                let file_size = file_size_label.text().to_string();
                let message = format!("DCC ACCEPT {} {}", sender, file_name_label.text());
                println!("Message to send to thread: {:?}", message);
                let lock_communication_hash = match communication_hash.as_ref().read(){
                    Ok(lock_communication_hash) => lock_communication_hash,
                    Err(_) => return,
                };
                println!("[DEBUG] communication hash locked from gui");
                let tx_sender = match lock_communication_hash.get(&sender) {
                    Some(tx) => tx,
                    None => return,
                };
                match tx_sender.send(message){
                    Ok(_) => {
                        println!("[DEBUG] accept message sent to thread");
                        let message_box: gtk::Box = add_file_message_box(
                            &builder,
                            tx.clone(),
                            sender.to_string(),
                            file_name.clone(),
                            file_size,
                            "message_received",
                            format!("{}-{}", file_name, sender)
                        );
                        let dcc_stack = stack_conversations
                            .child_by_name("DCC conversations")
                            .unwrap()
                            .downcast::<gtk::Stack>()
                            .unwrap();
                        let chats_view = dcc_stack.child_by_name(&sender).unwrap().downcast::<gtk::Box>().unwrap();
                        chats_view.add(&message_box);
                        chats_view.show_all();
                        dcc_confirmation_file_modal.close();
                    },
                    Err(e) => println!("Error sending accept: {e}"),
                }
                drop(lock_communication_hash);
            })
        );
        decline_file_transfer_button.connect_clicked(
            clone!( @weak dcc_confirmation_file_modal, @weak self.communication_hash as communication_hash, @weak file_sender_label, @weak file_name_label => move |_| {
                let sender = file_sender_label.text().to_string();
                let lock_communication_hash = match communication_hash.as_ref().read(){
                    Ok(lock_communication_hash) => lock_communication_hash,
                    Err(_) => return,
                };
                let tx_sender = match lock_communication_hash.get(&sender) {
                    Some(tx) => tx,
                    None => return,
                };
                let message = format!("DCC CLOSE {} {}", sender, file_name_label.text());
                match tx_sender.send(message){
                    Ok(_) => {
                        dcc_confirmation_file_modal.close();
                    },
                    Err(_) => println!("Error sending decline"),
                }
            })
        );
    }

    /// Removes the corresponding conversation box from the sender in the stack specified by the stack_name parameter.
    /// # Arguments
    /// * `sender` - The name of the client with whom you have the conversation that you want to delete.
    /// * `stack_name` - The name of the stack where the conversation box is located.
    pub fn remove_stack_box(&self, sender: String, stack_name: &str) {
        let stack = self
            .stack_conversations
            .child_by_name(stack_name)
            .unwrap()
            .downcast::<gtk::Stack>()
            .unwrap();
        println!("Removing {sender} from {stack_name}");
        let deleted_box = stack
            .child_by_name(&sender)
            .unwrap()
            .downcast::<gtk::Box>()
            .unwrap();
        stack.remove(&deleted_box);
    }

    /// Builds the resume dcc file transfer modal, giving it the functionality to resume the file send from another client.
    fn build_dcc_ip_port_resume_modal(&self, builder: &gtk::Builder, tx: Sender<String>) {
        let ip_port_dcc_modal_resume_file = builder
            .object::<gtk::Window>("ip_port_dcc_modal_resume_file")
            .unwrap();
        let resume_dcc_file_button = builder
            .object::<gtk::Button>("resume_dcc_file_button")
            .unwrap();
        let ip_dcc_entry_file_resume = builder
            .object::<gtk::Entry>("ip_dcc_entry_file_resume")
            .unwrap();
        let port_dcc_entry_file_resume = builder
            .object::<gtk::Entry>("port_dcc_entry_file_resume")
            .unwrap();
        let error_resume_file_ip_port = builder
            .object::<gtk::Label>("error_resume_file_ip_port")
            .unwrap();
        let file_name_resume_label = builder
            .object::<gtk::Label>("file_name_resume_label")
            .unwrap();
        let sender_resume_label = builder.object::<gtk::Label>("sender_resume_label").unwrap();
        ip_port_dcc_modal_resume_file.connect_delete_event(move |_win, _| _win.hide_on_delete());

        resume_dcc_file_button.connect_clicked(
            clone!(@weak ip_port_dcc_modal_resume_file, @weak error_resume_file_ip_port, @weak ip_dcc_entry_file_resume, @weak port_dcc_entry_file_resume, @weak sender_resume_label, @weak file_name_resume_label => move |_| {
                let ip = ip_dcc_entry_file_resume.text().to_string();
                let port = port_dcc_entry_file_resume.text().to_string();
                let sender = sender_resume_label.text().to_string();
                let file_name = file_name_resume_label.text().to_string();

                if ip.is_empty() || port.is_empty(){
                    error_resume_file_ip_port.set_text("Please fill out all fields");
                }
                else {
                    let message = format!("DCC RESUME {sender} {file_name} {ip} {port}");
                    match tx.send(message){
                        Ok(_) => {
                            ip_dcc_entry_file_resume.set_text("");
                            port_dcc_entry_file_resume.set_text("");
                            ip_port_dcc_modal_resume_file.close();
                        },
                        Err(_) => {
                            error_resume_file_ip_port.set_text("Error sending resume");
                        }
                    }
                }
            })
        );
    }

    /// Set the corresponding message box to an error message when the file transfer is declined.
    /// # Arguments
    /// * `sender` - The name of the client where you have the conversation of the message you want to set as declined.
    /// * `file_name` - The name of the file that you want to set as declined.
    pub fn set_transfer_declined_message(&self, sender: String, file_name: String) {
        let dcc_stack_conversations = self
            .stack_conversations
            .child_by_name("DCC conversations")
            .unwrap()
            .downcast::<gtk::Stack>()
            .unwrap();
        let conversation_dcc = dcc_stack_conversations
            .child_by_name(&sender)
            .unwrap()
            .downcast::<gtk::Box>()
            .unwrap();
        let messages = conversation_dcc.children();
        for message in messages {
            println!("el nombre del widget es {}", message.widget_name());
            if message.widget_name() == format!("{}-{}-box", file_name, sender) {
                let box_message = message.downcast::<gtk::Box>().unwrap();
                let css_provider = gtk::CssProvider::new();
                css_provider
                    .load_from_path("client/src/gui/style.scss")
                    .unwrap();
                let style_context = box_message.style_context();
                style_context.add_provider(&css_provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
                style_context.add_class("error_message");
                let message_children = box_message.children();
                for message_child in message_children {
                    if message_child.widget_name()
                        == format!("{file_name}-{sender}-progress-bar-box")
                    {
                        let progress_bar_box = message_child.downcast::<gtk::Box>().unwrap();
                        box_message.remove(&progress_bar_box);
                    }
                }
            }
        }
    }

    /// Creates the new DCC conversation box to start chatting to the user and then saves
    ///  the box in the DCC conversations stack. Sets the file chooser button, the close dcc
    /// button and the chat button to visible and sets the dcc button to not visible.
    /// # Arguments
    /// * `sender`: The name of the sender. It is used as key to save the new conversation box in the stack.
    pub fn create_dcc_box(&self, sender: String) {
        println!("[DEBUG] create_dcc_box {sender}");
        self.send_button.set_sensitive(true);
        self.message_entry.set_sensitive(true);
        self.current_chat.set_text(&sender);
        let dcc_stack = self
            .stack_conversations
            .child_by_name("DCC conversations")
            .unwrap()
            .downcast::<gtk::Stack>()
            .unwrap();
        let box_conversation = new_conversation(&sender);
        dcc_stack.add_named(&box_conversation, &sender);
        dcc_stack.set_visible_child_name(&sender);
        self.chat_button.set_visible(true);
        self.file_chooser_button.set_visible(true);
        self.dcc_button.set_visible(false);
        self.close_dcc_button.set_visible(true);
        self.stack_conversations
            .set_visible_child_name("DCC conversations");
        //adjust_scroll_to_bottom(&self.scrolled_window);
    }

    /// Updates the progress bar in the file message of the file that is being sent.
    /// # Arguments
    /// * `sender`: The name of the sender.
    /// * `file_name`: The name of the file.
    /// * `progress`: The progress of the file.
    pub fn update_progress_bar(&self, sender: String, file_name: String, progress: f64) {
        let dcc_stack_conversations = self
            .stack_conversations
            .child_by_name("DCC conversations")
            .unwrap()
            .downcast::<gtk::Stack>()
            .unwrap();
        let conversation_dcc = dcc_stack_conversations
            .child_by_name(&sender)
            .unwrap()
            .downcast::<gtk::Box>()
            .unwrap();
        let children = conversation_dcc.children();
        for child in children {
            if child.widget_name() == format!("{file_name}-{sender}-box") {
                let box_message = child.downcast::<gtk::Box>().unwrap();
                let box_children = box_message.children();
                for b in box_children {
                    if b.widget_name() == format!("{file_name}-{sender}-progress-bar-box") {
                        let progress_bar_box = b.downcast::<gtk::Box>().unwrap();
                        let progress_bar_box_children = progress_bar_box.children();
                        for p in progress_bar_box_children {
                            if p.widget_name() == format!("{file_name}-{sender}-pause-button") {
                                let pause_button = p.downcast::<gtk::Button>().unwrap();
                                pause_button.set_sensitive(true);
                            } else if p.widget_name() == format!("{file_name}-{sender}") {
                                let progress_bar = p.downcast::<gtk::ProgressBar>().unwrap();

                                progress_bar.set_fraction(progress);
                                progress_bar
                                    .set_text(Some(&format!("{}%", (progress * 100.0) as i32)));

                                if progress == 1.0 {
                                    box_message.remove(&progress_bar_box);
                                    box_message
                                        .set_widget_name(&format!("{file_name}-{sender}-box-done"));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Creates the loading screen that is shown when waiting to the user to accept the DCC connection.
    /// Then saves the screen in the Loadings stack
    /// # Arguments
    /// * `sender`: The name of the sender. It is used as key to save the new loading box in the stack.
    pub fn create_loading_screen(&self, sender: String) {
        let loading_stack = self
            .stack_conversations
            .child_by_name("Loadings")
            .unwrap()
            .downcast::<gtk::Stack>()
            .unwrap();
        if loading_stack.child_by_name(&sender).is_none() {
            println!("[DEBUG] create_loading_screen");
            let loading_screen = gtk::Box::new(gtk::Orientation::Vertical, 0);
            let label =
                gtk::Label::new(Some(&format!("Waiting for {sender} to accept connection")));
            let loading_icon = gtk::Spinner::new();
            loading_icon.set_size_request(50, 50);
            loading_icon.start();
            loading_screen.add(&label);
            loading_screen.add(&loading_icon);
            loading_screen.show_all();
            loading_stack.add_named(&loading_screen, &sender);
        }
        loading_stack.set_visible_child_name(&sender);
        loading_stack.show_all();
        self.stack_conversations.set_visible_child_name("Loadings");
    }

    /// Removes the corresponding button from the file message being sent, according visible id and invisible id.
    /// # Arguments
    /// * `sender`: The name of the sender.
    /// * `file_name`: The name of the file.
    /// * `visible_id`: The id of the button that will be visible.
    /// * `invisible_id`: The id of the button that will be not visible.
    pub fn set_visible_button(
        &self,
        sender: String,
        file_name: String,
        visible_id: String,
        invisible_id: String,
    ) {
        let dcc_stack_conversations = self
            .stack_conversations
            .child_by_name("DCC conversations")
            .unwrap()
            .downcast::<gtk::Stack>()
            .unwrap();
        let conversation_dcc = dcc_stack_conversations
            .child_by_name(&sender)
            .unwrap()
            .downcast::<gtk::Box>()
            .unwrap();
        let messages = conversation_dcc.children();
        for message in messages {
            if message.widget_name() == format!("{file_name}-{sender}-box") {
                let box_message = message.downcast::<gtk::Box>().unwrap();
                let message_children = box_message.children();

                for message_child in message_children {
                    if message_child.widget_name()
                        == format!("{file_name}-{sender}-progress-bar-box")
                    {
                        let progress_bar_box = message_child.downcast::<gtk::Box>().unwrap();
                        let progress_bar_box_children = progress_bar_box.children();
                        for p in progress_bar_box_children {
                            if p.widget_name() == format!("{file_name}-{sender}-{visible_id}") {
                                let resume_button = p.downcast::<gtk::Button>().unwrap();
                                resume_button.set_visible(true);
                            } else if p.widget_name()
                                == format!("{file_name}-{sender}-{invisible_id}")
                            {
                                let pause_button = p.downcast::<gtk::Button>().unwrap();
                                pause_button.set_visible(false);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Sets the message of the file being sent as paused.
    /// # Arguments
    /// * `sender`: The name of the sender.
    /// * `file_name`: The name of the file.
    pub fn set_transfer_paused_message(&self, sender: String, file_name: String) {
        self.set_visible_button(
            sender,
            file_name,
            "resume-button".to_string(),
            "pause-button".to_string(),
        );
    }

    /// Sets the message of the file being sent as resumed.
    /// # Arguments
    /// * `sender`: The name of the sender.
    /// * `file_name`: The name of the file.
    pub fn set_transfer_resumed_message(&self, sender: String, file_name: String) {
        self.set_visible_button(
            sender,
            file_name,
            "pause-button".to_string(),
            "resume-button".to_string(),
        );
    }
}
