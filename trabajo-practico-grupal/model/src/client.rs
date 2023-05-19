use crate::userflag::UserFlag;
use std::fmt::Display;

/// Struct that represents a client.
/// # Fields
/// * `nick`: The nick of the client, it identifies the client.
/// * `username`: The user of the client.
/// * `realname`: The real name of the client.
/// * `hostname`: The hostname of the client.
/// * `servername`: The servername of the client.
/// * `password`: The password of the client. It can be None.
/// * `connected`: A boolean that indicates if the client is connected.
/// * `away_message`: When it is Some, it is the message that is sent to other clients when they send a PRIVMSG to the client.
/// * `modes`: Vector that contains the modes of the client.
#[derive(Debug, Clone)]
pub struct Client {
    pub username: String,
    pub nickname: String,
    pub hostname: String,
    pub servername: String,
    pub realname: String,
    pub password: Option<String>,
    pub connected: bool,
    pub away_message: Option<String>,
    pub modes: Vec<UserFlag>,
}

impl Client {
    ///Creates a new client with the given parameters.
    pub fn from_connection(
        nickname: String,
        username: String,
        hostname: String,
        servername: String,
        realname: String,
        password: Option<String>,
        connected: bool,
    ) -> Client {
        Client {
            nickname,
            username,
            hostname,
            servername,
            realname,
            password,
            connected,
            away_message: None,
            modes: Vec::new(),
        }
    }
}

impl Display for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut client_data = vec![
            self.nickname.to_owned(),
            self.username.to_owned(),
            self.hostname.to_owned(),
            self.servername.to_owned(),
            self.realname.to_owned(),
        ];
        let mut pass = "".to_string();
        if let Some(p) = self.password.to_owned() {
            pass = p;
        }
        client_data.push(pass);
        let mut away = "".to_string();
        if let Some(a) = self.away_message.to_owned() {
            away = a;
        }
        client_data.push(away);
        client_data.push(
            self.modes
                .iter()
                .cloned()
                .map(|x| UserFlag::to_string(&x))
                .collect::<Vec<_>>()
                .join(","),
        );
        write!(f, "{}", client_data.join(";"))
    }
}
