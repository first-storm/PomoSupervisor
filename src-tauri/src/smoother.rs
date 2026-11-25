/// What one sample concluded, as the smoother sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    Focused,
    Distracted,
    /// Away, no conclusion, or a model error: neither counts nor resets.
    Neutral,
}

/// Seconds of quiet after the 1st, 2nd and later reminders of one unbroken distraction.
const BACKOFF: [i64; 3] = [60, 120, 300];

#[derive(Debug, Default, Clone)]
pub struct Smoother {
    /// Distracted samples in a row, capped at the threshold.
    pub streak: u32,
    /// No reminder before this time (unix seconds).
    quiet_until: i64,
    /// Reminders fired since the last focused sample.
    repeats: u32,
    /// When the last reminder fired.
    fired_at: i64,
}

impl Smoother {
    /// Feed one signal at time `now`; returns true when a reminder should fire.
    ///
    /// A reminder fires once the streak reaches the threshold. While the person stays
    /// distracted it repeats, but ever less often, rather than every few samples. Coming
    /// back to the task starts the back-off over, yet two reminders are never closer than
    /// the first step: hopping through an allowed window doesn't earn a new one at once.
    pub fn feed(&mut self, signal: Signal, threshold: u32, now: i64) -> bool {
        match signal {
            Signal::Focused => {
                self.streak = 0;
                self.repeats = 0;
                self.quiet_until = self.quiet_until.min(self.fired_at + BACKOFF[0]);
            }
            Signal::Neutral => {}
            Signal::Distracted => {
                let threshold = threshold.max(1);
                self.streak = (self.streak + 1).min(threshold);
                if self.streak >= threshold && now >= self.quiet_until {
                    let wait = BACKOFF[(self.repeats as usize).min(BACKOFF.len() - 1)];
                    self.repeats += 1;
                    self.quiet_until = now + wait;
                    self.fired_at = now;
                    return true;
                }
            }
        }
        false
    }

    /// Reminders fired during the current distraction (1 right after the first).
    pub fn repeats(&self) -> u32 {
        self.repeats
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Signal::*;

    #[test]
    fn alerts_after_three_in_a_row() {
        let mut s = Smoother::default();
        assert!(!s.feed(Distracted, 3, 0));
        assert!(!s.feed(Distracted, 3, 4));
        assert!(s.feed(Distracted, 3, 8));
        assert_eq!(s.streak, 3, "stays at the threshold while still distracted");
    }

    #[test]
    fn focused_resets_neutral_keeps() {
        let mut s = Smoother::default();
        s.feed(Distracted, 3, 0);
        s.feed(Distracted, 3, 4);
        s.feed(Neutral, 3, 8);
        assert_eq!(s.streak, 2);
        assert!(s.feed(Distracted, 3, 12));
        s.feed(Distracted, 3, 16);
        s.feed(Focused, 3, 20);
        assert_eq!(s.streak, 0);
    }

    #[test]
    fn repeats_back_off_while_still_distracted() {
        let mut s = Smoother::default();
        let fired: Vec<i64> = (0..150).map(|i| i * 4).filter(|&t| s.feed(Distracted, 3, t)).collect();
        // 3rd sample, then 60 s, 120 s and 300 s later.
        assert_eq!(fired, vec![8, 68, 188, 488]);
        assert_eq!(s.repeats(), 4);
    }

    #[test]
    fn coming_back_starts_over() {
        let mut s = Smoother::default();
        for t in [0, 4, 8] {
            s.feed(Distracted, 3, t);
        }
        s.feed(Focused, 3, 12);
        // A fresh distraction needs a full streak again, and at least a minute since the last reminder.
        let fired: Vec<i64> = (4..40).map(|i| i * 4).filter(|&t| s.feed(Distracted, 3, t)).collect();
        assert_eq!(fired.first(), Some(&68));
        // After a long stretch of work the next distraction is caught at the usual speed.
        s.feed(Focused, 3, 1000);
        assert!(!s.feed(Distracted, 3, 1004));
        assert!(!s.feed(Distracted, 3, 1008));
        assert!(s.feed(Distracted, 3, 1012));
    }
}
