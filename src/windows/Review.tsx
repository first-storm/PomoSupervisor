import { useEffect, useState } from "react";
import { intlTag, t } from "../i18n";
import { api, type Distraction, errText, hm, hostOf, onTick, onWindowShown, type Review, type TimerView } from "../lib/api";

const mins = (secs: number) => Math.round(secs / 60);
const short = t.common.dur;

function dayWord(at: number) {
  const d = new Date(at * 1000);
  const today = new Date();
  const y = new Date(today.getTime() - 86400000);
  if (d.toDateString() === today.toDateString()) return t.review.today;
  if (d.toDateString() === y.toDateString()) return t.review.yesterday;
  return d.toLocaleDateString(intlTag, { month: "long", day: "numeric" });
}

function label(d: Distraction) {
  const host = hostOf(d.url);
  if (host) return host;
  return d.title ? `${d.app} · ${d.title}` : d.app;
}

function SourceBadge({ d }: { d: Distraction }) {
  if (d.stage === "rules") return <span className="badge">{t.common.stage.block}</span>;
  if (d.stage === "text") return <span className="badge model">{t.common.stage.text}</span>;
  if (d.stage === "vision") return <span className="badge model">{t.common.stage.vision}</span>;
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
        type="button"
        className="btn small"
        disabled={state?.ok}
        onClick={() =>
          api
            .markAsWork(d.app, d.url)
            .then((name) => setState({ ok: true, text: t.review.addedToAllow(name) }))
            .catch((e) => setState({ ok: false, text: errText(e) }))
        }
      >
        {state?.ok ? t.review.added : t.review.actuallyWorking}
      </button>
    </div>
  );
}

function Timeline({ r }: { r: Review }) {
  const span = Math.max(1, r.ended - r.started);
  const names = { focus: t.review.focus, distract: t.review.distract, away: t.review.away };
  const aria = t.review.timelineAria(
    hm(r.started),
    r.segments.map((s) => t.review.segment(names[s.kind], short(s.to - s.from))),
  );
  const ticks = [1 / 3, 2 / 3].map((f) => ({ f, t: r.started + span * f }));
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
      <div className="section-label">{t.review.timeline}</div>
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
        <div className="section-label">{t.review.sources}</div>
        <div className="num muted" style={{ fontSize: 12 }}>
          {t.review.samples(total)}
        </div>
      </div>
      {total > 0 ? (
        <>
          <div className="srcbar" role="img" aria-label={t.review.sourcesAria(total, rules, text, vision)}>
            {parts.map((p) => (
              <div key={p.k} className={`seg-${p.k}`} style={{ flex: `${p.n} 1 0` }} />
            ))}
          </div>
          <div className="srclist">
            <div>
              <span className="sw rules" />
              <span style={{ flex: 1 }}>
                {t.review.rules} <span className="muted">{t.review.local}</span>
              </span>
              <b>{rules}</b>
            </div>
            <div>
              <span className="sw text" />
              <span style={{ flex: 1 }}>{t.review.textModel}</span>
              <b>{text}</b>
            </div>
            <div>
              <span className="sw vision" />
              <span style={{ flex: 1 }}>
                {t.review.visionModel} <span className="muted">{t.review.screenshot}</span>
              </span>
              <b>{vision}</b>
            </div>
          </div>
          <div className="hint">{modelShare === 0 ? t.review.allLocal : t.review.modelShare(modelShare)}</div>
        </>
      ) : (
        <div className="hint">{t.review.noSamples}</div>
      )}
    </div>
  );
}

export default function ReviewWindow() {
  const [r, setR] = useState<Review | null | undefined>(undefined);
  const [view, setView] = useState<TimerView | null>(null);
  const [err, setErr] = useState("");

  useEffect(() => {
    const load = () =>
      api
        .getReview()
        .then(setR)
        .catch((e) => setErr(errText(e)));
    load();
    api.getState().then(setView);
    const subs = [onWindowShown(load), onTick(setView)];
    return () =>
      subs.forEach((p) => {
        p.then((un) => un());
      });
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
        {t.review.title}
      </div>
      <div className="win-body">
        {err && <div className="review msg bad">{err}</div>}
        {r === null && (
          <div className="blank" style={{ height: "100%" }}>
            <b>{t.review.emptyTitle}</b>
            <span>{t.review.emptyBody}</span>
          </div>
        )}
        {r && (
          <div className="review">
            <div className="review-head">
              <div>
                <h1>{r.task || t.review.noTask}</h1>
                <div className="meta">{t.review.meta(dayWord(r.started), hm(r.started), hm(r.ended), r.ordinal, mins(r.planned_secs))}</div>
              </div>
              <span className={r.completed ? "pill hot" : "pill"}>{r.completed ? t.review.completed : t.review.endedEarly}</span>
            </div>

            <div className="tiles">
              <div className="tile">
                <div className="k">
                  <span className="sw focus" aria-hidden="true" />
                  {t.review.focus}
                </div>
                <div className="v">
                  <b>{mins(r.focus_secs)}</b>
                  <span>{t.review.focusUnit(mins(r.focus_secs))}</span>
                </div>
              </div>
              <div className="tile">
                <div className="k">
                  <span className="sw distract" aria-hidden="true" />
                  {t.review.distract}
                </div>
                <div className="v">
                  <b>{mins(r.distract_secs)}</b>
                  <span>{t.review.distractUnit(mins(r.distract_secs), r.alerts)}</span>
                </div>
              </div>
              <div className="tile">
                <div className="k">
                  <span className="sw away" aria-hidden="true" />
                  {t.review.away}
                </div>
                <div className="v">
                  <b>{mins(r.away_secs)}</b>
                  <span>{t.review.awayUnit(mins(r.away_secs))}</span>
                </div>
              </div>
            </div>

            <Timeline r={r} />

            <div className="review-cols">
              <div style={{ flex: "1.35 1 0" }}>
                <div className="section-label">{t.review.distractions}</div>
                {r.distractions.length ? (
                  <div className="dlist">
                    {r.distractions.map((d) => (
                      <DistractionRow key={d.at} d={d} />
                    ))}
                  </div>
                ) : (
                  <div className="empty">{t.review.noDistractions}</div>
                )}
                <div className="hint">{t.review.distractionsHint}</div>
              </div>
              <Sources r={r} />
            </div>

            <div className="review-foot">
              {owed != null && (
                <button type="button" className="btn" onClick={rest}>
                  {t.review.takeBreak(owed)}
                </button>
              )}
              {view && view.phase !== "focus" && (
                <button type="button" className="btn primary" onClick={again}>
                  {t.review.again}
                </button>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
