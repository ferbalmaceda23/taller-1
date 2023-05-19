use crate::server_errors::ServerError;
use model::{channel::Channel, channelflag::ChannelFlag, client::Client, userflag::UserFlag};
use std::{collections::HashMap, path::Path};

static CLIENTS_PATH: &str = "server/rsc/clients.txt";
static CHANNELS_PATH: &str = "server/rsc/channels.txt";

/// Function that loads the clients from the file of the database
pub fn load_clients() -> Result<HashMap<String, Client>, ServerError> {
    let mut hash = HashMap::new();
    if Path::new(CLIENTS_PATH).exists() {
        let clients_str = std::fs::read_to_string(CLIENTS_PATH)?;
        let mut clients = clients_str.split('\n').collect::<Vec<_>>();
        clients.pop();
        for client in clients {
            let client = client.split(';').collect::<Vec<_>>();
            let nickname = client[0].to_string();
            let username = client[1].to_string();
            let hostname = client[2].to_string();
            let servername = client[3].to_string();
            let realname = client[4].to_string();
            let mut password = None;
            if !client[5].is_empty() {
                password = Some(client[5].to_string());
            }
            let mut away_message = None;
            if !client[6].is_empty() {
                away_message = Some(client[6].to_string());
            }
            let mut modes = vec![];
            for mode in client[7].split(',') {
                let m = mode.chars().next();
                if let Some(f) = m {
                    modes.push(UserFlag::match_flag(f));
                }
            }

            let new_client = Client {
                nickname: nickname.to_owned(),
                username,
                hostname,
                servername,
                realname,
                password,
                connected: false,
                away_message,
                modes,
            };
            hash.insert(nickname.to_owned(), new_client);
            println!("Client loaded: {}", nickname);
        }
    }
    Ok(hash)
}

/// Function that loads network clients from the loaded clients
/// # Arguments
/// * `hash_clients` - The clients loaded from the database
pub fn load_network_clients(hash_clients: &HashMap<String, Client>) -> HashMap<String, u8> {
    let mut hash = HashMap::new();
    for client in hash_clients.keys() {
        hash.insert(client.to_owned(), 0);
    }
    hash
}

/// Function that loads the channels from the file of the database
pub fn load_channels() -> Result<HashMap<String, Channel>, ServerError> {
    let mut hash = HashMap::new();
    if Path::new(CHANNELS_PATH).exists() {
        let channels_str = std::fs::read_to_string(CHANNELS_PATH)?;
        let mut channels = channels_str.split('\n').collect::<Vec<_>>();
        channels.pop();
        for channel in channels {
            let channel = channel.split(';').collect::<Vec<_>>();
            if channel.len() != 9 {
                continue;
            }
            let name = channel[0].to_string();
            let topic = channel[1].to_string();
            let users = channel[2]
                .split(',')
                .map(|u| u.to_string())
                .collect::<Vec<_>>();
            let mut password = None;
            if !channel[3].is_empty() {
                password = Some(channel[3].to_string());
            }
            let banned_users = channel[4]
                .split(',')
                .filter_map(|u| {
                    if !u.is_empty() {
                        Some(u.to_string())
                    } else {
                        None
                    }
                })
                .collect::<Vec<String>>();
            let operators = channel[5]
                .split(',')
                .map(|u| u.to_string())
                .collect::<Vec<_>>();
            let mut modes = vec![];
            for mode in channel[6].split(',') {
                let m = mode.chars().next();
                if let Some(f) = m {
                    modes.push(ChannelFlag::match_flag(f));
                }
            }
            let mut limit = None;
            if !channel[7].is_empty() {
                limit = match channel[7].parse::<i32>() {
                    Ok(l) => Some(l),
                    Err(_) => None,
                }
            }
            let moderators = channel[8]
                .split(',')
                .map(|u| u.to_string())
                .collect::<Vec<_>>();

            let new_channel = Channel {
                name: name.to_owned(),
                topic,
                password,
                users,
                operators,
                modes,
                banned_users,
                limit,
                moderators,
            };
            hash.insert(name.to_owned(), new_channel);
            println!("Channel loaded: {}", name);
        }
    }
    Ok(hash)
}
