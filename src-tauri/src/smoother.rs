/// What one sample concluded, as the smoother sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    Focused,
    Distracted,
    /// Away, no conclusion, or a model error: neither counts nor resets.
    Neutral,
}

#[derive(Debug, Default, Clone)]
pub struct Smoother {
    pub streak: u32,
}

impl Smoother {
    /// Feed one signal; returns true when an alert should fire.
    pub fn feed(&mut self, signal: Signal, threshold: u32) -> bool {
        match signal {
            Signal::Focused => self.streak = 0,
            Signal::Neutral => {}
            Signal::Distracted => {
                self.streak += 1;
                if self.streak >= threshold.max(1) {
                    self.streak = 0;
                    return true;
                }
            }
        }
        false
    }

    pub fn reset(&mut self) {
        self.streak = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Signal::*;

    #[test]
    fn alerts_after_three_in_a_row() {
        let mut s = Smoother::default();
        assert!(!s.feed(Distracted, 3));
        assert!(!s.feed(Distracted, 3));
        assert!(s.feed(Distracted, 3));
        assert_eq!(s.streak, 0, "resets after alerting");
    }

    #[test]
    fn focused_resets_neutral_keeps() {
        let mut s = Smoother::default();
        s.feed(Distracted, 3);
        s.feed(Distracted, 3);
        s.feed(Neutral, 3);
        assert_eq!(s.streak, 2);
        assert!(s.feed(Distracted, 3));
        s.feed(Distracted, 3);
        s.feed(Focused, 3);
        assert_eq!(s.streak, 0);
    }
}
