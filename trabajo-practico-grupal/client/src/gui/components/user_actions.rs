use gtk::{
    glib::{self, clone},
    prelude::*,
};
use std::sync::mpsc::Sender;

/// A struct that contains the widgets for the user actions
/// and the functions to handle the user actions.
/// # Fields
/// * `tx` - The sender to send messages to the server.
pub struct UserActions {
    tx: Sender<String>,
}

impl UserActions {
    /// Creates a new UserActions struct.
    /// # Arguments
    /// * `tx` - The sender to send messages to the server.
    pub fn new(tx: Sender<String>) -> Self {
        UserActions { tx }
    }

    /// Builds the user actions buttons, giving them the correct functionality. This includes the
    /// QUIT, AWAY and OPER buttons.
    /// # Arguments
    /// * `builder` - The builder to get the widgets from.
    pub fn build(self, builder: &gtk::Builder) {
        self.active_quit_button(builder);
        self.active_away_button(builder);
        self.active_oper_button(builder);
    }

    /// Builds the QUIT button, giving it the correct functionality. It sends the QUIT message to the
    /// server once the user clicks on the QUIT button.
    fn active_quit_button(&self, builder: &gtk::Builder) {
        let quit_button = builder.object::<gtk::Button>("quit_button").unwrap();
        let tx_clone = self.tx.clone();
        let stack = builder.object::<gtk::Stack>("stack").unwrap();
        let main_window = builder.object::<gtk::Window>("main_window").unwrap();
        quit_button.connect_clicked(clone!( @weak stack=> move |_| {
            if tx_clone.send("QUIT".to_string()).is_ok() { main_window.close() }
        }));
    }

    /// Builds the OPER button, giving it the correct functionality. It opens the OPER modal and
    /// sends the OPER message to the server once the user clicks on the OPER button.
    fn active_oper_button(&self, builder: &gtk::Builder) {
        let oper_button = builder.object::<gtk::Button>("oper_button").unwrap();
        let tx_clone = self.tx.clone();
        let oper_modal = builder.object::<gtk::Window>("oper_modal").unwrap();
        let oper_modal_button = builder.object::<gtk::Button>("oper_modal_button").unwrap();
        let nick_oper_entry = builder.object::<gtk::Entry>("nick_oper_entry").unwrap();
        let pass_oper_entry = builder.object::<gtk::Entry>("nick_oper_entry1").unwrap();
        let away_error = builder.object::<gtk::Label>("error_topic1").unwrap();
        oper_modal.connect_delete_event(move |_win, _| _win.hide_on_delete());
        oper_button.connect_clicked(clone!( @weak oper_modal=> move |_| {
            oper_modal.show();
        }));
        oper_modal_button.connect_clicked(
            clone!( @weak oper_modal, @weak nick_oper_entry, @weak pass_oper_entry=> move |_| {
                   let oper_nick = nick_oper_entry.text();
                   let oper_pass = pass_oper_entry.text();
                   let message = format!("OPER {} {}", oper_nick, oper_pass);
                   match tx_clone.send(message){
                       Ok(_) => (),
                       Err(_) => away_error.set_text("Error sending topic message"),
                   }
                   oper_modal.close();
            }),
        );
    }

    /// Builds the AWAY button, giving it the correct functionality. It opens the AWAY modal and
    /// sends the AWAY message to the server once the user clicks on the AWAY button. It also sends AWAY when the
    /// user clicks on the DISABLE AWAY button, so it can be used to toggle the AWAY status.
    fn active_away_button(&self, builder: &gtk::Builder) {
        let away_modal = builder.object::<gtk::Window>("away_modal").unwrap();
        let away_modal_button = builder.object::<gtk::Button>("away_modal_button").unwrap();
        let disable_away_button = builder
            .object::<gtk::Button>("disable_away_button")
            .unwrap();
        let away_button = builder.object::<gtk::Button>("away_button").unwrap();
        let away_channel_entry = builder.object::<gtk::Entry>("away_entry").unwrap();
        let away_error = builder.object::<gtk::Label>("error_away").unwrap();
        let tx_clone = self.tx.clone();
        let tx_clone2 = self.tx.clone();

        away_modal.connect_delete_event(move |_win, _| _win.hide_on_delete());

        away_button.connect_clicked(clone!( @weak away_modal=> move |_| {
            away_modal.show();
        }));

        away_modal_button.connect_clicked(
            clone!( @weak away_modal, @weak away_channel_entry=> move |_| {
                   let away_message = away_channel_entry.text();
                   println!("{}", away_message);
                   let message = format!("AWAY :{}", away_message);
                   match tx_clone.send(message){
                       Ok(_) => {
                        away_modal.close();
                       },
                       Err(_) => away_error.set_text("Error sending away message"),
                   }

            }),
        );

        disable_away_button.connect_clicked(clone!( @weak away_modal=> move |_| {
            if tx_clone2.send("AWAY".to_string()).is_ok() {
                away_modal.close();
            }

        }));
    }
}
