import { useEffect, useLayoutEffect, useRef, useState } from "react";
import {
  allowLabel,
  api,
  hms,
  isPage,
  hostOf,
  mmss,
  onPopoverMode,
  onTick,
  onVerdict,
  openSettings,
  targetOf,
  type Review,
  type TimerView,
  type VerdictEvent,
} from "../lib/api";
import { Dial, MoreIcon, PauseIcon, PlusIcon, StageBadge, TimerGlyph } from "../components/ui";
import { t } from "../i18n";

const p = t.popover;
const dur = t.common.dur;
const PRESETS = [25, 45, 60];
const quoted = (task: string) => p.quote(task.length > 28 ? `${task.slice(0, 27)}…` : task);

function More() {
  return (
    <button className="icon-btn" aria-label={p.more} onClick={() => api.popoverMenu()}>
      <MoreIcon />
    </button>
  );
}

/** How the round will be supervised: by a model as well as the lists, or by the lists alone. */
type Ai = "on" | "off" | "broken";

/** Permissions still missing: screen recording, and Accessibility while window text is turned on. */
type Missing = { screen: boolean; ax: boolean };

function PermissionBanner({ missing }: { missing: Missing }) {
  if (!missing.screen) {
    return (
      <div className="perm">
        <b>{p.perm.axTitle}</b>
        <p>{p.perm.axBody}</p>
        <div className="btn-row">
          <button
            className="btn small"
            onClick={async () => {
              await api.requestAccessibility();
              await api.openPrivacy("accessibility");
            }}
          >
            {p.perm.grant}
          </button>
        </div>
      </div>
    );
  }
  return (
    <div className="perm">
      <b>{p.perm.screenTitle}</b>
      <p>{p.perm.screenBody}</p>
      <div className="btn-row">
        <button
          className="btn small"
          onClick={async () => {
            await api.requestScreenRecording();
            await api.openPrivacy("screen");
          }}
        >
          {p.perm.grant}
        </button>
        {missing.ax && (
          <button className="btn small" onClick={() => api.openPrivacy("accessibility")}>
            {p.perm.accessibility}
          </button>
        )}
        <button className="btn small" onClick={() => api.openPrivacy("automation")}>
          {p.perm.browserUrls}
        </button>
      </div>
    </div>
  );
}

/** One line that says, before the round starts, who will be watching. */
function Supervision({ ai, task }: { ai: Ai; task: string }) {
  if (ai === "on") {
    return task.trim() ? (
      <div className="sup on">
        <span className="dot" aria-hidden="true" />
        <span className="txt">{p.sup.ready}</span>
      </div>
    ) : (
      <div className="sup">
        <span className="dot" aria-hidden="true" />
        <span className="txt">{p.sup.noTask}</span>
      </div>
    );
  }
  return (
    <div className={ai === "broken" ? "sup bad" : "sup"}>
      <span className="dot" aria-hidden="true" />
      <span className="txt">
        {ai === "broken" ? p.sup.broken : p.sup.off}
      </span>
      <button className="link" onClick={() => openSettings("models")}>
        {ai === "broken" ? p.sup.check : p.sup.configure}
      </button>
    </div>
  );
}

