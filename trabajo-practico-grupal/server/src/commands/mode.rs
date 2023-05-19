use super::command_utils::{write_lock_channels, write_lock_clients};
use crate::{
    database::inform_database,
    server_errors::ServerError,
    socket::{inform_client, inform_network},
};
use model::{
    channel::Channel,
    channelflag::ChannelFlag,
    client::Client,
    message::Message,
    network::Network,
    persistence::PersistenceType,
    responses::{errors::ErrorResponse, replies::CommandResponse},
    session::Session,
    userflag::UserFlag,
};
use std::{collections::HashMap, str::Chars};

/// Handles the MODE command, if it receives a channel name as parameter, it will return the channel mode.
/// If it receives a nickname as parameter, it will return the user mode.
/// If it receives a channel name and a mode, it will set the channel mode.
/// If it receives a nickname and a mode, it will set the user mode.
pub fn handle_mode_command(
    message: Message,
    nickname: String,
    session: &Session,
    network: &Network,
    server_name: &String,
) -> Result<(), ServerError> {
    if message.parameters.is_empty() {
        let response = (ErrorResponse::NeedMoreParams {
            command: "MODE".to_string(),
        })
        .to_string();
        inform_client(session, &nickname, &response)?;
        return Err(ServerError::InvalidParameters);
    }
    if message.parameters[0].starts_with('&') || message.parameters[0].starts_with('#') {
        handle_channel_mode_command(message, session, network, nickname, server_name)?;
    } else {
        handle_user_mode_command(message, session, nickname, network)?;
    }
    Ok(())
}

/// Handles the MODE command when the first parameter is a nick.
/// If the user is not found, it returns an error.
/// If the user is found, but it doesn't match with the sender, it returns an error.
/// If it receives a mode and the nick matches the sender's nick, it will set the user mode.
fn handle_user_mode_command(
    message: Message,
    session: &Session,
    nickname: String,
    network: &Network,
) -> Result<(), ServerError> {
    if message.parameters[0] != nickname {
        let response = ErrorResponse::UsersDontMatch.to_string();
        inform_client(session, &nickname, response.as_str())?;
        return Err(ServerError::CannotChangeModesFromOtherUsers);
    }

    let mut clients_lock = write_lock_clients(session)?;
    let c = match clients_lock.get_mut(&message.parameters[0]) {
        Some(c) => c,
        None => {
            let response = (ErrorResponse::NoSuchNick {
                nickname: message.parameters[0].to_string(),
            })
            .to_string();
            inform_client(session, &nickname, response.as_str())?;
            return Err(ServerError::ClientNotFound);
        }
    };

    if message.parameters.len() == 1 {
        let hash_modes = get_user_modes_hash(c.clone());
        let response = (CommandResponse::UserMode {
            user: nickname.clone(),
            modes: hash_modes,
        })
        .to_string();
        inform_client(session, &nickname, response.as_str())?;
        return Ok(());
    }
    let mut flags = message.parameters[1].chars();
    match flags.next() {
        Some(action) => {
            if action != '+' && action != '-' {
                let response = ErrorResponse::UnknownModeFlag.to_string();
                inform_client(session, &nickname, response.as_str())?;
                return Err(ServerError::InvalidFlags);
            }
            handle_user_flags(flags, c, action, session, network)?;
        }
        _ => {
            let response = (ErrorResponse::NeedMoreParams {
                command: "MODE".to_string(),
            })
            .to_string();
            inform_client(session, &nickname, response.as_str())?;
            return Err(ServerError::InvalidParameters);
        }
    }
    Ok(())
}

