import { useEffect, useLayoutEffect, useRef, useState } from "react";
import {
  api,
  hms,
  mmss,
  onPopoverMode,
  onTick,
  onVerdict,
  targetOf,
  type PopoverMode,
  type TimerView,
  type VerdictEvent,
} from "../lib/api";
import { Dial, MoreIcon, PauseIcon, PlusIcon, StageBadge, StreakBars, TimerGlyph } from "../components/ui";

const dur = (secs: number) => (secs < 60 ? `${Math.max(0, Math.round(secs))} 秒` : `${Math.round(secs / 60)} 分钟`);
const PRESETS = [25, 45, 60];

function More() {
  return (
    <button className="icon-btn" aria-label="更多" onClick={() => api.popoverMenu()}>
      <MoreIcon />
    </button>
  );
}

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

function StartCard({ view, completed, missing }: { view: TimerView; completed: number; missing: Missing }) {
  const [task, setTask] = useState(view.task);
  const [minutes, setMinutes] = useState(Math.round(view.focus_secs / 60));
  const [allow, setAllow] = useState<string[]>(view.round_allow);
  const [adding, setAdding] = useState<string | null>(null);
  const presets = PRESETS.includes(Math.round(view.focus_secs / 60)) ? PRESETS : [Math.round(view.focus_secs / 60), ...PRESETS];

  const addAllow = () => {
    const v = adding?.trim();
    if (v && !allow.some((a) => a.toLowerCase() === v.toLowerCase())) setAllow([...allow, v]);
    setAdding(null);
  };
  const start = () => api.startFocus(task.trim(), minutes, allow);

  return (
    <>
      <div className="card-head">
        <div className="title">这一轮做什么？</div>
        <More />
      </div>
      {(missing.screen || missing.ax) && <PermissionBanner missing={missing} />}
      <div className="field-col">
        <label className="label" htmlFor="task">
          任务描述
        </label>
        <textarea
          id="task"
          className="textarea"
          rows={3}
          autoFocus
          maxLength={200}
          placeholder="比如：给采样模块写单元测试，覆盖空闲检测和截图去重"
          value={task}
          onChange={(e) => setTask(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey && !e.nativeEvent.isComposing) {
              e.preventDefault();
              start();
            }
          }}
        />
        <div className="hint">写具体一点。规则拿不准的窗口，会拿这段话去问模型是否相关。</div>
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
        <div className="label">本轮额外允许</div>
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
        <div className="hint">这些 App 和网站本轮直接算专注，不经过模型。</div>
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
        <button className="big-btn" onClick={start}>
          开始专注
        </button>
        <div className="center-note">{completed ? `今天已完成 ${completed} 个番茄` : "今天还没有完成番茄"}</div>
      </div>
    </>
  );
}

function Watching({ v }: { v: VerdictEvent | null }) {
  if (!v) {
    return (
      <div className="watching">
        <div className="app-tile" aria-hidden="true" />
        <div className="what">
          <div className="name muted">等待第一次采样…</div>
        </div>
      </div>
    );
  }
  const sub = v.url ?? (v.title || "（无标题）");
  return (
    <div className="watching">
      <div className="app-tile" aria-hidden="true">
        {v.app.slice(0, 1).toUpperCase()}
      </div>
      <div className="what">
        <div className="name ellipsis">{v.app}</div>
        <div className="muted ellipsis" style={{ fontSize: 12 }} title={sub}>
          {sub}
        </div>
      </div>
      <StageBadge v={v} />
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
      <div className="streak-line">
        <span>连续分心</span>
        <StreakBars n={view.streak} of={view.threshold} />
        <span>
          {view.streak} / {view.threshold} · 满 {view.threshold} 次才提醒
        </span>
      </div>
      <div className="rule" />
      <div className="task-block">
        <div className="cap">当前任务</div>
        <div className="task">{view.task || "（未填写任务）"}</div>
      </div>
      <div className="field-col">
        <div className="cap">正在看</div>
        <Watching v={verdict} />
      </div>
      <div className="btn-row">
        <button className="btn grow" onClick={() => api.pause()}>
          暂停
        </button>
        <button className="btn grow" onClick={() => api.stop()}>
          结束本轮
        </button>
      </div>
    </>
  );
}

function PausedCard({ view, completed }: { view: TimerView; completed: number }) {
  const away = view.paused === "away";
  const resting = view.rest_left != null;
  const progress = view.total ? 1 - view.remaining / view.total : 0;
  const idle = view.idle_secs;
  const pill = away ? "已暂停 · 你离开了" : resting ? "休息一下" : "已暂停";
  return (
    <>
      <div className="card-head">
        <span className="pill">
          <PauseIcon />
          {pill}
        </span>
        <div className="side">
          <span>今天第 {completed + 1} 个</span>
          <More />
        </div>
      </div>
      <Dial progress={progress} distracted={view.distracted_ticks} tone="dim">
        <div className="clock">{mmss(view.remaining)}</div>
        <div className="sub">{resting ? `${mmss(view.rest_left!)} 后继续` : "停在这里"}</div>
      </Dial>
      <div className="body-note">
        {away ? (
          <>
            键鼠 {dur(idle)}没有操作，计时已自动暂停。
            <br />
            离开的时间不算分心。
          </>
        ) : resting ? (
          <>
            休息 {view.short_break_min} 分钟，到时自动继续专注。
            <br />
            休息期间不采样。
          </>
        ) : (
          <>
            计时已暂停。
            <br />
            暂停期间不采样，也不算分心。
          </>
        )}
      </div>
      <div className="rule" />
      <div className="kv">
        <span className="muted">{away ? "已离开" : resting ? "已休息" : "已暂停"}</span>
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
      <div className="btn-row">
        <button className="btn grow" onClick={() => api.stop()}>
          结束本轮
        </button>
        <button className="btn primary grow" onClick={() => api.resume()}>
          继续专注
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
        休息结束会提醒你。
      </div>
      <div className="btn-row">
        <button className="btn grow" onClick={() => api.openWindow("review")}>
          本轮回顾
        </button>
        <button className="btn grow" onClick={() => api.stop()}>
          跳过休息
        </button>
      </div>
    </>
  );
}

