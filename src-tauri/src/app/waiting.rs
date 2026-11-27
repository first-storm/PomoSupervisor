//! A card that wants an answer: how long it stays up, when it is chased, when it is let go.

/// How long the card for a finished round or break stays up unanswered, and how long a "welcome back" does.
const CARD_SECS: u32 = 30;
pub(super) const RETURNED_SECS: u32 = 10;
/// After this long with no answer the round is simply over: no break owed, nothing waiting.
const GIVE_UP_SECS: u32 = 15 * 60;

/// Input within this many seconds: someone is at the keyboard to see a card.
pub(super) const PRESENT_SECS: u64 = 20;
/// When to chase an unanswered card. An overrun break (or a round held for someone who is
/// back) is where a pomodoro day falls apart, so it is chased sooner and twice; working past
/// the end of a round only gets one reminder that the break is still owed.
pub(super) const CHASE_BREAK: [u32; 2] = [120, 300];
pub(super) const CHASE_DONE: [u32; 1] = [300];

#[derive(Debug, PartialEq, Eq)]
pub enum EndStep {
    None,
    /// Take the unanswered card down; the tray keeps counting.
    Hide,
    /// Bring the card back up, with the sound.
    Remind,
    GiveUp,
}

/// A card that wants an answer and hasn't got one. Time on screen only counts while the
/// person is there to see it: a break ends with them away from the desk more often than not,
/// so the card waits for them, and a reminder that came due meanwhile fires when they return.
#[derive(Debug, Default, Clone)]
pub struct Waiting {
    /// Seconds since the card first went up.
    pub since: u32,
    /// Seconds the card has been up in front of someone.
    seen: u32,
    up: bool,
    /// A reminder came due (possibly with nobody there) and hasn't fired yet.
    owed: bool,
    /// Reminders that have come due so far.
    next: usize,
}

impl Waiting {
    pub fn start() -> Self {
        Self { up: true, ..Self::default() }
    }

    /// One second passes; `chase` lists when (in seconds since the start) to remind.
    pub fn tick(&mut self, present: bool, chase: &[u32]) -> EndStep {
        self.since += 1;
        while self.next < chase.len() && self.since >= chase[self.next] {
            self.owed = true;
            self.next += 1;
        }
        if present {
            if self.owed {
                self.owed = false;
                self.up = true;
                self.seen = 0;
                return EndStep::Remind;
            }
            if self.up {
                self.seen += 1;
                if self.seen >= CARD_SECS {
                    self.up = false;
                    return EndStep::Hide;
                }
            }
        }
        // Everything was shown to someone and nobody answered: let it go.
        if !self.up && !self.owed && self.next == chase.len() && self.since >= GIVE_UP_SECS {
            return EndStep::GiveUp;
        }
        EndStep::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs a waiting card for `secs`, with the person there when `present(t)` says so.
    fn steps(chase: &[u32], secs: u32, present: impl Fn(u32) -> bool) -> Vec<(u32, EndStep)> {
        let mut w = Waiting::start();
        let mut out = Vec::new();
        for t in 1..=secs {
            let step = w.tick(present(t), chase);
            if step != EndStep::None {
                let over = step == EndStep::GiveUp;
                out.push((t, step));
                // The loop drops the card here.
                if over {
                    break;
                }
            }
        }
        out
    }

    #[test]
    fn an_unanswered_round_gets_one_reminder_and_a_break_two() {
        use EndStep::*;
        assert_eq!(steps(&CHASE_DONE, 1000, |_| true), vec![(30, Hide), (300, Remind), (330, Hide), (900, GiveUp)]);
        assert_eq!(
            steps(&CHASE_BREAK, 1000, |_| true),
            vec![(30, Hide), (120, Remind), (150, Hide), (300, Remind), (330, Hide), (900, GiveUp)]
        );
    }

    #[test]
    fn the_card_waits_for_someone_who_is_away() {
        use EndStep::*;
        // The break ends with nobody at the desk; they come back 20 minutes in.
        let got = steps(&CHASE_BREAK, 1400, |t| t > 1200);
        // Nothing is hidden, reminded or given up into an empty room. On their return the
        // missed reminders fire once, the card gets its 30 seconds in front of them, and only then is it let go.
        assert_eq!(got, vec![(1201, Remind), (1231, Hide), (1232, GiveUp)]);
        // Stepping out mid-card pauses its 30 seconds rather than spending them.
        let got = steps(&CHASE_DONE, 200, |t| t <= 10 || t > 100);
        assert_eq!(got, vec![(120, Hide)]);
    }
}
