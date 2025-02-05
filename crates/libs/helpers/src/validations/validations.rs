// An extension trait to provide the `graphemes` method on `String` and `&str`
use unicode_segmentation::UnicodeSegmentation;
use serde::Deserialize;
use argon2::{self, password_hash::SaltString};
use errors::CustomError;

#[derive(Debug)]
pub struct UserName(String);

impl UserName {
    pub fn parse(s: String) -> std::result::Result<UserName, CustomError> {
        let is_empty_or_whitespace = s.trim().is_empty();
        let is_too_long = s.graphemes(true).count() > 256;
        let forbidden_characters = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];
        let contains_forbidden_characters = s.chars().any(|c| forbidden_characters.contains(&c));

        if is_empty_or_whitespace || is_too_long || contains_forbidden_characters {
            Err(CustomError::ValidationError(format!("{} is not a valid name", s)))
        } else {
            Ok(Self(s))
        }
    }
}
impl AsRef<str> for UserName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

use regex::Regex;

#[derive(Debug)]
pub struct UserEmail(String);

impl UserEmail {
    pub fn parse(s: String) -> std::result::Result<UserEmail, CustomError> {
        let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
        .map_err(|e| CustomError::ValidationError(format!("Invalid regex: {}", e)))?;

        if email_regex.is_match(&s) {
            Ok(Self(s))
        } else {
            Err(CustomError::ValidationError(format!("{} is not a valid email address.", s)))
        }
    }
}

impl AsRef<str> for UserEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Deserialize)]
pub struct CreateUserBody {
    pub username: String,
    pub password: String,
    pub email: String,
}
impl CreateUserBody {
    pub fn validate(self) -> Result<(UserName, UserEmail), CustomError> {
        let user_name = UserName::parse(self.username)?;
        let user_email = UserEmail::parse(self.email)?;
        Ok((user_name, user_email))
    }
}
#[derive(Deserialize)]
pub struct UpdateUserBody {
    pub username: String,
    pub email: String,
}
impl UpdateUserBody {
    pub fn validate(self) -> Result<(UserName, UserEmail), CustomError> {
        let user_name = UserName::parse(self.username)?;
        let user_email = UserEmail::parse(self.email)?;
        Ok((user_name, user_email))
    }
}


#[derive(Deserialize)]
pub struct LoginUserBody {
    pub email: String,
    pub password: String,
}

pub fn generate_random_salt() -> SaltString {
    let mut rng = rand::thread_rng();
    SaltString::generate(&mut rng)
}



#[cfg(test)]
mod tests {
    use super::{UserEmail, UserName};
    use claim::{assert_err, assert_ok};

    #[test]
    fn a_256_grapheme_long_name_is_valid() {
        let name = "a".repeat(256);
        assert_ok!(UserName::parse(name));
    }

    #[test]
    fn a_name_longer_than_256_graphemes_is_rejected() {
        let name = "a".repeat(257);
        assert_err!(UserName::parse(name));
    }

    #[test]
    fn whitespace_only_names_are_rejected() {
        let name = " ".to_string();
        assert_err!(UserName::parse(name));
    }

    #[test]
    fn empty_string_is_rejected() {
        let name = "".to_string();
        assert_err!(UserName::parse(name));
    }

    #[test]
    fn names_containing_an_invalid_character_are_rejected() {
        for name in &['/', '(', ')', '"', '<', '>', '\\', '{', '}'] {
            let name = name.to_string();
            assert_err!(UserName::parse(name));
        }
    }

    #[test]
    fn a_valid_name_is_parsed_successfully() {
        let name = "Kashish Kashyap".to_string();
        assert_ok!(UserName::parse(name));
    }

    #[test]
    fn empty_email_string_is_rejected() {
        let email = "".to_string();
        assert_err!(UserEmail::parse(email));
    }
    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "kkgmail.com".to_string();
        assert_err!(UserEmail::parse(email));
    }
    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@gmail.com".to_string();
        assert_err!(UserEmail::parse(email));
    }
}
