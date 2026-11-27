const secs = (s: number) => Math.max(0, Math.round(s));

const zhHans = {
  /** A short span: seconds under a minute, else rounded minutes. */
  dur: (s: number) => (secs(s) < 60 ? `${secs(s)} 秒` : `${Math.round(secs(s) / 60)} 分钟`),
  minutes: (n: number) => `${n} 分钟`,
  /** StageBadge: which step of the decision chain settled a sample. */
  stage: {
    allow: "规则 · 白名单",
    block: "规则 · 黑名单",
    round: "额外允许",
    text: "文本模型",
    vision: "视觉模型",
    idle: "空闲检测",
    self: "本应用",
    none: "未判定",
  },
  /** allowLabel for an entry that is one page of a site. */
  pageOf: (host: string) => `${host} 的一个页面`,
};

export type Dict = typeof zhHans;
export default zhHans;
