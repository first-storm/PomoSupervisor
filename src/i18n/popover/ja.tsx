import type { ReactNode } from "react";
import type { Dict } from "./zh-Hans";

const ja = {
  more: "その他",
  quote: (s: string) => `「${s}」`,

  perm: {
    axTitle: "「アクセシビリティ」の許可が必要です",
    axBody: "ウインドウ内のテキストを読み取り、タイトルだけでは分からない場合の判断に使います。Firefox の URL も補えます。使わない場合は設定 → プライバシーでオフにできます。",
    grant: "許可する",
    screenTitle: "「画面収録」の許可が必要です",
    screenBody: "許可がないと他の App のウインドウタイトルを読めず、スクリーンショットも撮れないため、App 名だけで判断します。許可したあとはアプリを再起動してください。",
    accessibility: "アクセシビリティ",
    browserUrls: "ブラウザの URL",
  },

  sup: {
    ready: "AI による見守りの準備ができました。リストにないウインドウは、上のタスクと照らし合わせて判断します。",
    noTask: "タスクがまだありません。何をするか分からないので、AI の判断は大まかになります。",
    broken: "モデルの準備ができていないため、今回はブロックリストと許可リストだけで判断します。",
    off: "モデルが未設定のため、ブロックリストと許可リストだけで判断します。リストにないウインドウは対象外です。",
    check: "確認する",
    configure: "モデルを設定",
  },

  start: {
    titleFresh: "次のタスクは？",
    title: "今回は何をしますか？",
    task: "タスク",
    taskPlaceholder: "例：サンプリングモジュールの単体テストを書く（アイドル検出とスクリーンショットの重複除去）",
    duration: "時間",
    allowed: "追加で許可",
    remove: (label: string) => `${label} を削除`,
    clickToRemove: "クリックで削除",
    add: "追加",
    appOrDomain: "App 名またはドメイン",
    allowedHint: "これらの App とサイトは集中として扱います。次のポモドーロにも引き継ぎ、「タスクを変更」で消去されます。",
    startFocus: "集中を開始",
    doneTodayEnter: (n: number) => `今日は ${n} ポモドーロ完了 · Return キーで開始`,
    enter: "Return キーで開始",
    breakFirst: (n: number) => `先に ${n} 分休憩`,
    back: "戻る",
  },

  judge: {
    firstLook: "様子を見ています…",
    off: "脱線しているようです",
    on: "タスクに集中しています",
    unsure: "判断できないため、カウントしません",
    allowPageTitle: "このページだけを許可します。このサイトの他のページは引き続き判断します",
    allowTargetTitle: (target: string) => `今回は ${target} について通知しません`,
    pageCounts: "このページはタスク",
    counts: "これはタスク",
    why: "判断の根拠",
  },

  end: {
    ask: (elapsed: string) => `ここで終わりにしますか？ ${elapsed}経過しています。`,
    taskDone: "タスク完了",
    countsHint: "1 ポモドーロとして記録し、いつも通り休憩します。",
    notHalfHint: "時間は振り返りに記録し、いつも通り休憩します。半分を過ぎていないため、ポモドーロにはカウントしません。",
    giveUp: "このポモドーロをやめる",
    keepFocusing: "集中を続ける",
    end: "終了…",
  },

  nthToday: (n: number) => `今日 ${n} 個目`,
  doneToday: (n: number) => `今日 ${n} 個完了`,
  left: (time: string) => `残り ${time}`,

  focus: {
    pill: "集中",
    remainingOf: (min: number) => `残り · 全 ${min} 分`,
    currentTask: "現在のタスク",
    noTask: "（タスクなし）",
    pauseFor: (min: number) => `${min} 分一時停止`,
  },

  paused: {
    pillAway: "一時停止 · 離席中",
    pillBack: "おかえりなさい",
    pill: "一時停止",
    resumesIn: (time: string) => `${time} 後に自動で再開`,
    waiting: "再開を待っています",
    holding: "停止中",
    awayNote: (idle: string) => (
      <>
        キーボードとマウスの操作が {idle}なかったため、タイマーを止めました。
        <br />
        離席していた時間は脱線になりません。
      </>
    ),
    backNote: (stopped: string) => (
      <>
        離席中、タイマーは {stopped}止まっていました。脱線にはなりません。
        <br />
        ずっと資料を読んでいましたか？「離席していない」をクリックすると、この時間を戻せます。
      </>
    ),
    timedNote: () => (
      <>
        ひと息つきましょう。時間になったら自動で続きを始めます。
        <br />
        一時停止中は何をしていても判断しません。
      </>
    ),
    heldNote: () => (
      <>
        タイマーは止まっていますが、このポモドーロは続いています。
        <br />
        一時停止中は何をしていても判断しません。
      </>
    ),
    awayFor: "離席",
    pausedFor: "一時停止",
    autoResume: "戻ったら自動で再開",
    stillHere: "離席していない",
    restMore: (min: number) => `あと ${min} 分休む`,
    resume: "集中を再開",
  },

  returned: {
    pill: "おかえりなさい",
    note: (stopped: string) => `離席中、タイマーは ${stopped}止まっていました。ここから再開します。離席していた時間は脱線になりません。`,
    countItTitle: "止まっていた時間をこのポモドーロに戻し、このポモドーロでは離席と判断するまでの時間も長くします",
    countIt: "離席していない、時間を戻す",
    ok: "OK",
  },

  rest: {
    longPill: "長い休憩",
    pill: "休憩中",
    breakOf: (min: number) => `休憩 · 全 ${min} 分`,
    note: () => (
      <>
        立ち上がって少し歩き、遠くを眺めましょう。
        <br />
        時間になったらお知らせします。休憩中は何をしていても判断しません。
      </>
    ),
    review: "集中の振り返り",
    skip: "休憩せずに開始",
  },

  done: {
    early: "早めに完了",
    pill: "ポモドーロ完了",
    roundOver: "このポモドーロが終わりました",
    statFocus: (min: number) => `集中 ${min} 分`,
    statDistracted: (min: number) => `脱線 ${min} 分`,
    statCalm: "ほぼ脱線なし",
    statAlerts: (n: number) => `通知 ${n} 回`,
    overtime: (over: string) => `${over}超過しています。休憩はまだ始まっていません。`,
    takeBreak: (min: number) => `${min} 分休憩`,
    again: "休憩せずにもう 1 回",
    changeTask: "タスクを変更",
    reviewRound: "今回の振り返りを見る",
  },

  breakOver: {
    pill: "休憩終了",
    continueTask: (quoted: string) => `${quoted}の続きをしますか？`,
    next: "次のポモドーロを始めますか？",
    same: (min: number) => `前回と同じ ${min} 分です。許可リストもそのままです。`,
    overtime: (over: string) => `休憩が ${over}超過しています。`,
    startFocus: "集中を開始",
    stop: "今はやめておく",
  },

  nudge: {
    otherWindow: "別のウインドウ",
    title: "よそ見していませんか？",
    body: (where: ReactNode, task: string | null) => (
      <>
        <strong>{where}</strong> にしばらくいますが、{task ?? "現在のタスク"}とは関係なさそうです。
      </>
    ),
    backToTask: "タスクに戻る",
    pagePart: "このページはタスクの一部",
    part: "これはタスクの一部",
    restFor: (min: number) => `${min} 分休む`,
    pageHint: (host: string) => `作業に戻ると、この通知は自動で消えます。${host} はブロックリストにあるため、このページだけを許可します。他のページは引き続き通知します。`,
    allowHint: (target: string) => `作業に戻ると、この通知は自動で消えます。「これはタスクの一部」を選ぶと、このタスクでは ${target} を許可し、以後は通知しません。`,
    noUrlHint: "作業に戻ると、この通知は自動で消えます。このページの URL を読み取れないため、このページだけを許可することはできません。",
    allowUrls: "ブラウザの URL の読み取りを許可",
    why: "通知された理由",
    distracted: "脱線",
    focused: "集中",
    unknown: "—",
    thresholdHint: (n: number) => `${n} 回続けて脱線と判断されたときに通知します。戻らない場合は、1 分後、2 分後、5 分後にもう一度通知します。`,
  },
} satisfies Dict;

export default ja;
