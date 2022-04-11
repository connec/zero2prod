#[derive(Clone, Copy)]
pub(crate) enum SubscriberStatus {
    Pending,
    Confirmed,
}

impl SubscriberStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Confirmed => "confirmed",
        }
    }
}

impl std::str::FromStr for SubscriberStatus {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, crate::Error> {
        match s {
            "pending" => Ok(Self::Pending),
            "confirmed" => Ok(Self::Confirmed),
            _ => Err(crate::Error::Internal(eyre::Report::msg(format!(
                "unknown subscriber status: {}",
                s
            )))),
        }
    }
}
