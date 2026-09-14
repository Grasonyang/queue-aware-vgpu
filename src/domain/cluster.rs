use super::{GpuMemoryMib, TenantId};

/// Read-only resource snapshot consumed by domain policies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClusterSnapshot {
    total_memory: GpuMemoryMib,
    reserved_memory: GpuMemoryMib,
}

impl ClusterSnapshot {
    pub const fn new(total_memory: GpuMemoryMib, reserved_memory: GpuMemoryMib) -> Self {
        Self {
            total_memory,
            reserved_memory,
        }
    }

    pub const fn total_memory(self) -> GpuMemoryMib {
        self.total_memory
    }

    pub const fn reserved_memory(self) -> GpuMemoryMib {
        self.reserved_memory
    }

    pub const fn available_memory(self) -> GpuMemoryMib {
        self.total_memory.saturating_sub(self.reserved_memory)
    }

    pub const fn can_fit_now(self, request: GpuMemoryMib) -> bool {
        self.available_memory().fits(request)
    }

    pub const fn within_total_budget(self, request: GpuMemoryMib) -> bool {
        self.total_memory.fits(request)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TenantBudget {
    tenant: TenantId,
    limit: GpuMemoryMib,
    reserved: GpuMemoryMib,
}

impl TenantBudget {
    pub const fn new(tenant: TenantId, limit: GpuMemoryMib, reserved: GpuMemoryMib) -> Self {
        Self {
            tenant,
            limit,
            reserved,
        }
    }

    pub fn tenant(&self) -> &TenantId {
        &self.tenant
    }

    pub fn limit(&self) -> GpuMemoryMib {
        self.limit
    }

    pub fn reserved(&self) -> GpuMemoryMib {
        self.reserved
    }

    pub fn available(&self) -> GpuMemoryMib {
        self.limit.saturating_sub(self.reserved)
    }

    pub fn can_fit(&self, request: GpuMemoryMib) -> bool {
        self.available().fits(request)
    }
}
