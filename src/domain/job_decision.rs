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
        // 先確認需求本身有沒有超過整個 cluster 的總預算；超過就直接拒絕。
        if !cluster.within_total_budget(self.requested_memory()) {
            return JobDecision::Reject {
                reason: RejectReason::ExceedsTotalBudget,
            };
        }
        // 再確認這個 queue 確實屬於目前的 tenant，避免把工作送進錯的治理範圍。
        if tenant_budget.tenant() != self.tenant() {
            return JobDecision::Reject {
                reason: RejectReason::TenantMismatch,
            };
        }
        // 不是 queue 的隊首時先等待，遵守租戶內 FIFO，不能讓後面的工作插隊。
        if !is_queue_head {
            return JobDecision::Wait {
                reason: WaitReason::NotQueueHead,
            };
        }
        // 確認 tenant 自己剩餘的 quota 是否放得下這份需求；放不下就暫時等待。
        if !tenant_budget.can_fit(self.requested_memory()) {
            return JobDecision::Wait {
                reason: WaitReason::TenantQuota,
            };
        }
        // 最後確認 cluster 當下的可用容量；目前放不下不代表需求不合法，只需要等待。
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
