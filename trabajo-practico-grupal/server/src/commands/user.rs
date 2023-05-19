use crate::server_errors::ServerError;
use model::{client::Client, message::Message};

/// Function that handles the "USER" command received from client.
/// # Arguments
/// * `message` - The message sent by the client.
/// * `nickname` - The nickname of the client.
/// * `user_parameters` - The username, servername, hostname and realname of the client.
/// * `password` - The password of the client.
pub fn handle_user_command(
    message: Message,
    nickname: &mut Option<String>,
    user_parameters: &mut Option<Vec<String>>,
    password: &mut Option<String>,
) -> Result<Option<Client>, ServerError> {
    if message.parameters.len() < 3 {
        return Err(ServerError::InvalidParameters);
    }
    if let Some(trailing) = message.trailing.to_owned() {
        *user_parameters = Option::Some(vec![
            message.parameters[0].to_owned(),
            message.parameters[1].to_owned(),
            message.parameters[2].to_owned(),
            trailing.to_owned(),
        ]);
        println!(
            "USER {} {} {} {}",
            message.parameters[0], message.parameters[1], message.parameters[2], trailing
        );
    } else {
        *user_parameters = Option::Some(vec![
            message.parameters[0].to_owned(),
            message.parameters[1].to_owned(),
            message.parameters[2].to_owned(),
            message.parameters[3].to_owned(),
        ]);
        println!(
            "USER {} {} {} {}",
            message.parameters[0],
            message.parameters[1],
            message.parameters[2],
            message.parameters[3]
        );
    }
    match nickname {
        Some(n) => {
            if let Some(user_params) = user_parameters.to_owned() {
                let client = Client::from_connection(
                    n.to_string(),
                    user_params[0].to_owned(),
                    user_params[1].to_owned(),
                    user_params[2].to_owned(),
                    user_params[3].to_owned(),
                    password.to_owned(),
                    true,
                );
                Ok(Option::Some(client))
            } else {
                Ok(Option::None)
            }
        }
        None => Ok(Option::None),
    }
}
