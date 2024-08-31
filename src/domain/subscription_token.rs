use rand::distributions::Alphanumeric;
use rand::thread_rng;
use rand::Rng;

const SUBSCRIPTION_TOKEN_SIZE: usize = 25;

#[derive(Debug)]
pub struct SubscriptionToken(String);

impl SubscriptionToken {
    pub fn parse(token: String) -> Result<Self, String> {
        match (
            token.chars().all(|c| c.is_alphanumeric()),
            token.len() == SUBSCRIPTION_TOKEN_SIZE,
        ) {
            (true, true) => Ok(Self(token)),
            (true, false) => Err(format!(
                "{} is not a valid subscription token: incorrect size",
                token
            )),
            (false, true) => Err(format!(
                "{} is not a valid subscription token: invalid characters",
                token
            )),
            _ => Err(format!("{} is not a valid subscription token", token)),
        }
    }

    pub fn generate_subscription_token() -> Self {
        let mut rng = thread_rng();
        let token = std::iter::repeat_with(|| rng.sample(Alphanumeric))
            .map(char::from)
            .take(SUBSCRIPTION_TOKEN_SIZE)
            .collect();

        Self(token)
    }
}

impl AsRef<str> for SubscriptionToken {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod test {
    use claims::{assert_err, assert_ok};
    use rand::{distributions::Alphanumeric, Rng};

    use crate::domain::SubscriptionToken;

    use super::SUBSCRIPTION_TOKEN_SIZE;

    #[test]
    fn empty_string_is_rejected() {
        let token = "".to_string();
        assert_err!(SubscriptionToken::parse(token));
    }

    #[test]
    fn non_alphanumeric_string_is_rejected() {
        let mut rng = rand::thread_rng();
        let non_alphanumeric_chars: Vec<char> = vec![
            '!', '@', '#', '$', '%', '^', '&', '*', '(', ')', '-', '_', '=',
            '+', '[', ']', '{', '}', '\\', '|', ';', ':', '\'', '"', ',', '<',
            '.', '>', '/', '?', '`', '~',
        ];
        let one_char = || {
            non_alphanumeric_chars
                [rng.gen_range(0..non_alphanumeric_chars.len())]
                as char
        };
        let invalid_token = std::iter::repeat_with(one_char)
            .take(SUBSCRIPTION_TOKEN_SIZE)
            .collect();

        assert_err!(SubscriptionToken::parse(invalid_token));
    }

    #[test]
    fn valid_token_is_parsed() {
        let valid_token = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(SUBSCRIPTION_TOKEN_SIZE)
            .map(char::from)
            .collect();

        assert_ok!(SubscriptionToken::parse(valid_token));
    }
}
