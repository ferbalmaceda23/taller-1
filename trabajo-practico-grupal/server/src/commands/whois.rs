use std::collections::HashMap;

use crate::{
    commands::command_utils::read_lock_channels, server_errors::ServerError, socket::inform_client,
};
use model::{
    channelflag::ChannelFlag,
    message::Message,
    responses::{errors::ErrorResponse, replies::CommandResponse},
    session::Session,
};

use super::command_utils::read_lock_clients;

/// Handles the WHOIS command.
/// # Arguments
/// * `session`: The session of the client that sent the command.
/// * `message`: The message that contains the command.
/// * `nickname`: The nickname of the client that sent the command.
///
/// # Errors
/// * `ErrorResponse::NeedMoreParams`: If the command is not followed by enough parameters. It will send the client a response with the error ErrorResponse::NeedMoreParams.
/// * `ErrorResponse::NoSuchNick`: If the nickname of the client that sent the command is not registered. It will send the client a response with the error ErrorResponse::NoSuchNick.
///
/// Sends the client a command response with the information of the client that was requested. Sends the command responses WhoIsChannels and WhoIsUser, and EndOfWhoIs
pub fn handle_whois_command(
    message: Message,
    nickname: String,
    session: &Session,
) -> Result<(), ServerError> {
    if message.parameters.is_empty() {
        let response = ErrorResponse::NeedMoreParams {
            command: "WHOIS".to_string(),
        }
        .to_string();
        inform_client(session, &nickname, &response)?;
        return Err(ServerError::InvalidParameters);
    }
    let nicknames = message.parameters[0]
        .split(',')
        .into_iter()
        .map(|n| n.trim())
        .collect::<Vec<_>>();
    for nick in nicknames {
        let clients_lock = read_lock_clients(session)?;
        match clients_lock.get(nick) {
            Some(c) => {
                if !c.connected {
                    println!("Client {} disconnected", nick);
                    continue;
                }
                println!("\nNickname: {}", c.nickname);
                println!(
                    "Username: {}, Hostname: {}, Servername: {}, Realname: {}",
                    c.username, c.hostname, c.servername, c.realname
                );
                let response = (CommandResponse::WhoIsUser {
                    nickname: c.nickname.to_owned(),
                    username: c.username.to_owned(),
                    hostname: c.hostname.to_owned(),
                    servername: c.servername.to_owned(),
                    realname: c.realname.to_owned(),
                })
                .to_string();
                inform_client(session, &nickname, response.as_str())?;

                print!("Channels: ");
                let mut channels_hash = HashMap::new();
                let channels_lock = read_lock_channels(session)?;
                for channel in channels_lock.values() {
                    if channel.users.contains(&c.nickname) {
                        print!("{}", channel.name);
                        if channel.operators.contains(&c.nickname.clone())
                            && channel.modes.contains(&ChannelFlag::ModeratedChannel)
                            && channel.moderators.contains(&c.nickname.clone())
                        {
                            print!("[operator]");
                            print!("+");
                            channels_hash.insert(channel.name.to_string(), "@+".to_string());
                        } else if channel.operators.contains(&c.nickname.clone()) {
                            channels_hash.insert(channel.name.to_string(), "@".to_string());
                        } else if channel.modes.contains(&ChannelFlag::ModeratedChannel)
                            && channel.moderators.contains(&c.nickname.clone())
                        {
                            channels_hash.insert(channel.name.to_string(), "+".to_string());
                        } else {
                            channels_hash.insert(channel.name.to_string(), "".to_string());
                        }
                        print!(" ");
                    }
                }
                drop(channels_lock);

                if !channels_hash.is_empty() {
                    let response = (CommandResponse::WhoIsChannels {
                        nickname: c.nickname.to_owned(),
                        channels: channels_hash,
                    })
                    .to_string();
                    inform_client(session, &nickname, response.as_str())?;
                }

                let response = CommandResponse::EndOfWhoIs.to_string();
                inform_client(session, &nickname, response.as_str())?;

                println!("\n");
            }
            None => {
                println!("Client {} not found", nick);
                return Err(ServerError::ClientNotFound);
            }
        };

        drop(clients_lock);
    }

    Ok(())
}
