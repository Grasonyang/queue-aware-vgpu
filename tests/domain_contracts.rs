use chrono::{Duration, TimeZone, Utc};
use queue_aware_vgpu_controller::domain::{
    AdmissionError, ClusterSnapshot, FailureReason, GpuMemoryMib, JobSpec, LifecycleError,
    QueueError, ReleaseState, ReservationError, ReservationId, ReservationLedger,
    ReservationStatus, TenantId, TenantQueue, WaitMode, WaitingPhase, WorkloadId, WorkloadState,
};

fn timestamp() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 15, 0, 0, 0).unwrap()
}

fn id(value: &str) -> WorkloadId {
    WorkloadId::new(value).unwrap()
}

fn tenant(value: &str) -> TenantId {
    TenantId::new(value).unwrap()
}

fn reservation(value: &str) -> ReservationId {
    ReservationId::new(value).unwrap()
}

fn accepted(job: JobSpec) -> queue_aware_vgpu_controller::domain::WorkloadLifecycle {
    let cluster = ClusterSnapshot::new(GpuMemoryMib::new(96), GpuMemoryMib::zero());
    job.accept_within(cluster, timestamp()).unwrap()
}

fn job(value: &str, memory: u64, mode: WaitMode) -> JobSpec {
    JobSpec::new(
        id(value),
        tenant("tenant-a"),
        queue_aware_vgpu_controller::domain::QueueId::new("queue-a").unwrap(),
        GpuMemoryMib::new(memory),
        mode,
    )
    .unwrap()
}

#[test]
fn request_over_total_budget_is_rejected() {
    let cluster = ClusterSnapshot::new(GpuMemoryMib::new(96), GpuMemoryMib::new(80));
    let result = job("too-large", 97, WaitMode::Forever).accept_within(cluster, timestamp());
    assert_eq!(result, Err(AdmissionError::ExceedsTotalBudget));
}

#[test]
fn request_that_waits_for_current_capacity_is_accepted() {
    let cluster = ClusterSnapshot::new(GpuMemoryMib::new(96), GpuMemoryMib::new(80));
    let lifecycle = job("waiting", 24, WaitMode::Forever)
        .accept_within(cluster, timestamp())
        .unwrap();
    assert_eq!(
        lifecycle.state(),
        &WorkloadState::Waiting(WaitingPhase::Queued)
    );
    assert_eq!(cluster.available_memory(), GpuMemoryMib::new(16));
}

#[test]
fn tenant_queue_is_strict_fifo() {
    let mut queue = TenantQueue::new(tenant("tenant-a"));
    let first = job("first", 8, WaitMode::Forever);
    let second = job("second", 8, WaitMode::Forever);
    queue.enqueue(&first).unwrap();
    queue.enqueue(&second).unwrap();
    assert_eq!(queue.head(), Some(first.id()));
    assert_eq!(queue.pop_head(second.id()), Err(QueueError::NotQueueHead));
    queue.pop_head(first.id()).unwrap();
    assert_eq!(queue.head(), Some(second.id()));
}

#[test]
fn timeout_uses_acceptance_time_and_keeps_reservation_pending() {
    let start = timestamp();
    let mode = WaitMode::timeout(Duration::seconds(10)).unwrap();
    let mut lifecycle = accepted(job("timeout", 24, mode));
    lifecycle.reserve(reservation("r-timeout")).unwrap();
    assert_eq!(
        lifecycle.expire(start + Duration::seconds(9)),
        Err(LifecycleError::NotExpired)
    );
    lifecycle.expire(start + Duration::seconds(10)).unwrap();
    assert_eq!(lifecycle.state(), &WorkloadState::TimedOut);
    assert_eq!(lifecycle.release_state(), ReleaseState::Pending);
}

#[test]
fn waiting_timeout_does_not_expire_running_workload() {
    let start = timestamp();
    let mode = WaitMode::timeout(Duration::seconds(1)).unwrap();
    let mut lifecycle = accepted(job("running", 24, mode));
    lifecycle.reserve(reservation("r-running")).unwrap();
    lifecycle.mark_running().unwrap();
    assert_eq!(
        lifecycle.expire(start + Duration::seconds(100)),
        Err(LifecycleError::InvalidTransition)
    );
    lifecycle.mark_succeeded().unwrap();
    assert_eq!(lifecycle.state(), &WorkloadState::Succeeded);
    assert_eq!(lifecycle.release_state(), ReleaseState::Pending);
}

#[test]
fn failed_allocation_is_terminal_and_cannot_retry_implicitly() {
    let mut lifecycle = accepted(job("failed", 24, WaitMode::Forever));
    lifecycle.reserve(reservation("r-failed")).unwrap();
    lifecycle.mark_failed(FailureReason::Allocation).unwrap();
    assert!(matches!(
        lifecycle.state(),
        WorkloadState::Failed(FailureReason::Allocation)
    ));
    assert_eq!(
        lifecycle.reserve(reservation("r-retry")),
        Err(LifecycleError::ReservationAlreadyHeld)
    );
}

#[test]
fn queued_timeout_has_no_resource_release_to_confirm() {
    let start = timestamp();
    let mode = WaitMode::timeout(Duration::seconds(1)).unwrap();
    let mut lifecycle = accepted(job("queued-timeout", 24, mode));
    lifecycle.expire(start + Duration::seconds(1)).unwrap();
    assert_eq!(lifecycle.release_state(), ReleaseState::NotRequired);
    assert_eq!(
        lifecycle.release_permit(),
        Err(LifecycleError::ReleaseNotPending)
    );
}

#[test]
fn reservation_release_waits_for_external_confirmation() {
    let workload = id("workload");
    let reservation_id = reservation("reservation");
    let mut ledger = ReservationLedger::new(GpuMemoryMib::new(100));
    ledger
        .reserve(reservation_id.clone(), workload, GpuMemoryMib::new(60))
        .unwrap();
    assert_eq!(ledger.available(), GpuMemoryMib::new(40));
    assert_eq!(
        ledger.confirm_release(&reservation_id),
        Err(ReservationError::InvalidReleasePermit)
    );
}

#[test]
fn confirmed_release_returns_capacity_once_and_keeps_audit_record() {
    let workload = id("workload");
    let reservation_id = reservation("reservation");
    let mut ledger = ReservationLedger::new(GpuMemoryMib::new(100));
    ledger
        .reserve(reservation_id.clone(), workload, GpuMemoryMib::new(60))
        .unwrap();
    let mut lifecycle = accepted(job("workload", 60, WaitMode::Forever));
    lifecycle.reserve(reservation_id.clone()).unwrap();
    lifecycle.mark_running().unwrap();
    lifecycle.mark_succeeded().unwrap();
    let permit = lifecycle.release_permit().unwrap();
    ledger.begin_release(permit.clone()).unwrap();
    assert_eq!(ledger.available(), GpuMemoryMib::new(40));
    assert_eq!(
        ledger.get(&reservation_id).unwrap().status(),
        ReservationStatus::ReleasePending
    );
    ledger.confirm_release(&reservation_id).unwrap();
    lifecycle.confirm_release(&reservation_id).unwrap();
    assert_eq!(ledger.available(), GpuMemoryMib::new(100));
    assert_eq!(
        ledger.confirm_release(&reservation_id).unwrap(),
        GpuMemoryMib::zero()
    );
    assert_eq!(lifecycle.release_state(), ReleaseState::Confirmed);
}