///Returns the HashMap with the user modes.
fn get_user_modes_hash(client: Client) -> HashMap<String, String> {
    let mut hash_modes = HashMap::new();
    let modes = client.modes;
    for flag in UserFlag::iter() {
        match flag {
            UserFlag::Invisible => {
                if modes.contains(&flag) {
                    hash_modes.insert(flag.to_string(), "+".to_string());
                } else {
                    hash_modes.insert(flag.to_string(), "-".to_string());
                }
            }
            UserFlag::Operator => {
                if modes.contains(&flag) {
                    hash_modes.insert(flag.to_string(), "+".to_string());
                } else {
                    hash_modes.insert(flag.to_string(), "-".to_string());
                }
            }
            UserFlag::ServerNotice => {
                if modes.contains(&flag) {
                    hash_modes.insert(flag.to_string(), "+".to_string());
                } else {
                    hash_modes.insert(flag.to_string(), "-".to_string());
                }
            }
            UserFlag::Wallops => {
                if modes.contains(&flag) {
                    hash_modes.insert(flag.to_string(), "+".to_string());
                } else {
                    hash_modes.insert(flag.to_string(), "-".to_string());
                }
            }
            UserFlag::Other => {
                continue;
            }
        }
    }

    hash_modes
}

/// Handles the changes for the user flags, depending on the mode sent.
/// Returns an error if the flag is not valid.
fn handle_user_flags(
    flags: std::str::Chars,
    client: &mut Client,
    action: char,
    session: &Session,
    network: &Network,
) -> Result<(), ServerError> {
    for f in flags {
        match UserFlag::match_flag(f) {
            UserFlag::Invisible => {
                set_user_flag(client, action, UserFlag::Invisible, session)?;
            }
            UserFlag::ServerNotice => {
                set_user_flag(client, action, UserFlag::ServerNotice, session)?;
            }
            UserFlag::Wallops => {
                set_user_flag(client, action, UserFlag::Wallops, session)?;
            }
            UserFlag::Operator => {
                handle_operator_flag(client, action, session, network)?;
            }
            UserFlag::Other => {
                let response = (ErrorResponse::UnknownMode { character: f }).to_string();
                inform_client(session, &client.nickname, response.as_str())?;
                println!("{:?} is an invalid flag", f);
            }
        }
    }
    Ok(())
}

/// Handles the operator flag, depending on the mode sent.
/// If the correct passwird and nicks are given, it will set the user as a server operator.
fn handle_operator_flag(
    client: &mut Client,
    action: char,
    session: &Session,
    network: &Network,
) -> Result<(), ServerError> {
    if action == '+' {
        println!("Cannot override OPER command");
        let response = ErrorResponse::UnknownModeFlag.to_string();
        inform_client(session, &client.nickname, response.as_str())?;
        return Err(ServerError::InvalidFlags);
    }

    let mut server_lock = match network.server.as_ref().write() {
        Ok(server_lock) => server_lock,
        Err(_) => {
            return Err(ServerError::LockError);
        }
    };

    if !server_lock.operators.contains(&client.nickname) {
        println!("{:?} is already not an operator", &client.nickname);
    } else {
        for (i, user) in server_lock.operators.iter().enumerate() {
            if user == &client.nickname {
                server_lock.operators.remove(i);
                break;
            }
        }
        println!("Operator removed: {:?}", server_lock);
    }
    drop(server_lock);

    Ok(())
}

fn set_user_flag(
    client: &mut Client,
    action: char,
    flag: UserFlag,
    session: &Session,
) -> Result<(), ServerError> {
    match action {
        '+' => {
            if client.modes.contains(&flag) {
                println!("{:?} is already {:?}", &client.nickname, flag);
            } else {
                println!("{:?} is now a {:?}", &client.nickname, flag);
                client.modes.push(flag);
                inform_database(
                    PersistenceType::ClientUpdate(client.nickname.to_owned()),
                    client.to_string(),
                    session,
                )?;
            }
        }
        '-' => {
            if !client.modes.contains(&flag) {
                println!("{:?} is already not {:?}", &client.nickname, flag);
            } else {
                for (i, mode) in client.modes.iter().enumerate() {
                    if *mode == flag {
                        client.modes.remove(i);
                        inform_database(
                            PersistenceType::ClientUpdate(client.nickname.to_owned()),
                            client.to_string(),
                            session,
                        )?;
                        break;
                    }
                }
                println!("{:?} is now not a {:?}", &client.nickname, flag);
            }
        }
        _ => {
            let response = ErrorResponse::UnknownModeFlag.to_string();
            inform_client(session, &client.nickname, response.as_str())?;
            return Err(ServerError::InvalidFlags);
        }
    }
    Ok(())
}

