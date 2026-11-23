import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen, type UnlistenFn } from "@tauri-apps/api/event";

export type Phase = "idle" | "focus" | "short_break" | "long_break";
export type Pause = "user" | "away" | null;

export interface TimerView {
  phase: Phase;
  remaining: number;
  total: number;
  paused: Pause;
  task: string;
  cycle: number;
  long_break_every: number;
  streak: number;
  threshold: number;
  focus_secs: number;
  paused_for: number;
  idle_secs: number;
  rest_left: number | null;
  auto_resume: boolean;
  next_break_min: number | null;
  short_break_min: number;
  round_allow: string[];
  distracted_ticks: number[];
}

export type Stage = "idle" | "rules" | "text" | "vision" | "none" | "self";

export interface VerdictEvent {
  at: number;
  stage: Stage;
  result: "focused" | "distracted" | "away" | "none";
  reason: string;
  confidence: number | null;
  cached: boolean;
  notes: string[];
  app: string;
  title: string;
  url: string | null;
  streak: number;
  threshold: number;
  alerted: boolean;
}

export type ProviderKind = "openai" | "anthropic" | "ollama" | "decision" | "openai_decisions";
export interface ProviderConfig {
  kind: ProviderKind;
  base_url: string;
  model: string;
}

export interface Rules {
  block_apps: string[];
  allow_apps: string[];
  block_domains: string[];
  allow_domains: string[];
  block_keywords: string[];
  allow_keywords: string[];
}

export interface Settings {
  focus_min: number;
  short_break_min: number;
  long_break_min: number;
  long_break_every: number;
  sample_secs: number;
  idle_secs: number;
  confidence: number;
  phash_threshold: number;
  streak: number;
  sound: boolean;
  auto_resume: boolean;
  text_enabled: boolean;
  vision_enabled: boolean;
  ax_enabled: boolean;
  rules: Rules;
  text_model: ProviderConfig | null;
  vision_model: ProviderConfig | null;
}

export interface ModelStatus {
  text_key: boolean;
  vision_key: boolean;
  text_ready: boolean;
  vision_ready: boolean;
  text_error: string | null;
  vision_error: string | null;
}

export interface Segment {
  kind: "focus" | "distract" | "away";
  from: number;
  to: number;
}

export interface Distraction {
  at: number;
  secs: number;
  app: string;
  title: string;
  url: string | null;
  stage: string;
  reason: string;
}

export interface StageCounts {
  rules: number;
  text: number;
  vision: number;
}

export interface Review {
  id: number;
  task: string;
  started: number;
  ended: number;
  planned_secs: number;
  focus_secs: number;
  distract_secs: number;
  away_secs: number;
  completed: boolean;
  ordinal: number;
  alerts: number;
  segments: Segment[];
  distractions: Distraction[];
  stages: StageCounts;
}

export type Slot = "text" | "vision";
export type PopoverMode = "normal" | "nudge";

type Invoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
type Listen = <T>(event: string, f: (e: { payload: T }) => void) => Promise<UnlistenFn>;

/** Outside Tauri (plain `pnpm dev` in a browser) the dev-only fixtures in mock.ts answer instead. */
export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
let invoke: Invoke = tauriInvoke;
let listen: Listen = tauriListen;
if (!inTauri && import.meta.env.DEV) {
  const mock = await import("./mock");
  invoke = mock.invoke;
  listen = mock.listen;
}

export const api = {
  getState: () => invoke<TimerView>("get_state"),
  lastVerdict: () => invoke<VerdictEvent | null>("last_verdict"),
  startFocus: (task: string, minutes: number, allow: string[]) => invoke<TimerView>("start_focus", { task, minutes, allow }),
  startBreak: () => invoke<TimerView>("start_break"),
  pause: () => invoke<TimerView>("pause"),
  resume: () => invoke<TimerView>("resume"),
  stop: () => invoke<TimerView>("stop"),
  rest: (minutes: number) => invoke<TimerView>("rest", { minutes }),
  allowForRound: (item: string) => invoke<TimerView>("allow_for_round", { item }),
  markAsWork: (appName: string, url: string | null) => invoke<string>("mark_as_work", { appName, url }),
  setAutoResume: (on: boolean) => invoke<TimerView>("set_auto_resume", { on }),
  recentVerdicts: () => invoke<VerdictEvent[]>("recent_verdicts"),
  getReview: (id?: number) => invoke<Review | null>("get_review", { id: id ?? null }),
  todayCompleted: () => invoke<number>("today_completed"),
  openWindow: (label: "review" | "settings") => invoke<void>("open_window", { label }),
  closeWindow: () => invoke<void>("close_window"),
  popoverFit: (width: number, height: number) => invoke<void>("popover_fit", { width, height }),
  popoverMenu: () => invoke<void>("popover_menu"),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<Settings>("save_settings", { settings }),
  modelStatus: () => invoke<ModelStatus>("model_status"),
  setApiKey: (slot: Slot, key: string) => invoke<void>("set_api_key", { slot, key }),
  testProvider: (slot: Slot) => invoke<string>("test_provider", { slot }),
  checkPermissions: () => invoke<{ screen_recording: boolean; accessibility: boolean }>("check_permissions"),
  requestScreenRecording: () => invoke<boolean>("request_screen_recording"),
  requestAccessibility: () => invoke<boolean>("request_accessibility"),
  openPrivacy: (pane: "screen" | "automation" | "accessibility") => invoke<void>("open_privacy", { pane }),
};

const on =
  <T,>(event: string) =>
  (f: (v: T) => void): Promise<UnlistenFn> =>
    listen<T>(event, (e) => f(e.payload));

export const onTick = on<TimerView>("timer://tick");
export const onVerdict = on<VerdictEvent>("supervisor://verdict");
export const onPopoverMode = on<PopoverMode>("popover://mode");
export const onWindowShown = on<null>("window://shown");
export const onWindowClosing = on<null>("window://closing");

/** Which window this webview is: the menu-bar popover, Review or Settings. */
export function windowLabel(): string {
  if (inTauri) {
    const meta = (window as unknown as { __TAURI_INTERNALS__: { metadata: { currentWindow: { label: string } } } }).__TAURI_INTERNALS__;
    return meta.metadata.currentWindow.label;
  }
  return new URLSearchParams(location.search).get("view") ?? "main";
}

export const mmss = (s: number) => `${String(Math.floor(s / 60)).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`;
export const hm = (t: number) => new Date(t * 1000).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit", hour12: false });
export const hms = (t: number) =>
  new Date(t * 1000).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit", second: "2-digit", hour12: false });

export const hostOf = (url: string | null) => {
  if (!url) return null;
  try {
    const h = new URL(url).hostname.toLowerCase();
    return h.startsWith("www.") ? h.slice(4) : h;
  } catch {
    return null;
  }
};

/** What to call the window a sample saw: its site, else its app. */
export const targetOf = (v: { app: string; url: string | null }) => hostOf(v.url) ?? v.app;

export const errText = (e: unknown) => (typeof e === "string" ? e : e instanceof Error ? e.message : JSON.stringify(e));
