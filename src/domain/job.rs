use super::{ClusterSnapshot, GpuMemoryMib, QueueId, TenantId, TenantQueueId, WorkloadId};
use super::{WaitMode, WorkloadLifecycle};
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobSpec {
    id: WorkloadId,
    tenant_queue: TenantQueueId,
    requested_memory: GpuMemoryMib,
    wait_mode: WaitMode,
}

impl JobSpec {
    pub fn new(
        id: WorkloadId,
        tenant_queue: TenantQueueId,
        requested_memory: GpuMemoryMib,
        wait_mode: WaitMode,
    ) -> Result<Self, QueueError> {
        if requested_memory == GpuMemoryMib::zero() {
            return Err(QueueError::ZeroMemoryRequest);
        }
        Ok(Self {
            id,
            tenant_queue,
            requested_memory,
            wait_mode,
        })
    }

    pub fn id(&self) -> &WorkloadId {
        &self.id
    }

    pub fn tenant(&self) -> &TenantId {
        self.tenant_queue.tenant()
    }

    pub fn queue(&self) -> &QueueId {
        self.tenant_queue.queue()
    }

    pub fn tenant_queue(&self) -> &TenantQueueId {
        &self.tenant_queue
    }

    pub const fn requested_memory(&self) -> GpuMemoryMib {
        self.requested_memory
    }

    pub const fn wait_mode(&self) -> WaitMode {
        self.wait_mode
    }

    fn accept(self, accepted_at: DateTime<Utc>) -> WorkloadLifecycle {
        WorkloadLifecycle::new(self.id, self.requested_memory, self.wait_mode, accepted_at)
    }

    pub fn accept_within(
        self,
        cluster: ClusterSnapshot,
        accepted_at: DateTime<Utc>,
    ) -> Result<WorkloadLifecycle, AdmissionError> {
        if !cluster.within_total_budget(self.requested_memory) {
            return Err(AdmissionError::ExceedsTotalBudget);
        }
        Ok(self.accept(accepted_at))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueueError {
    ZeroMemoryRequest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdmissionError {
    ExceedsTotalBudget,
}
