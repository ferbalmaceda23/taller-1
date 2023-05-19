/*
modos:
+i -> invite only
+p -> private channel
+s -> secret channel
+t -> topic settable by channel operator only
+m -> moderated channel
*/
use crate::channelflag::ChannelFlag;
use std::fmt::Display;

/// Struct that represents a channel.
/// # Fields
/// * `name`: The name of the channel.
/// * `topic`: The topic of the channel.
/// * `users`: Vector that contains the users that are in the channel.
/// * `operators`: Vector that contains the operators (users with special permissions) of the channel.
/// * `banned_users`: Vector that contains the banned users of the channel.
/// * `password`: The password of the channel.
/// * `modes`: The modes of the channel.
/// * `limit`: The limit of users that can be in the channel.
/// * `moderators`: The users that can talk in a moderated channel.
#[derive(Debug, Clone)]
pub struct Channel {
    pub name: String,
    pub topic: String,
    pub users: Vec<String>,
    pub operators: Vec<String>,
    pub banned_users: Vec<String>,
    pub password: Option<String>,
    pub modes: Vec<ChannelFlag>,
    pub limit: Option<i32>,
    pub moderators: Vec<String>,
}
impl Channel {
    /// Creates a new instance of the channel.
    /// # Arguments
    /// * `name` - The name of the channel.
    /// * `topic` - The topic of the channel.
    /// * `users` - Vector that contains the users that are in the channel.
    pub fn new(name: String, topic: String, users: Vec<String>) -> Channel {
        Channel {
            name,
            topic,
            users,
            password: None,
            banned_users: Vec::new(),
            operators: Vec::new(),
            modes: Vec::new(),
            limit: None,
            moderators: Vec::new(),
        }
    }
}

impl Display for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut channel_data = vec![
            self.name.to_owned(),
            self.topic.to_owned(),
            self.users.join(","),
        ];
        let mut pass = "".to_string();
        if let Some(p) = self.password.to_owned() {
            pass = p;
        }
        channel_data.push(pass);
        channel_data.push(self.banned_users.join(","));
        channel_data.push(self.operators.join(","));
        channel_data.push(
            self.modes
                .iter()
                .cloned()
                .map(|x| ChannelFlag::to_string(&x))
                .collect::<Vec<_>>()
                .join(","),
        );
        let mut limit = "".to_string();
        if let Some(l) = self.limit {
            limit = l.to_string();
        }
        channel_data.push(limit);
        channel_data.push(self.moderators.join(","));
        write!(f, "{}", channel_data.join(";"))
    }
}
