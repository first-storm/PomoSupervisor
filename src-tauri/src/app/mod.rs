//! App state and the actions on a round. The 1-second loop lives in `ticker`, one sample's trip
//! through the decision chain in `chain`, the menu-bar item and sounds in `tray`.
mod chain;
mod ticker;
mod tray;
mod waiting;

pub use ticker::spawn_loop;
pub use tray::emit_tick;
pub use waiting::Waiting;

use crate::ax;
use crate::llm::{self, Judge, Judgement};
use crate::settings::Settings;
use crate::smoother::Smoother;
use crate::store::Store;
use crate::timer::{PauseReason, Phase, Timer};
use image_hasher::ImageHash;
use lru::LruCache;
use serde::Serialize;
use std::collections::VecDeque;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, RwLock};

/// The longest a pause can be stretched to.
const MAX_REST_SECS: u32 = 30 * 60;

/// Ticks on the popover's dial.
pub const DIAL_TICKS: u32 = 50;

pub struct Core {
    pub timer: Timer,
    pub smoother: Smoother,
    pub session_id: Option<i64>,
    pub last: Option<VerdictEvent>,
    pub text_cache: LruCache<String, Judgement>,
    pub last_vision: Option<(ImageHash, Judgement)>,
    /// Apps and sites allowed for this round only (kept as the next round's starting list).
    pub round_allow: Vec<String>,
    /// The latest verdicts, newest last; the nudge shows why it fired from these.
    pub recent: VecDeque<VerdictEvent>,
    /// Dial ticks (0..DIAL_TICKS) during which a sample was judged distracted, this round.
    pub distracted_ticks: Vec<u32>,
    /// Seconds left of a "rest a few minutes" pause taken during focus.
    pub rest_left: Option<u32>,
    /// The distraction reminder is on screen, until a button is clicked or a sample comes back focused.
    pub nudging: bool,
    /// The site or app the reminder is about.
    pub nudge_target: Option<String>,
    /// A break just ran out and nothing has been started since.
    pub break_over: bool,
    /// Length of the last focus round started, to offer the same again.
    pub last_focus_secs: u32,
    /// A round or a break ran out, or the round is held for someone just back, and no answer yet.
    pub waiting: Waiting,
    /// Just back from being away: how long the timer stood still, until the card about it is answered or lapses.
    pub returned: Option<u32>,
    pub returned_age: u32,
    /// The person said they were there all along (reading, thinking): be slower to call them away this round.
    pub idle_lenient: bool,
}

/// Seconds without input before the person counts as away.
pub fn idle_limit(base: u64, lenient: bool) -> u64 {
    if lenient { base * 3 } else { base }
}

/// Asks for Accessibility once per launch, on the first focus round that needs it.
static AX_ASKED: AtomicBool = AtomicBool::new(false);

#[derive(Default)]
pub struct Models {
    pub text: Option<Arc<Judge>>,
    pub vision: Option<Arc<Judge>>,
    pub text_error: Option<String>,
    pub vision_error: Option<String>,
}

pub struct AppState {
    pub core: Mutex<Core>,
    pub settings: RwLock<Settings>,
    pub settings_path: PathBuf,
    pub store: Mutex<Store>,
    pub models: RwLock<Models>,
    pub busy: AtomicBool,
    pub since_sample: AtomicU32,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimerView {
    pub phase: Phase,
    pub remaining: u32,
    pub total: u32,
    pub paused: Option<PauseReason>,
    pub task: String,
    pub cycle: u32,
    pub long_break_every: u32,
    pub streak: u32,
    pub threshold: u32,
    pub focus_secs: u32,
    pub paused_for: u32,
    pub idle_secs: u64,
    pub rest_left: Option<u32>,
    pub auto_resume: bool,
    /// Minutes of the break owed by the round that just finished (None: no break owed).
    pub next_break_min: Option<u32>,
    pub short_break_min: u32,
    pub round_allow: Vec<String>,
    pub distracted_ticks: Vec<u32>,
    pub nudging: bool,
    pub break_over: bool,
    /// Minutes of the last focus round started (the setting before any round).
    pub last_focus_min: u32,
    /// Seconds a finished round or break has gone unanswered.
    pub overtime: u32,
    /// The window in front was last judged off the task (the reminder may not have fired yet).
    pub drifting: bool,
    /// Just back from being away: seconds the timer stood still.
    pub returned: Option<u32>,
}

/// Where a sample's verdict came from. The names are what the webviews and the events table see.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    /// No input for long enough: the person is away.
    Idle,
    Rules,
    Text,
    Vision,
    /// Nothing in the chain could say.
    None,
    /// Our own popover was in front.
    #[serde(rename = "self")]
    Own,
}

