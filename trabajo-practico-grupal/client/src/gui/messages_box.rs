use gtk::prelude::*;

///This function is used to create a new message box to display in the conversation screen when a message is sent
/// by the user.
/// The message is displayed in the left side of the screen.
pub fn message_sent_box(command: String, style: &str) -> gtk::Box {
    let message_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    message_box.set_halign(gtk::Align::End);
    message_box.set_size_request(100, 50);
    let message_label = gtk::Label::new(Some(&command));
    let css_provider = gtk::CssProvider::new();
    css_provider
        .load_from_path("client/src/gui/style.scss")
        .unwrap();
    let style_context = message_box.style_context();
    style_context.add_provider(&css_provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
    style_context.add_class(style);
    message_box.pack_start(&message_label, true, true, 0);

    message_box
}

///This function is used to create a new message box to display in the conversation screen when a message is received
/// by the user.
/// The message is displayed in the left side of the screen.
pub fn message_received_box(response: String, style: &str) -> gtk::Box {
    let message_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    message_box.set_halign(gtk::Align::Start);
    message_box.set_size_request(100, 50);
    let message_label = gtk::Label::new(Some(&response));
    let css_provider = gtk::CssProvider::new();
    css_provider
        .load_from_path("client/src/gui/style.scss")
        .unwrap();
    let style_context = message_box.style_context();
    style_context.add_provider(&css_provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
    style_context.add_class(style);
    message_box.pack_start(&message_label, true, true, 0);
    message_box
}
