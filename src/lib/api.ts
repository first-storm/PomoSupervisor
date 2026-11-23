import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

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

export type ProviderKind = "openai" | "anthropic" | "ollama";
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
  vision_enabled: boolean;
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

export interface AlertRow {
  at: number;
  stage: string;
  app: string;
  title: string;
  url: string | null;
  reason: string;
  task: string;
}

export interface Stats {
  completed: number;
  focus_minutes: number;
  alerts: number;
  focused_samples: number;
  distracted_samples: number;
  llm_calls: number;
  llm_cached: number;
  stages: { rules: number; text: number; vision: number };
  recent_alerts: AlertRow[];
  week: { day: string; completed: number; focus_minutes: number }[];
}

export type Slot = "text" | "vision";

export const api = {
  getState: () => invoke<TimerView>("get_state"),
  lastVerdict: () => invoke<VerdictEvent | null>("last_verdict"),
  startFocus: (task: string) => invoke<TimerView>("start_focus", { task }),
  startBreak: () => invoke<TimerView>("start_break"),
  pause: () => invoke<TimerView>("pause"),
  resume: () => invoke<TimerView>("resume"),
  stop: () => invoke<TimerView>("stop"),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<Settings>("save_settings", { settings }),
  modelStatus: () => invoke<ModelStatus>("model_status"),
  setApiKey: (slot: Slot, key: string) => invoke<void>("set_api_key", { slot, key }),
  testProvider: (slot: Slot) => invoke<string>("test_provider", { slot }),
  getStats: () => invoke<Stats>("get_stats"),
  checkPermissions: () => invoke<{ screen_recording: boolean }>("check_permissions"),
  requestScreenRecording: () => invoke<boolean>("request_screen_recording"),
  openPrivacy: (pane: "screen" | "automation") => invoke<void>("open_privacy", { pane }),
};

export const onTick = (f: (v: TimerView) => void): Promise<UnlistenFn> =>
  listen<TimerView>("timer://tick", (e) => f(e.payload));
export const onVerdict = (f: (v: VerdictEvent) => void): Promise<UnlistenFn> =>
  listen<VerdictEvent>("supervisor://verdict", (e) => f(e.payload));

export const mmss = (s: number) =>
  `${String(Math.floor(s / 60)).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`;

export const STAGE_LABEL: Record<string, string> = {
  idle: "空闲检测",
  rules: "规则匹配",
  text: "文本模型",
  vision: "视觉模型",
  none: "无结论",
  self: "跳过",
};

export const errText = (e: unknown) => (typeof e === "string" ? e : e instanceof Error ? e.message : JSON.stringify(e));
