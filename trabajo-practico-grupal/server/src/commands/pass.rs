use model::message::Message;

use crate::server_errors::ServerError;

/// Function that sets the password of a new client connected.
/// If already connected it returns an error.
/// # Arguments
/// * `message` - Message received from the client.
/// * `nickname` - Nickname of the client.
/// * `password` - Password of the client.
/// * `user_parameters` - username, hostname, realname y servername of the client.
pub fn handle_pass_command(
    message: Message,
    nickname: &mut Option<String>,
    user_parameters: &mut Option<Vec<String>>,
    password: &mut Option<String>,
) -> Result<(), ServerError> {
    if message.parameters.len() > 1 {
        return Err(ServerError::InvalidParameters);
    }
    if message.parameters.is_empty() {
        return Ok(());
    }
    println!("PASS {}", message.parameters[0]);
    if nickname.is_some() {
        return Err(ServerError::PassMustBeSetBeforeNickname);
    }
    if user_parameters.is_some() {
        return Err(ServerError::PassMustBeSetBeforeUser);
    }
    *password = Some(message.parameters[0].to_owned());
    Ok(())
}

#[cfg(test)]
mod pass_tests {
    use model::message::MessageType;

    use super::handle_pass_command;
    use crate::{commands::command_utils::create_message_for_test, server_errors::ServerError};

    #[test]
    fn test_pass_invalid_parameters() {
        let msg = create_message_for_test(
            MessageType::Pass,
            vec!["pass".to_string(), "word".to_string()],
        );
        let mut nickname = None;
        let mut username = None;
        let mut password = None;
        let result = handle_pass_command(msg, &mut nickname, &mut username, &mut password);

        assert_eq!(Err(ServerError::InvalidParameters), result);
    }

    #[test]
    fn test_pass_username_not_none() {
        let msg = create_message_for_test(MessageType::Pass, vec!["password".to_string()]);
        let mut nickname = None;
        let mut user_parameters = Some(vec![
            "username".to_string(),
            "hostname".to_string(),
            "servername".to_string(),
            "realname".to_string(),
        ]);
        let mut password = None;

        let result = handle_pass_command(msg, &mut nickname, &mut user_parameters, &mut password);

        assert_eq!(Err(ServerError::PassMustBeSetBeforeUser), result);
    }

    #[test]
    fn test_pass_nickname_not_none() {
        let msg = create_message_for_test(MessageType::Pass, vec!["password".to_string()]);
        let mut nickname = Some("nickname".to_string());
        let mut user_parameters = None;
        let mut password = None;
        let result = handle_pass_command(msg, &mut nickname, &mut user_parameters, &mut password);

        assert_eq!(Err(ServerError::PassMustBeSetBeforeNickname), result);
    }

    #[test]
    fn test_pass_nickname_and_username_need_to_be_set_after() {
        let msg = create_message_for_test(MessageType::Pass, vec!["password".to_string()]);
        let mut nickname = None;
        let mut user_parameters = None;
        let mut password = None;
        let result = handle_pass_command(msg, &mut nickname, &mut user_parameters, &mut password);

        assert!(result.is_ok());
        assert_eq!(password, Some("password".to_string()));
    }
}
