//! App state, the 1-second loop, and one sample's trip through the decision chain.
use crate::ax::{self, AxInfo};
use crate::llm::{self, Judge, Judgement};
use crate::pipeline::{Outcome, Stage, StageResult, Verdict, run_chain};
use crate::sampler::{self, Sample};
use crate::settings::Settings;
use crate::smoother::{Signal, Smoother};
use crate::store::{self, Store};
use crate::timer::{PauseReason, Phase, Timer};
use image_hasher::ImageHash;
use lru::LruCache;
use serde::Serialize;
use std::collections::VecDeque;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

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

/// How long the card for a finished round or break stays up unanswered, and how long a "welcome back" does.
const CARD_SECS: u32 = 30;
const RETURNED_SECS: u32 = 10;
/// After this long with no answer the round is simply over: no break owed, nothing waiting.
const GIVE_UP_SECS: u32 = 15 * 60;

/// Input within this many seconds: someone is at the keyboard to see a card.
const PRESENT_SECS: u64 = 20;
/// When to chase an unanswered card. An overrun break (or a round held for someone who is
/// back) is where a pomodoro day falls apart, so it is chased sooner and twice; working past
/// the end of a round only gets one reminder that the break is still owed.
const CHASE_BREAK: [u32; 2] = [120, 300];
const CHASE_DONE: [u32; 1] = [300];

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

/// Seconds without input before the person counts as away.
pub fn idle_limit(base: u64, lenient: bool) -> u64 {
    if lenient { base * 3 } else { base }
}

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

