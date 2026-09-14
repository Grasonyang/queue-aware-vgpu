use super::reservation_types::{Reservation, ReservationError, ReservationStatus};
use super::{GpuMemoryMib, ReleasePermit, ReservationId, WorkloadId};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReservationLedger {
    capacity: GpuMemoryMib,
    reserved: GpuMemoryMib,
    reservations: BTreeMap<ReservationId, Reservation>,
}

impl ReservationLedger {
    pub fn new(capacity: GpuMemoryMib) -> Self {
        Self {
            capacity,
            reserved: GpuMemoryMib::zero(),
            reservations: BTreeMap::new(),
        }
    }

    pub const fn capacity(&self) -> GpuMemoryMib {
        self.capacity
    }

    pub const fn reserved(&self) -> GpuMemoryMib {
        self.reserved
    }

    pub const fn available(&self) -> GpuMemoryMib {
        self.capacity.saturating_sub(self.reserved)
    }

    pub fn reserve(
        &mut self,
        id: ReservationId,
        workload: WorkloadId,
        amount: GpuMemoryMib,
    ) -> Result<(), ReservationError> {
        if amount == GpuMemoryMib::zero() {
            return Err(ReservationError::ZeroAmount);
        }
        if self.reservations.contains_key(&id) {
            return Err(ReservationError::Duplicate);
        }
        if !self.available().fits(amount) {
            return Err(ReservationError::CapacityExceeded);
        }
        self.reserved = self.reserved.saturating_add(amount);
        self.reservations.insert(
            id.clone(),
            Reservation {
                id,
                workload,
                amount,
                status: ReservationStatus::Held,
            },
        );
        Ok(())
    }

    pub fn begin_release(&mut self, permit: ReleasePermit) -> Result<(), ReservationError> {
        let reservation = self
            .reservations
            .get_mut(permit.reservation())
            .ok_or(ReservationError::Unknown)?;
        match reservation.status {
            ReservationStatus::Held => reservation.status = ReservationStatus::ReleasePending,
            ReservationStatus::ReleasePending => {}
            ReservationStatus::Released => return Err(ReservationError::InvalidReleasePermit),
        }
        Ok(())
    }

    pub fn confirm_release(
        &mut self,
        id: &ReservationId,
    ) -> Result<GpuMemoryMib, ReservationError> {
        let reservation = self
            .reservations
            .get_mut(id)
            .ok_or(ReservationError::Unknown)?;
        if reservation.status == ReservationStatus::Released {
            return Ok(GpuMemoryMib::zero());
        }
        if reservation.status != ReservationStatus::ReleasePending {
            return Err(ReservationError::InvalidReleasePermit);
        }
        reservation.status = ReservationStatus::Released;
        self.reserved = self.reserved.saturating_sub(reservation.amount);
        Ok(reservation.amount)
    }

    pub fn get(&self, id: &ReservationId) -> Option<&Reservation> {
        self.reservations.get(id)
    }
}
