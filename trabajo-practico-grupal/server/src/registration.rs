use crate::commands::nick::handle_nick_command;
use crate::commands::pass::handle_pass_command;
use crate::commands::user::handle_user_command;
use crate::server_errors::ServerError;
use model::client::Client;
use model::message::Message;
use model::message::MessageType;
use model::network::Network;
use model::session::Session;

/// This function is called when a client is not registered or hasn't sent make the login yet
/// It will check the registration message type and call the appropriate function to handle the message
/// If the message type is not handled, it will return an error.
/// # Errors
/// * ServerError::ClientMustRegisterOrAuthenticate if the client is not registered or authenticated
///
/// Returns a client if the registration is successful, if it is not completed yet, it will return None.
pub fn handle_registration(
    message: Message,
    nickname: &mut Option<String>,
    user_parameters: &mut Option<Vec<String>>,
    password: &mut Option<String>,
    session: &Session,
    network: &Network,
) -> Result<Option<Client>, ServerError> {
    match message.command {
        MessageType::Pass => handle_pass_command(message, nickname, user_parameters, password)?,
        MessageType::Nick => {
            return handle_nick_command(
                message,
                nickname,
                user_parameters,
                password,
                session,
                network,
            )
        }
        MessageType::User => {
            return handle_user_command(message, nickname, user_parameters, password)
        }
        MessageType::Quit => println!("Unregistered client left the server"),
        _ => return Err(ServerError::ClientMustRegisterOrAuthenticate),
    }
    Ok(Option::None)
}
