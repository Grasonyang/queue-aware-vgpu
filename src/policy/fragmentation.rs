use chrono::{DateTime, Utc};

#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub id: String,
    pub requested_memory_mib: u64,
    pub waiting_since: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Selection {
    pub id: String,
    pub remainder_mib: u64,
    pub score: f64,
}

pub fn choose_best_fit(
    free_memory_mib: u64,
    candidates: &[Candidate],
    lookahead: usize,
    safety_margin_mib: u64,
    starvation_timeout_seconds: u64,
    now: DateTime<Utc>,
) -> Option<Selection> {
    if candidates.is_empty() || lookahead == 0 {
        return None;
    }

    let window = candidates.iter().take(lookahead);
    let oldest = candidates.first().map(|candidate| {
        (now - candidate.waiting_since).num_seconds().max(0) as u64 >= starvation_timeout_seconds
    });

    let eligible = window.filter(|candidate| {
        let fits = free_memory_mib >= candidate.requested_memory_mib;
        let remainder = free_memory_mib.saturating_sub(candidate.requested_memory_mib);
        fits && remainder >= safety_margin_mib
    });

    let selected = if oldest == Some(true) {
        eligible
            .filter(|candidate| candidate.id == candidates[0].id)
            .min_by_key(|candidate| candidate.requested_memory_mib)
    } else {
        eligible.min_by(|left, right| {
            let left_remainder = free_memory_mib - left.requested_memory_mib;
            let right_remainder = free_memory_mib - right.requested_memory_mib;
            left_remainder
                .cmp(&right_remainder)
                .then_with(|| left.waiting_since.cmp(&right.waiting_since))
                .then_with(|| left.id.cmp(&right.id))
        })
    }?;

    let remainder_mib = free_memory_mib - selected.requested_memory_mib;
    Some(Selection {
        id: selected.id.clone(),
        remainder_mib,
        score: remainder_mib as f64 / free_memory_mib.max(1) as f64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn job(id: &str, memory: u64, age: i64, now: DateTime<Utc>) -> Candidate {
        Candidate {
            id: id.to_string(),
            requested_memory_mib: memory,
            waiting_since: now - Duration::seconds(age),
        }
    }

    #[test]
    fn best_fit_chooses_smallest_legal_remainder() {
        let now = Utc::now();
        let choice = choose_best_fit(
            20,
            &[
                job("a", 18, 1, now),
                job("b", 12, 2, now),
                job("c", 8, 3, now),
            ],
            8,
            0,
            120,
            now,
        )
        .expect("choice");
        assert_eq!(choice.id, "a");
        assert_eq!(choice.remainder_mib, 2);
    }

    #[test]
    fn lookahead_bounds_reordering() {
        let now = Utc::now();
        let choice = choose_best_fit(
            20,
            &[job("head", 8, 1, now), job("outside", 18, 1, now)],
            1,
            0,
            120,
            now,
        )
        .expect("choice");
        assert_eq!(choice.id, "head");
    }

    #[test]
    fn old_head_cannot_be_bypassed() {
        let now = Utc::now();
        let choice = choose_best_fit(
            20,
            &[job("old", 25, 121, now), job("small", 8, 1, now)],
            8,
            0,
            120,
            now,
        );
        assert!(choice.is_none());
    }
}
