use super::{subscriber_email::SubscriberEmail, subscriber_name::SubscriberName};

pub(crate) struct NewSubscriber {
    pub email: SubscriberEmail,
    pub name: SubscriberName,
}
