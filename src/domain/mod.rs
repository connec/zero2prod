mod new_subscriber;
mod subscriber_email;
mod subscriber_name;

pub(crate) use self::{
    new_subscriber::NewSubscriber, subscriber_email::SubscriberEmail,
    subscriber_name::SubscriberName,
};

#[derive(Debug)]
pub(crate) struct Error(String);

impl From<Error> for crate::Error {
    fn from(error: Error) -> Self {
        Self::Validation(error.0)
    }
}
