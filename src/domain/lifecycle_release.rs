use super::{LifecycleError, ReleasePermit, ReleaseState, ReservationId, WorkloadLifecycle};

impl WorkloadLifecycle {
    pub fn release_permit(&self) -> Result<ReleasePermit, LifecycleError> {
        if self.release_state != ReleaseState::Pending {
            return Err(LifecycleError::ReleaseNotPending);
        }
        let reservation = self
            .reservation
            .clone()
            .ok_or(LifecycleError::ReservationMissing)?;
        Ok(ReleasePermit::new(reservation))
    }

    pub fn confirm_release(&mut self, reservation: &ReservationId) -> Result<(), LifecycleError> {
        if self.release_state != ReleaseState::Pending {
            return Err(LifecycleError::ReleaseNotPending);
        }
        if self.reservation.as_ref() != Some(reservation) {
            return Err(LifecycleError::ReservationMismatch);
        }
        self.release_state = ReleaseState::Confirmed;
        Ok(())
    }
}