/** `fresh`: a different task was asked for, so neither the last task nor what was allowed for it carries over. */
function StartCard({ view, completed, missing, ai, fresh, onBack }: { view: TimerView; completed: number; missing: Missing; ai: Ai; fresh: boolean; onBack: () => void }) {
  const [task, setTask] = useState(fresh ? "" : view.task);
  const [minutes, setMinutes] = useState(view.last_focus_min);
  const [allow, setAllow] = useState<string[]>(fresh ? [] : view.round_allow);
  const [adding, setAdding] = useState<string | null>(null);
  // The usual lengths, plus the one from Settings and the one used last.
  const presets = [...new Set([...PRESETS, Math.round(view.focus_secs / 60), view.last_focus_min])].sort((a, b) => a - b);

  const addAllow = () => {
    const v = adding?.trim();
    if (v && !allow.some((a) => a.toLowerCase() === v.toLowerCase())) setAllow([...allow, v]);
    setAdding(null);
  };
  const start = () => api.startFocus(task.trim(), minutes, allow).then(() => api.closeWindow());

  return (
    <>
      <div className="card-head">
        <div className="title">{fresh ? p.start.titleFresh : p.start.title}</div>
        <More />
      </div>
      {(missing.screen || missing.ax) && <PermissionBanner missing={missing} />}
      <div className="field-col">
        <label className="label" htmlFor="task">
          {p.start.task}
        </label>
        <textarea
          id="task"
          className="textarea"
          rows={3}
          autoFocus
          maxLength={200}
          placeholder={p.start.taskPlaceholder}
          value={task}
          onFocus={(e) => e.target.select()}
          onChange={(e) => setTask(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey && !e.nativeEvent.isComposing) {
              e.preventDefault();
              start();
            }
          }}
        />
        <Supervision ai={ai} task={task} />
      </div>
      <div className="field-col">
        <div className="label">{p.start.duration}</div>
        <div className="segmented" role="group" aria-label={p.start.duration}>
          {presets.map((m) => (
            <button key={m} aria-pressed={minutes === m} onClick={() => setMinutes(m)}>
              {t.common.minutes(m)}
            </button>
          ))}
        </div>
      </div>
      <div className="field-col">
        <div className="label">{p.start.allowed}</div>
        <div className="chips">
          {allow.map((a) => (
            <button key={a} className="chip" aria-label={p.start.remove(allowLabel(a))} title={isPage(a) ? `${a}\n${p.start.clickToRemove}` : p.start.clickToRemove} onClick={() => setAllow(allow.filter((x) => x !== a))}>
              {allowLabel(a)}
              <span className="x" aria-hidden="true">
                ×
              </span>
            </button>
          ))}
          {adding === null ? (
            <button className="chip add" onClick={() => setAdding("")}>
              <PlusIcon />
              <span>{p.start.add}</span>
            </button>
          ) : (
            <input
              className="input chip-input"
              aria-label={p.start.appOrDomain}
              placeholder={p.start.appOrDomain}
              autoFocus
              value={adding}
              onChange={(e) => setAdding(e.target.value)}
              onBlur={addAllow}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.nativeEvent.isComposing) addAllow();
                if (e.key === "Escape") setAdding(null);
              }}
            />
          )}
        </div>
        <div className="hint">{p.start.allowedHint}</div>
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
        <button className="big-btn" onClick={start}>
          {p.start.startFocus}
        </button>
        <div className="center-note">{completed ? p.start.doneTodayEnter(completed) : p.start.enter}</div>
      </div>
      {fresh && (
        <div className="links">
          {view.next_break_min != null && (
            <button className="link" onClick={() => api.startBreak().then(() => api.closeWindow())}>
              {p.start.breakFirst(view.next_break_min)}
            </button>
          )}
          <button className="link" onClick={onBack}>
            {p.start.back}
          </button>
        </div>
      )}
    </>
  );
}

/** What the supervisor makes of the window in front, in plain words; the pipeline detail is one click away. */
function Judgement({ v }: { v: VerdictEvent | null }) {
  if (!v) {
    return (
      <div className="state">
        <span className="mark" aria-hidden="true" />
        <div className="what">
          <div className="name muted">{p.judge.firstLook}</div>
        </div>
      </div>
    );
  }
  const where = targetOf(v);
  const off = v.result === "distracted";
  const tone = off ? "state hot" : v.result === "focused" ? "state ok" : "state";
  const name = off ? p.judge.off : v.result === "focused" ? p.judge.on : p.judge.unsure;
  return (
    <div className="field-col">
      <div className={tone}>
        <span className="mark" aria-hidden="true" />
        <div className="what">
          <div className="name">{name}</div>
          <div className="sub ellipsis" title={v.url ?? v.title}>
            {where}
          </div>
        </div>
        {off && v.allow_target && (
          <button
            className="btn small"
            title={isPage(v.allow_target) ? p.judge.allowPageTitle : p.judge.allowTargetTitle(v.allow_target)}
            onClick={() => api.allowForRound(v.allow_target!).then(() => api.closeWindow())}
          >
            {isPage(v.allow_target) ? p.judge.pageCounts : p.judge.counts}
          </button>
        )}
      </div>
      <details className="why">
        <summary>{p.judge.why}</summary>
        <div className="inner">
          <div className="why-line">
            <StageBadge v={v} />
            <span className="ellipsis" title={v.reason}>
              {v.reason}
            </span>
          </div>
          <div className="hint ellipsis" title={v.title}>
            {v.app}
            {v.title ? ` · ${v.title}` : ""}
          </div>
        </div>
      </details>
    </div>
  );
}

