// ───── Body ─────────────────────────────────────────────────────────────── //

/// This type guarantees us that `SubscriberName` is properly formed.
pub struct SubscriberName(String);

impl SubscriberName {
    /// Returns an instance of `SubscriberName` if the input satisfies
    /// our validation constraints on subscriber names.
    pub fn parse(name: &str) -> Result<SubscriberName, &'static str> {
        let is_empty_or_whitespace = name.trim().is_empty();
        let is_too_long = name.chars().count() > 256;
        let forbidden_characters =
            ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];
        let contains_forbidden_chars =
            name.chars().any(|g| forbidden_characters.contains(&g));

        if is_empty_or_whitespace {
            Err("String is emtpy")
        } else if is_too_long {
            Err("String is too long")
        } else if contains_forbidden_chars {
            Err("String contains forbidden chars")
        } else {
            Ok(SubscriberName(name.to_string()))
        }
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// ───── Unit tests ───────────────────────────────────────────────────────── //

#[cfg(test)]
mod tests {
    use super::SubscriberName;

    #[test]
    fn a_256_char_long_name_is_valid() {
        let name = "a".repeat(256);
        assert!(SubscriberName::parse(&name).is_ok());
    }

    #[test]
    fn a_name_longer_than_256_chars_is_rejected() {
        let name = "a".repeat(257);
        assert!(SubscriberName::parse(&name).is_err());
    }

    #[test]
    fn whitespace_only_names_are_rejected() {
        let name = " ".to_string();
        assert!(SubscriberName::parse(&name).is_err());
    }

    #[test]
    fn emtpy_string_is_rejected() {
        let name = "".to_string();
        assert!(SubscriberName::parse(&name).is_err());
    }

    #[test]
    fn names_contanining_an_invalid_character_are_rejected() {
        for name in &['/', '(', ')', '"', '<', '>', '\\', '{', '}'] {
            let name = name.to_string();
            assert!(SubscriberName::parse(&name).is_err());
        }
    }

    #[test]
    fn a_valid_name_is_parsed_successfully() {
        let name = "Ursula Le Guin".to_string();
        assert!(SubscriberName::parse(&name).is_ok());
    }
}
