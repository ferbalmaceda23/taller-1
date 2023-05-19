use model::responses::errors::ErrorResponse;
use model::responses::replies::CommandResponse;
use model::responses::response::Response;

/// Receives the sender for the screens and the parsed message received from the server
/// and sends the message to the screen where it should be displayed.
pub fn send_to_screen(
    tx_connection: gtk::glib::Sender<Response>,
    tx_registration: gtk::glib::Sender<Response>,
    tx_chats: gtk::glib::Sender<Response>,
    message: Response,
) {
    match &message {
        Response::CommandResponse { response } => match response {
            CommandResponse::ConnectionSuccees => {
                send_response_to_screen(tx_connection, message);
            }
            CommandResponse::Welcome {
                nickname: _,
                username: _,
                hostname: _,
            } => {
                send_response_to_screen(tx_registration, message);
            }
            CommandResponse::Topic {
                channel: _,
                topic: _,
            } => {
                send_response_to_screen(tx_chats, message);
            }
            CommandResponse::Names {
                channel: _,
                names: _,
            } => {
                send_response_to_screen(tx_chats, message);
            }
            CommandResponse::EndNames => {
                send_response_to_screen(tx_chats, message);
            }
            CommandResponse::ListStart => {
                send_response_to_screen(tx_chats, message);
            }
            CommandResponse::List {
                channel: _,
                topic: _,
            } => {
                send_response_to_screen(tx_chats, message);
            }
            CommandResponse::ListEnd => {
                send_response_to_screen(tx_chats, message);
            }
            CommandResponse::NowAway => {
                send_response_to_screen(tx_chats, message);
            }
            CommandResponse::UnAway => {
                send_response_to_screen(tx_chats, message);
            }
            CommandResponse::Away {
                nickname: _,
                message: _,
            } => {
                send_response_to_screen(tx_chats, message);
            }
            CommandResponse::ChannelMode {
                channel: _,
                modes: _,
            } => {
                send_response_to_screen(tx_chats, message);
            }
            CommandResponse::UserMode { user: _, modes: _ } => {
                send_response_to_screen(tx_chats, message);
            }
            CommandResponse::EndBanList => {
                send_response_to_screen(tx_chats, message);
            }
            CommandResponse::BanList {
                channel: _,
                ban_list: _,
            } => {
                send_response_to_screen(tx_chats, message);
            }
            CommandResponse::YouAreOperator => {
                send_response_to_screen(tx_chats, message);
            }
            CommandResponse::WhoIsUser {
                nickname: _,
                username: _,
                hostname: _,
                servername: _,
                realname: _,
            } => {
                send_response_to_screen(tx_chats, message);
            }
            CommandResponse::WhoIsServer {
                nickname: _,
                servername: _,
                serverinfo: _,
            } => {
                send_response_to_screen(tx_chats, message);
            }
            CommandResponse::WhoIsChannels {
                nickname: _,
                channels: _,
            } => {
                send_response_to_screen(tx_chats, message);
            }
            CommandResponse::WhoReply { users: _ } => {
                send_response_to_screen(tx_chats, message);
            }
            CommandResponse::EndOfWho => {
                send_response_to_screen(tx_chats, message);
            }
            CommandResponse::EndOfWhoIs => {
                send_response_to_screen(tx_chats, message);
            }
            _ => {
                println!("Error");
            }
        },
        Response::ErrorResponse { response } => match response {
            ErrorResponse::NeedMoreParams { command: _ } => {
                send_response_to_screen(tx_registration, message);
            }
            ErrorResponse::AlreadyRegistered { nickname: _ } => {
                send_response_to_screen(tx_registration, message);
            }
            ErrorResponse::NickInUse { nickname: _ } => {
                send_response_to_screen(tx_registration, message);
            }
            ErrorResponse::PasswordMismatch => {
                send_response_to_screen(tx_chats, message);
            }
            ErrorResponse::NotRegistered => {
                send_response_to_screen(tx_registration, message);
            }
            ErrorResponse::ErrorWhileConnecting => {
                send_response_to_screen(tx_connection, message);
            }
            ErrorResponse::CannotSendToChannel { channel: _ } => {
                send_response_to_screen(tx_chats, message);
            }
            ErrorResponse::ChannelIsFull { channel: _ } => {
                send_response_to_screen(tx_chats, message);
            }
            ErrorResponse::ChanOPrivsNeeded { channel: _ } => {
                send_response_to_screen(tx_chats, message);
            }
            ErrorResponse::BannedFromChannel { channel: _ } => {
                send_response_to_screen(tx_chats, message);
            }
            ErrorResponse::InviteOnlyChannel { channel: _ } => {
                send_response_to_screen(tx_chats, message);
            }
            ErrorResponse::BadChannelKey { channel: _ } => {
                send_response_to_screen(tx_chats, message);
            }
            ErrorResponse::NoSuchChannel { channel: _ } => {
                send_response_to_screen(tx_chats, message);
            }
            ErrorResponse::KeySet { channel: _ } => {
                send_response_to_screen(tx_chats, message);
            }
            ErrorResponse::NoPrivileges => {
                send_response_to_screen(tx_chats, message);
            }
            ErrorResponse::NotOnChannel { channel: _ } => {
                send_response_to_screen(tx_chats, message);
            }
            ErrorResponse::UserOnChannel {
                nickname: _,
                channel: _,
            } => {
                send_response_to_screen(tx_chats, message);
            }
            ErrorResponse::ClientDisconnected { nickname: _ } => {
                send_response_to_screen(tx_chats, message);
            }
            _ => {
                println!("Error");
            }
        },
        Response::MessageResponse { response: _ } => {
            send_response_to_screen(tx_chats, message);
        }
        _ => {}
    }
}

pub fn send_response_to_screen(tx: gtk::glib::Sender<Response>, message: Response) {
    match tx.send(message) {
        Ok(_) => {}
        Err(_) => {
            println!("Error sending to screen");
        }
    }
}
