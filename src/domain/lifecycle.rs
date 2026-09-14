use super::WorkloadState;
use super::{FailureReason, LifecycleError, ReleaseState, WaitMode, WaitingPhase};
use super::{GpuMemoryMib, ReservationId, WorkloadId};
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkloadLifecycle {
    workload: WorkloadId,
    requested_memory: GpuMemoryMib,
    accepted_at: DateTime<Utc>,
    wait_mode: WaitMode,
    state: WorkloadState,
    pub(super) reservation: Option<ReservationId>,
    pub(super) release_state: ReleaseState,
}

impl WorkloadLifecycle {
    pub(crate) fn new(
        workload: WorkloadId,
        requested_memory: GpuMemoryMib,
        wait_mode: WaitMode,
        accepted_at: DateTime<Utc>,
    ) -> Self {
        Self {
            workload,
            requested_memory,
            accepted_at,
            wait_mode,
            state: WorkloadState::Waiting(WaitingPhase::Queued),
            reservation: None,
            release_state: ReleaseState::NotRequired,
        }
    }

    pub fn workload(&self) -> &WorkloadId {
        &self.workload
    }

    pub const fn requested_memory(&self) -> GpuMemoryMib {
        self.requested_memory
    }

    pub const fn accepted_at(&self) -> DateTime<Utc> {
        self.accepted_at
    }

    pub const fn state(&self) -> &WorkloadState {
        &self.state
    }

    pub const fn release_state(&self) -> ReleaseState {
        self.release_state
    }

    pub fn deadline(&self) -> Option<DateTime<Utc>> {
        self.wait_mode.deadline(self.accepted_at)
    }

    pub fn reserve(&mut self, reservation: ReservationId) -> Result<(), LifecycleError> {
        if self.reservation.is_some() {
            return Err(LifecycleError::ReservationAlreadyHeld);
        }
        if !matches!(self.state, WorkloadState::Waiting(WaitingPhase::Queued)) {
            return Err(LifecycleError::InvalidTransition);
        }
        self.reservation = Some(reservation);
        self.release_state = ReleaseState::Held;
        self.state = WorkloadState::Waiting(WaitingPhase::Allocating);
        Ok(())
    }

    pub fn mark_running(&mut self) -> Result<(), LifecycleError> {
        if !matches!(self.state, WorkloadState::Waiting(WaitingPhase::Allocating)) {
            return Err(LifecycleError::InvalidTransition);
        }
        self.state = WorkloadState::Running;
        Ok(())
    }

    pub fn mark_succeeded(&mut self) -> Result<(), LifecycleError> {
        if self.state != WorkloadState::Running {
            return Err(LifecycleError::InvalidTransition);
        }
        self.finish(WorkloadState::Succeeded)
    }

    pub fn mark_failed(&mut self, reason: FailureReason) -> Result<(), LifecycleError> {
        self.finish(WorkloadState::Failed(reason))
    }

    pub fn expire(&mut self, now: DateTime<Utc>) -> Result<(), LifecycleError> {
        if !matches!(self.state, WorkloadState::Waiting(_)) {
            return Err(LifecycleError::InvalidTransition);
        }
        if self.deadline().is_none_or(|deadline| now < deadline) {
            return Err(LifecycleError::NotExpired);
        }
        self.finish(WorkloadState::TimedOut)
    }

    fn finish(&mut self, state: WorkloadState) -> Result<(), LifecycleError> {
        if !matches!(
            self.state,
            WorkloadState::Waiting(_) | WorkloadState::Running
        ) {
            return Err(LifecycleError::InvalidTransition);
        }
        self.state = state;
        if self.release_state == ReleaseState::Held {
            self.release_state = ReleaseState::Pending;
        }
        Ok(())
    }
}