#[derive(Debug, Clone, Serialize)]
pub struct VerdictEvent {
    pub at: i64,
    /// "idle" | "rules" | "text" | "vision" | "none" | "self"
    pub stage: String,
    /// "focused" | "distracted" | "away" | "none"
    pub result: String,
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
    pub streak: u32,
    pub threshold: u32,
    pub alerted: bool,
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
            match llm::build(cfg, llm::get_key("text")) {
                Ok(model) => m.text = Some(Arc::new(model)),
                Err(e) => m.text_error = Some(e.to_string()),
            }
        }
        if let Some(cfg) = &s.vision_model {
            match llm::build(cfg, llm::get_key("vision")) {
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
            drifting: t.phase == Phase::Focus && t.pause.is_none() && core.last.as_ref().is_some_and(|v| v.result == "distracted"),
            returned: core.returned,
        }
    }

    // ---- timer actions ----

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
        core.rest_left = None;
        core.nudging = false;
        core.break_over = false;
        core.waiting = Waiting::default();
        core.returned = None;
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
        core.rest_left = None;
        core.nudging = false;
        core.break_over = false;
        core.waiting = Waiting::default();
        core.returned = None;
    }

    pub fn stop(&self) {
        let mut core = self.core.lock().unwrap();
        self.close_session(&mut core, false);
        core.timer.stop();
        core.timer.pending_break = None;
        core.smoother.reset();
        core.rest_left = None;
        core.nudging = false;
        core.break_over = false;
        core.waiting = Waiting::default();
        core.returned = None;
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

// ---- side effects ----

pub fn play(sound: &str) {
    let _ = std::process::Command::new("afplay").arg(format!("/System/Library/Sounds/{sound}.aiff")).spawn();
}

pub fn notify(app: &AppHandle, title: &str, body: &str) {
    let _ = app.notification().builder().title(title).body(body).show();
}

fn fmt_mmss(secs: u32) -> String {
    format!("{:02}:{:02}", secs / 60, secs % 60)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TrayLook {
    Idle,
    Focus,
    Paused,
}

static TRAY_LOOK: Mutex<Option<TrayLook>> = Mutex::new(None);

fn update_tray(app: &AppHandle, v: &TimerView) {
    let Some(tray) = app.tray_by_id("main") else { return };
    let title = match v.phase {
        // A finished round or break nobody answered: count up, so the overrun is in plain sight.
        Phase::Idle if v.overtime > 0 => format!(" +{}", fmt_mmss(v.overtime)),
        Phase::Idle => String::new(),
        // Drifting: a quiet mark by the clock before (and while) the reminder is up.
        Phase::Focus if v.paused.is_none() && (v.nudging || v.drifting) => format!(" {} ⚠\u{FE0E}", fmt_mmss(v.remaining)),
        _ => format!(" {}", fmt_mmss(v.remaining)),
    };
    let _ = tray.set_title(Some(title));
    let look = match (v.phase, v.paused) {
        (Phase::Idle, _) => TrayLook::Idle,
        (_, Some(_)) => TrayLook::Paused,
        (Phase::Focus, None) => TrayLook::Focus,
        (_, None) => TrayLook::Idle,
    };
    let mut cur = TRAY_LOOK.lock().unwrap();
    if *cur == Some(look) {
        return;
    }
    *cur = Some(look);
    // The focus icon is the tomato in colour; the others are template icons that follow the menu bar.
    let (img, template) = match look {
        TrayLook::Idle => (tauri::include_image!("icons/tray.png"), true),
        TrayLook::Focus => (tauri::include_image!("icons/tray-focus.png"), false),
        TrayLook::Paused => (tauri::include_image!("icons/tray-paused.png"), true),
    };
    let _ = tray.set_icon(Some(img));
    let _ = tray.set_icon_as_template(template);
}

pub fn emit_tick(app: &AppHandle, state: &AppState) {
    let v = state.view();
    update_tray(app, &v);
    let _ = app.emit("timer://tick", &v);
}

// ---- the loop ----

pub fn spawn_loop(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut iv = tokio::time::interval(std::time::Duration::from_secs(1));
        loop {
            iv.tick().await;
            let state = app.state::<Arc<AppState>>().inner().clone();
            let (ended, rest_done, step, lapsed) = {
                let mut core = state.core.lock().unwrap();
                let mut rest_done = false;
                // A finished round or break waiting for an answer, or a round held for someone just back.
                let ended = core.timer.phase == Phase::Idle && (core.timer.pending_break.is_some() || core.break_over);
                let held = core.timer.phase == Phase::Focus
                    && core.timer.pause == Some(PauseReason::User)
                    && core.rest_left.is_none()
                    && core.returned.is_some();
                let step = if ended || held {
                    let chase: &[u32] = if held || core.break_over { &CHASE_BREAK } else { &CHASE_DONE };
                    core.waiting.tick(sampler::idle_secs() <= PRESENT_SECS, chase)
                } else {
                    core.waiting = Waiting::default();
                    EndStep::None
                };
                if step == EndStep::GiveUp {
                    if held {
                        // Back at the desk for a quarter of an hour without picking the round up: it is over.
                        state.close_session(&mut core, false);
                        core.timer.stop();
                        core.smoother.reset();
                        core.returned = None;
                    }
                    core.timer.pending_break = None;
                    core.break_over = false;
                    core.waiting = Waiting::default();
                }
                // "Welcome back" needs no answer once the timer runs again: let it lapse.
                let mut lapsed = false;
                if core.returned.is_some() && core.timer.pause.is_none() {
                    core.returned_age += 1;
                    if core.returned_age >= RETURNED_SECS {
                        core.returned = None;
                        lapsed = true;
                    }
                }
                // A timed rest during focus counts down, then the round carries on.
                if let Some(left) = core.rest_left {
                    if core.timer.pause != Some(PauseReason::User) {
                        core.rest_left = None;
                    } else if left <= 1 {
                        core.rest_left = None;
                        core.timer.pause = None;
                        rest_done = true;
                    } else {
                        core.rest_left = Some(left - 1);
                    }
                }
                (core.timer.tick(), rest_done, step, lapsed)
            };
            match step {
                EndStep::Hide | EndStep::GiveUp => crate::windows::hide_passive(&app),
                EndStep::Remind => {
                    if state.settings.read().unwrap().sound {
                        play("Glass");
                    }
                    emit_tick(&app, &state);
                    crate::windows::show_popover(&app, "passive");
                }
                _ => {}
            }
            if lapsed {
                crate::windows::hide_passive(&app);
            }
            if rest_done {
                notify(&app, "暂停结束", "计时继续了，回到任务上吧。");
                if state.settings.read().unwrap().sound {
                    play("Glass");
                }
            }
            if let Some(phase) = ended {
                on_phase_end(&app, &state, phase);
            }
            emit_tick(&app, &state);

            let (phase, pause) = {
                let c = state.core.lock().unwrap();
                (c.timer.phase, c.timer.pause)
            };
            // Sample during focus, and keep polling idle while away so we can resume.
            if phase != Phase::Focus || pause == Some(PauseReason::User) {
                continue;
            }
            let every = state.settings.read().unwrap().sample_secs;
            if state.since_sample.fetch_add(1, Ordering::Relaxed) + 1 < every {
                continue;
            }
            // One pipeline at a time: skip this tick while the last one still waits on a model.
            if state.busy.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_err() {
                continue;
            }
            state.since_sample.store(0, Ordering::Relaxed);
            let app2 = app.clone();
            tauri::async_runtime::spawn(async move {
                run_sample(&app2, &state).await;
                state.busy.store(false, Ordering::Release);
            });
        }
    });
}

