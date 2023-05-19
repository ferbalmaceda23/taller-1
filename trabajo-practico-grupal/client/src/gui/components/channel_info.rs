use gtk::{
    glib::{self, clone},
    prelude::*,
};
use std::{collections::HashMap, sync::mpsc::Sender};

use crate::gui::utils::no_channel_selected_screen;

use super::{actions::fetch_information, channel_mode::ChannelMode};

/// The channel info box.
/// # Fields
/// * `tx`: The sender of the channel that sends the messages to the server.
/// * `stack_channels_info`: The stack that contains the channels info.
/// * `channels_hash`: A hashmap that maps the name of the channel to the list of users in that channel.
/// * `channel_mode`: The channel mode struct.
/// * `topic_hash`: A hashmap that maps the name of the channel to the topic of that channel.
/// * `banned_users`: A hashmap that maps the name of the channel to the list of banned users in that channel.
/// * `error_modal`: Window that shows an error message.
/// * `current_chat`: The label containing the name of the current chat.
/// * `nick_label`: The label that contains the nick of the user.
/// * `banned_users_modal`: The window that shows the list of banned users.
/// * `banned_users_box`: The box that contains the list of banned users.
pub struct ChannelInfo {
    tx: Sender<String>,
    stack_channels_info: gtk::Stack,
    channels_hash: HashMap<String, Vec<String>>,
    topic_hash: HashMap<String, String>,
    banned_users: HashMap<String, Vec<String>>,
    error_modal: gtk::Window,
    current_chat: gtk::Label,
    nick_label: gtk::Label,
    banned_users_modal: gtk::Window,
    banned_users_box: gtk::Box,
    channel_mode: ChannelMode,
}

impl ChannelInfo {
    /// Creates a new `ChannelInfo` struct.
    /// # Arguments
    /// * `builder`: The builder of the glade file that builds the application.
    /// * `tx`: The sender of the channel that sends the messages to the server.
    pub fn new(builder: &gtk::Builder, tx: Sender<String>) -> Self {
        let stack_channels_info: gtk::Stack = builder.object("stack_channels_info").unwrap();
        let current_chat: gtk::Label = builder.object("current_chat").unwrap();
        let error_modal: gtk::Window = builder.object("error_modal").unwrap();
        let nick_label: gtk::Label = builder.object("user_nick").unwrap();
        let banned_users_modal: gtk::Window = builder.object("banned_users_modal").unwrap();
        let banned_users_box: gtk::Box = builder.object("banned_users_box").unwrap();
        let mut channel_mode = ChannelMode::new(builder, tx.clone());
        channel_mode.build(builder);

        banned_users_modal.connect_delete_event(move |_win, _| _win.hide_on_delete());

        ChannelInfo {
            tx,
            stack_channels_info,
            channels_hash: HashMap::new(),
            topic_hash: HashMap::new(),
            banned_users: HashMap::new(),
            error_modal,
            current_chat,
            nick_label,
            banned_users_modal,
            banned_users_box,
            channel_mode,
        }
    }

    /// Updates the channels_hash with the new list of channels and users.
    pub fn update_clients(&mut self, names: Vec<String>, channel: String) {
        self.channels_hash.insert(channel, names);
    }

    /// Updates the banned_users with the new list of banned users.
    pub fn update_banned_clients(&mut self, names: Vec<String>, channel: String) {
        self.banned_users.insert(channel, names);
    }

    /// Creates channel info boxes for each channel, it includes the list of users in that channel, the topic of the channel, and the banned users.
    pub fn add_channels_to_stack(&mut self) {
        self.stack_channels_info.foreach(|widget| {
            self.stack_channels_info.remove(widget);
        });

        let hash_clone = self.channels_hash.clone();
        for (channel, names) in &hash_clone {
            let channel_info = gtk::Box::new(gtk::Orientation::Vertical, 5);
            let channel_name = gtk::Label::new(Some(channel));
            let channel_topic = gtk::Label::new(Some(
                self.topic_hash.get(channel).unwrap_or(&"".to_string()),
            ));
            channel_info.set_child(Some(&channel_name));
            channel_info.set_child(Some(&channel_topic));
            for name in names {
                let client_box = self.create_client_box(name.to_string(), channel.to_string());
                channel_info.add(&client_box);
            }
            self.add_banned_users(&channel_info, channel);
            channel_info.show_all();
            self.stack_channels_info.add_named(&channel_info, channel);
        }

        let empty_channel = no_channel_selected_screen();
        self.stack_channels_info
            .add_named(&empty_channel, "empty_channel");
        self.stack_channels_info
            .set_visible_child_name("empty_channel");
        let current_chat = self.current_chat.text();
        if !current_chat.is_empty() {
            self.stack_channels_info
                .set_visible_child_name(&current_chat);
        }
    }