impl Stage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Rules => "rules",
            Self::Text => "text",
            Self::Vision => "vision",
            Self::None => "none",
            Self::Own => "self",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SampleResult {
    Focused,
    Distracted,
    Away,
    None,
}

impl SampleResult {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Focused => "focused",
            Self::Distracted => "distracted",
            Self::Away => "away",
            Self::None => "none",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct VerdictEvent {
    pub at: i64,
    pub stage: Stage,
    pub result: SampleResult,
    pub reason: String,
    pub confidence: Option<f64>,
    pub cached: bool,
    pub notes: Vec<String>,
    pub app: String,
    pub title: String,
    pub url: Option<String>,
    /// What "this is part of the task" would allow: the site, else the app. None for a
    /// browser whose URL we couldn't read, where the only thing to allow is the whole browser.
    pub allow_target: Option<String>,
    /// Settled by this round's allow list ("rules" stage).
    pub round_allowed: bool,
    pub streak: u32,
    pub threshold: u32,
    pub alerted: bool,
}

impl Core {
    /// Nothing is on screen asking for an answer any more: no reminder, no timed pause, no finished-round or welcome-back card.
    fn clear_cards(&mut self) {
        self.rest_left = None;
        self.nudging = false;
        self.break_over = false;
        self.waiting = Waiting::default();
        self.returned = None;
    }
}

impl AppState {
    pub fn new(settings_path: PathBuf, db_path: PathBuf) -> anyhow::Result<Self> {
        let settings = Settings::load(&settings_path);
        let state = Self {
            core: Mutex::new(Core {
                timer: Timer::default(),
                smoother: Smoother::default(),
                session_id: None,
                last: None,
                text_cache: LruCache::new(NonZeroUsize::new(128).unwrap()),
                last_vision: None,
                round_allow: Vec::new(),
                recent: VecDeque::new(),
                distracted_ticks: Vec::new(),
                rest_left: None,
                nudging: false,
                nudge_target: None,
                break_over: false,
                last_focus_secs: 0,
                waiting: Waiting::default(),
                returned: None,
                returned_age: 0,
                idle_lenient: false,
            }),
            settings: RwLock::new(settings),
            settings_path,
            store: Mutex::new(Store::open(&db_path)?),
            models: RwLock::new(Models::default()),
            busy: AtomicBool::new(false),
            since_sample: AtomicU32::new(0),
        };
        state.rebuild_models();
        Ok(state)
    }

    pub fn settings(&self) -> Settings {
        self.settings.read().unwrap().clone()
    }

    pub fn rebuild_models(&self) {
        let s = self.settings();
        let mut m = Models::default();
        if let Some(cfg) = &s.text_model {
            match llm::build(cfg, crate::keys::get("text")) {
                Ok(model) => m.text = Some(Arc::new(model)),
                Err(e) => m.text_error = Some(e.to_string()),
            }
        }
        if let Some(cfg) = &s.vision_model {
            match llm::build(cfg, crate::keys::get("vision")) {
                Ok(model) => m.vision = Some(Arc::new(model)),
                Err(e) => m.vision_error = Some(e.to_string()),
            }
        }
        *self.models.write().unwrap() = m;
        let mut core = self.core.lock().unwrap();
        core.text_cache.clear();
        core.last_vision = None;
    }

    pub fn view(&self) -> TimerView {
        let s = self.settings.read().unwrap();
        let core = self.core.lock().unwrap();
        let t = &core.timer;
        TimerView {
            phase: t.phase,
            remaining: t.remaining,
            total: t.total,
            paused: t.pause,
            task: t.task.clone(),
            cycle: t.cycle,
            long_break_every: s.long_break_every,
            streak: core.smoother.streak,
            threshold: s.streak,
            focus_secs: s.focus_min * 60,
            paused_for: t.paused_for,
            idle_secs: s.idle_secs,
            rest_left: core.rest_left,
            auto_resume: s.auto_resume,
            next_break_min: t.pending_break.map(|p| if p == Phase::LongBreak { s.long_break_min } else { s.short_break_min }),
            short_break_min: s.short_break_min,
            round_allow: core.round_allow.clone(),
            distracted_ticks: core.distracted_ticks.clone(),
            nudging: core.nudging,
            break_over: core.break_over,
            last_focus_min: if core.last_focus_secs > 0 { core.last_focus_secs / 60 } else { s.focus_min },
            overtime: if t.phase == Phase::Idle && (t.pending_break.is_some() || core.break_over) { core.waiting.since } else { 0 },
            drifting: t.phase == Phase::Focus
                && t.pause.is_none()
                && core.last.as_ref().is_some_and(|v| v.result == SampleResult::Distracted),
            returned: core.returned,
        }
    }

