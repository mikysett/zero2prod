use validator::validate_email;

#[derive(Debug)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(s: String) -> Result<SubscriberEmail, String> {
        if validate_email(&s) {
            Ok(Self(s))
        } else {
            Err(format!("{} is not a valid subscriber email", s))
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SubscriberEmail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::SubscriberEmail;
    use claims::assert_err;
    use fake::faker::internet::en::SafeEmail;
    use fake::Fake;

    #[derive(Debug, Clone)]
    struct ValidEmailFixture(pub String);

    impl quickcheck::Arbitrary for ValidEmailFixture {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            let email = SafeEmail().fake_with_rng(g);
            Self(email)
        }
    }

    #[test]
    fn empty_string_is_rejected() {
        let name = "".to_string();
        assert_err!(SubscriberEmail::parse(name));
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let name = "ursuladomain.com".to_string();
        assert_err!(SubscriberEmail::parse(name));
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        let name = "@domain.com".to_string();
        assert_err!(SubscriberEmail::parse(name));
    }

    #[quickcheck_macros::quickcheck]
    fn valid_emails_are_parsed_successfully(
        valid_email: ValidEmailFixture,
    ) -> bool {
        SubscriberEmail::parse(valid_email.0).is_ok()
    }
}
