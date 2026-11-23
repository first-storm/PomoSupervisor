import { useState } from "react";
import { api, mmss, type TimerView, type VerdictEvent } from "../lib/api";
import Ring from "../components/Ring";
import SupervisorStrip from "../components/SupervisorStrip";
import PermissionGuide from "../components/PermissionGuide";

const PHASE_LABEL = { idle: "准备", focus: "专注中", short_break: "短休息", long_break: "长休息" } as const;

interface Props {
  view: TimerView;
  verdict: VerdictEvent | null;
  onView: (v: TimerView) => void;
}

export default function TimerPage({ view, verdict, onView }: Props) {
  const [task, setTask] = useState(view.task);
  const idle = view.phase === "idle";
  const focus = view.phase === "focus";
  const progress = view.total ? 1 - view.remaining / view.total : 0;
  const pill = view.paused === "away" ? "已离开 · 暂停计时" : view.paused === "user" ? "已暂停" : PHASE_LABEL[view.phase];
  // During a break the round just finished is shown; otherwise the one in progress / next up.
  const roundNo = view.phase === "short_break" || view.phase === "long_break" ? view.cycle || view.long_break_every : view.cycle + 1;

  const run = (p: Promise<TimerView>) => p.then(onView);

  return (
    <div className="timer">
      <PermissionGuide />

      <div className="task">
        {idle ? (
          <input
            className="task-input"
            autoFocus
            placeholder="这一轮要做什么？（监督模型会参考它）"
            value={task}
            maxLength={120}
            onChange={(e) => setTask(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && run(api.startFocus(task))}
          />
        ) : (
          <div className="task-text" title={view.task}>
            {view.phase === "focus" ? view.task || "（未填写任务）" : "放松一下，离开屏幕走一走"}
          </div>
        )}
      </div>

      <Ring progress={progress}>
        <span className={`pill ${view.paused ?? view.phase}`}>{pill}</span>
        <span className="clock">{mmss(idle ? view.focus_secs : view.remaining)}</span>
        <span className="round">
          第 {roundNo} / {view.long_break_every} 个番茄
        </span>
      </Ring>

      <div className="controls">
        {idle && (
          <>
            <button className="btn primary" onClick={() => run(api.startFocus(task))}>
              开始专注
            </button>
            <button className="btn ghost" onClick={() => run(api.startBreak())}>
              休息
            </button>
          </>
        )}
        {!idle && (
          <>
            {view.paused === "user" ? (
              <button className="btn primary" onClick={() => run(api.resume())}>
                继续
              </button>
            ) : (
              <button className="btn primary" onClick={() => run(api.pause())}>
                暂停
              </button>
            )}
            <button className="btn ghost" onClick={() => run(api.stop())}>
              {focus ? "放弃本轮" : "跳过休息"}
            </button>
          </>
        )}
      </div>

      {focus && <SupervisorStrip v={verdict} streak={view.streak} threshold={view.threshold} />}
    </div>
  );
}