    /// Creates a box that contains the name of the ban users and a button to ban each user.
    /// It also creates a button to show the list of banned users.
    fn add_banned_users(&mut self, channel_info: &gtk::Box, channel: &str) {
        let nick = self.nick_label.text().to_string();
        let opers = self.channel_mode.get_opers();
        if opers.contains(&nick) {
            let button = gtk::Button::with_label("Banned users");
            let tx = self.tx.clone();
            let channel_clone = channel.to_string();
            button.connect_clicked(
                move |_| {
                    if tx.send(format!("MODE {} +b", channel_clone)).is_ok() {}
                },
            );
            channel_info.add(&button);
        }
    }

    /// Opens the window that shows the list of banned users.
    pub fn show_banned_list(&mut self) {
        let channel = self.current_chat.text().to_string();
        let empty_banned_list = Vec::new();
        let banned_users = match self.banned_users.get(&channel) {
            Some(users) => users.clone(),
            None => empty_banned_list,
        };
        self.banned_users_box.foreach(|widget| {
            self.banned_users_box.remove(widget);
        });
        if banned_users.is_empty() || banned_users[0].is_empty() {
            let label = gtk::Label::new(Some("No banned users"));
            self.banned_users_box.add(&label);
            self.banned_users_modal.show_all();
        } else {
            for user in banned_users {
                let user_box = gtk::Box::new(gtk::Orientation::Horizontal, 5);
                let user_label = gtk::Label::new(Some(&user));
                let button = gtk::Button::with_label("Unban");
                let tx = self.tx.clone();
                let channel_clone = channel.clone();
                button.connect_clicked(clone!(@weak self.banned_users_modal as modal => move |_| {
                    println!("MODE {} -b {}", channel_clone, user);
                    if tx.clone().send(format!("MODE {} -b {}", channel_clone, user)).is_ok(){}
                    if tx.send(format!("MODE {} +b", channel_clone)).is_ok() { modal.hide() }
                }));

                user_box.add(&user_label);
                user_box.add(&button);
                user_box.show_all();
                self.banned_users_box.add(&user_box);
                self.banned_users_box.show_all();
            }
        }
        self.banned_users_modal.show();
    }

    /// Updates the topic_hash with the new topic of the channel.
    pub fn update_topic(&mut self, channel: String, topic: String) {
        self.topic_hash.insert(channel, topic);
    }

    /// Creates a box that contains the name of the user and buttons to kick, ban, set or remove operator and set or remove
    /// moderator fot each user.
    fn create_client_box(&self, name: String, channel: String) -> gtk::Box {
        let client_box = gtk::Box::new(gtk::Orientation::Horizontal, 5);
        let opers = self.channel_mode.get_opers();
        let moderators = self.channel_mode.get_moderators();

        let nick = self.nick_label.text().to_string();
        let kick_button = self.create_kick_button(channel.clone(), name.clone());
        let ban_button = self.create_ban_button(channel.clone(), name.clone());
        let oper_button = self.create_oper_button(channel.clone(), name.clone());
        let deoper_button = self.create_desoper_button(channel.clone(), name.clone());
        let moderator_button = self.create_moderator_button(channel.clone(), name.clone());
        let demoderator_button = self.create_cancel_moderator_button(channel, name.clone());

        let mut button_oper = oper_button;
        let mut moder_button = moderator_button;
        if name == nick {
            kick_button.set_sensitive(false);
            ban_button.set_sensitive(false);
        }
        let client_name = gtk::Label::new(Some(&name));
        if opers.contains(&name) {
            client_name.set_text(&format!("{} (OPER)", name));
            button_oper = deoper_button;
        }
        if moderators.contains(&name) {
            let client_text = client_name.text().to_string();
            client_name.set_text(&format!("{} (MODERATOR)", client_text));
            moder_button = demoderator_button;
        }

        client_name.set_width_chars(10);
        client_box.add(&client_name);

        // create drop down menu for client
        let main_menu_bar = gtk::MenuBar::new();
        let menu = gtk::Menu::new();
        let menu_dropdown = gtk::MenuItem::with_label("Options");

        menu_dropdown.set_submenu(Some(&menu));

        menu.append(&ban_button);
        menu.append(&kick_button);
        menu.append(&button_oper);
        menu.append(&moder_button);

        main_menu_bar.append(&menu_dropdown);
        main_menu_bar.show_all();
        menu.show_all();

        client_box.add(&main_menu_bar);

        client_box.show_all();

        client_box
    }

