use std::fmt::Display;

/// The user flags that can be set on a user.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum UserFlag {
    Invisible,
    ServerNotice,
    Wallops,
    Operator,
    Other,
}

impl UserFlag {
    /// Returns the character representation of the user flag.
    pub fn to_string(flag: &UserFlag) -> String {
        match flag {
            UserFlag::Invisible => "i".to_string(),
            UserFlag::ServerNotice => "s".to_string(),
            UserFlag::Wallops => "w".to_string(),
            UserFlag::Operator => "o".to_string(),
            UserFlag::Other => "-".to_string(),
        }
    }

    /// Returns the user flag from the character representation.
    pub fn match_flag(flag: char) -> UserFlag {
        match flag {
            'i' => UserFlag::Invisible,
            's' => UserFlag::ServerNotice,
            'w' => UserFlag::Wallops,
            'o' => UserFlag::Operator,
            _ => UserFlag::Other,
        }
    }

    /// Returns a vector of user flags.
    pub fn iter() -> Vec<UserFlag> {
        vec![
            UserFlag::Invisible,
            UserFlag::ServerNotice,
            UserFlag::Wallops,
            UserFlag::Operator,
            UserFlag::Other,
        ]
    }
}

impl Display for UserFlag {
    /// Gives format to the user flag.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let r = match self {
            UserFlag::Invisible => "Invisible".to_string(),
            UserFlag::ServerNotice => "ServerNotice".to_string(),
            UserFlag::Wallops => "Wallops".to_string(),
            UserFlag::Operator => "Operator".to_string(),
            UserFlag::Other => "Other".to_string(),
        };
        write!(f, "{}", r)
    }
}
