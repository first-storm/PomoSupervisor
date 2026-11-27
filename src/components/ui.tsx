import { DIAL_TICKS } from "./dial-consts";
import type { VerdictEvent } from "../lib/api";
import { t } from "../i18n";

export function MoreIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 16 16" width="16" height="16">
      <circle cx="3.5" cy="8" r="1.3" fill="#44474D" />
      <circle cx="8" cy="8" r="1.3" fill="#44474D" />
      <circle cx="12.5" cy="8" r="1.3" fill="#44474D" />
    </svg>
  );
}

export function PlusIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 12 12" width="10" height="10">
      <path d="M6 1.5v9M1.5 6h9" fill="none" stroke="#44474D" strokeWidth="1.5" strokeLinecap="round" />
    </svg>
  );
}

export function PauseIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 12 12" width="10" height="10">
      <path d="M4 2.5v7M8 2.5v7" fill="none" stroke="#44474D" strokeWidth="1.8" strokeLinecap="round" />
    </svg>
  );
}

export function TimerGlyph({ color = "#FFFFFF", size = 13 }: { color?: string; size?: number }) {
  return (
    <svg aria-hidden="true" viewBox="0 0 16 16" width={size} height={size}>
      <circle cx="8" cy="9.2" r="5.3" fill="none" stroke={color} strokeWidth="1.6" />
      <path d="M8 3.9V1.8M8 3.9L5.9 2.7M8 3.9l2.1-1.2" fill="none" stroke={color} strokeWidth="1.6" strokeLinecap="round" />
    </svg>
  );
}

export function LockIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 16 16" width="14" height="14" style={{ flex: "none" }}>
      <rect x="3.2" y="7" width="9.6" height="6.6" rx="1.6" fill="none" stroke="#5E5E63" strokeWidth="1.4" />
      <path d="M5.4 7V5.2a2.6 2.6 0 0 1 5.2 0V7" fill="none" stroke="#5E5E63" strokeWidth="1.4" strokeLinecap="round" />
    </svg>
  );
}

/** The badge naming which step of the decision chain settled a sample. */
export function StageBadge({ v }: { v: Pick<VerdictEvent, "stage" | "result" | "round_allowed"> }) {
  switch (v.stage) {
    case "rules":
      if (v.round_allowed) return <span className="badge">{t.common.stage.round}</span>;
      return <span className="badge">{v.result === "focused" ? t.common.stage.allow : t.common.stage.block}</span>;
    case "text":
      return <span className="badge model">{t.common.stage.text}</span>;
    case "vision":
      return <span className="badge model">{t.common.stage.vision}</span>;
    case "idle":
      return <span className="badge">{t.common.stage.idle}</span>;
    case "self":
      return <span className="badge">{t.common.stage.self}</span>;
    default:
      return <span className="badge">{t.common.stage.none}</span>;
  }
}

export type DialTone = "focus" | "dim" | "rest";

/**
 * 50 ticks around the dial. Elapsed ticks take the tone's colour, ticks during which
 * a sample was judged distracted are dark, and the current tick is longer.
 */
export function Dial({
  progress,
  distracted = [],
  tone,
  children,
}: {
  progress: number;
  distracted?: number[];
  tone: DialTone;
  children: React.ReactNode;
}) {
  const p = Math.min(1, Math.max(0, progress));
  const current = Math.min(DIAL_TICKS - 1, Math.floor(p * DIAL_TICKS));
  const done = tone === "rest" ? "#3D5275" : "#C8381F";
  const todo = tone === "dim" ? "#B4B8BF" : "#DADCE0";
  const marks = new Set(distracted);
  return (
    <div className={tone === "dim" ? "dial dim" : "dial"}>
      <div className="ticks" aria-hidden="true">
        {Array.from({ length: DIAL_TICKS }, (_, i) => {
          const past = i < current;
          const bg = past ? (marks.has(i) ? "#2B2D33" : done) : i === current && tone !== "dim" ? done : todo;
          const style: React.CSSProperties = { transform: `rotate(${(i * 360) / DIAL_TICKS}deg)`, background: bg };
          if (i === current && tone !== "dim") style.height = 24;
          return <span key={i} className="tk" style={style} />;
        })}
      </div>
      <div className="face">{children}</div>
    </div>
  );
}
