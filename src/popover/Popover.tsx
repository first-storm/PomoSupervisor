import { useEffect, useLayoutEffect, useRef, useState } from "react";
import {
  api,
  hms,
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

const dur = (secs: number) => (secs < 60 ? `${Math.max(0, Math.round(secs))} 秒` : `${Math.round(secs / 60)} 分钟`);
const PRESETS = [25, 45, 60];
const quoted = (task: string) => (task.length > 28 ? `「${task.slice(0, 27)}…」` : `「${task}」`);

function More() {
  return (
    <button className="icon-btn" aria-label="更多" onClick={() => api.popoverMenu()}>
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
        <b>需要“辅助功能”权限</b>
        <p>用来读取窗口里的文字，帮模型判断标题看不出来的情况，也能补上 Firefox 的网址。不想用可以在设置 → 隐私里关掉。</p>
        <div className="btn-row">
          <button
            className="btn small"
            onClick={async () => {
              await api.requestAccessibility();
              await api.openPrivacy("accessibility");
            }}
          >
            去授权
          </button>
        </div>
      </div>
    );
  }
  return (
    <div className="perm">
      <b>需要“屏幕录制”权限</b>
      <p>没有它读不到其他 App 的窗口标题，也无法截图，只能按 App 名判断。授权后需重启应用。</p>
      <div className="btn-row">
        <button
          className="btn small"
          onClick={async () => {
            await api.requestScreenRecording();
            await api.openPrivacy("screen");
          }}
        >
          去授权
        </button>
        {missing.ax && (
          <button className="btn small" onClick={() => api.openPrivacy("accessibility")}>
            辅助功能
          </button>
        )}
        <button className="btn small" onClick={() => api.openPrivacy("automation")}>
          浏览器网址
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
        <span className="txt">AI 监督已就绪：名单里没有的窗口，会对照上面的任务来判断。</span>
      </div>
    ) : (
      <div className="sup">
        <span className="dot" aria-hidden="true" />
        <span className="txt">还没写任务。AI 不知道你要做什么，判断会很粗。</span>
      </div>
    );
  }
  return (
    <div className={ai === "broken" ? "sup bad" : "sup"}>
      <span className="dot" aria-hidden="true" />
      <span className="txt">
        {ai === "broken" ? "模型没有就绪，这一轮只按黑白名单判断。" : "还没配置模型，只按黑白名单判断，名单里没有的窗口不会管。"}
      </span>
      <button className="link" onClick={() => openSettings("models")}>
        {ai === "broken" ? "去检查" : "配置模型"}
      </button>
    </div>
  );
}

function StartCard({ view, completed, missing, ai }: { view: TimerView; completed: number; missing: Missing; ai: Ai }) {
  const [task, setTask] = useState(view.task);
  const [minutes, setMinutes] = useState(view.last_focus_min);
  const [allow, setAllow] = useState<string[]>(view.round_allow);
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
        <div className="title">这一轮做什么？</div>
        <More />
      </div>
      {(missing.screen || missing.ax) && <PermissionBanner missing={missing} />}
      <div className="field-col">
        <label className="label" htmlFor="task">
          任务
        </label>
        <textarea
          id="task"
          className="textarea"
          rows={3}
          autoFocus
          maxLength={200}
          placeholder="比如：给采样模块写单元测试，覆盖空闲检测和截图去重"
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
        <div className="label">时长</div>
        <div className="segmented" role="group" aria-label="时长">
          {presets.map((m) => (
            <button key={m} aria-pressed={minutes === m} onClick={() => setMinutes(m)}>
              {m} 分钟
            </button>
          ))}
        </div>
      </div>
      <div className="field-col">
        <div className="label">额外允许</div>
        <div className="chips">
          {allow.map((a) => (
            <button key={a} className="chip" aria-label={`移除 ${a}`} title="点击移除" onClick={() => setAllow(allow.filter((x) => x !== a))}>
              {a}
              <span className="x" aria-hidden="true">
                ×
              </span>
            </button>
          ))}
          {adding === null ? (
            <button className="chip add" onClick={() => setAdding("")}>
              <PlusIcon />
              <span>添加</span>
            </button>
          ) : (
            <input
              className="input chip-input"
              aria-label="App 名或域名"
              placeholder="App 名或域名"
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
        <div className="hint">这些 App 和网站直接算专注。下一轮会沿用，不需要了点一下移除。</div>
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
        <button className="big-btn" onClick={start}>
          开始专注
        </button>
        <div className="center-note">{completed ? `今天已完成 ${completed} 个番茄 · 回车直接开始` : "回车直接开始"}</div>
      </div>
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
          <div className="name muted">正在看第一眼…</div>
        </div>
      </div>
    );
  }
  const where = targetOf(v);
  const off = v.result === "distracted";
  const tone = off ? "state hot" : v.result === "focused" ? "state ok" : "state";
  const name = off ? "看起来跑偏了" : v.result === "focused" ? "在任务上" : "拿不准，不计入";
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
          <button className="btn small" onClick={() => api.allowForRound(v.allow_target!).then(() => api.closeWindow())}>
            这算任务
          </button>
        )}
      </div>
      <details className="why">
        <summary>判断依据</summary>
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