    /// `minutes`: this round's length (the setting when None). `allow`: this round's extra allow list.
    pub fn start_focus(&self, task: String, minutes: Option<u32>, allow: Vec<String>) {
        let s = self.settings();
        if s.ax_enabled && !ax::trusted() && !AX_ASKED.swap(true, Ordering::Relaxed) {
            ax::request();
        }
        let secs = minutes.unwrap_or(s.focus_min).clamp(1, 180) * 60;
        let mut core = self.core.lock().unwrap();
        self.close_session(&mut core, false);
        if core.timer.task != task || core.round_allow != allow {
            core.text_cache.clear();
            core.last_vision = None;
        }
        core.timer.task = task.clone();
        core.timer.start(Phase::Focus, secs);
        core.round_allow = allow;
        core.smoother.reset();
        core.last = None;
        core.recent.clear();
        core.distracted_ticks.clear();
        core.clear_cards();
        core.idle_lenient = false;
        core.last_focus_secs = secs;
        core.session_id = self.store.lock().unwrap().start_session(&task, secs).ok();
        self.since_sample.store(0, Ordering::Relaxed);
    }

    /// Takes the break the last focus round earned; from idle with nothing owed, a short one.
    pub fn start_break(&self) {
        let s = self.settings();
        let mut core = self.core.lock().unwrap();
        self.close_session(&mut core, false);
        let phase = core.timer.pending_break.take().unwrap_or(Phase::ShortBreak);
        let mins = if phase == Phase::LongBreak { s.long_break_min } else { s.short_break_min };
        core.timer.start(phase, mins * 60);
        core.clear_cards();
    }

    pub fn stop(&self) {
        let mut core = self.core.lock().unwrap();
        self.close_session(&mut core, false);
        core.timer.stop();
        core.timer.pending_break = None;
        core.smoother.reset();
        core.clear_cards();
    }

    /// The task is done before the bell: the round ends here and still earns its break.
    /// It counts as a finished pomodoro once at least half of it was worked.
    pub fn finish_early(&self) {
        let every = self.settings.read().unwrap().long_break_every;
        let mut core = self.core.lock().unwrap();
        if core.timer.phase != Phase::Focus {
            return;
        }
        let completed = core.timer.elapsed() * 2 >= core.timer.total;
        self.close_session(&mut core, completed);
        let next = if completed { core.timer.next_break(every) } else { Phase::ShortBreak };
        core.timer.stop();
        core.timer.pending_break = Some(next);
        core.smoother.reset();
        core.rest_left = None;
        core.nudging = false;
        core.waiting = Waiting::start();
        core.returned = None;
    }

    pub fn resume(&self) {
        let mut core = self.core.lock().unwrap();
        if core.timer.phase == Phase::Idle {
            return;
        }
        core.timer.pause = None;
        core.rest_left = None;
        core.nudging = false;
        core.returned = None;
    }

    /// Pause the focus round for a few minutes, then carry on by itself. A pause is always
    /// timed: an open-ended one is a way out of the round that nobody notices. Asking again
    /// while paused adds the minutes on.
    pub fn rest(&self, minutes: u32) {
        let mut core = self.core.lock().unwrap();
        if core.timer.phase != Phase::Focus {
            return;
        }
        let add = minutes.clamp(1, 60) * 60;
        let had = if core.timer.pause == Some(PauseReason::User) { core.rest_left.unwrap_or(0) } else { 0 };
        core.timer.pause = Some(PauseReason::User);
        core.rest_left = Some((had + add).min(MAX_REST_SECS));
        core.smoother.reset();
        core.nudging = false;
        core.returned = None;
    }

    /// "I never left": the timer stopped while the person was reading or thinking. Give the
    /// stopped time back to the round and be slower to call them away for the rest of it.
    pub fn still_here(&self) {
        let mut core = self.core.lock().unwrap();
        let Some(secs) = core.returned.take() else { return };
        if core.timer.phase != Phase::Focus {
            return;
        }
        core.timer.pause = None;
        core.rest_left = None;
        core.timer.remaining = core.timer.remaining.saturating_sub(secs).max(1);
        core.idle_lenient = true;
    }

    /// "Back to the task": take the reminder down. The smoother keeps its quiet period, so
    /// there is time to actually get back before it can fire again.
    pub fn dismiss_nudge(&self) {
        self.core.lock().unwrap().nudging = false;
    }

