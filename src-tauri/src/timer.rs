use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Idle,
    Focus,
    ShortBreak,
    LongBreak,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PauseReason {
    User,
    Away,
}

#[derive(Debug, Clone)]
pub struct Timer {
    pub phase: Phase,
    pub remaining: u32,
    pub total: u32,
    pub pause: Option<PauseReason>,
    /// Focus rounds finished in the current long-break cycle.
    pub cycle: u32,
    pub task: String,
}

impl Default for Timer {
    fn default() -> Self {
        Self { phase: Phase::Idle, remaining: 0, total: 0, pause: None, cycle: 0, task: String::new() }
    }
}

impl Timer {
    pub fn start(&mut self, phase: Phase, secs: u32) {
        self.phase = phase;
        self.total = secs;
        self.remaining = secs;
        self.pause = None;
    }

    pub fn stop(&mut self) {
        self.phase = Phase::Idle;
        self.remaining = 0;
        self.total = 0;
        self.pause = None;
    }

    pub fn elapsed(&self) -> u32 {
        self.total.saturating_sub(self.remaining)
    }

    /// One second passes. Returns the phase that just ran out, if any.
    pub fn tick(&mut self) -> Option<Phase> {
        if self.phase == Phase::Idle || self.pause.is_some() {
            return None;
        }
        self.remaining = self.remaining.saturating_sub(1);
        (self.remaining == 0).then_some(self.phase)
    }

    /// The break that follows a finished focus round (counts it toward the long break).
    pub fn next_break(&mut self, long_every: u32) -> Phase {
        self.cycle += 1;
        if self.cycle >= long_every.max(1) {
            self.cycle = 0;
            Phase::LongBreak
        } else {
            Phase::ShortBreak
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_down_and_reports_end() {
        let mut t = Timer::default();
        t.start(Phase::Focus, 2);
        assert_eq!(t.tick(), None);
        assert_eq!(t.tick(), Some(Phase::Focus));
    }

    #[test]
    fn paused_does_not_tick() {
        let mut t = Timer::default();
        t.start(Phase::Focus, 5);
        t.pause = Some(PauseReason::Away);
        assert_eq!(t.tick(), None);
        assert_eq!(t.remaining, 5);
    }

    #[test]
    fn long_break_every_n() {
        let mut t = Timer::default();
        let seq: Vec<_> = (0..4).map(|_| t.next_break(4)).collect();
        assert_eq!(seq, vec![Phase::ShortBreak, Phase::ShortBreak, Phase::ShortBreak, Phase::LongBreak]);
        assert_eq!(t.next_break(4), Phase::ShortBreak);
    }
}