/// Handles the MODE command when the first parameter is a channel.
/// If the channel is not found, it returns an error.
/// If it does not have the correct permissions, it returns an error.
/// If the mode is not valid, it returns an error.
/// If it does not receive a flag, it returns the hash with the modes.
/// If it receives a flag, it handles the changes for the channel flags if the nickname has permissions.
fn handle_channel_mode_command(
    message: Message,
    session: &Session,
    network: &Network,
    nickname: String,
    server_name: &String,
) -> Result<(), ServerError> {
    let mut channel_lock = write_lock_channels(session)?;
    let channel = match channel_lock.get_mut(&message.parameters[0]) {
        Some(channel) => channel,
        None => {
            let response = (ErrorResponse::NoSuchChannel {
                channel: message.parameters[0].to_string(),
            })
            .to_string();
            inform_client(session, &nickname, response.as_str())?;
            return Err(ServerError::ChannelNotFound);
        }
    };
    if message.parameters.len() == 1 {
        let hash_modes = get_channel_modes_hash(channel);
        let response = (CommandResponse::ChannelMode {
            channel: channel.name.clone(),
            modes: hash_modes,
        })
        .to_string();
        inform_client(session, &nickname, response.as_str())?;
        return Ok(());
    }
    if !channel.operators.contains(&nickname) {
        let response = (ErrorResponse::ChanOPrivsNeeded {
            channel: channel.name.to_string(),
        })
        .to_string();
        inform_client(session, &nickname, response.as_str())?;
        return Err(ServerError::UserNotOperator);
    }
    let mut flags = message.parameters[1].chars();
    match flags.next() {
        Some(action) => {
            if action != '+' && action != '-' {
                let response = ErrorResponse::UnknownModeFlag.to_string();
                inform_client(session, &nickname, response.as_str())?;
                return Err(ServerError::InvalidFlags);
            }
            handle_channel_flags(
                channel,
                (action, &mut flags),
                &message,
                session,
                network,
                &nickname,
                server_name,
            )?;
        }
        _ => {
            let response = (ErrorResponse::NeedMoreParams {
                command: "MODE".to_string(),
            })
            .to_string();
            inform_client(session, &nickname, response.as_str())?;
            return Err(ServerError::InvalidParameters);
        }
    }

    Ok(())
}

