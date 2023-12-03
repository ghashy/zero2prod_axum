use rand::{distributions::Alphanumeric, thread_rng, Rng};

#[derive(Debug)]
pub struct SubscriberToken(String);

impl SubscriberToken {
    pub fn parse(name: &str) -> Result<SubscriberToken, &'static str> {
        let is_empty_or_whitespace = name.trim().is_empty();
        let is_too_long = name.chars().count() != 25;
        let contains_forbidden_chars =
            name.chars().any(|c| !c.is_alphanumeric());

        if is_empty_or_whitespace {
            Err("String is emtpy")
        } else if is_too_long {
            Err("String is too long")
        } else if contains_forbidden_chars {
            Err("String contains forbidden chars")
        } else {
            Ok(SubscriberToken(name.to_string()))
        }
    }

    /// Using 25 characters we get roughly ~10^45 possible tokens -
    /// it should be more than enough for our use case.
    pub fn generate() -> SubscriberToken {
        let mut rng = thread_rng();
        SubscriberToken(
            std::iter::repeat_with(|| rng.sample(Alphanumeric))
                .map(char::from)
                .take(25)
                .collect(),
        )
    }
}

impl AsRef<str> for SubscriberToken {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
