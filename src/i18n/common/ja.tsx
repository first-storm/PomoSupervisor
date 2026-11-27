import type { Dict } from "./zh-Hans";

const secs = (s: number) => Math.max(0, Math.round(s));

const ja = {
  dur: (s: number) => (secs(s) < 60 ? `${secs(s)}秒` : `${Math.round(secs(s) / 60)}分`),
  minutes: (n: number) => `${n}分`,
  stage: {
    allow: "ルール · 許可リスト",
    block: "ルール · ブロックリスト",
    round: "追加で許可",
    text: "テキストモデル",
    vision: "画像モデル",
    idle: "アイドル検出",
    self: "このアプリ",
    none: "未判定",
  },
  pageOf: (host: string) => `${host} のページ`,
} satisfies Dict;

export default ja;
