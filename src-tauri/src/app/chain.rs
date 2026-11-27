//! One sample's trip through the decision chain, and what its verdict sets off.
use super::tray::{emit_tick, play};
use super::waiting::Waiting;
use super::{AppState, DIAL_TICKS, SampleResult, VerdictEvent, idle_limit};
use crate::ax::{self, AxInfo};
use crate::llm;
use crate::pipeline::{Decider, Outcome, Stage, StageResult, Verdict, run_chain};
use crate::sampler::{self, Sample};
use crate::settings::Settings;
use crate::smoother::Signal;
use crate::store;
use crate::timer::{PauseReason, Phase};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

/// Window text helps where titles say little (chat apps, Electron), so it is part of the key, digits aside.
fn cache_key(task: &str, s: &Sample, ax: Option<&AxInfo>) -> String {
    let ax = ax.map(ax::fingerprint).unwrap_or_default();
    format!("{task}\u{1}{}\u{1}{}\u{1}{}\u{1}{ax}", s.app, s.title, s.url.as_deref().unwrap_or(""))
}

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
                    StageResult::Pass(Some(crate::i18n::tr().idle_but_media.into()))
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
                // The round's own list is reported as the rules stage, flagged as round-allowed (see run_sample).
                let hit = crate::rules::round_allow(&extra, &s)
                    .map(|h| (h, Decider::Round))
                    .or_else(|| crate::rules::evaluate(&settings.rules, &s).map(|h| (h, Decider::Rules)));
                match hit {
                    Some(((focused, reason), by)) => StageResult::Decided(Verdict { focused, by, reason, confidence: 1.0, cached: false }),
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
                            return StageResult::Pass(Some((crate::i18n::tr().text_model_err)(&e.to_string())));
                        }
                    },
                };
                let _ = state.store.lock().unwrap().log_llm("text", was_cached, true);
                match j.focused() {
                    Some(focused) if j.confidence >= settings.confidence => StageResult::Decided(Verdict {
                        focused,
                        by: Decider::Text,
                        reason: j.reason,
                        confidence: j.confidence,
                        cached: was_cached,
                    }),
                    _ => StageResult::Pass(Some((crate::i18n::tr().text_model_unsure)(j.confidence, &j.reason))),
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
                    Ok(Err(e)) => return StageResult::Pass(Some((crate::i18n::tr().shot_err)(&e.to_string()))),
                    Err(e) => return StageResult::Pass(Some((crate::i18n::tr().shot_err)(&e.to_string()))),
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
                            return StageResult::Pass(Some((crate::i18n::tr().vision_model_err)(&e.to_string())));
                        }
                    },
                };
                let _ = state.store.lock().unwrap().log_llm("vision", was_cached, true);
                match j.focused() {
                    Some(focused) => StageResult::Decided(Verdict {
                        focused,
                        by: Decider::Vision,
                        reason: j.reason,
                        confidence: j.confidence,
                        cached: was_cached,
                    }),
                    None => StageResult::Pass(Some((crate::i18n::tr().vision_model_unsure)(&j.reason))),
                }
            })
        }));
    }
    stages
}

pub(super) async fn run_sample(app: &AppHandle, state: &Arc<AppState>) {
    let Some(sample) = sampler::sample().await else { return };
    let settings = Arc::new(state.settings());
    let task = Arc::new({
        let core = state.core.lock().unwrap();
        llm::task_context(&core.timer.task, &core.round_allow)
    });

    let own = sample.pid == std::process::id() as u64;
    let (outcome, notes) = if own && sample.idle_secs <= settings.idle_secs {
        (Outcome::Undecided, vec![crate::i18n::tr().self_in_front.to_string()])
    } else {
        let s = Arc::new(sample.clone());
        run_chain(build_stages(state, &s, &settings, &task)).await
    };

    // The round may have ended or been paused by the user while we waited on a model.
    let shown_app = sample.bundle_name.clone().unwrap_or(sample.app.clone());
    let target = crate::rules::target_of(&shown_app, sample.url.as_deref());
    let (alerted, recovered, streak, session, came_back, first) = {
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
        (alerted, recovered, core.smoother.streak, core.session_id, came_back, core.smoother.repeats() <= 1)
    };

    let (stage, result, reason, confidence, cached) = match &outcome {
        Outcome::Away => (super::Stage::Idle, SampleResult::Away, (crate::i18n::tr().away)(sample.idle_secs), None, false),
        Outcome::Decided(v) => (
            match v.by {
                Decider::Round | Decider::Rules => super::Stage::Rules,
                Decider::Text => super::Stage::Text,
                Decider::Vision => super::Stage::Vision,
            },
            if v.focused { SampleResult::Focused } else { SampleResult::Distracted },
            v.reason.clone(),
            Some(v.confidence),
            v.cached,
        ),
        Outcome::Undecided => (
            if own { super::Stage::Own } else { super::Stage::None },
            SampleResult::None,
            crate::i18n::tr().undecided.to_string(),
            None,
            false,
        ),
    };

    if !own {
        let db = state.store.lock().unwrap();
        let _ = db.log_event(session, stage.as_str(), result.as_str(), &sample, &reason);
        if alerted {
            let _ = db.log_alert(session, stage.as_str(), &sample, &reason);
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
        result,
        reason,
        confidence,
        cached,
        notes,
        app: shown_app,
        title: sample.title.clone(),
        url: sample.url.clone(),
        allow_target,
        round_allowed: matches!(&outcome, Outcome::Decided(v) if v.by == Decider::Round),
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
