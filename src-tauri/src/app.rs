//! App state, the 1-second loop, and one sample's trip through the decision chain.
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
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

pub struct Core {
    pub timer: Timer,
    pub smoother: Smoother,
    pub session_id: Option<i64>,
    pub last: Option<VerdictEvent>,
    pub text_cache: LruCache<String, Judgement>,
    pub last_vision: Option<(ImageHash, Judgement)>,
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
        }
    }

    // ---- timer actions ----

    pub fn start_focus(&self, task: String) {
        let s = self.settings();
        let mut core = self.core.lock().unwrap();
        self.close_session(&mut core, false);
        if core.timer.task != task {
            core.text_cache.clear();
            core.last_vision = None;
        }
        core.timer.task = task.clone();
        core.timer.start(Phase::Focus, s.focus_min * 60);
        core.smoother.reset();
        core.last = None;
        core.session_id = self.store.lock().unwrap().start_session(&task, s.focus_min * 60).ok();
        self.since_sample.store(0, Ordering::Relaxed);
    }

    /// `after_focus`: a focus round just completed and counts toward the long break.
    /// A break started by hand from idle is always a short one and counts nothing.
    pub fn start_break(&self, after_focus: bool) {
        let s = self.settings();
        let mut core = self.core.lock().unwrap();
        self.close_session(&mut core, false);
        let phase = if after_focus { core.timer.next_break(s.long_break_every) } else { Phase::ShortBreak };
        let mins = if phase == Phase::LongBreak { s.long_break_min } else { s.short_break_min };
        core.timer.start(phase, mins * 60);
    }

    pub fn stop(&self) {
        let mut core = self.core.lock().unwrap();
        self.close_session(&mut core, false);
        core.timer.stop();
        core.smoother.reset();
    }

    pub fn set_user_pause(&self, paused: bool) {
        let mut core = self.core.lock().unwrap();
        if core.timer.phase == Phase::Idle {
            return;
        }
        core.timer.pause = paused.then_some(PauseReason::User);
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

fn update_tray(app: &AppHandle, v: &TimerView) {
    let Some(tray) = app.tray_by_id("main") else { return };
    let title = match (v.phase, v.paused) {
        (Phase::Idle, _) => String::new(),
        (_, Some(PauseReason::Away)) => format!(" 离开 {}", fmt_mmss(v.remaining)),
        (_, Some(PauseReason::User)) => format!(" ⏸ {}", fmt_mmss(v.remaining)),
        (Phase::Focus, None) => format!(" {}", fmt_mmss(v.remaining)),
        (_, None) => format!(" ☕ {}", fmt_mmss(v.remaining)),
    };
    let _ = tray.set_title(Some(title));
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
            let ended = state.core.lock().unwrap().timer.tick();
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
            {
                let mut core = state.core.lock().unwrap();
                state.close_session(&mut core, true);
            }
            state.start_break(true);
            let mins = state.view().total / 60;
            notify(app, "🍅 番茄完成！", &format!("休息 {mins} 分钟吧。"));
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

fn cache_key(task: &str, s: &Sample) -> String {
    format!("{task}\u{1}{}\u{1}{}\u{1}{}", s.app, s.title, s.url.as_deref().unwrap_or(""))
}

fn build_stages(state: &Arc<AppState>, s: &Arc<Sample>, settings: &Arc<Settings>, task: &Arc<String>) -> Vec<Stage> {
    let mut stages: Vec<Stage> = Vec::new();

    // 1. Idle for longer than the threshold: the person has left.
    {
        let (s, settings) = (s.clone(), settings.clone());
        stages.push(Box::new(move || {
            Box::pin(async move { if s.idle_secs > settings.idle_secs { StageResult::Away } else { StageResult::Pass(None) } })
        }));
    }

    // 2. App / domain / title rules.
    {
        let (s, settings) = (s.clone(), settings.clone());
        stages.push(Box::new(move || {
            Box::pin(async move {
                match crate::rules::evaluate(&settings.rules, &s) {
                    Some((focused, reason)) => {
                        StageResult::Decided(Verdict { focused, stage: "rules", reason, confidence: 1.0, cached: false })
                    }
                    None => StageResult::Pass(None),
                }
            })
        }));
    }

    // 3. Text model: title + URL + task. Only a confident answer concludes.
    {
        let (state, s, settings, task) = (state.clone(), s.clone(), settings.clone(), task.clone());
        stages.push(Box::new(move || {
            Box::pin(async move {
                let model = state.models.read().unwrap().text.clone();
                let Some(model) = model else { return StageResult::Pass(None) };
                let key = cache_key(&task, &s);
                let cached = state.core.lock().unwrap().text_cache.get(&key).cloned();
                let (j, was_cached) = match cached {
                    Some(j) => (j, true),
                    None => match llm::judge_text(&model, &task, &s).await {
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
        let (state, s, settings, task) = (state.clone(), s.clone(), settings.clone(), task.clone());
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
                    None => match llm::judge_image(&model, &task, &s, shot.jpeg_b64).await {
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
            _ if core.timer.pause == Some(PauseReason::Away) => core.timer.pause = None,
            _ => {}
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
    if alerted {
        let what = sample.bundle_name.clone().unwrap_or_else(|| sample.app.clone());
        notify(app, "⚠️ 你好像分心了", &format!("{what}：{reason}"));
        if settings.sound {
            play("Sosumi");
        }
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
    state.core.lock().unwrap().last = Some(ev.clone());
    let _ = app.emit("supervisor://verdict", &ev);
    emit_tick(app, state);
}
