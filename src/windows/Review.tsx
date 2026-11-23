import { useEffect, useState } from "react";
import { api, errText, hm, hostOf, onTick, onWindowShown, type Distraction, type Review, type TimerView } from "../lib/api";

const mins = (secs: number) => Math.round(secs / 60);
const short = (secs: number) => (secs < 60 ? `${secs} 秒` : `${mins(secs)} 分钟`);

function dayWord(t: number) {
  const d = new Date(t * 1000);
  const today = new Date();
  const y = new Date(today.getTime() - 86400000);
  if (d.toDateString() === today.toDateString()) return "今天";
  if (d.toDateString() === y.toDateString()) return "昨天";
  return `${d.getMonth() + 1} 月 ${d.getDate()} 日`;
}

function label(d: Distraction) {
  const host = hostOf(d.url);
  if (host) return host;
  return d.title ? `${d.app} · ${d.title}` : d.app;
}

function SourceBadge({ d }: { d: Distraction }) {
  if (d.stage === "rules") return <span className="badge">规则 · 黑名单</span>;
  if (d.stage === "text") return <span className="badge model">文本模型</span>;
  if (d.stage === "vision") return <span className="badge model">视觉模型</span>;
  return <span className="badge">{d.stage}</span>;
}

function DistractionRow({ d }: { d: Distraction }) {
  const [state, setState] = useState<{ ok: boolean; text: string } | null>(null);
  return (
    <div className="drow">
      <div className="when">
        <b>{hm(d.at)}</b>
        <span className="cap">{short(d.secs)}</span>
      </div>
      <div className="what">
        <div className="name ellipsis" title={d.url ?? d.title}>
          {label(d)}
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 6, minWidth: 0 }}>
          <SourceBadge d={d} />
          {state && (
            <span className={state.ok ? "status ok" : "status no"} style={{ fontSize: 11 }}>
              {state.text}
            </span>
          )}
        </div>
      </div>
      <button
        className="btn small"
        disabled={state?.ok}
        onClick={() =>
          api
            .markAsWork(d.app, d.url)
            .then((t) => setState({ ok: true, text: `${t} 已加入白名单` }))
            .catch((e) => setState({ ok: false, text: errText(e) }))
        }
      >
        {state?.ok ? "已加入" : "其实在工作"}
      </button>
    </div>
  );
}

function Timeline({ r }: { r: Review }) {
  const span = Math.max(1, r.ended - r.started);
  const names = { focus: "专注", distract: "分心", away: "离开" } as const;
  const aria = `时间线：${hm(r.started)} 起` + r.segments.map((s) => `${names[s.kind]} ${short(s.to - s.from)}`).join("，");
  const ticks = [1 / 3, 2 / 3].map((f) => ({ f, t: r.started + span * f }));
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
      <div className="section-label">时间线</div>
      <div className="timeline" role="img" aria-label={aria}>
        {r.segments.map((s) => (
          <div key={s.from} className={s.kind} style={{ flex: `${s.to - s.from} 1 0` }} />
        ))}
      </div>
      <div className="axis" aria-hidden="true">
        <span style={{ left: 0 }}>{hm(r.started)}</span>
        {span >= 6 * 60 &&
          ticks.map(({ f, t }) => (
            <span key={f} style={{ left: `${f * 100}%`, transform: "translateX(-50%)" }}>
              {hm(t)}
            </span>
          ))}
        <span style={{ right: 0 }}>{hm(r.ended)}</span>
      </div>
    </div>
  );
}