/** Ending a round early throws work away, so it takes a second click; pausing doesn't. */
function RoundActions({ view, children }: { view: TimerView; children: React.ReactNode }) {
  const [asking, setAsking] = useState(false);
  if (asking) {
    return (
      <div className="confirm">
        <div className="body-note">
          提前结束这一轮？
          <br />
          已经过去的 {dur(view.total - view.remaining)}会留在回顾里，但不算完成一个番茄。
        </div>
        <div className="btn-row">
          <button className="btn grow danger" onClick={() => api.stop()}>
            提前结束
          </button>
          <button className="btn primary grow" onClick={() => setAsking(false)}>
            接着专注
          </button>
        </div>
      </div>
    );
  }
  return (
    <div className="btn-row">
      <button className="btn grow" onClick={() => setAsking(true)}>
        提前结束
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
          专注中
        </span>
        <div className="side">
          <span>今天第 {completed + 1} 个</span>
          <More />
        </div>
      </div>
      <Dial progress={progress} distracted={view.distracted_ticks} tone="focus">
        <div className="clock">{mmss(view.remaining)}</div>
        <div className="sub">剩余 · 共 {Math.round(view.total / 60)} 分钟</div>
      </Dial>
      <div className="task-block">
        <div className="cap">当前任务</div>
        <div className="task">{view.task || "（没写任务）"}</div>
      </div>
      <Judgement v={verdict} />
      <RoundActions view={view}>
        <button className="btn grow" onClick={() => api.pause()}>
          暂停
        </button>
      </RoundActions>
    </>
  );
}

function PausedCard({ view, completed }: { view: TimerView; completed: number }) {
  const away = view.paused === "away";
  const timed = view.rest_left != null;
  const progress = view.total ? 1 - view.remaining / view.total : 0;
  const idle = view.idle_secs;
  return (
    <>
      <div className="card-head">
        <span className="pill">
          <PauseIcon />
          {away ? "已暂停 · 你离开了" : "已暂停"}
        </span>
        <div className="side">
          <span>今天第 {completed + 1} 个</span>
          <More />
        </div>
      </div>
      <Dial progress={progress} distracted={view.distracted_ticks} tone="dim">
        <div className="clock">{mmss(view.remaining)}</div>
        <div className="sub">{timed ? `${mmss(view.rest_left!)} 后自动继续` : "停在这里"}</div>
      </Dial>
      <div className="body-note">
        {away ? (
          <>
            键鼠 {dur(idle)}没有操作，计时自动停了。
            <br />
            离开的时间不算分心。
          </>
        ) : timed ? (
          <>
            先歇一下，到点自动接着这一轮。
            <br />
            暂停期间不看你在做什么。
          </>
        ) : (
          <>
            计时停着，这一轮还在。
            <br />
            暂停期间不看你在做什么。
          </>
        )}
      </div>
      <div className="rule" />
      <div className="kv">
        <span className="muted">{away ? "已离开" : "已暂停"}</span>
        <span className="num" style={{ fontWeight: 500 }}>
          {dur(view.paused_for + (away ? idle : 0))}
        </span>
      </div>
      {away && (
        <label className="check">
          <input type="checkbox" checked={view.auto_resume} onChange={(e) => api.setAutoResume(e.target.checked)} />
          <span>回来后自动继续</span>
        </label>
      )}
      <RoundActions view={view}>
        <button className="btn primary grow" onClick={() => api.resume()}>
          继续专注
        </button>
      </RoundActions>
    </>
  );
}

