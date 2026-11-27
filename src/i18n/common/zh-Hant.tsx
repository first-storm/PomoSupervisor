import type { Dict } from "./zh-Hans";

const secs = (s: number) => Math.max(0, Math.round(s));

const zhHant = {
  dur: (s: number) => (secs(s) < 60 ? `${secs(s)} 秒` : `${Math.round(secs(s) / 60)} 分鐘`),
  minutes: (n: number) => `${n} 分鐘`,
  stage: {
    allow: "規則 · 白名單",
    block: "規則 · 黑名單",
    round: "額外允許",
    text: "文字模型",
    vision: "視覺模型",
    idle: "閒置偵測",
    self: "本程式",
    none: "未判定",
  },
  pageOf: (host: string) => `${host} 的一個頁面`,
} satisfies Dict;

export default zhHant;