function Sources({ r }: { r: Review }) {
  const { rules, text, vision } = r.stages;
  const total = rules + text + vision;
  const modelShare = total ? Math.round(((text + vision) / total) * 100) : 0;
  const parts = [
    { k: "rules", n: rules },
    { k: "text", n: text },
    { k: "vision", n: vision },
  ].filter((p) => p.n > 0);
  return (
    <div style={{ flex: "1 1 0" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline" }}>
        <div className="section-label">判断来源</div>
        <div className="num muted" style={{ fontSize: 12 }}>
          共 {total} 次采样
        </div>
      </div>
      {total > 0 ? (
        <>
          <div className="srcbar" role="img" aria-label={`${total} 次采样中，规则匹配 ${rules} 次，文本模型 ${text} 次，视觉模型 ${vision} 次`}>
            {parts.map((p) => (
              <div key={p.k} className={`seg-${p.k}`} style={{ flex: `${p.n} 1 0` }} />
            ))}
          </div>
          <div className="srclist">
            <div>
              <span className="sw rules" />
              <span style={{ flex: 1 }}>
                规则匹配 <span className="muted">· 本地</span>
              </span>
              <b>{rules}</b>
            </div>
            <div>
              <span className="sw text" />
              <span style={{ flex: 1 }}>文本模型</span>
              <b>{text}</b>
            </div>
            <div>
              <span className="sw vision" />
              <span style={{ flex: 1 }}>
                视觉模型 <span className="muted">· 截图</span>
              </span>
              <b>{vision}</b>
            </div>
          </div>
          <div className="hint">
            {modelShare === 0 ? "所有采样都在本地判完，没有用到模型。" : `只有 ${modelShare}% 的采样用到了模型，其余都在本地判完。`}
          </div>
        </>
      ) : (
        <div className="hint">这一轮没有得出结论的采样。</div>
      )}
    </div>
  );
}

export default function ReviewWindow() {
  const [r, setR] = useState<Review | null | undefined>(undefined);
  const [view, setView] = useState<TimerView | null>(null);
  const [err, setErr] = useState("");

  useEffect(() => {
    const load = () => api.getReview().then(setR).catch((e) => setErr(errText(e)));
    load();
    api.getState().then(setView);
    const subs = [onWindowShown(load), onTick(setView)];
    return () => subs.forEach((p) => p.then((un) => un()));
  }, []);

  const again = async () => {
    await api.startFocus(r?.task ?? "", r ? Math.round(r.planned_secs / 60) : 25, view?.round_allow ?? []);
    api.closeWindow();
  };
  const rest = async () => {
    await api.startBreak();
    api.closeWindow();
  };
  const owed = view?.phase === "idle" ? (view.next_break_min ?? view.short_break_min) : null;

  return (
    <div className="win">
      <div className="titlebar" data-tauri-drag-region>
        本轮回顾
      </div>
      <div className="win-body">
        {err && <div className="review msg bad">{err}</div>}
        {r === null && (
          <div className="blank" style={{ height: "100%" }}>
            <b>还没有结束的番茄</b>
            <span>完成或结束一轮后，这里会显示它的专注、分心和判断来源。</span>
          </div>
        )}
        {r && (
          <div className="review">
            <div className="review-head">
              <div>
                <h1>{r.task || "（未填写任务）"}</h1>
                <div className="meta">
                  {dayWord(r.started)} {hm(r.started)} – {hm(r.ended)} · 第 {r.ordinal} 个番茄 · {mins(r.planned_secs)} 分钟
                </div>
              </div>
              <span className={r.completed ? "pill hot" : "pill"}>{r.completed ? "已完成" : "提前结束"}</span>
            </div>

            <div className="tiles">
              <div className="tile">
                <div className="k">
                  <span className="sw focus" aria-hidden="true" />
                  专注
                </div>
                <div className="v">
                  <b>{mins(r.focus_secs)}</b>
                  <span>分钟</span>
                </div>
              </div>
              <div className="tile">
                <div className="k">
                  <span className="sw distract" aria-hidden="true" />
                  分心
                </div>
                <div className="v">
                  <b>{mins(r.distract_secs)}</b>
                  <span>分钟 · 提醒 {r.alerts} 次</span>
                </div>
              </div>
              <div className="tile">
                <div className="k">
                  <span className="sw away" aria-hidden="true" />
                  离开
                </div>
                <div className="v">
                  <b>{mins(r.away_secs)}</b>
                  <span>分钟 · 不计入番茄</span>
                </div>
              </div>
            </div>

            <Timeline r={r} />

            <div className="review-cols">
              <div style={{ flex: "1.35 1 0" }}>
                <div className="section-label">分心片段</div>
                {r.distractions.length ? (
                  <div className="dlist">
                    {r.distractions.map((d) => (
                      <DistractionRow key={d.at} d={d} />
                    ))}
                  </div>
                ) : (
                  <div className="empty">这一轮没有分心。</div>
                )}
                <div className="hint">判错了就点「其实在工作」，对应的 App 或域名会进白名单。</div>
              </div>
              <Sources r={r} />
            </div>

            <div className="review-foot">
              {owed != null && (
                <button className="btn" onClick={rest}>
                  休息 {owed} 分钟
                </button>
              )}
              {view && view.phase !== "focus" && (
                <button className="btn primary" onClick={again}>
                  再来一轮
                </button>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
