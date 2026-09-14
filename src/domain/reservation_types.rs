use super::{GpuMemoryMib, ReservationId, WorkloadId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReservationStatus {
    Held,
    ReleasePending,
    Released,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reservation {
    pub(super) id: ReservationId,
    pub(super) workload: WorkloadId,
    pub(super) amount: GpuMemoryMib,
    pub(super) status: ReservationStatus,
}

impl Reservation {
    pub fn id(&self) -> &ReservationId {
        &self.id
    }

    pub fn workload(&self) -> &WorkloadId {
        &self.workload
    }

    pub const fn amount(&self) -> GpuMemoryMib {
        self.amount
    }

    pub const fn status(&self) -> ReservationStatus {
        self.status
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReservationError {
    ZeroAmount,
    Duplicate,
    Unknown,
    CapacityExceeded,
    InvalidReleasePermit,
}
