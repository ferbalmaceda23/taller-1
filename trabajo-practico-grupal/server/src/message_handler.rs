use crate::{
    commands::{
        away::handle_away_command, command_utils::fetch_info, dcc::handle_dcc_command,
        invite::handle_invite_command, join::handle_join_command, kick::handle_kick_command,
        list::handle_list_command, mode::handle_mode_command, names::handle_names_command,
        oper::handle_oper_command, part::handle_part_command, privmsg::handle_privmsg_command,
        quit::handle_quit_command, topic::handle_topic_command, who::handle_who_command,
        whois::handle_whois_command,
    },
    server_errors::ServerError,
};
use model::{
    message::{Message, MessageType},
    network::Network,
    session::Session,
};

/// This function is called when a client is registered and authenticated
/// It will check the message type and call the appropriate function to handle the message
/// If the message type is not handled, it will return an error.
pub fn handle_client_message(
    message: Message,
    nickname: String,
    session: &Session,
    network: &Network,
    server_name: &String,
) -> Result<(), ServerError> {
    match message.command {
        MessageType::Quit => {
            handle_quit_command(message, nickname, session)?;
            fetch_info(session, network, server_name)?;
        }
        MessageType::Privmsg => {
            handle_privmsg_command(message, &nickname, session, network, server_name)?;
        }
        MessageType::Join => {
            handle_join_command(message, &nickname, session, network, server_name)?;
            fetch_info(session, network, server_name)?;
        }
        MessageType::Part => {
            handle_part_command(message, nickname, session, network, server_name)?;
            fetch_info(session, network, server_name)?;
        }
        MessageType::Kick => {
            handle_kick_command(message, nickname, session, network, server_name)?;
            fetch_info(session, network, server_name)?;
        }
        MessageType::Names => {
            handle_names_command(message, nickname, session, network, None)?;
        }
        MessageType::Topic => {
            handle_topic_command(message, nickname, session, network, server_name)?;
            fetch_info(session, network, server_name)?;
        }
        MessageType::List => {
            handle_list_command(message, &nickname, session, network, None)?;
        }
        MessageType::Mode => {
            handle_mode_command(message, nickname, session, network, server_name)?;
        }
        MessageType::Oper => {
            handle_oper_command(message, nickname, session, network)?;
        }
        MessageType::Invite => {
            handle_invite_command(message, nickname, session, network, server_name)?;
            fetch_info(session, network, server_name)?;
        }
        MessageType::Who => {
            handle_who_command(message, nickname, session, network, None)?;
        }
        MessageType::WhoIs => {
            handle_whois_command(message, nickname, session)?;
        }
        MessageType::Away => {
            handle_away_command(message, nickname, session)?;
        }
        MessageType::Dcc => {
            handle_dcc_command(message, nickname, session, network, server_name)?;
        }
        MessageType::Nick => return Err(ServerError::ClientAlreadyRegistered),
        MessageType::Pass => return Err(ServerError::ClientAlreadyRegistered),
        MessageType::User => return Err(ServerError::ClientAlreadyRegistered),
        _ => return Err(ServerError::InvalidCommand),
    }

    Ok(())
}