fn on_phase_end(app: &AppHandle, state: &AppState, phase: Phase) {
    let sound = state.settings.read().unwrap().sound;
    match phase {
        Phase::Focus => {
            let every = state.settings.read().unwrap().long_break_every;
            {
                let mut core = state.core.lock().unwrap();
                state.close_session(&mut core, true);
                let next = core.timer.next_break(every);
                core.timer.stop();
                core.timer.pending_break = Some(next);
                core.smoother.reset();
                core.nudging = false;
                core.waiting = Waiting::start();
                core.returned = None;
            }
            notify(app, "番茄完成", "休息一下，或者接着再来一轮。");
            // The person is probably mid-keystroke: show the card without taking focus.
            emit_tick(app, state);
            crate::windows::show_popover(app, "passive");
        }
        Phase::ShortBreak | Phase::LongBreak => {
            {
                let mut core = state.core.lock().unwrap();
                core.timer.stop();
                core.break_over = true;
                core.waiting = Waiting::start();
            }
            notify(app, "休息结束", "准备好就开始下一个番茄。");
            emit_tick(app, state);
            crate::windows::show_popover(app, "passive");
        }
        Phase::Idle => {}
    }
    if sound {
        play("Glass");
    }
}

/// Window text helps where titles say little (chat apps, Electron), so it is part of the key, digits aside.
fn cache_key(task: &str, s: &Sample, ax: Option<&AxInfo>) -> String {
    let ax = ax.map(ax::fingerprint).unwrap_or_default();
    format!("{task}\u{1}{}\u{1}{}\u{1}{}\u{1}{ax}", s.app, s.title, s.url.as_deref().unwrap_or(""))
}

/// Asks for Accessibility once per launch, on the first focus round that needs it.
static AX_ASKED: AtomicBool = AtomicBool::new(false);

/// The frontmost window's Accessibility text, read at most once per sample and only when a model needs it.
type AxCell = Arc<tokio::sync::OnceCell<Option<AxInfo>>>;

async fn ax_for(cell: &AxCell, s: &Sample, settings: &Settings) -> Option<AxInfo> {
    cell.get_or_init(|| async {
        if !settings.ax_enabled || s.pid == std::process::id() as u64 {
            return None;
        }
        let (pid, name) = (s.pid as i32, s.bundle_name.clone().unwrap_or_else(|| s.app.clone()));
        tokio::task::spawn_blocking(move || ax::read(pid, &name)).await.ok().flatten()
    })
    .await
    .clone()
}

