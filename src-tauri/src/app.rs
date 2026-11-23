//! App state, the 1-second loop, and one sample's trip through the decision chain.
use crate::ax::{self, AxInfo};
use crate::llm::{self, ChatModel, Judgement};
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
}

#[derive(Default)]
pub struct Models {
    pub text: Option<Arc<ChatModel>>,
    pub vision: Option<Arc<ChatModel>>,
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
    }

    pub fn stop(&self) {
        let mut core = self.core.lock().unwrap();
        self.close_session(&mut core, false);
        core.timer.stop();
        core.timer.pending_break = None;
        core.smoother.reset();
        core.rest_left = None;
    }

    pub fn set_user_pause(&self, paused: bool) {
        let mut core = self.core.lock().unwrap();
        if core.timer.phase == Phase::Idle {
            return;
        }
        core.timer.pause = paused.then_some(PauseReason::User);
        core.rest_left = None;
    }

    /// Pause the focus round for a few minutes, then carry on by itself.
    pub fn rest(&self, minutes: u32) {
        let mut core = self.core.lock().unwrap();
        if core.timer.phase != Phase::Focus {
            return;
        }
        core.timer.pause = Some(PauseReason::User);
        core.rest_left = Some(minutes.clamp(1, 60) * 60);
        core.smoother.reset();
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
        Phase::Idle => String::new(),
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
            let ended = {
                let mut core = state.core.lock().unwrap();
                // A timed rest during focus counts down, then the round carries on.
                if let Some(left) = core.rest_left {
                    if core.timer.pause != Some(PauseReason::User) {
                        core.rest_left = None;
                    } else if left <= 1 {
                        core.rest_left = None;
                        core.timer.pause = None;
                    } else {
                        core.rest_left = Some(left - 1);
                    }
                }
                core.timer.tick()
            };
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
            }
            notify(app, "番茄完成", "看看这一轮的回顾，再决定休息还是继续。");
            crate::windows::open(app, "review");
        }
        Phase::ShortBreak | Phase::LongBreak => {
            state.core.lock().unwrap().timer.stop();
            notify(app, "休息结束", "准备好就开始下一个番茄。");
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
        let (s, settings) = (s.clone(), settings.clone());
        stages.push(Box::new(move || {
            Box::pin(async move { if s.idle_secs > settings.idle_secs { StageResult::Away } else { StageResult::Pass(None) } })
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
                    None => match llm::judge_text(&model, &task, &s, ax.as_ref()).await {
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
                    None => match llm::judge_image(&model, &task, &s, ax_for(&ax_cell, &s, &settings).await.as_ref(), shot.jpeg_b64).await {
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
    let task = Arc::new(state.core.lock().unwrap().timer.task.clone());

    let own = sample.pid == std::process::id() as u64;
    let (outcome, notes) = if own && sample.idle_secs <= settings.idle_secs {
        (Outcome::Undecided, vec!["本应用在前台，跳过".to_string()])
    } else {
        let s = Arc::new(sample.clone());
        run_chain(build_stages(state, &s, &settings, &task)).await
    };

    // The round may have ended or been paused by the user while we waited on a model.
    let (signal, alerted, streak, session) = {
        let mut core = state.core.lock().unwrap();
        if core.timer.phase != Phase::Focus || core.timer.pause == Some(PauseReason::User) {
            return;
        }
        match outcome {
            Outcome::Away => {
                core.timer.pause.get_or_insert(PauseReason::Away);
            }
            // Back from being away: carry on, or hold until the person clicks to continue.
            _ if core.timer.pause == Some(PauseReason::Away) => {
                core.timer.pause = (!settings.auto_resume).then_some(PauseReason::User);
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
        let alerted = core.smoother.feed(signal, settings.streak);
        (signal, alerted, core.smoother.streak, core.session_id)
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
    if alerted && settings.sound {
        play("Sosumi");
    }

    let ev = VerdictEvent {
        at: store::now(),
        stage,
        result: result.to_string(),
        reason,
        confidence,
        cached,
        notes,
        app: sample.bundle_name.clone().unwrap_or(sample.app.clone()),
        title: sample.title.clone(),
        url: sample.url.clone(),
        streak,
        threshold: settings.streak,
        alerted,
    };
    {
        let mut core = state.core.lock().unwrap();
        core.last = Some(ev.clone());
        if !own {
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
    }
}
