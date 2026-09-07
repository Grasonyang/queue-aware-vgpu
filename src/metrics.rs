use anyhow::Result;
use prometheus::{
    Encoder, GaugeVec, HistogramVec, IntCounterVec, IntGaugeVec, Opts, Registry, TextEncoder,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct Metrics {
    registry: Arc<Registry>,
    pending: IntGaugeVec,
    running: IntGaugeVec,
    allocated: IntGaugeVec,
    admissions: IntCounterVec,
    wait_seconds: HistogramVec,
    decisions: IntCounterVec,
    fragmentation: GaugeVec,
    overcommit: GaugeVec,
    errors: IntCounterVec,
}

impl Metrics {
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Arc::new(Registry::new());
        let pending = IntGaugeVec::new(
            Opts::new("queue_vgpu_pending_jobs", "Pending jobs by queue"),
            &["queue"],
        )?;
        let running = IntGaugeVec::new(
            Opts::new("queue_vgpu_running_jobs", "Running jobs by queue"),
            &["queue"],
        )?;
        let allocated = IntGaugeVec::new(
            Opts::new(
                "queue_vgpu_allocated_memory_mib",
                "Allocated memory by queue",
            ),
            &["queue"],
        )?;
        let admissions = IntCounterVec::new(
            Opts::new("queue_vgpu_admissions_total", "Admission decisions"),
            &["queue", "result"],
        )?;
        let wait_seconds = HistogramVec::new(
            Opts::new("queue_vgpu_admission_wait_seconds", "Admission wait time").into(),
            &["queue"],
        )?;
        let decisions = IntCounterVec::new(
            Opts::new("queue_vgpu_policy_decisions_total", "Policy decisions"),
            &["queue", "reason"],
        )?;
        let fragmentation = GaugeVec::new(
            Opts::new("queue_vgpu_fragmentation_score", "Fragmentation score"),
            &["queue"],
        )?;
        let overcommit = GaugeVec::new(
            Opts::new("queue_vgpu_overcommit_ratio", "Current overcommit ratio"),
            &["queue"],
        )?;
        let errors = IntCounterVec::new(
            Opts::new("queue_vgpu_errors_total", "Controller errors"),
            &["type"],
        )?;

        for collector in [
            Box::new(pending.clone()) as Box<dyn prometheus::core::Collector>,
            Box::new(running.clone()),
            Box::new(allocated.clone()),
            Box::new(admissions.clone()),
            Box::new(wait_seconds.clone()),
            Box::new(decisions.clone()),
            Box::new(fragmentation.clone()),
            Box::new(overcommit.clone()),
            Box::new(errors.clone()),
        ] {
            registry.register(collector)?;
        }

        Ok(Self {
            registry,
            pending,
            running,
            allocated,
            admissions,
            wait_seconds,
            decisions,
            fragmentation,
            overcommit,
            errors,
        })
    }

    pub fn set_queue(&self, queue: &str, pending: i64, running: i64, allocated_mib: i64) {
        self.pending.with_label_values(&[queue]).set(pending);
        self.running.with_label_values(&[queue]).set(running);
        self.allocated
            .with_label_values(&[queue])
            .set(allocated_mib);
    }

    pub fn admission(&self, queue: &str, result: &str) {
        self.admissions.with_label_values(&[queue, result]).inc();
    }

    pub fn observe_wait(&self, queue: &str, seconds: f64) {
        self.wait_seconds
            .with_label_values(&[queue])
            .observe(seconds.max(0.0));
    }

    pub fn decision(&self, queue: &str, reason: &str) {
        self.decisions.with_label_values(&[queue, reason]).inc();
    }

    pub fn set_fragmentation(&self, queue: &str, score: f64) {
        self.fragmentation.with_label_values(&[queue]).set(score);
    }

    pub fn set_overcommit(&self, queue: &str, ratio: f64) {
        self.overcommit.with_label_values(&[queue]).set(ratio);
    }

    pub fn error(&self, kind: &str) {
        self.errors.with_label_values(&[kind]).inc();
    }

    pub fn render(&self) -> Result<String, prometheus::Error> {
        let families = self.registry.gather();
        let mut output = Vec::new();
        TextEncoder::new().encode(&families, &mut output)?;
        Ok(String::from_utf8_lossy(&output).into_owned())
    }
}
