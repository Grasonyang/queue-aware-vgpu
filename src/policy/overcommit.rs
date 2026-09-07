#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PressureSignal {
    Safe,
    Oom,
    MemoryPressure,
    AbnormalRestart,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DecisionInput {
    pub current_ratio: f64,
    pub min_ratio: f64,
    pub max_ratio: f64,
    pub step: f64,
    pub pending_jobs: u32,
    pub signal: PressureSignal,
    pub cooldown_elapsed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Decision {
    Increase { ratio: f64 },
    Decrease { ratio: f64 },
    Hold { ratio: f64 },
}

pub fn decide(input: DecisionInput) -> Decision {
    let current = input.current_ratio.clamp(input.min_ratio, input.max_ratio);
    if !input.cooldown_elapsed {
        return Decision::Hold { ratio: current };
    }

    match input.signal {
        PressureSignal::Oom | PressureSignal::MemoryPressure | PressureSignal::AbnormalRestart => {
            Decision::Decrease {
                ratio: round_ratio((current - input.step).max(input.min_ratio)),
            }
        }
        PressureSignal::Safe if input.pending_jobs > 0 => Decision::Increase {
            ratio: round_ratio((current + input.step).min(input.max_ratio)),
        },
        _ => Decision::Hold { ratio: current },
    }
}

fn round_ratio(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(signal: PressureSignal) -> DecisionInput {
        DecisionInput {
            current_ratio: 1.1,
            min_ratio: 1.0,
            max_ratio: 1.2,
            step: 0.05,
            pending_jobs: 1,
            signal,
            cooldown_elapsed: true,
        }
    }

    #[test]
    fn pressure_decreases_ratio() {
        assert_eq!(
            decide(input(PressureSignal::Oom)),
            Decision::Decrease { ratio: 1.05 }
        );
    }

    #[test]
    fn safe_pending_work_increases_ratio() {
        assert_eq!(
            decide(input(PressureSignal::Safe)),
            Decision::Increase { ratio: 1.15 }
        );
    }

    #[test]
    fn cooldown_holds_ratio() {
        let mut value = input(PressureSignal::Safe);
        value.cooldown_elapsed = false;
        assert_eq!(decide(value), Decision::Hold { ratio: 1.1 });
    }

    #[test]
    fn bounds_are_enforced() {
        let mut value = input(PressureSignal::Safe);
        value.current_ratio = 1.2;
        assert_eq!(decide(value), Decision::Increase { ratio: 1.2 });
        value.current_ratio = 1.0;
        value.signal = PressureSignal::Oom;
        assert_eq!(decide(value), Decision::Decrease { ratio: 1.0 });
    }
}
