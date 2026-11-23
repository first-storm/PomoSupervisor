import { useEffect, useRef, useState } from "react";
import { api, onTick, onVerdict, type TimerView, type VerdictEvent } from "./lib/api";
import TimerPage from "./pages/Timer";
import StatsPage from "./pages/Stats";
import SettingsPage from "./pages/Settings";

type Tab = "timer" | "stats" | "settings";
const TABS: [Tab, string][] = [
  ["timer", "计时"],
  ["stats", "统计"],
  ["settings", "设置"],
];

export default function App() {
  const [tab, setTab] = useState<Tab>("timer");
  const [view, setView] = useState<TimerView | null>(null);
  const [verdict, setVerdict] = useState<VerdictEvent | null>(null);

  // A new focus round starts with a clean supervisor strip.
  const prevPhase = useRef<string | null>(null);
  useEffect(() => {
    if (view && view.phase === "focus" && prevPhase.current !== null && prevPhase.current !== "focus") setVerdict(null);
    if (view) prevPhase.current = view.phase;
  }, [view?.phase]);

  useEffect(() => {
    api.getState().then(setView);
    api.lastVerdict().then(setVerdict);
    const subs = [onTick(setView), onVerdict(setVerdict)];
    return () => subs.forEach((p) => p.then((un) => un()));
  }, []);

  const mood = view?.paused === "away" ? "away" : view?.phase === "focus" ? "focus" : view && view.phase !== "idle" ? "rest" : "idle";

  return (
    <div className="app" data-mood={mood}>
      <header className="titlebar" data-tauri-drag-region>
        <nav className="tabs">
          {TABS.map(([id, label]) => (
            <button key={id} className={tab === id ? "tab on" : "tab"} onClick={() => setTab(id)}>
              {label}
            </button>
          ))}
        </nav>
      </header>
      <main className="page">
        {tab === "timer" && view && <TimerPage view={view} verdict={verdict} onView={setView} />}
        {tab === "stats" && <StatsPage />}
        {tab === "settings" && <SettingsPage />}
      </main>
    </div>
  );
}
