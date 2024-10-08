use secrecy::Zeroize;

pub struct Password(String);

pub enum PasswordError {
    TooShort,
    TooLong,
}

impl Password {
    pub fn parse(s: String) -> Result<Password, PasswordError> {
        let password_len = s.len();
        if password_len < 12 {
            return Err(PasswordError::TooShort);
        } else if password_len > 128 {
            return Err(PasswordError::TooLong);
        }
        Ok(Password(s))
    }
}

impl AsRef<str> for Password {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// To use Password in Secret<Password>
impl Zeroize for Password {
    fn zeroize(&mut self) {
        self.0 = "".to_string()
    }
}
