import type { ReactNode } from "react";
import type { Dict } from "./zh-Hans";

const zhHant = {
  more: "更多",
  quote: (s: string) => `「${s}」`,

  perm: {
    axTitle: "需要「輔助使用」權限",
    axBody: "用來讀取視窗裡的文字，幫模型判斷標題看不出來的情況，也能補上 Firefox 的網址。不想用可以在設定 → 隱私權裡關掉。",
    grant: "去授權",
    screenTitle: "需要「螢幕錄製」權限",
    screenBody: "沒有它讀不到其他 App 的視窗標題，也無法截圖，只能按 App 名稱判斷。授權後需重新啟動程式。",
    accessibility: "輔助使用",
    browserUrls: "瀏覽器網址",
  },

  sup: {
    ready: "AI 監督已就緒：名單裡沒有的視窗，會對照上面的任務來判斷。",
    noTask: "還沒寫任務。AI 不知道你要做什麼，判斷會很粗略。",
    broken: "模型沒有就緒，這一輪只按黑白名單判斷。",
    off: "還沒設定模型，只按黑白名單判斷，名單裡沒有的視窗不會管。",
    check: "去檢查",
    configure: "設定模型",
  },

  start: {
    titleFresh: "換成什麼任務？",
    title: "這一輪做什麼？",
    task: "任務",
    taskPlaceholder: "例如：幫取樣模組寫單元測試，涵蓋閒置偵測和截圖去重",
    duration: "時長",
    allowed: "額外允許",
    remove: (label: string) => `移除 ${label}`,
    clickToRemove: "按一下移除",
    add: "新增",
    appOrDomain: "App 名稱或網域",
    allowedHint: "這些 App 和網站直接算專注。下一輪沿用；按「換個任務」時清空。",
    startFocus: "開始專注",
    doneTodayEnter: (n: number) => `今天已完成 ${n} 個番茄 · 按 Return 直接開始`,
    enter: "按 Return 直接開始",
    breakFirst: (n: number) => `先休息 ${n} 分鐘`,
    back: "返回",
  },

  judge: {
    firstLook: "正在看第一眼…",
    off: "看起來跑偏了",
    on: "在任務上",
    unsure: "拿不準，不計入",
    allowPageTitle: "只放行這個頁面，這個網站的其他頁面照舊",
    allowTargetTitle: (target: string) => `這一輪不再提醒 ${target}`,
    pageCounts: "這頁算任務",
    counts: "這算任務",
    why: "判斷依據",
  },

  end: {
    ask: (elapsed: string) => `這一輪到這裡？已經過了 ${elapsed}。`,
    taskDone: "任務做完了",
    countsHint: "記為一個番茄，照常休息。",
    notHalfHint: "時間記進回顧，照常休息；還沒過半，不算一個番茄。",
    giveUp: "放棄這一輪",
    keepFocusing: "繼續專注",
    end: "結束…",
  },

  nthToday: (n: number) => `今天第 ${n} 個`,
  doneToday: (n: number) => `今天已完成 ${n} 個`,
  left: (time: string) => `還剩 ${time}`,

  focus: {
    pill: "專注中",
    remainingOf: (min: number) => `剩餘 · 共 ${min} 分鐘`,
    currentTask: "目前任務",
    noTask: "（沒寫任務）",
    pauseFor: (min: number) => `暫停 ${min} 分鐘`,
  },

  paused: {
    pillAway: "已暫停 · 你離開了",
    pillBack: "你回來了",
    pill: "已暫停",
    resumesIn: (time: string) => `${time} 後自動繼續`,
    waiting: "等你繼續",
    holding: "停在這裡",
    awayNote: (idle: string) => (
      <>
        鍵盤和滑鼠 {idle}沒有操作，計時自動停了。
        <br />
        離開的時間不算分心。
      </>
    ),
    backNote: (stopped: string) => (
      <>
        離開期間計時停了 {stopped}，不算分心。
        <br />
        其實一直在看資料？按「我沒離開」把這段時間算回來。
      </>
    ),
    timedNote: () => (
      <>
        先歇一下，時間到自動接著這一輪。
        <br />
        暫停期間不看你在做什麼。
      </>
    ),
    heldNote: () => (
      <>
        計時停著，這一輪還在。
        <br />
        暫停期間不看你在做什麼。
      </>
    ),
    awayFor: "已離開",
    pausedFor: "已暫停",
    autoResume: "回來後自動繼續",
    stillHere: "我沒離開",
    restMore: (min: number) => `再歇 ${min} 分鐘`,
    resume: "繼續專注",
  },

  returned: {
    pill: "歡迎回來",
    note: (stopped: string) => `離開期間計時停了 ${stopped}，現在繼續走。離開的時間不算分心。`,
    countItTitle: "把停掉的時間算回這一輪，這一輪裡也會更晚才判定你離開",
    countIt: "我沒離開，算進去",
    ok: "好",
  },

  rest: {
    longPill: "長休息",
    pill: "休息中",
    breakOf: (min: number) => `休息 · 共 ${min} 分鐘`,
    note: () => (
      <>
        站起來走走，看看遠處。
        <br />
        時間到會叫你，休息期間不看你在做什麼。
      </>
    ),
    review: "專注回顧",
    skip: "不休息了，開始",
  },

  done: {
    early: "提前做完",
    pill: "番茄完成",
    roundOver: "這一輪結束了",
    statFocus: (min: number) => `專注 ${min} 分鐘`,
    statDistracted: (min: number) => `分心 ${min} 分鐘`,
    statCalm: "沒怎麼分心",
    statAlerts: (n: number) => `提醒 ${n} 次`,
    overtime: (over: string) => `已經超出 ${over}，這次休息還沒開始。`,
    takeBreak: (min: number) => `休息 ${min} 分鐘`,
    again: "不休息，再來一輪",
    changeTask: "換個任務",
    reviewRound: "看這一輪的回顧",
  },

  breakOver: {
    pill: "休息結束",
    continueTask: (quoted: string) => `接著做${quoted}？`,
    next: "開始下一輪？",
    same: (min: number) => `還是 ${min} 分鐘，允許清單照舊。`,
    overtime: (over: string) => `休息已經超出 ${over}。`,
    startFocus: "開始專注",
    stop: "先不做了",
  },

  nudge: {
    otherWindow: "別的視窗",
    title: "分心了嗎？",
    body: (where: ReactNode, task: string | null) => (
      <>
        你在 <strong>{where}</strong> 待了一會兒，看起來和{task ?? "目前的任務"}沒有關係。
      </>
    ),
    backToTask: "回到任務",
    pagePart: "這個頁面算任務",
    part: "這算任務的一部分",
    restFor: (min: number) => `先歇 ${min} 分鐘`,
    pageHint: (host: string) => `切回去工作，這則提醒會自己消失。${host} 在黑名單裡，所以只放行目前這個頁面，其他頁面照舊提醒。`,
    allowHint: (target: string) => `切回去工作，這則提醒會自己消失。選「這算任務的一部分」會在這個任務裡放行 ${target}，之後不再提醒。`,
    noUrlHint: "切回去工作，這則提醒會自己消失。讀不到這個網頁的網址，沒辦法只放行它。",
    allowUrls: "允許讀取瀏覽器網址",
    why: "為什麼提醒我",
    distracted: "分心",
    focused: "專注",
    unknown: "—",
    thresholdHint: (n: number) => `連續 ${n} 次都判定為分心才會提醒。還不回來的話，隔 1 分鐘、2 分鐘、5 分鐘再提醒一次。`,
  },
} satisfies Dict;

export default zhHant;