/**
 * Ending before the bell is two different things: the task got done (good news: keep the
 * time, take the break) or the round is being given up. Ask which; pausing needs no question.
 */
function RoundActions({ view, children }: { view: TimerView; children: React.ReactNode }) {
  const [asking, setAsking] = useState(false);
  if (asking) {
    const elapsed = view.total - view.remaining;
    return (
      <div className="confirm">
        <div className="body-note">{p.end.ask(dur(elapsed))}</div>
        <button className="btn primary" style={{ height: 34, borderRadius: 9 }} onClick={() => api.finishEarly()}>
          {p.end.taskDone}
        </button>
        <div className="hint">{elapsed * 2 >= view.total ? p.end.countsHint : p.end.notHalfHint}</div>
        <div className="btn-row">
          <button className="btn grow danger" onClick={() => api.stop()}>
            {p.end.giveUp}
          </button>
          <button className="btn grow" onClick={() => setAsking(false)}>
            {p.end.keepFocusing}
          </button>
        </div>
      </div>
    );
  }
  return (
    <div className="btn-row">
      <button className="btn" onClick={() => setAsking(true)}>
        {p.end.end}
      </button>
      {children}
    </div>
  );
}

function FocusCard({ view, verdict, completed }: { view: TimerView; verdict: VerdictEvent | null; completed: number }) {
  const progress = view.total ? 1 - view.remaining / view.total : 0;
  return (
    <>
      <div className="card-head">
        <span className="pill hot">
          <span className="dot" />
          {p.focus.pill}
        </span>
        <div className="side">
          <span>{p.nthToday(completed + 1)}</span>
          <More />
        </div>
      </div>
      <Dial progress={progress} distracted={view.distracted_ticks} tone="focus">
        <div className="clock">{mmss(view.remaining)}</div>
        <div className="sub">{p.focus.remainingOf(Math.round(view.total / 60))}</div>
      </Dial>
      <div className="task-block">
        <div className="cap">{p.focus.currentTask}</div>
        <div className="task">{view.task || p.focus.noTask}</div>
      </div>
      <Judgement v={verdict} />
      <RoundActions view={view}>
        <button className="btn grow" onClick={() => api.pause()}>
          {p.focus.pauseFor(view.short_break_min)}
        </button>
      </RoundActions>
    </>
  );
}

function PausedCard({ view, completed }: { view: TimerView; completed: number }) {
  const away = view.paused === "away";
  const timed = view.rest_left != null;
  // Back from being away, with the round held until they say so.
  const back = !away && view.returned != null;
  const progress = view.total ? 1 - view.remaining / view.total : 0;
  const idle = view.idle_secs;
  return (
    <>
      <div className="card-head">
        <span className="pill">
          <PauseIcon />
          {away ? p.paused.pillAway : back ? p.paused.pillBack : p.paused.pill}
        </span>
        <div className="side">
          <span>{p.nthToday(completed + 1)}</span>
          <More />
        </div>
      </div>
      <Dial progress={progress} distracted={view.distracted_ticks} tone="dim">
        <div className="clock">{mmss(view.remaining)}</div>
        <div className="sub">{timed ? p.paused.resumesIn(mmss(view.rest_left!)) : back ? p.paused.waiting : p.paused.holding}</div>
      </Dial>
      <div className="body-note">
        {away ? p.paused.awayNote(dur(idle)) : back ? p.paused.backNote(dur(view.returned!)) : timed ? p.paused.timedNote() : p.paused.heldNote()}
      </div>
      <div className="rule" />
      {!back && (
        <div className="kv">
          <span className="muted">{away ? p.paused.awayFor : p.paused.pausedFor}</span>
          <span className="num" style={{ fontWeight: 500 }}>
            {dur(view.paused_for + (away ? idle : 0))}
          </span>
        </div>
      )}
      {(away || back) && (
        <label className="check">
          <input type="checkbox" checked={view.auto_resume} onChange={(e) => api.setAutoResume(e.target.checked)} />
          <span>{p.paused.autoResume}</span>
        </label>
      )}
      <RoundActions view={view}>
        {back && (
          <button className="btn grow" onClick={() => api.stillHere()}>
            {p.paused.stillHere}
          </button>
        )}
        {timed && (
          <button className="btn grow" onClick={() => api.rest(view.short_break_min)}>
            {p.paused.restMore(view.short_break_min)}
          </button>
        )}
        <button className="btn primary grow" onClick={() => api.resume()}>
          {p.paused.resume}
        </button>
      </RoundActions>
    </>
  );
}

