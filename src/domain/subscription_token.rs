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
