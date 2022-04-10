mod new_subscriber;
mod subscriber_email;
mod subscriber_name;

use std::fmt;

pub(crate) use self::{new_subscriber::NewSubscriber, subscriber_name::SubscriberName};

pub use self::subscriber_email::SubscriberEmail;

#[derive(Debug)]
pub(crate) struct Error(String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Error> for crate::Error {
    fn from(error: Error) -> Self {
        Self::Validation(error.to_string())
    }
}
