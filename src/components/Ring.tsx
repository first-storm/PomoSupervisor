interface Props {
  progress: number; // 0..1, elapsed share
  size?: number;
  children?: React.ReactNode;
}

/** The countdown ring: the arc that remains shrinks clockwise. */
export default function Ring({ progress, size = 248, children }: Props) {
  const stroke = 10;
  const r = (size - stroke) / 2;
  const c = 2 * Math.PI * r;
  const p = Math.min(1, Math.max(0, progress));
  // 60 minute ticks around the dial
  const ticks = Array.from({ length: 60 }, (_, i) => {
    const a = (i / 60) * 2 * Math.PI - Math.PI / 2;
    const r1 = r - stroke - 4;
    const r2 = r1 - (i % 5 === 0 ? 7 : 3);
    return (
      <line
        key={i}
        x1={size / 2 + r1 * Math.cos(a)}
        y1={size / 2 + r1 * Math.sin(a)}
        x2={size / 2 + r2 * Math.cos(a)}
        y2={size / 2 + r2 * Math.sin(a)}
        className={i % 5 === 0 ? "tick major" : "tick"}
      />
    );
  });
  return (
    <div className="ring" style={{ width: size, height: size }}>
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} aria-hidden>
        {ticks}
        <circle className="ring-track" cx={size / 2} cy={size / 2} r={r} strokeWidth={stroke} fill="none" />
        <circle
          className="ring-arc"
          cx={size / 2}
          cy={size / 2}
          r={r}
          strokeWidth={stroke}
          fill="none"
          strokeLinecap="round"
          strokeDasharray={c}
          strokeDashoffset={c * p}
          transform={`rotate(-90 ${size / 2} ${size / 2})`}
        />
      </svg>
      <div className="ring-center">{children}</div>
    </div>
  );
}