function BreakCard({ view, completed }: { view: TimerView; completed: number }) {
  const progress = view.total ? 1 - view.remaining / view.total : 0;
  const long = view.phase === "long_break";
  return (
    <>
      <div className="card-head">
        <span className="pill">{long ? "长休息" : "休息中"}</span>
        <div className="side">
          <span>今天已完成 {completed} 个</span>
          <More />
        </div>
      </div>
      <Dial progress={progress} tone="rest">
        <div className="clock">{mmss(view.remaining)}</div>
        <div className="sub">休息 · 共 {Math.round(view.total / 60)} 分钟</div>
      </Dial>
      <div className="body-note">
        站起来走走，看看远处。
        <br />
        到点会叫你，休息期间不看你在做什么。
      </div>
      <div className="btn-row">
        <button className="btn grow" onClick={() => api.openWindow("review")}>
          专注回顾
        </button>
        <button className="btn grow" onClick={() => api.startFocus(view.task, view.last_focus_min, view.round_allow).then(() => api.closeWindow())}>
          不休息了，开始
        </button>
      </div>
    </>
  );
}

/** Back to the start card to type a different task; the card may have come up without the keyboard. */
const changeTask = () => api.stop().then(() => api.focusPopover());

/** A round just finished: take the break it earned, go again, or switch task. Never loses the break. */
function DoneCard({ view, completed, review }: { view: TimerView; completed: number; review: Review | null }) {
  const again = () => api.startFocus(view.task, view.last_focus_min, view.round_allow).then(() => api.closeWindow());
  const rest = () => api.startBreak().then(() => api.closeWindow());
  const m = (s: number) => Math.round(s / 60);
  return (
    <>
      <div className="card-head">
        <span className="pill hot">番茄完成</span>
        <div className="side">
          <span>今天第 {Math.max(1, completed)} 个</span>
          <More />
        </div>
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
        <div className="after-task">{view.task || "这一轮结束了"}</div>
        {review && (
          <div className="after-stats">
            专注 {m(review.focus_secs)} 分钟
            {review.distract_secs >= 30 ? ` · 分心 ${m(review.distract_secs)} 分钟` : " · 没怎么分心"}
            {review.alerts > 0 ? ` · 提醒 ${review.alerts} 次` : ""}
          </div>
        )}
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        <button className="big-btn" onClick={rest}>
          休息 {view.next_break_min} 分钟
        </button>
        <div className="btn-row">
          <button className="btn grow" onClick={again}>
            不休息，再来一轮
          </button>
          <button className="btn grow" onClick={changeTask}>
            换个任务
          </button>
        </div>
      </div>
      <div className="links">
        <button className="link" onClick={() => api.openWindow("review")}>
          看这一轮的回顾
        </button>
      </div>
    </>
  );
}

