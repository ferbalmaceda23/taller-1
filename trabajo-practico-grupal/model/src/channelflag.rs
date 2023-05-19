use std::fmt::Display;

///Enum that represents the flags of the channel, to set the channel mode.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ChannelFlag {
    Private,
    Secret,
    InviteOnly,
    TopicSettableOnlyOperators,
    NoMessageFromOutside,
    ModeratedChannel,
    ChannelOperator,
    UserLimit,
    Ban,
    ChannelKey,
    SpeakInModeratedChannel,
    Other,
}

impl ChannelFlag {
    /// Returns the string that represents the flag.
    pub fn to_string(flag: &ChannelFlag) -> String {
        match flag {
            ChannelFlag::Private => "p".to_string(),
            ChannelFlag::Secret => "s".to_string(),
            ChannelFlag::InviteOnly => "i".to_string(),
            ChannelFlag::TopicSettableOnlyOperators => "t".to_string(),
            ChannelFlag::NoMessageFromOutside => "n".to_string(),
            ChannelFlag::ModeratedChannel => "m".to_string(),
            ChannelFlag::UserLimit => "l".to_string(),
            ChannelFlag::Ban => "b".to_string(),
            ChannelFlag::ChannelOperator => "o".to_string(),
            ChannelFlag::ChannelKey => "k".to_string(),
            ChannelFlag::SpeakInModeratedChannel => "v".to_string(),
            ChannelFlag::Other => "-".to_string(),
        }
    }

    /// Returns the flag that corresponds to the given string.
    pub fn match_flag(flag: char) -> ChannelFlag {
        match flag {
            'p' => ChannelFlag::Private,
            's' => ChannelFlag::Secret,
            'i' => ChannelFlag::InviteOnly,
            't' => ChannelFlag::TopicSettableOnlyOperators,
            'n' => ChannelFlag::NoMessageFromOutside,
            'm' => ChannelFlag::ModeratedChannel,
            'l' => ChannelFlag::UserLimit,
            'b' => ChannelFlag::Ban,
            'k' => ChannelFlag::ChannelKey,
            'v' => ChannelFlag::SpeakInModeratedChannel,
            'o' => ChannelFlag::ChannelOperator,
            _ => ChannelFlag::Other,
        }
    }

    /// Return a vector of channel flags to iterate them.
    pub fn iter() -> Vec<ChannelFlag> {
        vec![
            ChannelFlag::Private,
            ChannelFlag::Secret,
            ChannelFlag::InviteOnly,
            ChannelFlag::TopicSettableOnlyOperators,
            ChannelFlag::NoMessageFromOutside,
            ChannelFlag::ModeratedChannel,
            ChannelFlag::UserLimit,
            ChannelFlag::Ban,
            ChannelFlag::ChannelKey,
            ChannelFlag::SpeakInModeratedChannel,
            ChannelFlag::ChannelOperator,
        ]
    }
}

impl Display for ChannelFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let r = match self {
            ChannelFlag::Private => "Private".to_string(),
            ChannelFlag::Secret => "Secret".to_string(),
            ChannelFlag::InviteOnly => "InviteOnly".to_string(),
            ChannelFlag::TopicSettableOnlyOperators => "TopicSettableOnlyOperators".to_string(),
            ChannelFlag::NoMessageFromOutside => "NoMessageFromOutside".to_string(),
            ChannelFlag::ModeratedChannel => "ModeratedChannel".to_string(),
            ChannelFlag::UserLimit => "UserLimit".to_string(),
            ChannelFlag::Ban => "Ban".to_string(),
            ChannelFlag::ChannelOperator => "ChannelOperator".to_string(),
            ChannelFlag::ChannelKey => "ChannelKey".to_string(),
            ChannelFlag::SpeakInModeratedChannel => "SpeakInModeratedChannel".to_string(),
            ChannelFlag::Other => "Other".to_string(),
        };
        write!(f, "{}", r)
    }
}