    /// Creates a button to set a user as moderator.
    fn create_moderator_button(&self, channel: String, name: String) -> gtk::MenuItem {
        let button = gtk::MenuItem::with_label("SET AS MODERATOR");
        let tx = self.tx.clone();
        button.connect_activate(move |_| {
            if tx.send(format!("MODE {} +v {}", channel, name)).is_ok()
                && tx.send(format!("MODE {}", channel)).is_ok()
            {}
        });
        button
    }

    /// Creates a button to remove the moderator status of a user.
    fn create_cancel_moderator_button(&self, channel: String, name: String) -> gtk::MenuItem {
        let button = gtk::MenuItem::with_label("REMOVE AS MODERATOR");
        let tx = self.tx.clone();
        button.connect_activate(move |_| {
            if tx.send(format!("MODE {} -v {}", channel, name)).is_ok()
                && tx.send(format!("MODE {}", channel)).is_ok()
            {}
        });
        button
    }

    /// Creates a button to kick a user frmo channel
    fn create_kick_button(&self, channel: String, name: String) -> gtk::MenuItem {
        let kick_button = gtk::MenuItem::with_label("KICK");
        let tx_clone = self.tx.clone();
        let channel_clone = channel;
        kick_button.connect_activate(clone!(@weak self.error_modal as error_modal => move |_| {
            if tx_clone.send(format!("KICK {} {}", channel_clone, name)).is_ok() {
                    fetch_information(tx_clone.clone(), &error_modal)
                }
        }));
        kick_button
    }

    /// Creates a button to ban a user from channel.
    fn create_ban_button(&self, channel: String, name: String) -> gtk::MenuItem {
        let ban_button: gtk::MenuItem = gtk::MenuItem::with_label("BAN");
        let tx_clone = self.tx.clone();
        let channel_clone = channel;
        ban_button.connect_activate(
            clone!(@weak self.error_modal as error_modal => move |_| {
            if tx_clone.send(format!("MODE {} +b {}", channel_clone, name)).is_ok() && tx_clone.send(format!("MODE {}", channel_clone)).is_ok() { }
        })
        );
        ban_button
    }

    /// Creates a button to set a user as operator.
    fn create_oper_button(&self, channel: String, name: String) -> gtk::MenuItem {
        let oper_button: gtk::MenuItem = gtk::MenuItem::with_label("SET AS OPER");
        let tx_clone = self.tx.clone();
        let channel_clone = channel;
        oper_button.connect_activate(
            clone!(@weak self.error_modal as error_modal => move |_| {
                if tx_clone.send(format!("MODE {} +o {}", channel_clone, name)).is_ok() && tx_clone.send(format!("MODE {}", channel_clone)).is_ok(){}
        })
        );
        oper_button
    }

    /// Creates a button to remove the operator status of a user.
    fn create_desoper_button(&self, channel: String, name: String) -> gtk::MenuItem {
        let desoper_button: gtk::MenuItem = gtk::MenuItem::with_label("REMOVE OPER");
        let tx_clone = self.tx.clone();
        let channel_clone = channel;
        desoper_button.connect_activate(
            clone!(@weak self.error_modal as error_modal => move |_| {
            if tx_clone.send(format!("MODE {} -o {}", channel_clone, name)).is_ok() && tx_clone.send(format!("MODE {}", channel_clone)).is_ok(){
        }
        })
        );
        desoper_button
    }

    /// Updates the channel mode and then calls NAMES to update the list of users.
    pub fn update_channel_modes(&mut self, channel: String, modes: HashMap<String, String>) {
        self.channel_mode.update_channel_modes(channel, modes);
        if self.tx.send("NAMES".to_owned()).is_ok() {}
    }
}
