//! The 1-second loop: counts the timer down, chases unanswered cards, and starts a sample when one is due.
use super::tray::{emit_tick, notify, play};
use super::waiting::{CHASE_BREAK, CHASE_DONE, EndStep, PRESENT_SECS, RETURNED_SECS, Waiting};
use super::{AppState, chain};
use crate::sampler;
use crate::timer::{PauseReason, Phase};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Manager};

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
                let (title, body) = crate::i18n::tr().pause_over;
                notify(&app, title, body);
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
                chain::run_sample(&app2, &state).await;
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
            let (title, body) = crate::i18n::tr().round_done;
            notify(app, title, body);
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
            let (title, body) = crate::i18n::tr().break_over;
            notify(app, title, body);
            emit_tick(app, state);
            crate::windows::show_popover(app, "passive");
        }
        Phase::Idle => {}
    }
    if sound {
        play("Glass");
    }
}
