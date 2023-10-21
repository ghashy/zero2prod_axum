/// This type guarantees correctness of `subscriber's` email address.
#[derive(Clone)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(email: &str) -> Result<Self, &'static str> {
        if validator::validate_email(email) {
            Ok(Self(email.to_string()))
        } else {
            Err("{} is not a valid subscriber email.")
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::SubscriberEmail;
    use fake::faker::internet::en::SafeEmail;
    use fake::Fake;

    #[test]
    fn valied_emails_are_parsed_successfully() {
        let mut rng = rand::thread_rng();
        for _ in 0..100 {
            let valid_email: String = SafeEmail().fake_with_rng(&mut rng);
            assert!(SubscriberEmail::parse(&valid_email).is_ok());
        }
    }

    #[test]
    fn empty_string_is_rejected() {
        let email = "".to_string();
        assert!(SubscriberEmail::parse(&email).is_err())
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "ursuladomail.com".to_string();
        assert!(SubscriberEmail::parse(&email).is_err())
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@domail.com".to_string();
        assert!(SubscriberEmail::parse(&email).is_err())
    }

    #[test]
    fn valid_emails_are_parsed_successfully() {
        let email: String = SafeEmail().fake();
        assert!(SubscriberEmail::parse(&email).is_ok())
    }
}