function NudgeCard({ view, verdict, recent, onDone }: { view: TimerView; verdict: VerdictEvent | null; recent: VerdictEvent[]; onDone: () => void }) {
  const last = recent.filter((r) => r.result === "distracted").pop() ?? verdict;
  const target = last ? targetOf(last) : "别的窗口";
  const rows = recent.slice(-view.threshold);
  return (
    <>
      <div className="nudge-head">
        <div className="logo" aria-hidden="true">
          <TimerGlyph />
        </div>
        <span className="from">PomoSupervisor</span>
        <span className="muted" style={{ fontSize: 12 }}>
          现在
        </span>
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
        <div className="nudge-title">走神了吗？</div>
        <div className="nudge-body">
          你在 <strong>{target}</strong> 待了一会儿，看起来和{view.task ? `「${view.task}」` : "当前任务"}没有关系。
        </div>
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        <button className="btn primary" style={{ height: 34, borderRadius: 9 }} onClick={() => api.closeWindow().then(onDone)}>
          回到任务
        </button>
        <div className="btn-row">
          <button className="btn grow" onClick={() => api.allowForRound(target).then(() => api.closeWindow()).then(onDone)}>
            这算任务的一部分
          </button>
          <button className="btn grow" onClick={() => api.rest(view.short_break_min).then(onDone)}>
            休息 {view.short_break_min} 分钟
          </button>
        </div>
        <div className="hint">选「这算任务的一部分」会把 {target} 加进本轮白名单，之后不再提醒。</div>
      </div>
      <div className="rule" />
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        <div className="why-head">
          <div className="label">为什么提醒我</div>
          <div className="count">
            <StreakBars n={view.threshold} of={view.threshold} />
            <span>
              连续 {view.threshold} / {view.threshold}
            </span>
          </div>
        </div>
        {rows.length > 0 && (
          <div className="samples">
            {rows.map((r) => (
              <div className="row" key={r.at + r.app}>
                <span className="t">{hms(r.at)}</span>
                <span className="src ellipsis" title={r.reason}>
                  {r.stage === "rules" ? r.reason : "规则未命中"}
                </span>
                <StageBadge v={r} />
                <span className="res">{r.result === "distracted" ? "分心" : r.result === "focused" ? "专注" : "—"}</span>
              </div>
            ))}
          </div>
        )}
        <div className="hint">连续 {view.threshold} 次采样都判为分心才会提醒，偶尔切一下窗口不会打扰你。</div>
      </div>
    </>
  );
}

export default function Popover() {
  const [view, setView] = useState<TimerView | null>(null);
  const [verdict, setVerdict] = useState<VerdictEvent | null>(null);
  const [mode, setMode] = useState<PopoverMode>("normal");
  const [recent, setRecent] = useState<VerdictEvent[]>([]);
  const [completed, setCompleted] = useState(0);
  const [missing, setMissing] = useState<Missing>({ screen: false, ax: false });
  const card = useRef<HTMLDivElement>(null);

  const refresh = () => {
    api.todayCompleted().then(setCompleted);
    Promise.all([api.checkPermissions(), api.getSettings()]).then(([p, s]) =>
      setMissing({ screen: !p.screen_recording, ax: s.ax_enabled && !p.accessibility }),
    );
  };

  useEffect(() => {
    document.body.classList.add("popover");
    api.getState().then(setView);
    api.lastVerdict().then(setVerdict);
    refresh();
    const subs = [
      onTick(setView),
      onVerdict(setVerdict),
      onPopoverMode((m) => {
        setMode(m);
        refresh();
        if (m === "nudge") api.recentVerdicts().then(setRecent);
      }),
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

  // The window is exactly the card: tell Rust whenever the card changes size.
  useLayoutEffect(() => {
    const el = card.current;
    if (!el) return;
    const fit = () => api.popoverFit(el.offsetWidth, el.offsetHeight);
    fit();
    const ro = new ResizeObserver(fit);
    ro.observe(el);
    return () => ro.disconnect();
  }, [view === null, mode]);

  if (!view) return null;
  const nudge = mode === "nudge" && view.phase === "focus" && view.paused === null;

  let body: React.ReactNode;
  if (nudge) body = <NudgeCard view={view} verdict={verdict} recent={recent} onDone={() => setMode("normal")} />;
  else if (view.phase === "idle") body = <StartCard key="start" view={view} completed={completed} missing={missing} />;
  else if (view.phase !== "focus") body = <BreakCard view={view} completed={completed} />;
  else if (view.paused) body = <PausedCard view={view} completed={completed} />;
  else body = <FocusCard view={view} verdict={verdict} completed={completed} />;

  return (
    <div ref={card} className={nudge ? "card wide" : view.phase === "idle" ? "card roomy" : "card"}>
      {body}
    </div>
  );
}
