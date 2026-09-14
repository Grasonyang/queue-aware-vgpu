use super::{JobSpec, TenantId, WorkloadId};
use std::collections::VecDeque;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueueError {
    TenantMismatch,
    DuplicateWorkload,
    NotQueueHead,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TenantQueue {
    tenant: TenantId,
    entries: VecDeque<WorkloadId>,
}

impl TenantQueue {
    pub fn new(tenant: TenantId) -> Self {
        Self {
            tenant,
            entries: VecDeque::new(),
        }
    }

    pub fn enqueue(&mut self, job: &JobSpec) -> Result<(), QueueError> {
        if job.tenant() != &self.tenant {
            return Err(QueueError::TenantMismatch);
        }
        if self.entries.iter().any(|id| id == job.id()) {
            return Err(QueueError::DuplicateWorkload);
        }
        self.entries.push_back(job.id().clone());
        Ok(())
    }

    pub fn head(&self) -> Option<&WorkloadId> {
        self.entries.front()
    }

    pub fn pop_head(&mut self, id: &WorkloadId) -> Result<(), QueueError> {
        if self.head() != Some(id) {
            return Err(QueueError::NotQueueHead);
        }
        self.entries.pop_front();
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