/// Returns the hash with the modes of the channel.
pub fn get_channel_modes_hash(channel: &Channel) -> HashMap<String, String> {
    let mut hash_modes = HashMap::new();
    let modes = channel.modes.to_owned();
    for flag in ChannelFlag::iter() {
        match flag {
            ChannelFlag::Private => {
                if modes.contains(&flag) {
                    hash_modes.insert(flag.to_string(), "+".to_string());
                } else {
                    hash_modes.insert(flag.to_string(), "-".to_string());
                }
            }
            ChannelFlag::Secret => {
                if modes.contains(&flag) {
                    hash_modes.insert(flag.to_string(), "+".to_string());
                } else {
                    hash_modes.insert(flag.to_string(), "-".to_string());
                }
            }
            ChannelFlag::InviteOnly => {
                if modes.contains(&flag) {
                    hash_modes.insert(flag.to_string(), "+".to_string());
                } else {
                    hash_modes.insert(flag.to_string(), "-".to_string());
                }
            }
            ChannelFlag::TopicSettableOnlyOperators => {
                if modes.contains(&flag) {
                    hash_modes.insert(flag.to_string(), "+".to_string());
                } else {
                    hash_modes.insert(flag.to_string(), "-".to_string());
                }
            }
            ChannelFlag::NoMessageFromOutside => {
                if modes.contains(&flag) {
                    hash_modes.insert(flag.to_string(), "+".to_string());
                } else {
                    hash_modes.insert(flag.to_string(), "-".to_string());
                }
            }
            ChannelFlag::ModeratedChannel => {
                if modes.contains(&flag) {
                    hash_modes.insert(flag.to_string(), "+".to_string());
                } else {
                    hash_modes.insert(flag.to_string(), "-".to_string());
                }
            }
            ChannelFlag::ChannelOperator => {
                hash_modes.insert(flag.to_string(), channel.operators.join(","));
            }
            ChannelFlag::UserLimit => {
                if let Some(limit) = channel.limit.to_owned() {
                    hash_modes.insert(flag.to_string(), limit.to_string());
                } else {
                    hash_modes.insert(flag.to_string(), "".to_string());
                }
            }
            ChannelFlag::Ban => {
                hash_modes.insert(flag.to_string(), channel.banned_users.join(","));
            }
            ChannelFlag::ChannelKey => {
                hash_modes.insert(flag.to_string(), "".to_string());
            }
            ChannelFlag::SpeakInModeratedChannel => {
                hash_modes.insert(flag.to_string(), channel.moderators.join(","));
            }
            ChannelFlag::Other => {
                continue;
            }
        }
    }

    hash_modes
}

fn handle_channel_flags(
    channel: &mut Channel,
    flag_info: (char, &mut Chars),
    message: &Message,
    session: &Session,
    network: &Network,
    nickname: &String,
    server_name: &String,
) -> Result<(), ServerError> {
    let mut flag = flag_info.1.next();
    while let Some(f) = flag {
        match ChannelFlag::match_flag(f) {
            ChannelFlag::Private => {
                set_channel_flag(
                    channel,
                    (flag_info.0, ChannelFlag::Private),
                    message,
                    session,
                    network,
                    nickname,
                    server_name,
                )?;
            }
            ChannelFlag::Secret => {
                set_channel_flag(
                    channel,
                    (flag_info.0, ChannelFlag::Secret),
                    message,
                    session,
                    network,
                    nickname,
                    server_name,
                )?;
            }
            ChannelFlag::InviteOnly => {
                set_channel_flag(
                    channel,
                    (flag_info.0, ChannelFlag::InviteOnly),
                    message,
                    session,
                    network,
                    nickname,
                    server_name,
                )?;
            }
            ChannelFlag::ModeratedChannel => {
                set_channel_flag(
                    channel,
                    (flag_info.0, ChannelFlag::ModeratedChannel),
                    message,
                    session,
                    network,
                    nickname,
                    server_name,
                )?;
            }
            ChannelFlag::TopicSettableOnlyOperators => {
                set_channel_flag(
                    channel,
                    (flag_info.0, ChannelFlag::TopicSettableOnlyOperators),
                    message,
                    session,
                    network,
                    nickname,
                    server_name,
                )?;
            }
            ChannelFlag::NoMessageFromOutside => {
                set_channel_flag(
                    channel,
                    (flag_info.0, ChannelFlag::NoMessageFromOutside),
                    message,
                    session,
                    network,
                    nickname,
                    server_name,
                )?;
            }
            ChannelFlag::ChannelOperator => {
                handle_channel_operator_flag(
                    channel,
                    flag_info.0,
                    message,
                    session,
                    network,
                    nickname,
                    server_name,
                )?;
            }
            ChannelFlag::ChannelKey => {
                handle_key_flag(
                    channel,
                    flag_info.0,
                    message,
                    session,
                    network,
                    nickname.clone(),
                    server_name,
                )?;
            }
            ChannelFlag::Ban => {
                handle_ban_flag(
                    channel,
                    flag_info.0,
                    message,
                    session,
                    network,
                    nickname,
                    server_name,
                )?;
            }
            ChannelFlag::UserLimit => {
                handle_user_limit_flag(
                    channel,
                    flag_info.0,
                    message,
                    session,
                    network,
                    nickname,
                    server_name,
                )?;
            }
            ChannelFlag::SpeakInModeratedChannel => {
                handle_speak_in_moderated_channel_flag(
                    channel,
                    flag_info.0,
                    message,
                    session,
                    network,
                    nickname,
                    server_name,
                )?;
            }
            ChannelFlag::Other => {
                let response = (ErrorResponse::UnknownMode { character: f }).to_string();
                inform_client(session, nickname, response.as_str())?;
                println!("{:?} is an invalid flag", flag);
            }
        }
        flag = flag_info.1.next();
    }
    Ok(())
}

