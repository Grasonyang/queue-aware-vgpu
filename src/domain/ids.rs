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

/// Defines a validated domain identifier backed by a `String`.
/// The generated identifier rejects empty or whitespace-only values.
macro_rules! define_id {
    ($name:ident) => {
        // ident 是一種類型，匹配 Rust 的識別字，例如型別名、函式名、變數名
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            /// build a new id by input string
            // Into 可以自動轉換輸入值為 String
            pub fn new(value: impl Into<String>) -> Result<Self, DomainIdError> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(DomainIdError::Empty);
                }
                Ok(Self(value))
            }

            /// rust 借用字串，外部 fn 可以此獲取實際的 id: String
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

/// Identifies a queue together with the tenant that owns it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TenantQueueId {
    tenant: TenantId,
    queue: QueueId,
}

impl TenantQueueId {
    pub const fn new(tenant: TenantId, queue: QueueId) -> Self {
        Self { tenant, queue }
    }

    pub fn tenant(&self) -> &TenantId {
        &self.tenant
    }

    pub fn queue(&self) -> &QueueId {
        &self.queue
    }
}
