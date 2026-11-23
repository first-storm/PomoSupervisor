import { useEffect, useState } from "react";
import { api, errText, STAGE_LABEL, type Stats } from "../lib/api";

const time = (t: number) => new Date(t * 1000).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" });

export default function StatsPage() {
  const [s, setS] = useState<Stats | null>(null);
  const [err, setErr] = useState("");

  useEffect(() => {
    const load = () => api.getStats().then(setS).catch((e) => setErr(errText(e)));
    load();
    const t = setInterval(load, 10000);
    return () => clearInterval(t);
  }, []);

  if (err) return <div className="error">{err}</div>;
  if (!s) return null;

  const maxMin = Math.max(25, ...s.week.map((d) => d.focus_minutes));
  const decided = s.stages.rules + s.stages.text + s.stages.vision;
  const samples = s.focused_samples + s.distracted_samples;
  const focusRate = samples ? Math.round((s.focused_samples / samples) * 100) : null;

  return (
    <div className="stats">
      <div className="tiles">
        <div className="tile">
          <b>{s.completed}</b>
          <span>完成番茄</span>
        </div>
        <div className="tile">
          <b>{s.focus_minutes}</b>
          <span>专注分钟</span>
        </div>
        <div className="tile">
          <b>{s.alerts}</b>
          <span>分心提醒</span>
        </div>
      </div>

      <section className="card">
        <h3>近 7 天专注</h3>
        <div className="bars">
          {s.week.map((d, i) => (
            <div className="bar-col" key={d.day} title={`${d.focus_minutes} 分钟 · ${d.completed} 个番茄`}>
              <span className="bar-val">{d.focus_minutes || ""}</span>
              <div className={i === s.week.length - 1 ? "bar today" : "bar"} style={{ height: `${(d.focus_minutes / maxMin) * 100}%` }} />
              <span className="bar-day">{i === s.week.length - 1 ? "今天" : d.day}</span>
            </div>
          ))}
        </div>
      </section>

      <section className="card">
        <h3>
          今日判定 <span className="muted">{focusRate == null ? "" : `专注占比 ${focusRate}%`}</span>
        </h3>
        {decided ? (
          <>
            <div className="stack">
              {(["rules", "text", "vision"] as const).map((k) =>
                s.stages[k] ? <div key={k} className={`seg ${k}`} style={{ flex: s.stages[k] }} /> : null,
              )}
            </div>
            <div className="legend">
              {(["rules", "text", "vision"] as const).map((k) => (
                <span key={k}>
                  <i className={`sw ${k}`} />
                  {STAGE_LABEL[k]} {s.stages[k]}
                </span>
              ))}
            </div>
          </>
        ) : (
          <p className="muted">还没有判定记录</p>
        )}
        <p className="muted small">
          模型实际调用 {s.llm_calls} 次 · 缓存/去重命中 {s.llm_cached} 次
        </p>
      </section>

      <section className="card">
        <h3>今日分心提醒</h3>
        {s.recent_alerts.length === 0 ? (
          <p className="muted">今天没有分心提醒 🎉</p>
        ) : (
          <ul className="timeline">
            {s.recent_alerts.map((a) => (
              <li key={a.at + a.app}>
                <span className="t">{time(a.at)}</span>
                <div>
                  <div>
                    <b>{a.app}</b> <span className={`tag ${a.stage}`}>{STAGE_LABEL[a.stage] ?? a.stage}</span>
                  </div>
                  <div className="muted small ellipsis" title={a.url ?? a.title}>
                    {a.reason}
                    {a.task && ` · 任务：${a.task}`}
                  </div>
                </div>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