/** Back from being away and the timer already runs again: say what happened, offer the correction, go away by itself. */
function ReturnedCard({ view }: { view: TimerView }) {
  const close = () => api.closeWindow();
  return (
    <>
      <div className="card-head">
        <span className="pill">{p.returned.pill}</span>
        <div className="side">
          <span className="num">{p.left(mmss(view.remaining))}</span>
        </div>
      </div>
      <div className="body-note" style={{ textAlign: "left" }}>
        {p.returned.note(dur(view.returned!))}
      </div>
      <div className="btn-row">
        <button className="btn grow" title={p.returned.countItTitle} onClick={() => api.stillHere().then(close)}>
          {p.returned.countIt}
        </button>
        <button className="btn primary grow" onClick={() => api.resume().then(close)}>
          {p.returned.ok}
        </button>
      </div>
    </>
  );
}

function BreakCard({ view, completed }: { view: TimerView; completed: number }) {
  const progress = view.total ? 1 - view.remaining / view.total : 0;
  const long = view.phase === "long_break";
  return (
    <>
      <div className="card-head">
        <span className="pill">{long ? p.rest.longPill : p.rest.pill}</span>
        <div className="side">
          <span>{p.doneToday(completed)}</span>
          <More />
        </div>
      </div>
      <Dial progress={progress} tone="rest">
        <div className="clock">{mmss(view.remaining)}</div>
        <div className="sub">{p.rest.breakOf(Math.round(view.total / 60))}</div>
      </Dial>
      <div className="body-note">{p.rest.note()}</div>
      <div className="btn-row">
        <button className="btn grow" onClick={() => api.openWindow("review")}>
          {p.rest.review}
        </button>
        <button className="btn grow" onClick={() => api.startFocus(view.task, view.last_focus_min, view.round_allow).then(() => api.closeWindow())}>
          {p.rest.skip}
        </button>
      </div>
    </>
  );
}

/** Back to the start card to type a different task; the card may have come up without the keyboard. */
/** A round just finished: take the break it earned, go again, or switch task. Never loses the break. */
function DoneCard({ view, completed, review, onChangeTask }: { view: TimerView; completed: number; review: Review | null; onChangeTask: () => void }) {
  const again = () => api.startFocus(view.task, view.last_focus_min, view.round_allow).then(() => api.closeWindow());
  const rest = () => api.startBreak().then(() => api.closeWindow());
  const m = (s: number) => Math.round(s / 60);
  return (
    <>
      <div className="card-head">
        <span className="pill hot">{review && !review.completed ? p.done.early : p.done.pill}</span>
        <div className="side">
          <span>{p.nthToday(Math.max(1, completed))}</span>
          <More />
        </div>
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
        <div className="after-task">{view.task || p.done.roundOver}</div>
        {review && (
          <div className="after-stats">
            {p.done.statFocus(m(review.focus_secs))}
            {review.distract_secs >= 30 ? ` · ${p.done.statDistracted(m(review.distract_secs))}` : ` · ${p.done.statCalm}`}
            {review.alerts > 0 ? ` · ${p.done.statAlerts(review.alerts)}` : ""}
          </div>
        )}
        {view.overtime >= 60 && <div className="after-stats over">{p.done.overtime(dur(view.overtime))}</div>}
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        <button className="big-btn" onClick={rest}>
          {p.done.takeBreak(view.next_break_min!)}
        </button>
        <div className="btn-row">
          <button className="btn grow" onClick={again}>
            {p.done.again}
          </button>
          <button className="btn grow" onClick={onChangeTask}>
            {p.done.changeTask}
          </button>
        </div>
      </div>
      <div className="links">
        <button className="link" onClick={() => api.openWindow("review")}>
          {p.done.reviewRound}
        </button>
      </div>
    </>
  );
}

