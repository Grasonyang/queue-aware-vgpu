//! Kubernetes-independent contracts for the V1 execution chain.

mod cluster;
mod ids;
mod job;
mod job_decision;
mod lifecycle;
mod lifecycle_release;
mod lifecycle_types;
mod memory;
mod queue;
mod reservation;
mod reservation_types;

pub use cluster::{ClusterSnapshot, TenantBudget};
pub use ids::{DomainIdError, QueueId, ReservationId, TenantId, WorkloadId};
pub use job::{AdmissionError, JobSpec};
pub use job_decision::{JobDecision, RejectReason, ReservationIntent, WaitReason};
pub use lifecycle::WorkloadLifecycle;
pub use lifecycle_types::{
    FailureReason, LifecycleError, ReleasePermit, ReleaseState, WaitMode, WaitingPhase,
    WorkloadState,
};
pub use memory::GpuMemoryMib;
pub use queue::{QueueError, TenantQueue};
pub use reservation::ReservationLedger;
pub use reservation_types::{Reservation, ReservationError, ReservationStatus};
