use serde::Deserialize;

// ───── Current Crate Imports ────────────────────────────────────────────── //

use crate::routes::FormData;

use super::subscriber_email::SubscriberEmail;
use super::subscriber_name::SubscriberName;

// ───── Body ─────────────────────────────────────────────────────────────── //

pub struct NewSubscriber {
    pub email: SubscriberEmail,
    pub name: SubscriberName,
}

impl TryFrom<&FormData> for NewSubscriber {
    type Error = String;
    fn try_from(form: &FormData) -> Result<Self, Self::Error> {
        let name = match SubscriberName::parse(&form.name) {
            Ok(v) => v,
            Err(e) => {
                return Err(e.to_string());
            }
        };
        let email = match SubscriberEmail::parse(&form.email) {
            Ok(v) => v,
            Err(e) => {
                return Err(e.to_string());
            }
        };
        Ok(NewSubscriber { email, name })
    }
}
