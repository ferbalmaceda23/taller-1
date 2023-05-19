use gtk::{
    glib::{self, clone},
    prelude::*,
};
use std::{collections::HashMap, sync::mpsc::Sender};

/// Represents the search bar that uses the WHO and WHOIS messages to fetch information
pub struct SearchWho {
    tx: Sender<String>,
    search_entry: gtk::Entry,
    who_button: gtk::Button,
    who_is_button: gtk::Button,
    type_search_label: gtk::Label,
    search_results: gtk::Box,
    search_modal: gtk::Window,
    error_modal: gtk::Window,
    results: Vec<gtk::Box>,
}

impl SearchWho {
    /// Creates a new instance of the search bar
    /// # Arguments
    /// * `builder` - The builder that contains the widgets
    /// * `tx` - The channel to send messages to the server
    pub fn new(builder: &gtk::Builder, tx: Sender<String>) -> Self {
        let search_entry: gtk::Entry = builder.object("search_entry").unwrap();
        let who_button: gtk::Button = builder.object("who_button").unwrap();
        let who_is_button: gtk::Button = builder.object("who_is_button").unwrap();
        let type_search_label: gtk::Label = builder.object("type_search_label").unwrap();
        let search_results: gtk::Box = builder.object("search_results").unwrap();
        let search_modal: gtk::Window = builder.object("search_modal").unwrap();
        let error_modal: gtk::Window = builder.object("error_modal").unwrap();
        let results: Vec<gtk::Box> = Vec::new();

        search_modal.connect_delete_event(move |_win, _| _win.hide_on_delete());

        SearchWho {
            tx,
            search_entry,
            who_button,
            who_is_button,
            type_search_label,
            search_results,
            search_modal,
            error_modal,
            results,
        }
    }

    /// Builds the who and whois buttons of the search bar, giving them the functionality to send the corresponding messages.
    pub fn build(&mut self) {
        self.active_who_button();
        self.active_who_is_button();
    }

    ///Gives functionality to the button that is used to send the WHO message to the server.
    /// Gets the text in the search entry and sends the message.
    fn active_who_button(&mut self) {
        let tx = self.tx.clone();
        self.who_button.connect_clicked(
            clone!(@weak self.type_search_label as type_search,
            @weak self.search_entry as search_entry, @weak self.error_modal as error_modal => move |_| {
            type_search.set_text("Who: ");
            let search_text = search_entry.text();
            match tx.send(format!("WHO {}", search_text)) {
                Ok(_) => {},
                Err(_) => error_modal.show(),
            }
        })
        );
    }

    ///Gives functionality to the button that is used to send the WHOIS message to the server.
    /// Gets the text in the search entry and sends the message.
    fn active_who_is_button(&mut self) {
        let tx = self.tx.clone();
        self.who_is_button.connect_clicked(
            clone!(@weak self.type_search_label as type_search, @weak self.search_entry as search_entry, @weak self.error_modal as error_modal => move |_| {
            type_search.set_text("Who is:");
            let search_text = search_entry.text();
            println!("WHOIS {}", search_text);
            match tx.send(format!("WHOIS {}", search_text)) {
                Ok(_) => {},
                Err(_) => error_modal.show(),
            }
        })
        );
    }

    // When the EndOfWhoIs response us received, this functions show the results by putting them in the search_modal and then openning the modal
    pub fn show_search_who_is_results(&mut self) {
        for result in &self.results {
            self.search_results.add(result);
            self.search_results.show_all();
        }
        self.search_modal.show();
        self.results.clear();
    }

    /// Updates the WHOIS user message results by saving them in the results hashmap
    pub fn update_search_who_is_user_results(
        &mut self,
        nickname: String,
        username: String,
        hostname: String,
        servername: String,
        realname: String,
    ) {
        for element in self.search_results.children() {
            self.search_results.remove(&element);
        }
        let result_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let nickname_label = gtk::Label::new(Some(format!("Nickname: {}", nickname).as_str()));
        let username_label = gtk::Label::new(Some(format!("Username: {}", username).as_str()));
        let hostname_label = gtk::Label::new(Some(format!("Hostname: {}", hostname).as_str()));
        let servername_label =
            gtk::Label::new(Some(format!("Servername: {}", servername).as_str()));
        let realname_label = gtk::Label::new(Some(format!("Realname: {}", realname).as_str()));
        result_box.set_child(Some(&nickname_label));
        result_box.set_child(Some(&username_label));
        result_box.set_child(Some(&hostname_label));
        result_box.set_child(Some(&servername_label));
        result_box.set_child(Some(&realname_label));

        result_box.show_all();
        self.results.push(result_box);
    }

    /// Updates the WHOIS channels message results by saving them in the results hashmap
    pub fn update_search_who_is_channels_results(
        &mut self,
        nickname: String,
        channels: HashMap<String, String>,
    ) {
        let result_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let nickname_label = gtk::Label::new(Some(&nickname));
        result_box.pack_start(&nickname_label, false, false, 0);
        for (channel, mode) in channels {
            let channel_label = gtk::Label::new(Some(format!("{}: {}", channel, mode).as_str()));
            result_box.set_child(Some(&channel_label));
        }
        result_box.show_all();
        self.results.push(result_box);
    }

    /// It will add the results in the search_modal and then open it
    pub fn show_search_who_results(&mut self) {
        for result in &self.results {
            self.search_results.add(result);
            self.search_results.show_all();
        }
        self.search_modal.show();
        self.results.clear();
    }

    /// Updates the WHO message results by saving them in the results hashmap
    pub fn update_search_who_results(&mut self, users: Vec<String>) {
        for element in self.search_results.children() {
            self.search_results.remove(&element);
        }
        for user in users {
            let result_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            let user_label = gtk::Label::new(Some(&user));
            result_box.set_child(Some(&user_label));
            result_box.show_all();
            self.results.push(result_box);
        }
    }
}
