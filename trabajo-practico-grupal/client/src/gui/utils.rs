use gtk::prelude::*;

/// This function is used to adjust the scroll to the bottom of the conversation
/// when a new message is received.
pub fn adjust_scroll_to_bottom(scrolled_window: &gtk::ScrolledWindow) {
    let adjustment = scrolled_window.vadjustment();
    adjustment.set_value(adjustment.upper() - adjustment.page_size() - 100.0);
}

/// This function is used to create a new empty conversation screen between the user
/// and the other client.
pub fn new_conversation(receiver: &String) -> gtk::Box {
    let box_conversation = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let label = gtk::Label::new(Some(&format!("You're talking with {receiver}")));
    box_conversation.set_spacing(10);
    box_conversation.set_child(Some(&label));
    box_conversation.show_all();
    box_conversation
}

/// This function is used to create a new empty message box to display when there is no channel selected
pub fn no_channel_selected_screen() -> gtk::Box {
    let empty_channel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    empty_channel.expands();
    let channel_name = gtk::Label::new(Some("No channel selected"));
    empty_channel.set_child(Some(&channel_name));
    empty_channel.show_all();
    empty_channel
}