fn set_channel_flag(
    channel: &mut Channel,
    flag_info: (char, ChannelFlag),
    message: &Message,
    session: &Session,
    network: &Network,
    nickname: &String,
    server_name: &String,
) -> Result<(), ServerError> {
    match flag_info.0 {
        '+' => {
            if channel.modes.contains(&flag_info.1) {
                println!("{:?} is already {:?}", &channel.name, flag_info.1);
            } else {
                println!("{:?} is now a {:?} channel", &channel.name, flag_info.1);
                channel.modes.push(flag_info.1);
                inform_database(
                    PersistenceType::ChannelUpdate(channel.name.to_owned()),
                    channel.to_string(),
                    session,
                )?;
            }
        }
        '-' => {
            if !channel.modes.contains(&flag_info.1) {
                println!("{:?} is already not {:?}", &channel.name, flag_info.1);
            } else {
                for (i, mode) in channel.modes.iter().enumerate() {
                    if *mode == flag_info.1 {
                        channel.modes.remove(i);
                        inform_database(
                            PersistenceType::ChannelUpdate(channel.name.to_owned()),
                            channel.to_string(),
                            session,
                        )?;
                        break;
                    }
                }
                println!("{:?} is now a not {:?} channel", &channel.name, flag_info.1);
            }
        }
        _ => {
            let response = ErrorResponse::UnknownModeFlag.to_string();
            inform_client(session, nickname, response.as_str())?;
            return Err(ServerError::InvalidFlags);
        }
    }
    if channel.name.starts_with('#') {
        let mut msg = message.clone();
        msg.prefix = Some(nickname.to_owned());
        let msg = Message::deserialize(msg)?;
        inform_network(network, server_name, &msg)?;
    }
    Ok(())
}

fn handle_channel_operator_flag(
    channel: &mut Channel,
    action: char,
    message: &Message,
    session: &Session,
    network: &Network,
    nickname: &String,
    server_name: &String,
) -> Result<(), ServerError> {
    if message.parameters.len() > 3 {
        return Err(ServerError::InvalidParameters);
    }
    let nick = &message.parameters[2];
    match action {
        '+' => {
            if channel.operators.contains(nick) {
                let response = (ErrorResponse::KeySet {
                    channel: channel.name.to_owned(),
                })
                .to_string();
                inform_client(session, &channel.name, response.as_str())?;
                println!("{:?} is already an operator of {:?}", nick, &channel.name);
                return Ok(());
            } else {
                println!("{:?} is now an operator of {:?}", nick, &channel.name);
                channel.operators.push(nick.to_string());
                inform_database(
                    PersistenceType::ChannelUpdate(channel.name.to_owned()),
                    channel.to_string(),
                    session,
                )?;
            }
        }
        '-' => {
            if !channel.operators.contains(nick) {
                println!(
                    "{:?} is already not an operator of {:?}",
                    nick, &channel.name
                );
            } else {
                if channel.operators.len() == 1 {
                    return Err(ServerError::CannotRemoveLastOperator);
                }
                for (i, operator) in channel.operators.iter().enumerate() {
                    if *operator == *nick {
                        channel.operators.remove(i);
                        inform_database(
                            PersistenceType::ChannelUpdate(channel.name.to_owned()),
                            channel.to_string(),
                            session,
                        )?;
                        break;
                    }
                }
                println!("{:?} is now not an operator of {:?}", nick, &channel.name);
            }
        }
        _ => {
            let response = ErrorResponse::UnknownModeFlag.to_string();
            inform_client(session, nickname, response.as_str())?;
            return Err(ServerError::InvalidFlags);
        }
    }
    if channel.name.starts_with('#') {
        let mut msg = message.clone();
        msg.prefix = Some(nickname.to_owned());
        let msg = Message::deserialize(msg)?;
        inform_network(network, server_name, &msg)?;
    }
    Ok(())
}

