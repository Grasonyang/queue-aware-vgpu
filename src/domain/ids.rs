use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DomainIdError {
    Empty,
}

impl fmt::Display for DomainIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("domain identifiers must not be empty"),
        }
    }
}

impl std::error::Error for DomainIdError {}

macro_rules! define_id {
    ($name:ident) => {
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, DomainIdError> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(DomainIdError::Empty);
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

define_id!(TenantId);
define_id!(QueueId);
define_id!(WorkloadId);
define_id!(ReservationId);