/** A break just ran out: one click carries on with the same task. */
function BreakOverCard({ view, completed }: { view: TimerView; completed: number }) {
  const again = () => api.startFocus(view.task, view.last_focus_min, view.round_allow).then(() => api.closeWindow());
  return (
    <>
      <div className="card-head">
        <span className="pill">休息结束</span>
        <div className="side">
          <span>今天已完成 {completed} 个</span>
          <More />
        </div>
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
        <div className="after-task">{view.task ? `接着做${quoted(view.task)}？` : "开始下一轮？"}</div>
        <div className="after-stats">还是 {view.last_focus_min} 分钟，允许列表照旧。</div>
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        <button className="big-btn" onClick={again}>
          开始专注
        </button>
        <div className="btn-row">
          <button className="btn grow" onClick={changeTask}>
            换个任务
          </button>
          <button className="btn grow" onClick={() => api.stop().then(() => api.closeWindow())}>
            先不做了
          </button>
        </div>
      </div>
    </>
  );
}

function NudgeCard({ view, verdict, recent }: { view: TimerView; verdict: VerdictEvent | null; recent: VerdictEvent[] }) {
  // The live sample when it is the distraction; the fetched list can lag behind a repeat reminder.
  const last = verdict?.result === "distracted" ? verdict : (recent.filter((r) => r.result === "distracted").pop() ?? verdict);
  const where = last ? targetOf(last) : "别的窗口";
  const allow = last?.allow_target ?? null;
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
          还剩 {mmss(view.remaining)}
        </span>
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
        <div className="nudge-title">走神了吗？</div>
        <div className="nudge-body">
          你在 <strong>{where}</strong> 待了一会儿，看起来和{view.task ? quoted(view.task) : "当前任务"}没有关系。
        </div>
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        <button className="btn primary" style={{ height: 34, borderRadius: 9 }} onClick={() => api.dismissNudge().then(close)}>
          回到任务
        </button>
        <div className="btn-row">
          <button className="btn grow" disabled={!allow} onClick={() => allow && api.allowForRound(allow).then(close)}>
            这算任务的一部分
          </button>
          <button className="btn grow" onClick={() => api.rest(view.short_break_min).then(close)}>
            暂停 {view.short_break_min} 分钟
          </button>
        </div>
        {allow ? (
          <div className="hint">
            切回去干活，这条提醒会自己消失。选「这算任务的一部分」会把 {allow} 加进额外允许，之后不再提醒。
          </div>
        ) : (
          <div className="hint">
            切回去干活，这条提醒会自己消失。读不到这个网页的网址，没法只放行它。{" "}
            <button className="link" style={{ fontSize: 11 }} onClick={() => api.openPrivacy("automation")}>
              允许读取浏览器网址
            </button>
          </div>
        )}
      </div>
      {rows.length > 0 && (
        <details className="why">
          <summary>为什么提醒我</summary>
          <div className="inner">
            <div className="samples">
              {rows.map((r) => (
                <div className="row" key={r.at + r.app}>
                  <span className="t">{hms(r.at)}</span>
                  <span className="src ellipsis" title={r.reason}>
                    {r.reason}
                  </span>
                  <StageBadge v={r} />
                  <span className="res">{r.result === "distracted" ? "分心" : r.result === "focused" ? "专注" : "—"}</span>
                </div>
              ))}
            </div>
            <div className="hint">连续 {view.threshold} 次都判为分心才会提醒。还不回来的话，隔 1 分钟、2 分钟、5 分钟再提醒一次。</div>
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
  if (nudge) body = <NudgeCard view={view} verdict={verdict} recent={recent} />;
  else if (owed) body = <DoneCard view={view} completed={completed} review={review} />;
  else if (view.phase === "idle" && view.break_over) body = <BreakOverCard view={view} completed={completed} />;
  else if (view.phase === "idle") body = <StartCard key="start" view={view} completed={completed} missing={missing} ai={ai} />;
  else if (view.phase !== "focus") body = <BreakCard view={view} completed={completed} />;
  else if (view.paused) body = <PausedCard view={view} completed={completed} />;
  else body = <FocusCard view={view} verdict={verdict} completed={completed} />;

  return (
    <div ref={card} className={nudge ? "card wide" : view.phase === "idle" ? "card roomy" : "card"}>
      {body}
    </div>
  );
}