fn handle_key_flag(
    channel: &mut Channel,
    action: char,
    message: &Message,
    session: &Session,
    network: &Network,
    nickname: String,
    server_name: &String,
) -> Result<(), ServerError> {
    if message.parameters.len() != 3 {
        return Err(ServerError::InvalidParameters);
    }
    let key = &message.parameters[2];
    match action {
        '+' => {
            set_channel_flag(
                channel,
                (action, ChannelFlag::ChannelKey),
                message,
                session,
                network,
                &nickname,
                server_name,
            )?;
            channel.password = Some(key.to_string());
            inform_database(
                PersistenceType::ChannelUpdate(channel.name.to_owned()),
                channel.to_string(),
                session,
            )?;
        }
        '-' => {
            if channel.password.is_none() {
                println!("{:?} already has no key", &channel.name);
                return Ok(());
            } else {
                set_channel_flag(
                    channel,
                    (action, ChannelFlag::ChannelKey),
                    message,
                    session,
                    network,
                    &nickname,
                    server_name,
                )?;
                channel.password = None;
                inform_database(
                    PersistenceType::ChannelUpdate(channel.name.to_owned()),
                    channel.to_string(),
                    session,
                )?;
            }
        }
        _ => {
            let response = ErrorResponse::UnknownModeFlag.to_string();
            inform_client(session, &nickname, response.as_str())?;
            return Err(ServerError::InvalidFlags);
        }
    }
    if channel.name.starts_with('#') {
        let mut msg = message.clone();
        msg.prefix = Some(nickname.to_owned());
        let msg = Message::deserialize(msg)?;
        inform_network(network, server_name, &msg)?;
    }
    Ok(())
}

fn handle_ban_flag(
    channel: &mut Channel,
    action: char,
    message: &Message,
    session: &Session,
    network: &Network,
    nickname: &String,
    server_name: &String,
) -> Result<(), ServerError> {
    if message.parameters.len() > 3 {
        return Err(ServerError::InvalidParameters);
    }
    if message.parameters.len() == 2 {
        println!(
            "Ban list for {:?} is {:?}",
            &channel.name, &channel.banned_users
        );
        let response = (CommandResponse::BanList {
            channel: channel.name.clone(),
            ban_list: channel.banned_users.clone(),
        })
        .to_string();
        inform_client(session, nickname, response.as_str())?;
        let response = CommandResponse::EndBanList.to_string();
        inform_client(session, nickname, response.as_str())?;
    } else {
        let user = &message.parameters[2];
        match action {
            '+' => {
                if channel.banned_users.contains(user) {
                    println!("{:?} is already banned from {:?}", user, &channel.name);
                } else {
                    println!("{:?} is now banned from {:?}", user, &channel.name);
                    channel.banned_users.push(user.to_string());
                    for (i, nick) in channel.users.iter().enumerate() {
                        if nick == user {
                            channel.users.remove(i);
                            inform_database(
                                PersistenceType::ChannelUpdate(channel.name.to_owned()),
                                channel.to_string(),
                                session,
                            )?;
                            break;
                        }
                    }
                }
            }
            '-' => {
                if !channel.banned_users.contains(user) {
                    println!("{:?} is already not banned from {:?}", user, &channel.name);
                } else {
                    for (i, nick) in channel.banned_users.iter().enumerate() {
                        if nick == user {
                            channel.banned_users.remove(i);
                            inform_database(
                                PersistenceType::ChannelUpdate(channel.name.to_owned()),
                                channel.to_string(),
                                session,
                            )?;
                            break;
                        }
                    }
                    println!("{:?} is now not banned from {:?}", user, &channel.name);
                }
            }
            _ => {
                let response = ErrorResponse::UnknownModeFlag.to_string();
                inform_client(session, nickname, response.as_str())?;
                return Err(ServerError::InvalidFlags);
            }
        }
        if channel.name.starts_with('#') {
            let mut msg = message.clone();
            msg.prefix = Some(nickname.to_owned());
            let msg = Message::deserialize(msg)?;
            inform_network(network, server_name, &msg)?;
        }
    }

    Ok(())
}