fn build_stages(state: &Arc<AppState>, s: &Arc<Sample>, settings: &Arc<Settings>, task: &Arc<String>) -> Vec<Stage> {
    let mut stages: Vec<Stage> = Vec::new();
    let ax_cell: AxCell = Arc::default();

    // 1. Idle for longer than the threshold: the person has left.
    {
        let (state, s, settings) = (state.clone(), s.clone(), settings.clone());
        stages.push(Box::new(move || {
            Box::pin(async move {
                let lenient = state.core.lock().unwrap().idle_lenient;
                if s.idle_secs <= idle_limit(settings.idle_secs, lenient) {
                    StageResult::Pass(None)
                } else if sampler::display_held_awake().await {
                    // Watching a lecture or sitting in a call: let the rest of the chain say whether it is the task.
                    StageResult::Pass(Some("长时间没有键鼠操作，但有视频或通话在进行，不算离开".into()))
                } else {
                    StageResult::Away
                }
            })
        }));
    }

    // 2. This round's allow list, then app / domain / title rules.
    {
        let (state, s, settings) = (state.clone(), s.clone(), settings.clone());
        stages.push(Box::new(move || {
            Box::pin(async move {
                let extra = state.core.lock().unwrap().round_allow.clone();
                let hit = crate::rules::round_allow(&extra, &s).or_else(|| crate::rules::evaluate(&settings.rules, &s));
                match hit {
                    Some((focused, reason)) => {
                        StageResult::Decided(Verdict { focused, stage: "rules", reason, confidence: 1.0, cached: false })
                    }
                    None => StageResult::Pass(None),
                }
            })
        }));
    }

    // 3. Text model: title + URL + window text + task. Only a confident answer concludes.
    {
        let (state, s, settings, task, ax_cell) = (state.clone(), s.clone(), settings.clone(), task.clone(), ax_cell.clone());
        stages.push(Box::new(move || {
            Box::pin(async move {
                if !settings.text_enabled {
                    return StageResult::Pass(None);
                }
                let model = state.models.read().unwrap().text.clone();
                let Some(model) = model else { return StageResult::Pass(None) };
                let ax = ax_for(&ax_cell, &s, &settings).await;
                let key = cache_key(&task, &s, ax.as_ref());
                let cached = state.core.lock().unwrap().text_cache.get(&key).cloned();
                let (j, was_cached) = match cached {
                    Some(j) => (j, true),
                    None => match llm::judge_text(&model, &task, &s, ax.as_ref(), settings.confidence).await {
                        Ok(j) => {
                            state.core.lock().unwrap().text_cache.put(key, j.clone());
                            (j, false)
                        }
                        Err(e) => {
                            let _ = state.store.lock().unwrap().log_llm("text", false, false);
                            return StageResult::Pass(Some(format!("文本模型：{e}")));
                        }
                    },
                };
                let _ = state.store.lock().unwrap().log_llm("text", was_cached, true);
                match j.focused() {
                    Some(focused) if j.confidence >= settings.confidence => StageResult::Decided(Verdict {
                        focused,
                        stage: "text",
                        reason: j.reason,
                        confidence: j.confidence,
                        cached: was_cached,
                    }),
                    _ => StageResult::Pass(Some(format!("文本模型没把握（{:.2}）：{}", j.confidence, j.reason))),
                }
            })
        }));
    }

    // 4. Screenshot to the vision model, de-duplicated by pHash.
    {
        let (state, s, settings, task, ax_cell) = (state.clone(), s.clone(), settings.clone(), task.clone(), ax_cell.clone());
        stages.push(Box::new(move || {
            Box::pin(async move {
                if !settings.vision_enabled {
                    return StageResult::Pass(None);
                }
                let model = state.models.read().unwrap().vision.clone();
                let Some(model) = model else { return StageResult::Pass(None) };
                let (cx, cy) = (s.cx, s.cy);
                let shot = match tokio::task::spawn_blocking(move || crate::vision::capture(cx, cy)).await {
                    Ok(Ok(shot)) => shot,
                    Ok(Err(e)) => return StageResult::Pass(Some(format!("截图：{e}"))),
                    Err(e) => return StageResult::Pass(Some(format!("截图：{e}"))),
                };
                let reuse = state
                    .core
                    .lock()
                    .unwrap()
                    .last_vision
                    .as_ref()
                    .filter(|(h, _)| h.dist(&shot.hash) <= settings.phash_threshold)
                    .map(|(_, j)| j.clone());
                let (j, was_cached) = match reuse {
                    Some(j) => (j, true),
                    None => match llm::judge_image(
                        &model,
                        &task,
                        &s,
                        ax_for(&ax_cell, &s, &settings).await.as_ref(),
                        llm::Picture { jpeg_b64: shot.jpeg_b64, width: shot.width, height: shot.height },
                        settings.confidence,
                    )
                    .await
                    {
                        Ok(j) => {
                            state.core.lock().unwrap().last_vision = Some((shot.hash, j.clone()));
                            (j, false)
                        }
                        Err(e) => {
                            let _ = state.store.lock().unwrap().log_llm("vision", false, false);
                            return StageResult::Pass(Some(format!("视觉模型：{e}")));
                        }
                    },
                };
                let _ = state.store.lock().unwrap().log_llm("vision", was_cached, true);
                match j.focused() {
                    Some(focused) => StageResult::Decided(Verdict {
                        focused,
                        stage: "vision",
                        reason: j.reason,
                        confidence: j.confidence,
                        cached: was_cached,
                    }),
                    None => StageResult::Pass(Some(format!("视觉模型也无法判断：{}", j.reason))),
                }
            })
        }));
    }
    stages
}

