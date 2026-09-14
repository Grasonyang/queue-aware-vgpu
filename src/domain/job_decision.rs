use super::{ClusterSnapshot, GpuMemoryMib, JobSpec, ReservationId, TenantBudget, WorkloadId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RejectReason {
    ExceedsTotalBudget,
    TenantMismatch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaitReason {
    NotQueueHead,
    TenantQuota,
    CurrentCapacity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReservationIntent {
    reservation: ReservationId,
    workload: WorkloadId,
    amount: GpuMemoryMib,
}

impl ReservationIntent {
    pub fn reservation(&self) -> &ReservationId {
        &self.reservation
    }

    pub fn workload(&self) -> &WorkloadId {
        &self.workload
    }

    pub const fn amount(&self) -> GpuMemoryMib {
        self.amount
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobDecision {
    Reject { reason: RejectReason },
    Wait { reason: WaitReason },
    Place { intent: ReservationIntent },
}

impl JobSpec {
    pub fn evaluate(
        &self,
        cluster: ClusterSnapshot,
        tenant_budget: &TenantBudget,
        is_queue_head: bool,
        reservation: ReservationId,
    ) -> JobDecision {
        if !cluster.within_total_budget(self.requested_memory()) {
            return JobDecision::Reject {
                reason: RejectReason::ExceedsTotalBudget,
            };
        }
        if tenant_budget.tenant() != self.tenant() {
            return JobDecision::Reject {
                reason: RejectReason::TenantMismatch,
            };
        }
        if !is_queue_head {
            return JobDecision::Wait {
                reason: WaitReason::NotQueueHead,
            };
        }
        if !tenant_budget.can_fit(self.requested_memory()) {
            return JobDecision::Wait {
                reason: WaitReason::TenantQuota,
            };
        }
        if !cluster.can_fit_now(self.requested_memory()) {
            return JobDecision::Wait {
                reason: WaitReason::CurrentCapacity,
            };
        }
        JobDecision::Place {
            intent: ReservationIntent {
                reservation,
                workload: self.id().clone(),
                amount: self.requested_memory(),
            },
        }
    }
}