/** A break just ran out: one click carries on with the same task. */
function BreakOverCard({ view, completed, onChangeTask }: { view: TimerView; completed: number; onChangeTask: () => void }) {
  const again = () => api.startFocus(view.task, view.last_focus_min, view.round_allow).then(() => api.closeWindow());
  return (
    <>
      <div className="card-head">
        <span className="pill">{p.breakOver.pill}</span>
        <div className="side">
          <span>{p.doneToday(completed)}</span>
          <More />
        </div>
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
        <div className="after-task">{view.task ? p.breakOver.continueTask(quoted(view.task)) : p.breakOver.next}</div>
        <div className="after-stats">{p.breakOver.same(view.last_focus_min)}</div>
        {view.overtime >= 60 && <div className="after-stats over">{p.breakOver.overtime(dur(view.overtime))}</div>}
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        <button className="big-btn" onClick={again}>
          {p.breakOver.startFocus}
        </button>
        <div className="btn-row">
          <button className="btn grow" onClick={onChangeTask}>
            {p.done.changeTask}
          </button>
          <button className="btn grow" onClick={() => api.stop().then(() => api.closeWindow())}>
            {p.breakOver.stop}
          </button>
        </div>
      </div>
    </>
  );
}

function NudgeCard({ view, verdict, recent }: { view: TimerView; verdict: VerdictEvent | null; recent: VerdictEvent[] }) {
  // The live sample when it is the distraction; the fetched list can lag behind a repeat reminder.
  const last = verdict?.result === "distracted" ? verdict : (recent.filter((r) => r.result === "distracted").pop() ?? verdict);
  const where = last ? targetOf(last) : p.nudge.otherWindow;
  const allow = last?.allow_target ?? null;
  const page = isPage(allow ?? "");
  const rows = recent.slice(-view.threshold);
  const close = () => api.closeWindow();
  return (
    <>
      <div className="nudge-head">
        <div className="logo" aria-hidden="true">
          <TimerGlyph />
        </div>
        <span className="from">PomoSupervisor</span>
        <span className="muted num" style={{ fontSize: 12 }}>
          {p.left(mmss(view.remaining))}
        </span>
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
        <div className="nudge-title">{p.nudge.title}</div>
        <div className="nudge-body">
          {p.nudge.body(where, view.task ? quoted(view.task) : null)}
        </div>
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        <button className="btn primary" style={{ height: 34, borderRadius: 9 }} onClick={() => api.dismissNudge().then(close)}>
          {p.nudge.backToTask}
        </button>
        <div className="btn-row">
          <button className="btn grow" disabled={!allow} onClick={() => allow && api.allowForRound(allow).then(close)}>
            {page ? p.nudge.pagePart : p.nudge.part}
          </button>
          <button className="btn grow" onClick={() => api.rest(view.short_break_min).then(close)}>
            {p.nudge.restFor(view.short_break_min)}
          </button>
        </div>
        {allow ? (
          <div className="hint">
            {page
              ? p.nudge.pageHint(hostOf(allow) ?? allow)
              : p.nudge.allowHint(allow)}
          </div>
        ) : (
          <div className="hint">
            {p.nudge.noUrlHint}{" "}
            <button className="link" style={{ fontSize: 11 }} onClick={() => api.openPrivacy("automation")}>
              {p.nudge.allowUrls}
            </button>
          </div>
        )}
      </div>
      {rows.length > 0 && (
        <details className="why">
          <summary>{p.nudge.why}</summary>
          <div className="inner">
            <div className="samples">
              {rows.map((r) => (
                <div className="row" key={r.at + r.app}>
                  <span className="t">{hms(r.at)}</span>
                  <span className="src ellipsis" title={r.reason}>
                    {r.reason}
                  </span>
                  <StageBadge v={r} />
                  <span className="res">{r.result === "distracted" ? p.nudge.distracted : r.result === "focused" ? p.nudge.focused : p.nudge.unknown}</span>
                </div>
              ))}
            </div>
            <div className="hint">{p.nudge.thresholdHint(view.threshold)}</div>
          </div>
        </details>
      )}
    </>
  );
}

