use gtk::{glib, prelude::*};
use model::responses::dcc::DccResponse;
use model::responses::errors::ErrorResponse;
use model::responses::message::MessageResponse;
use model::responses::replies::CommandResponse;
use model::responses::response::Response;
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::mpsc::SyncSender;
use std::sync::Arc;
use std::sync::RwLock;

use crate::gui::components::channel_actions::ChannelActions;
use crate::gui::components::channel_info::ChannelInfo;
use crate::gui::components::chats_container::ChatsContainer;
use crate::gui::components::dcc_feature::DCCFeature;
use crate::gui::components::names_list::NamesList;
use crate::gui::components::user_actions::UserActions;
use crate::gui::components::user_mode::UserMode;
use crate::gui::components::who_action::SearchWho;

pub struct ChatsScreen {
    tx: Sender<String>,
}

impl ChatsScreen {
    pub fn new(tx: Sender<String>) -> Self {
        Self { tx }
    }
    pub fn build(
        self,
        builder: &gtk::Builder,
        rx: glib::Receiver<Response>,
        arc_dcc_interface_communication: Arc<RwLock<HashMap<String, SyncSender<String>>>>,
    ) {
        let mut names_list = NamesList::new(builder, self.tx.clone());
        let chats_container = ChatsContainer::new(builder);
        let channel_actions = ChannelActions::new(self.tx.clone(), builder);
        let mut search_who = SearchWho::new(builder, self.tx.clone());
        let user_actions = UserActions::new(self.tx.clone());
        let dcc_feature =
            DCCFeature::new(builder, self.tx.clone(), arc_dcc_interface_communication);
        let mut channel_info = ChannelInfo::new(builder, self.tx.clone());
        let mut user_mode = UserMode::new(builder, self.tx.clone());
        let ip_port_dcc_modal_file = builder
            .object::<gtk::Window>("ip_port_dcc_modal_file")
            .unwrap();
        let notification_modal = builder.object::<gtk::Window>("notification_modal").unwrap();
        notification_modal.connect_delete_event(move |_win, _| _win.hide_on_delete());
        let notification_label = builder.object::<gtk::Label>("notification_label").unwrap();
        let user_nick: gtk::Label = builder.object("user_nick").unwrap();
        let notification_receiver: gtk::Label = builder.object("notification_receiver").unwrap();
        let error_modal = builder.object::<gtk::Window>("error_modal").unwrap();
        error_modal.connect_delete_event(move |_win, _| _win.hide_on_delete());
        let error_label = builder.object::<gtk::Label>("error_label").unwrap();
        let oper_modal = builder.object::<gtk::Window>("oper_modal").unwrap();
        let message_entry = builder.object::<gtk::Entry>("chat_input").unwrap();
        let send_button = builder.object::<gtk::Button>("send_message").unwrap();
        let dcc_modal = builder.object::<gtk::Window>("dcc_confirmation").unwrap();
        let dcc_confirmation_file_modal = builder
            .object::<gtk::Window>("dcc_confirmation_file")
            .unwrap();
        let dcc_sender: gtk::Label = builder.object("dcc_sender_label").unwrap();
        let dcc_file_name: gtk::Label = builder.object("file_name_label").unwrap();
        let dcc_file_size: gtk::Label = builder.object("file_size_label").unwrap();
        let dcc_file_sender: gtk::Label = builder.object("file_sender_label").unwrap();

        dcc_feature.build(builder, self.tx.clone());
        chats_container.build(builder, self.tx);
        channel_actions.build(builder);
        user_actions.build(builder);
        user_mode.build(builder);
        search_who.build();

        rx.attach(None, move |message| {
            match message {
                Response::ErrorResponse { response } => match response {
                    ErrorResponse::CannotSendToChannel { channel } => {
                        error_label.set_text(&format!("Cannot send to channel {channel}"));
                        chats_container.remove_last_message(channel);
                        error_modal.show();
                    }
                    ErrorResponse::ChannelIsFull { channel } => {
                        println!("Channel is full");
                        error_label.set_text(&format!("Channel {channel} is full"));
                        error_modal.show();
                    }
                    ErrorResponse::ChanOPrivsNeeded { channel } => {
                        error_label
                            .set_text(&format!("Channel {channel} requires operator privileges"));
                        error_modal.show();
                    }
                    ErrorResponse::BannedFromChannel { channel } => {
                        error_label.set_text(&format!("You are banned from channel {channel}"));
                        error_modal.show();
                    }
                    ErrorResponse::InviteOnlyChannel { channel } => {
                        error_label.set_text(&format!("Channel {channel} is invite only"));
                        error_modal.show();
                    }
                    ErrorResponse::NotOnChannel { channel } => {
                        channel_actions.show();
                        error_label.set_text(&format!("You are not on channel {channel}"));
                        error_modal.show();
                    }
                    ErrorResponse::PasswordMismatch => {
                        error_label.set_text("Invalid credentials");
                        error_modal.show();
                    }
                    ErrorResponse::BadChannelKey { channel } => {
                        error_label.set_text(&format!("Invalid key for channel {channel}"));
                        error_modal.show();
                    }
                    ErrorResponse::NoPrivileges => {
                        error_label.set_text("You don't have privileges");
                        error_modal.show();
                    }
                    ErrorResponse::KeySet { channel } => {
                        error_label.set_text(&format!("Key for channel {channel} set"));
                        error_modal.show();
                    }
                    ErrorResponse::NoSuchChannel { channel } => {
                        error_label.set_text(&format!("Channel {channel} does not exist"));
                        error_modal.show();
                    }
                    ErrorResponse::ClientDisconnected { nickname } => {
                        println!("{nickname} is disconnected");
                        dcc_feature.remove_stack_box(nickname.clone(), "Loadings");
                        chats_container.set_no_channel_selected_screen();
                        let msg_error = format!("{nickname} is disconnected");
                        error_label.set_text(&msg_error);
                        error_modal.show();
                    }
                    _ => (),
                },
                Response::CommandResponse { response } => match response {
                    CommandResponse::Names { channel, names } => {
                        channel_info.update_clients(names.clone(), channel.clone());
                        names_list.update_clients(names, channel);
                    }
                    CommandResponse::EndNames => {
                        names_list.add_names_to_list();
                        channel_info.add_channels_to_stack();
                    }
                    CommandResponse::Topic { channel, topic } => {
                        channel_info.update_topic(channel, topic);
                    }
                    CommandResponse::ListStart => {}
                    CommandResponse::List { channel, topic } => {
                        channel_info.update_topic(channel, topic);
                    }
                    CommandResponse::ListEnd => {
                        channel_info.add_channels_to_stack();
                    }
                    CommandResponse::Away { nickname, message } => {
                        chats_container.add_message_received(
                            nickname,
                            message,
                            "User conversations",
                        );
                    }
                    CommandResponse::ChannelMode { channel, modes } => {
                        channel_info.update_channel_modes(channel, modes);
                    }
                    CommandResponse::UserMode { user: _, modes } => {
                        user_mode.update_user_modes(modes);
                    }
                    CommandResponse::YouAreOperator => {
                        notification_label.set_text("You are now an operator");
                        notification_modal.show();
                        oper_modal.close();
                    }
                    CommandResponse::WhoIsUser {
                        nickname,
                        username,
                        hostname,
                        servername,
                        realname,
                    } => {
                        search_who.update_search_who_is_user_results(
                            nickname, username, hostname, servername, realname,
                        );
                    }
                    CommandResponse::WhoIsChannels { nickname, channels } => {
                        search_who.update_search_who_is_channels_results(nickname, channels);
                    }
                    CommandResponse::WhoReply { users } => {
                        search_who.update_search_who_results(users);
                    }
                    CommandResponse::EndOfWho => {
                        search_who.show_search_who_results();
                    }
                    CommandResponse::EndOfWhoIs => {
                        search_who.show_search_who_is_results();
                    }
                    CommandResponse::BanList { channel, ban_list } => {
                        channel_info.update_banned_clients(ban_list, channel);
                    }
                    CommandResponse::EndBanList => {
                        channel_info.show_banned_list();
                    }
                    _ => (),
                },
                Response::MessageResponse { response } => match response {
                    MessageResponse::UserPrivMsg { message, sender } => {
                        println!("UserPrivMsg: {message}");
                        println!("Sender: {sender}");
                        chats_container.add_message_received(sender, message, "User conversations");
                    }
                    MessageResponse::ChannelPrivMsg {
                        channel,
                        message,
                        sender,
                    } => {
                        chats_container.add_message_channel_received(channel, sender, message);
                    }
                    MessageResponse::KickMsg { message } => {
                        notification_receiver.set_text(user_nick.text().as_str());
                        notification_label.set_text(&message);
                        notification_modal.set_visible(true);
                    }
                    MessageResponse::InviteMsg { message } => {
                        notification_receiver.set_text(user_nick.text().as_str());
                        notification_label.set_text(&message);
                        notification_modal.set_visible(true);
                    }
                },
                Response::DccResponse { response } => match response {
                    DccResponse::Accepted { sender } => {
                        println!("DCC accepted");
                        dcc_feature.create_dcc_box(sender.clone());
                        dcc_feature.remove_stack_box(sender, "Loadings");
                    }
                    DccResponse::Pending { sender: _ } => {
                        println!("DCC pending");
                        //dcc_feature.create_loading_screen(sender);
                    }
                    DccResponse::ChatRequest { sender } => {
                        println!("DCC request");
                        dcc_sender.set_text(&sender);
                        dcc_modal.show();
                    }
                    DccResponse::ChatMessage { sender, message } => {
                        println!("DCC message");
                        chats_container.add_message_received(sender, message, "DCC conversations");
                    }
                    DccResponse::CloseConnection { sender } => {
                        println!("DCC closed");
                        dcc_feature.remove_stack_box(sender, "DCC conversations");
                        chats_container.set_no_channel_selected_screen();
                    }
                    DccResponse::TransferProgress {
                        sender,
                        file_name,
                        progress,
                    } => {
                        //println!("DCC progress");
                        dcc_feature.update_progress_bar(sender, file_name, progress);
                    }
                    DccResponse::TransferRequest {
                        sender,
                        file_name,
                        file_size,
                    } => {
                        println!("DCC transfer request");
                        dcc_file_name.set_text(&file_name);
                        dcc_file_size.set_text(&file_size.to_string());
                        dcc_file_sender.set_text(&sender);
                        dcc_confirmation_file_modal.show();
                    }
                    DccResponse::TransferDeclined { sender, file_name } => {
                        println!("DCC transfer declined");
                        dcc_feature.set_transfer_declined_message(sender, file_name);
                    }
                    DccResponse::TransferPaused { sender, file_name } => {
                        println!("DCC transfer paused");
                        dcc_feature.set_transfer_paused_message(sender, file_name);
                    }
                    DccResponse::TransferResumed { sender, file_name } => {
                        println!("DCC transfer resumed");
                        dcc_feature.set_transfer_resumed_message(sender, file_name);
                    }
                    DccResponse::Rejected { sender } => {
                        println!("DCC rejected");
                        send_button.set_sensitive(true);
                        message_entry.set_sensitive(true);
                        chats_container.set_no_channel_selected_screen();
                        notification_receiver.set_text(&sender);
                        notification_label
                            .set_text(&format!("DCC connection with {sender} was rejected"));
                        notification_modal.set_visible(true);
                        dcc_feature.remove_stack_box(sender, "Loadings");
                    }
                    DccResponse::ErrorResponse { description } => {
                        error_label.set_text(&description);
                        error_modal.show();
                    }
                    DccResponse::ResumeAddressErrorResponse { sender, file_name } => {
                        dcc_feature.set_transfer_paused_message(sender, file_name);
                        error_label.set_text("Error selecting address");
                        error_modal.show();
                    }
                    DccResponse::SendAddressErrorResponse { sender, file_name } => {
                        dcc_feature.set_transfer_declined_message(sender, file_name);
                        error_label.set_text("Error selecting address");
                        error_modal.show();
                        ip_port_dcc_modal_file.hide();
                    }
                    DccResponse::ChatAddressErrorResponse { sender } => {
                        dcc_feature.remove_stack_box(sender, "Loadings");
                        error_label.set_text("Error selecting address");
                        error_modal.show();
                    }
                    DccResponse::OngoingTransfer { sender, file_name } => {
                        error_label.set_text("The file is already being sent");
                        error_modal.show();
                        chats_container.remove_last_message(sender.clone());
                        dcc_feature.set_transfer_paused_message(sender, file_name);
                        ip_port_dcc_modal_file.hide();
                    }
                },
            }
            glib::Continue(true)
        });
    }
}