fn handle_user_limit_flag(
    channel: &mut Channel,
    action: char,
    message: &Message,
    session: &Session,
    network: &Network,
    nickname: &String,
    server_name: &String,
) -> Result<(), ServerError> {
    if message.parameters.len() == 2 && action == '-' {
        channel.limit = None;
        println!("{:?} has no user limit", &channel.name);
    } else if message.parameters.len() == 3 && action == '+' {
        channel.limit = match message.parameters[2].parse::<i32>() {
            Ok(limit) => Some(limit),
            Err(_) => {
                return Err(ServerError::Other);
            }
        };
        inform_database(
            PersistenceType::ChannelUpdate(channel.name.to_owned()),
            channel.to_string(),
            session,
        )?;
        println!("Limit is now set to {:?}", message.parameters[2]);
        if channel.name.starts_with('#') {
            let mut msg = message.clone();
            msg.prefix = Some(nickname.to_owned());
            let msg = Message::deserialize(msg)?;
            inform_network(network, server_name, &msg)?;
        }
    } else {
        let response = (ErrorResponse::NeedMoreParams {
            command: "MODE".to_string(),
        })
        .to_string();
        inform_client(session, &channel.name, response.as_str())?;
        return Err(ServerError::InvalidParameters);
    }
    Ok(())
}

fn handle_speak_in_moderated_channel_flag(
    channel: &mut Channel,
    action: char,
    message: &Message,
    session: &Session,
    network: &Network,
    nickname: &String,
    server_name: &String,
) -> Result<(), ServerError> {
    if message.parameters.len() != 3 {
        return Err(ServerError::InvalidParameters);
    }
    let user = &message.parameters[2];
    match action {
        '+' => {
            if channel.moderators.contains(user) {
                println!("{:?} is already a moderator of {:?}", user, &channel.name);
                return Ok(());
            } else {
                println!("{:?} is now a moderator of {:?}", user, &channel.name);
                channel.moderators.push(user.to_string());
                inform_database(
                    PersistenceType::ChannelUpdate(channel.name.to_owned()),
                    channel.to_string(),
                    session,
                )?;
            }
        }
        '-' => {
            if !channel.moderators.contains(user) {
                println!(
                    "{:?} is already not a moderator of {:?}",
                    user, &channel.name
                );
                return Ok(());
            } else {
                for (i, nick) in channel.moderators.iter().enumerate() {
                    if nick == user {
                        channel.moderators.remove(i);
                        inform_database(
                            PersistenceType::ChannelUpdate(channel.name.to_owned()),
                            channel.to_string(),
                            session,
                        )?;
                        break;
                    }
                }
                println!("{:?} is now not a moderator of {:?}", user, &channel.name);
            }
        }
        _ => {
            let response = ErrorResponse::UnknownModeFlag.to_string();
            inform_client(session, nickname, response.as_str())?;
            return Err(ServerError::InvalidFlags);
        }
    }
    if channel.name.starts_with('#') {
        let mut msg = message.clone();
        msg.prefix = Some(nickname.to_owned());
        let msg = Message::deserialize(msg)?;
        inform_network(network, server_name, &msg)?;
    }
    Ok(())
}
