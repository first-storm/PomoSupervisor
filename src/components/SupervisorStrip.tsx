import { STAGE_LABEL, type VerdictEvent } from "../lib/api";

const CHAIN = ["idle", "rules", "text", "vision"] as const;
const LOCAL = new Set(["idle", "rules"]);

function SmootherDots({ streak, threshold }: { streak: number; threshold: number }) {
  return (
    <span className="dots" title={`连续分心 ${streak}/${threshold} 次后提醒`}>
      {Array.from({ length: threshold }, (_, i) => (
        <i key={i} className={i < streak ? "dot hot" : "dot"} />
      ))}
    </span>
  );
}

/** Live view of the decision chain: which stage decided, what it said, and the smoother. */
export default function SupervisorStrip({ v, streak, threshold }: { v: VerdictEvent | null; streak: number; threshold: number }) {
  const resultText =
    v?.result === "focused" ? "专注" : v?.result === "distracted" ? "分心" : v?.result === "away" ? "已离开" : "未判定";
  return (
    <section className="strip">
      <div className="chain">
        {CHAIN.map((s, i) => {
          const hit = v?.stage === s;
          return (
            <span key={s} className="chain-step">
              {i > 0 && <span className="chain-arrow">›</span>}
              <span className={`chip ${LOCAL.has(s) ? "local" : "model"} ${hit ? "hit" : ""}`}>{STAGE_LABEL[s]}</span>
            </span>
          );
        })}
      </div>

      {v ? (
        <>
          <div className="verdict-line">
            <span className={`verdict ${v.result}`}>{resultText}</span>
            <span className="reason">{v.reason}</span>
            {v.confidence != null && v.stage !== "rules" && <span className="conf">{Math.round(v.confidence * 100)}%</span>}
            {v.cached && <span className="cached">缓存</span>}
          </div>
          <div className="window-line" title={v.url ?? v.title}>
            <b>{v.app}</b>
            <span>{v.url ?? (v.title || "（无标题）")}</span>
          </div>
          {v.notes.length > 0 && <div className="notes">{v.notes.join(" · ")}</div>}
        </>
      ) : (
        <div className="verdict-line muted">等待第一次采样…</div>
      )}

      <div className="smoother">
        <span>平滑器</span>
        <SmootherDots streak={streak} threshold={threshold} />
        <span className="muted">连续 {threshold} 次分心才提醒</span>
      </div>
    </section>
  );
}
