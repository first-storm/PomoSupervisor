import type { Dict } from "./zh-Hans";

const secs = (s: number) => Math.max(0, Math.round(s));
const minutes = (n: number) => (n === 1 ? "1 minute" : `${n} minutes`);

const enUS = {
  dur: (s: number) => {
    const n = secs(s);
    if (n < 60) return n === 1 ? "1 second" : `${n} seconds`;
    return minutes(Math.round(n / 60));
  },
  minutes,
  stage: {
    allow: "Rules · Allowed",
    block: "Rules · Blocked",
    round: "Also allowed",
    text: "Text model",
    vision: "Vision model",
    idle: "Idle",
    self: "This app",
    none: "Undecided",
  },
  pageOf: (host: string) => `A page on ${host}`,
} satisfies Dict;

export default enUS;
