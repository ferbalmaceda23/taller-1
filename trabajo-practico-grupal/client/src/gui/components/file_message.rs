use gtk::{
    glib::{self, clone},
    prelude::*,
};
use std::sync::mpsc::Sender;

/// Adds a new file_message to the corresponding conversation.
/// # Arguments
/// * `builder` - The gtk::Builder object that contains all the widgets of the application.
/// * `tx` - The Sender object that is used to send messages to the client.
/// * `receiver` - The name of the user that the message is sent to.
/// * `file_name` - The name of the file that is sent or received.
/// * `file_size` - The size of the file that is sent or received.
/// * `style` - The style of the message box. Either "sent" or "received".
/// * `id` - The id of the message box.
pub fn add_file_message_box(
    builder: &gtk::Builder,
    tx: Sender<String>,
    receiver: String,
    file_name: String,
    file_size: String,
    style: &str,
    id: String,
) -> gtk::Box {
    let message_box = gtk::Box::new(gtk::Orientation::Vertical, 10);
    message_box.set_widget_name(&format!("{}-box", id));

    let message_box_info = create_message_info_box(file_name.clone(), file_size);

    let css_provider = gtk::CssProvider::new();
    css_provider
        .load_from_path("client/src/gui/style.scss")
        .unwrap();
    let style_context = message_box.style_context();
    style_context.add_provider(&css_provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
    style_context.add_class(style);

    let progress_bar_box = create_progress_bar_box(id, tx, file_name, receiver, builder);
    message_box.pack_start(&message_box_info, true, true, 0);
    message_box.pack_start(&progress_bar_box, true, true, 5);

    message_box
}

/// Creates a box that contains the file name and file size.
/// # Arguments
/// * `file_name` - The name of the file that is sent or received.
/// * `file_size` - The size of the file that is sent or received.
fn create_message_info_box(file_name: String, file_size: String) -> gtk::Box {
    let message_box_info = gtk::Box::new(gtk::Orientation::Horizontal, 10);

    let info_box = gtk::Box::new(gtk::Orientation::Vertical, 0);

    let file_name_label = gtk::Label::new(Some(&file_name));
    let file_size_label = gtk::Label::new(Some(&(file_size + " bytes")));

    info_box.pack_start(&file_name_label, true, true, 0);
    info_box.pack_start(&file_size_label, true, true, 0);

    message_box_info.set_halign(gtk::Align::Center);
    message_box_info.set_size_request(100, 50);

    message_box_info.pack_start(&info_box, true, true, 0);

    message_box_info
}

/// Creates a box that contains the progress bar and the pause and resume buttons.
/// The box will have a particular id that is used to identify it.
/// Gives functionality to the pause and resume buttons.
/// If the resume button is clicked, it opens a modal that asks for a new ip and port to resume it.
/// # Arguments
/// * `id` - The id of the message box where the progress bar is going to be added.
/// * `tx` - The Sender object that is used to send messages to the client.
/// * `file_name` - The name of the file that is goinge to be resumed or paused
/// * `receiver` - The name of the user that the message is sent to.
/// * `builder` - The gtk::Builder object that contains all the widgets of the application.
fn create_progress_bar_box(
    id: String,
    tx: Sender<String>,
    file_name: String,
    receiver: String,
    builder: &gtk::Builder,
) -> gtk::Box {
    let ip_port_dcc_modal_resume_file = builder
        .object::<gtk::Window>("ip_port_dcc_modal_resume_file")
        .unwrap();
    let file_name_resume_label = builder
        .object::<gtk::Label>("file_name_resume_label")
        .unwrap();
    let sender_resume_label = builder.object::<gtk::Label>("sender_resume_label").unwrap();

    let ip_dcc_entry_file_resume = builder
        .object::<gtk::Entry>("ip_dcc_entry_file_resume")
        .unwrap();
    let port_dcc_entry_file_resume = builder
        .object::<gtk::Entry>("port_dcc_entry_file_resume")
        .unwrap();

    let progress_bar_box = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    progress_bar_box.set_widget_name(&format!("{}-progress-bar-box", id));
    let progress_bar = gtk::ProgressBar::new();
    progress_bar.set_widget_name(&id);
    progress_bar.set_show_text(true);
    progress_bar.set_text(Some("0%"));
    progress_bar.set_fraction(0.0);

    //make pause and resume buttons with images
    let pause_button = gtk::Button::new();
    let pause_image =
        gtk::Image::from_icon_name(Some("media-playback-pause"), gtk::IconSize::Button);
    pause_button.set_image(Some(&pause_image));

    let resume_button = gtk::Button::new();
    let resume_image =
        gtk::Image::from_icon_name(Some("media-playback-start"), gtk::IconSize::Button);
    resume_button.set_image(Some(&resume_image));

    let file_name_clone = file_name.clone();
    let receiver_clone = receiver.clone();

    resume_button.set_visible(false);
    resume_button.set_no_show_all(true);

    pause_button.set_sensitive(false);

    pause_button.connect_clicked(clone!(@weak pause_button, @weak resume_button => move |_| {
        println!("Pause button clicked");
        match tx.send(format!("DCC STOP {} {}", receiver, file_name)) {
            Ok(_) => println!("Message sent"),
            Err(_) => println!("Error sending message")
        }
        pause_button.set_visible(false);
        resume_button.set_visible(true);
    }));

    resume_button.connect_clicked(
        clone!(@weak pause_button, @weak resume_button, @weak ip_dcc_entry_file_resume, @weak port_dcc_entry_file_resume, @weak ip_port_dcc_modal_resume_file, @weak sender_resume_label, @weak file_name_resume_label => move |_| {
        pause_button.set_visible(true);
        resume_button.set_visible(false);
        file_name_resume_label.set_text(&file_name_clone);
        sender_resume_label.set_text(&receiver_clone);

        ip_dcc_entry_file_resume.set_text("");
        port_dcc_entry_file_resume.set_text("");
        ip_port_dcc_modal_resume_file.set_visible(true);
    })
    );
    resume_button.set_widget_name(&format!("{}-resume-button", id));
    pause_button.set_widget_name(&format!("{}-pause-button", id));

    progress_bar_box.pack_start(&progress_bar, true, true, 0);
    progress_bar_box.pack_start(&pause_button, true, true, 0);
    progress_bar_box.pack_start(&resume_button, true, true, 0);

    progress_bar_box
}
