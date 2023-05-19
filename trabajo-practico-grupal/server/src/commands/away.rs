use super::command_utils::write_lock_clients;
use crate::{database::inform_database, server_errors::ServerError, socket::inform_client};
use model::{
    message::Message, persistence::PersistenceType, responses::replies::CommandResponse,
    session::Session,
};

/// Handles the away message, which sets the client as away or not away. If it receives a message, it sets the client as away and sets the away message.
/// If it receives no message, it sets the client as not away.
/// Sends the client a command response with the new away status.
pub fn handle_away_command(
    message: Message,
    nickname: String,
    session: &Session,
) -> Result<(), ServerError> {
    if let Some(client) = write_lock_clients(session)?.get_mut(&nickname) {
        if !message.parameters.is_empty() || message.trailing.is_some() {
            client.away_message = Some(get_away_message(message));
            let response = CommandResponse::NowAway.to_string();
            inform_client(session, &nickname, &response)?;
            println!("{} is now away", nickname);
        } else if message.parameters.is_empty() && message.trailing.is_none() {
            client.away_message = None;
            let response = CommandResponse::UnAway.to_string();
            inform_client(session, &nickname, &response)?;
        } else {
            return Err(ServerError::InvalidParameters);
        }
        inform_database(
            PersistenceType::ClientUpdate(nickname.to_owned()),
            client.to_string(),
            session,
        )?;
    }
    Ok(())
}

fn get_away_message(message: Message) -> String {
    let msg;
    if !message.parameters.is_empty() {
        if let Some(trailing) = message.trailing.to_owned() {
            let mut params = message.parameters[0..].to_owned();
            params.push(trailing);
            msg = params.join(" ");
        } else {
            let params = message.parameters[0..].to_owned();
            msg = params.join(" ");
        }
    } else if let Some(traling) = message.trailing {
        msg = traling;
    } else {
        msg = "".to_owned();
    }
    msg
}