    pub fn allow_for_round(&self, item: &str) {
        let item = item.trim();
        let mut core = self.core.lock().unwrap();
        if !item.is_empty() && !core.round_allow.iter().any(|a| a.eq_ignore_ascii_case(item)) {
            core.round_allow.push(item.to_string());
            core.text_cache.clear();
            core.last_vision = None;
        }
        core.smoother.reset();
        core.nudging = false;
    }

    /// Move `target` (an app or a site) onto the allow list for good.
    pub fn mark_as_work(&self, target: &str) -> anyhow::Result<()> {
        let mut s = self.settings();
        crate::rules::mark_as_work(&mut s.rules, target);
        s.save(&self.settings_path)?;
        *self.settings.write().unwrap() = s;
        let mut core = self.core.lock().unwrap();
        core.text_cache.clear();
        core.last_vision = None;
        Ok(())
    }

    fn close_session(&self, core: &mut Core, completed: bool) {
        if let Some(id) = core.session_id.take() {
            let _ = self.store.lock().unwrap().end_session(id, core.timer.elapsed(), completed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saying_you_were_there_stretches_the_idle_limit() {
        assert_eq!(idle_limit(120, false), 120);
        assert_eq!(idle_limit(120, true), 360);
    }

    /// The webviews and the events table (rows written by earlier versions included) know these by name.
    #[test]
    fn stage_and_result_names_are_pinned() {
        use SampleResult as R;
        let stages = [
            (Stage::Idle, "idle"),
            (Stage::Rules, "rules"),
            (Stage::Text, "text"),
            (Stage::Vision, "vision"),
            (Stage::None, "none"),
            (Stage::Own, "self"),
        ];
        for (stage, name) in stages {
            assert_eq!(stage.as_str(), name);
            assert_eq!(serde_json::to_value(stage).unwrap(), name);
        }
        for (result, name) in [(R::Focused, "focused"), (R::Distracted, "distracted"), (R::Away, "away"), (R::None, "none")] {
            assert_eq!(result.as_str(), name);
            assert_eq!(serde_json::to_value(result).unwrap(), name);
        }
    }

    fn state() -> AppState {
        let dir = std::env::temp_dir().join(format!("pomo-app-test-{}-{:?}", std::process::id(), std::thread::current().id()));
        let _ = std::fs::remove_dir_all(&dir);
        AppState::new(dir.join("settings.json"), dir.join("t.db")).unwrap()
    }

    #[test]
    fn a_pause_is_timed_and_can_be_stretched() {
        let st = state();
        st.start_focus("写测试".into(), Some(25), vec![]);
        st.rest(5);
        assert_eq!(st.view().rest_left, Some(300));
        st.rest(5);
        assert_eq!(st.view().rest_left, Some(600));
        for _ in 0..10 {
            st.rest(5);
        }
        assert_eq!(st.view().rest_left, Some(MAX_REST_SECS));
        st.resume();
        let v = st.view();
        assert_eq!((v.paused, v.rest_left), (None, None));
    }

    #[test]
    fn finishing_early_still_earns_the_break() {
        let st = state();
        st.start_focus("写测试".into(), Some(10), vec![]);
        st.core.lock().unwrap().timer.remaining = 4 * 60;
        st.finish_early();
        let v = st.view();
        assert_eq!(v.phase, Phase::Idle);
        assert_eq!(v.next_break_min, Some(5));
        assert_eq!(st.core.lock().unwrap().timer.cycle, 1, "past the half: a finished pomodoro");
        // Barely started: the break is still offered, but it is not a pomodoro.
        st.start_focus("写测试".into(), Some(10), vec![]);
        st.finish_early();
        assert_eq!(st.view().next_break_min, Some(5));
        assert_eq!(st.core.lock().unwrap().timer.cycle, 1);
        // Abandoning owes nothing.
        st.start_focus("写测试".into(), Some(10), vec![]);
        st.stop();
        assert_eq!(st.view().next_break_min, None);
    }

    #[test]
    fn never_left_gives_the_stopped_time_back() {
        let st = state();
        st.start_focus("读论文".into(), Some(25), vec![]);
        {
            let mut core = st.core.lock().unwrap();
            core.timer.remaining = 600;
            core.timer.pause = Some(PauseReason::User);
            core.returned = Some(240);
        }
        st.still_here();
        let v = st.view();
        assert_eq!((v.remaining, v.paused, v.returned), (360, None, None));
        assert!(st.core.lock().unwrap().idle_lenient);
        // Nothing to give back twice.
        st.still_here();
        assert_eq!(st.view().remaining, 360);
    }
}
