import type { Dict } from "./zh-Hans";

const num = new Intl.NumberFormat("ja-JP");

const ja = {
  title: "集中の振り返り",
  today: "今日",
  yesterday: "昨日",
  emptyTitle: "終わったポモドーロはまだありません",
  emptyBody: "1回分を完了または終了すると、集中・脱線と判定の内訳がここに表示されます。",
  noTask: "（タスク未入力）",
  meta: (day: string, start: string, end: string, ordinal: number, minutes: number) =>
    `${day} ${start} – ${end} · ${ordinal}個目のポモドーロ · ${minutes}分`,
  completed: "完了",
  endedEarly: "途中で終了",
  focus: "集中",
  distract: "脱線",
  away: "離席",
  focusUnit: (_minutes: number) => "分",
  distractUnit: (_minutes: number, alerts: number) => `分 · 通知 ${num.format(alerts)}回`,
  awayUnit: (_minutes: number) => "分 · ポモドーロに含めない",
  timeline: "タイムライン",
  segment: (what: string, dur: string) => `${what} ${dur}`,
  timelineAria: (start: string, segments: string[]) => `タイムライン：${start} から、${segments.join("、")}`,
  distractions: "脱線した時間",
  noDistractions: "今回は脱線しませんでした。",
  distractionsHint: "判定が間違っていたら「実は作業中」をクリックすると、そのアプリやドメインが許可リストに入ります。",
  actuallyWorking: "実は作業中",
  added: "追加済み",
  addedToAllow: (name: string) => `${name} を許可リストに追加しました`,
  sources: "判定の内訳",
  samples: (n: number) => `サンプル ${num.format(n)}件`,
  sourcesAria: (total: number, rules: number, text: number, vision: number) =>
    `サンプル ${num.format(total)}件のうち、ルール ${num.format(rules)}件、テキストモデル ${num.format(text)}件、画像モデル ${num.format(vision)}件`,
  rules: "ルール",
  local: "· ローカル",
  textModel: "テキストモデル",
  visionModel: "画像モデル",
  screenshot: "· スクリーンショット",
  allLocal: "すべてのサンプルをローカルで判定しました。モデルは使っていません。",
  modelShare: (pct: number) => `モデルを使ったのはサンプルの ${pct}% だけで、残りはローカルで判定しました。`,
  noSamples: "今回は判定が出たサンプルがありません。",
  takeBreak: (minutes: number) => `${minutes}分休憩`,
  again: "もう1回",
} satisfies Dict;

export default ja;
