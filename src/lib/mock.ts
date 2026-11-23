// Dev-only fixtures for previewing the screens in a plain browser (`pnpm dev`, no Tauri).
//   /?state=start|focus|away|paused|rest|break|nudge   the popover in that state
//   /?view=review   /?view=settings                    the windows
// Never bundled into a release build: api.ts only imports this when import.meta.env.DEV.
import type { Review, Settings, TimerView, VerdictEvent } from "./api";

const q = new URLSearchParams(location.search);
const state = q.get("state") ?? "focus";
const now = Math.floor(Date.now() / 1000);

if ((q.get("view") ?? "main") === "main") {
  document.documentElement.style.cssText = "background:#CCD2D8;padding:24px 40px;height:auto";
}

const base: TimerView = {
  phase: "focus",
  remaining: 18 * 60 + 42,
  total: 25 * 60,
  paused: null,
  task: "给采样模块写单元测试",
  cycle: 1,
  long_break_every: 4,
  streak: 0,
  threshold: 3,
  focus_secs: 25 * 60,
  paused_for: 0,
  idle_secs: 120,
  rest_left: null,
  auto_resume: true,
  next_break_min: null,
  short_break_min: 5,
  round_allow: ["github.com", "developer.apple.com", "终端"],
  distracted_ticks: [7],
};

const views: Record<string, Partial<TimerView>> = {
  start: { phase: "idle", remaining: 0, total: 0, task: "给采样模块写单元测试，覆盖空闲检测和截图去重", distracted_ticks: [] },
  focus: {},
  away: { paused: "away", remaining: 12 * 60 + 30, paused_for: 4 * 60 },
  paused: { paused: "user", remaining: 12 * 60 + 30, paused_for: 95 },
  rest: { paused: "user", remaining: 12 * 60 + 30, paused_for: 40, rest_left: 260 },
  break: { phase: "short_break", remaining: 3 * 60 + 12, total: 5 * 60 },
  nudge: { remaining: 9 * 60 + 12 },
};
const view: TimerView = { ...base, ...(views[state] ?? {}) };

const v = (at: number, over: Partial<VerdictEvent>): VerdictEvent => ({
  at,
  stage: "rules",
  result: "focused",
  reason: "白名单应用：Xcode",
  confidence: 1,
  cached: false,
  notes: [],
  app: "Xcode",
  title: "SamplerTests.swift",
  url: null,
  streak: 0,
  threshold: 3,
  alerted: false,
  ...over,
});
const lastVerdict = v(now - 2, {});
const distracted = (at: number) =>
  v(at, { stage: "text", result: "distracted", reason: "视频网站，与写测试无关", app: "Safari", title: "【游戏】…", url: "https://www.bilibili.com/video/BV1" });
const recent = [distracted(now - 8), distracted(now - 4), distracted(now)];

const t0 = now - 29 * 60;
const m = (n: number) => t0 + n * 60;
const review: Review = {
  id: 1,
  task: "给采样模块写单元测试",
  started: t0,
  ended: m(29),
  planned_secs: 1500,
  focus_secs: 22 * 60,
  distract_secs: 3 * 60,
  away_secs: 4 * 60,
  completed: true,
  ordinal: 1,
  alerts: 2,
  segments: [
    { kind: "focus", from: m(0), to: m(9) },
    { kind: "distract", from: m(9), to: m(11) },
    { kind: "focus", from: m(11), to: m(17) },
    { kind: "away", from: m(17), to: m(21) },
    { kind: "focus", from: m(21), to: m(26) },
    { kind: "distract", from: m(26), to: m(27) },
    { kind: "focus", from: m(27), to: m(29) },
  ],
  distractions: [
    { at: m(9), secs: 120, app: "Google Chrome", title: "bilibili", url: "https://www.bilibili.com/", stage: "rules", reason: "黑名单网站：bilibili.com" },
    { at: m(26), secs: 60, app: "Safari", title: "机械键盘选购指南", url: null, stage: "text", reason: "购物，与任务无关" },
  ],
  stages: { rules: 312, text: 55, vision: 8 },
};

const settings: Settings = {
  focus_min: 25,
  short_break_min: 5,
  long_break_min: 15,
  long_break_every: 4,
  sample_secs: 4,
  idle_secs: 120,
  confidence: 0.75,
  phash_threshold: 6,
  streak: 3,
  sound: true,
  auto_resume: true,
  text_enabled: true,
  vision_enabled: true,
  ax_enabled: true,
  rules: {
    block_apps: ["Steam", "Netflix", "哔哩哔哩", "抖音"],
    allow_apps: ["Xcode", "Terminal", "Visual Studio Code"],
    block_domains: ["bilibili.com", "youtube.com", "weibo.com"],
    allow_domains: ["github.com", "developer.apple.com"],
    block_keywords: ["bilibili", "YouTube"],
    allow_keywords: [],
  },
  text_model: { kind: "openai", base_url: "https://api.deepseek.com/v1", model: "deepseek-chat" },
  vision_model: null,
};

const answers: Record<string, unknown> = {
  get_state: view,
  last_verdict: state === "nudge" ? recent[2] : lastVerdict,
  recent_verdicts: recent,
  today_completed: 1,
  get_review: review,
  get_settings: settings,
  save_settings: settings,
  model_status: { text_key: true, vision_key: false, text_ready: true, vision_ready: false, text_error: null, vision_error: null },
  check_permissions: { screen_recording: true, accessibility: true },
  mark_as_work: "bilibili.com",
};

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (cmd !== "popover_fit") console.debug("[mock] invoke", cmd, args ?? "");
  if (cmd === "save_settings") return args?.settings as T;
  return (cmd in answers ? answers[cmd] : view) as T;
}

export async function listen<T>(event: string, f: (e: { payload: T }) => void) {
  if (event === "popover://mode" && state === "nudge") setTimeout(() => f({ payload: "nudge" as T }), 0);
  return () => {};
}
