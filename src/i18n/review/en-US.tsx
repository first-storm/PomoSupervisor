import type { Dict } from "./zh-Hans";

const num = new Intl.NumberFormat("en-US");
const plural = (n: number, one: string, many: string) => `${num.format(n)} ${n === 1 ? one : many}`;
const minuteUnit = (n: number) => (n === 1 ? "minute" : "minutes");

const enUS = {
  title: "Focus Review",
  today: "Today",
  yesterday: "Yesterday",
  emptyTitle: "No finished pomodoros yet",
  emptyBody: "Once you finish or end a round, its focus, distractions and how they were judged show up here.",
  noTask: "(No task)",
  meta: (day: string, start: string, end: string, ordinal: number, minutes: number) =>
    `${day} ${start} – ${end} · Pomodoro ${ordinal} · ${plural(minutes, "minute", "minutes")}`,
  completed: "Completed",
  endedEarly: "Ended early",
  focus: "Focus",
  distract: "Distracted",
  away: "Away",
  focusUnit: (minutes: number) => minuteUnit(minutes),
  distractUnit: (minutes: number, alerts: number) => `${minuteUnit(minutes)} · ${plural(alerts, "reminder", "reminders")}`,
  awayUnit: (minutes: number) => `${minuteUnit(minutes)} · not counted`,
  timeline: "Timeline",
  segment: (what: string, dur: string) => `${what} ${dur}`,
  timelineAria: (start: string, segments: string[]) => `Timeline from ${start}: ${segments.join(", ")}`,
  distractions: "Distractions",
  noDistractions: "No distractions this round.",
  distractionsHint: "If one was judged wrong, click “Actually working” and that app or domain goes on the allow list.",
  actuallyWorking: "Actually working",
  added: "Added",
  addedToAllow: (name: string) => `Added ${name} to the allow list`,
  sources: "How it was judged",
  samples: (n: number) => plural(n, "sample", "samples"),
  sourcesAria: (total: number, rules: number, text: number, vision: number) =>
    `Of ${plural(total, "sample", "samples")}, rules matched ${num.format(rules)}, the text model judged ${num.format(text)} and the vision model judged ${num.format(vision)}`,
  rules: "Rules",
  local: "· on this Mac",
  textModel: "Text model",
  visionModel: "Vision model",
  screenshot: "· screenshot",
  allLocal: "Every sample was judged on this Mac. No models were used.",
  modelShare: (pct: number) => `Only ${pct}% of samples needed a model. The rest were judged on this Mac.`,
  noSamples: "No samples reached a verdict this round.",
  takeBreak: (minutes: number) => `Take a ${minutes}-minute break`,
  again: "Another round",
} satisfies Dict;

export default enUS;
