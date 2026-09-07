use chrono::{DateTime, Utc};

#[derive(Clone, Debug, PartialEq)]
pub struct QueueLoad {
    pub name: String,
    pub weight: u32,
    pub allocated_memory_mib: u64,
    pub running_jobs: u32,
    pub pending_jobs: u32,
    pub oldest_pending_since: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueueChoice {
    pub queue: String,
    pub weight: u32,
    pub normalized_allocation: f64,
    pub waiting_age_seconds: i64,
}

pub fn choose_queue(
    queues: &[QueueLoad],
    now: DateTime<Utc>,
    starvation_timeout_seconds: u64,
) -> Option<QueueChoice> {
    let eligible: Vec<_> = queues
        .iter()
        .filter(|queue| queue.pending_jobs > 0 && queue.weight > 0)
        .map(|queue| {
            let waiting_age_seconds = queue
                .oldest_pending_since
                .map(|since| (now - since).num_seconds().max(0))
                .unwrap_or_default();
            QueueChoice {
                queue: queue.name.clone(),
                weight: queue.weight,
                normalized_allocation: queue.allocated_memory_mib as f64 / queue.weight as f64,
                waiting_age_seconds,
            }
        })
        .collect();

    if eligible.is_empty() {
        return None;
    }

    let starved: Vec<_> = eligible
        .iter()
        .filter(|choice| choice.waiting_age_seconds as u64 >= starvation_timeout_seconds)
        .collect();
    let candidates = if starved.is_empty() {
        eligible.iter().collect()
    } else {
        starved
    };

    candidates
        .into_iter()
        .min_by(|left, right| {
            left.normalized_allocation
                .total_cmp(&right.normalized_allocation)
                .then_with(|| right.weight.cmp(&left.weight))
                .then_with(|| right.waiting_age_seconds.cmp(&left.waiting_age_seconds))
                .then_with(|| left.queue.cmp(&right.queue))
        })
        .cloned()
}

pub fn within_queue_limits(
    load: &QueueLoad,
    max_running: Option<u32>,
    max_memory_mib: Option<u64>,
    requested_memory_mib: u64,
) -> bool {
    max_running.is_none_or(|limit| load.running_jobs < limit)
        && max_memory_mib.is_none_or(|limit| {
            load.allocated_memory_mib
                .saturating_add(requested_memory_mib)
                <= limit
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn queue(name: &str, weight: u32, allocation: u64, pending: u32) -> QueueLoad {
        QueueLoad {
            name: name.to_string(),
            weight,
            allocated_memory_mib: allocation,
            running_jobs: 0,
            pending_jobs: pending,
            oldest_pending_since: None,
        }
    }

    #[test]
    fn equal_weights_choose_lower_normalized_allocation() {
        let choice = choose_queue(&[queue("a", 1, 8, 1), queue("b", 1, 4, 1)], Utc::now(), 120)
            .expect("choice");
        assert_eq!(choice.queue, "b");
    }

    #[test]
    fn higher_weight_receives_more_share_at_same_load() {
        let choice = choose_queue(
            &[queue("interactive", 2, 8, 1), queue("batch", 1, 4, 1)],
            Utc::now(),
            120,
        )
        .expect("choice");
        assert_eq!(choice.queue, "interactive");
    }

    #[test]
    fn empty_and_zero_weight_queues_are_ignored() {
        let choice = choose_queue(
            &[queue("empty", 1, 0, 0), queue("zero", 0, 0, 1)],
            Utc::now(),
            120,
        );
        assert!(choice.is_none());
    }

    #[test]
    fn starvation_protection_wins_over_fairness() {
        let now = Utc::now();
        let mut old = queue("old", 1, 1000, 1);
        old.oldest_pending_since = Some(now - Duration::seconds(121));
        let choice = choose_queue(&[old, queue("new", 1, 0, 1)], now, 120).expect("choice");
        assert_eq!(choice.queue, "old");
    }

    #[test]
    fn max_running_limit_blocks_a_queue() {
        let load = QueueLoad {
            running_jobs: 4,
            ..queue("research", 1, 0, 1)
        };
        assert!(!within_queue_limits(&load, Some(4), None, 1));
        assert!(within_queue_limits(&load, Some(5), None, 1));
    }
}