export default function Popover() {
  const [view, setView] = useState<TimerView | null>(null);
  const [verdict, setVerdict] = useState<VerdictEvent | null>(null);
  const [recent, setRecent] = useState<VerdictEvent[]>([]);
  const [completed, setCompleted] = useState(0);
  const [review, setReview] = useState<Review | null>(null);
  const [missing, setMissing] = useState<Missing>({ screen: false, ax: false });
  const [ai, setAi] = useState<Ai>("on");
  // "换个任务": the start card, empty, over whatever idle card was showing. The break owed stays owed.
  const [editing, setEditing] = useState(false);
  const card = useRef<HTMLDivElement>(null);

  const refresh = () => {
    api.todayCompleted().then(setCompleted);
    Promise.all([api.checkPermissions(), api.getSettings(), api.modelStatus()]).then(([p, s, m]) => {
      setMissing({ screen: !p.screen_recording, ax: s.ax_enabled && !p.accessibility });
      const wanted = (s.text_enabled && !!s.text_model) || (s.vision_enabled && !!s.vision_model);
      const ready = (s.text_enabled && m.text_ready) || (s.vision_enabled && m.vision_ready);
      setAi(ready ? "on" : wanted ? "broken" : "off");
    });
  };

  useEffect(() => {
    document.body.classList.add("popover");
    api.getState().then(setView);
    api.lastVerdict().then(setVerdict);
    refresh();
    const subs = [
      onTick(setView),
      // With the popover open we are the window in front, which says nothing about the work.
      onVerdict((v) => {
        if (v.stage !== "self") setVerdict(v);
        if (v.alerted) api.recentVerdicts().then(setRecent);
      }),
      onPopoverMode(refresh),
    ];
    return () => subs.forEach((p) => p.then((un) => un()));
  }, []);

  // Today's count changes when a round ends; the verdict resets when a new one starts.
  const phase = view?.phase;
  useEffect(() => {
    refresh();
    if (phase !== "idle") setEditing(false);
    if (phase === "focus") return;
    setVerdict(null);
  }, [phase]);

  const nudging = view?.nudging ?? false;
  useEffect(() => {
    if (nudging) api.recentVerdicts().then(setRecent);
  }, [nudging]);

  const owed = view?.phase === "idle" && view.next_break_min != null;
  useEffect(() => {
    if (owed) api.getReview().then(setReview, () => setReview(null));
  }, [owed]);

  // The window is exactly the card: tell Rust whenever the card changes size.
  useLayoutEffect(() => {
    const el = card.current;
    if (!el) return;
    const fit = () => api.popoverFit(el.offsetWidth, el.offsetHeight);
    fit();
    const ro = new ResizeObserver(fit);
    ro.observe(el);
    return () => ro.disconnect();
  }, [view === null]);

  if (!view) return null;
  const nudge = view.nudging && view.phase === "focus" && view.paused === null;

  let body: React.ReactNode;
  // The card may have come up without the keyboard, and now there is something to type.
  const changeTask = () => {
    setEditing(true);
    api.focusPopover();
  };
  const start = (fresh: boolean) => (
    <StartCard key={fresh ? "fresh" : "start"} view={view} completed={completed} missing={missing} ai={ai} fresh={fresh} onBack={() => setEditing(false)} />
  );
  if (nudge) body = <NudgeCard view={view} verdict={verdict} recent={recent} />;
  else if (view.phase === "idle" && editing) body = start(true);
  else if (owed) body = <DoneCard view={view} completed={completed} review={review} onChangeTask={changeTask} />;
  else if (view.phase === "idle" && view.break_over) body = <BreakOverCard view={view} completed={completed} onChangeTask={changeTask} />;
  else if (view.phase === "idle") body = start(false);
  else if (view.phase !== "focus") body = <BreakCard view={view} completed={completed} />;
  else if (view.returned != null && !view.paused) body = <ReturnedCard view={view} />;
  else if (view.paused) body = <PausedCard view={view} completed={completed} />;
  else body = <FocusCard view={view} verdict={verdict} completed={completed} />;

  return (
    <div ref={card} className={nudge ? "card wide" : view.phase === "idle" ? "card roomy" : "card"}>
      {body}
    </div>
  );
}
