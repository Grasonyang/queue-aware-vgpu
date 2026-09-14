use super::ReservationId;
use chrono::{DateTime, Duration, Utc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaitMode {
    Forever,
    Timeout(Duration),
}

impl WaitMode {
    pub fn timeout(duration: Duration) -> Result<Self, LifecycleError> {
        if duration <= Duration::zero() {
            return Err(LifecycleError::InvalidTimeout);
        }
        Ok(Self::Timeout(duration))
    }

    pub fn deadline(self, accepted_at: DateTime<Utc>) -> Option<DateTime<Utc>> {
        match self {
            Self::Forever => None,
            Self::Timeout(duration) => accepted_at.checked_add_signed(duration),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaitingPhase {
    Queued,
    Allocating,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkloadState {
    Waiting(WaitingPhase),
    Running,
    Succeeded,
    Failed(FailureReason),
    TimedOut,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FailureReason {
    Allocation,
    Execution,
    External,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReleaseState {
    NotRequired,
    Held,
    Pending,
    Confirmed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LifecycleError {
    InvalidTimeout,
    InvalidTransition,
    ReservationAlreadyHeld,
    ReservationMissing,
    ReservationMismatch,
    ReleaseNotPending,
    NotExpired,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleasePermit {
    reservation: ReservationId,
}

impl ReleasePermit {
    pub(crate) fn new(reservation: ReservationId) -> Self {
        Self { reservation }
    }

    pub fn reservation(&self) -> &ReservationId {
        &self.reservation
    }
}
