const zhHans = {
  title: "专注回顾",
  today: "今天",
  yesterday: "昨天",
  emptyTitle: "还没有结束的番茄",
  emptyBody: "完成或结束一轮后，这里会显示它的专注、分心和判断来源。",
  noTask: "（未填写任务）",
  /** The line under the task: day, time span, which pomodoro of the day, planned length. */
  meta: (day: string, start: string, end: string, ordinal: number, minutes: number) =>
    `${day} ${start} – ${end} · 第 ${ordinal} 个番茄 · ${minutes} 分钟`,
  completed: "已完成",
  endedEarly: "提前结束",
  focus: "专注",
  distract: "分心",
  away: "离开",
  /** The unit after a tile's big number. */
  focusUnit: (_minutes: number) => "分钟",
  distractUnit: (_minutes: number, alerts: number) => `分钟 · 提醒 ${alerts} 次`,
  awayUnit: (_minutes: number) => "分钟 · 不计入番茄",
  timeline: "时间线",
  /** One stretch of the timeline for screen readers, e.g. "专注 3 分钟". */
  segment: (what: string, dur: string) => `${what} ${dur}`,
  timelineAria: (start: string, segments: string[]) => `时间线：${start} 起` + segments.join("，"),
  distractions: "分心片段",
  noDistractions: "这一轮没有分心。",
  distractionsHint: "判错了就点「其实在工作」，对应的 App 或域名会进白名单。",
  actuallyWorking: "其实在工作",
  added: "已加入",
  addedToAllow: (name: string) => `${name} 已加入白名单`,
  sources: "判断来源",
  samples: (n: number) => `共 ${n} 次采样`,
  sourcesAria: (total: number, rules: number, text: number, vision: number) =>
    `${total} 次采样中，规则匹配 ${rules} 次，文本模型 ${text} 次，视觉模型 ${vision} 次`,
  rules: "规则匹配",
  local: "· 本地",
  textModel: "文本模型",
  visionModel: "视觉模型",
  screenshot: "· 截图",
  allLocal: "所有采样都在本地判完，没有用到模型。",
  modelShare: (pct: number) => `只有 ${pct}% 的采样用到了模型，其余都在本地判完。`,
  noSamples: "这一轮没有得出结论的采样。",
  takeBreak: (minutes: number) => `休息 ${minutes} 分钟`,
  again: "再来一轮",
};

export type Dict = typeof zhHans;
export default zhHans;
