// ───── Current Crate Imports ────────────────────────────────────────────── //

use super::subscriber_email::SubscriberEmail;
use super::subscriber_name::SubscriberName;

// ───── Body ─────────────────────────────────────────────────────────────── //

pub struct NewSubscriber {
    pub email: SubscriberEmail,
    pub name: SubscriberName,
}