async fn run_sample(app: &AppHandle, state: &Arc<AppState>) {
    let Some(sample) = sampler::sample().await else { return };
    let settings = Arc::new(state.settings());
    let task = Arc::new({
        let core = state.core.lock().unwrap();
        llm::task_context(&core.timer.task, &core.round_allow)
    });

    let own = sample.pid == std::process::id() as u64;
    let (outcome, notes) = if own && sample.idle_secs <= settings.idle_secs {
        (Outcome::Undecided, vec!["本应用在前台，跳过".to_string()])
    } else {
        let s = Arc::new(sample.clone());
        run_chain(build_stages(state, &s, &settings, &task)).await
    };

    // The round may have ended or been paused by the user while we waited on a model.
    let shown_app = sample.bundle_name.clone().unwrap_or(sample.app.clone());
    let target = crate::rules::target_of(&shown_app, sample.url.as_deref());
    let (signal, alerted, recovered, streak, session, came_back, first) = {
        let mut core = state.core.lock().unwrap();
        if core.timer.phase != Phase::Focus || core.timer.pause == Some(PauseReason::User) {
            return;
        }
        let mut came_back = false;
        match outcome {
            Outcome::Away => {
                core.timer.pause.get_or_insert(PauseReason::Away);
                core.nudging = false;
            }
            // Back from being away: carry on, or hold until the person clicks to continue.
            _ if core.timer.pause == Some(PauseReason::Away) => {
                core.returned = Some(core.timer.paused_for);
                core.returned_age = 0;
                core.waiting = Waiting::start();
                core.timer.pause = (!settings.auto_resume).then_some(PauseReason::User);
                came_back = true;
            }
            _ => {}
        }
        if matches!(&outcome, Outcome::Decided(v) if !v.focused) && core.timer.total > 0 {
            let tick = core.timer.elapsed() * DIAL_TICKS / core.timer.total;
            let tick = tick.min(DIAL_TICKS - 1);
            if !core.distracted_ticks.contains(&tick) {
                core.distracted_ticks.push(tick);
            }
        }
        let signal = match &outcome {
            Outcome::Decided(v) if v.focused => Signal::Focused,
            Outcome::Decided(_) => Signal::Distracted,
            _ => Signal::Neutral,
        };
        let alerted = core.smoother.feed(signal, settings.streak, store::now());
        if alerted {
            core.nudging = true;
            core.nudge_target = Some(target.clone());
        }
        // The reminder has done its job once the person is back on the task, or (when nothing
        // can tell, as with no model set up) has at least left what it was about. Away counts
        // too: nobody is there to read it.
        let left = match signal {
            Signal::Focused => true,
            Signal::Distracted => false,
            Signal::Neutral => outcome == Outcome::Away || (!own && core.nudge_target.as_deref() != Some(target.as_str())),
        };
        let recovered = !alerted && left && std::mem::take(&mut core.nudging);
        (signal, alerted, recovered, core.smoother.streak, core.session_id, came_back, core.smoother.repeats() <= 1)
    };

    let (stage, result, reason, confidence, cached) = match &outcome {
        Outcome::Away => ("idle".to_string(), "away", format!("{} 秒无键鼠操作，已暂停计时", sample.idle_secs), None, false),
        Outcome::Decided(v) => (
            v.stage.to_string(),
            if v.focused { "focused" } else { "distracted" },
            v.reason.clone(),
            Some(v.confidence),
            v.cached,
        ),
        Outcome::Undecided => ((if own { "self" } else { "none" }).to_string(), "none", "无法判断，不计入".to_string(), None, false),
    };
    let _ = signal;

    if !own {
        let db = state.store.lock().unwrap();
        let _ = db.log_event(session, &stage, result, &sample, &reason);
        if alerted {
            let _ = db.log_alert(session, &stage, &sample, &reason);
        }
    }
    // The first reminder of a distraction is a tap on the shoulder; only the repeats insist.
    if alerted && settings.sound {
        play(if first { "Tink" } else { "Sosumi" });
    }

    let allow_target = crate::rules::round_allow_target(&settings.rules, &shown_app, sample.url.as_deref())
        .filter(|_| sample.url.is_some() || !crate::rules::is_browser(&sample.app));
    let ev = VerdictEvent {
        at: store::now(),
        stage,
        result: result.to_string(),
        reason,
        confidence,
        cached,
        notes,
        app: shown_app,
        title: sample.title.clone(),
        url: sample.url.clone(),
        allow_target,
        streak,
        threshold: settings.streak,
        alerted,
    };
    {
        let mut core = state.core.lock().unwrap();
        // Our own popover in front says nothing about the work: keep showing the last real sample.
        if !own {
            core.last = Some(ev.clone());
            core.recent.push_back(ev.clone());
            while core.recent.len() > 20 {
                core.recent.pop_front();
            }
        }
    }
    let _ = app.emit("supervisor://verdict", &ev);
    emit_tick(app, state);
    if alerted {
        crate::windows::show_popover(app, "nudge");
    } else if recovered {
        crate::windows::hide_passive(app);
    } else if came_back {
        // Say what happened while they were gone, and (when it waits for them) offer to carry on.
        crate::windows::show_popover(app, "passive");
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

    #[test]
    fn saying_you_were_there_stretches_the_idle_limit() {
        assert_eq!(idle_limit(120, false), 120);
        assert_eq!(idle_limit(120, true), 360);
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
