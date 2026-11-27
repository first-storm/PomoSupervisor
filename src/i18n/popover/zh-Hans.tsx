import type { ReactNode } from "react";

const zhHans = {
  more: "更多",
  quote: (s: string) => `「${s}」`,

  perm: {
    axTitle: "需要“辅助功能”权限",
    axBody: "用来读取窗口里的文字，帮模型判断标题看不出来的情况，也能补上 Firefox 的网址。不想用可以在设置 → 隐私里关掉。",
    grant: "去授权",
    screenTitle: "需要“屏幕录制”权限",
    screenBody: "没有它读不到其他 App 的窗口标题，也无法截图，只能按 App 名判断。授权后需重启应用。",
    accessibility: "辅助功能",
    browserUrls: "浏览器网址",
  },

  sup: {
    ready: "AI 监督已就绪：名单里没有的窗口，会对照上面的任务来判断。",
    noTask: "还没写任务。AI 不知道你要做什么，判断会很粗。",
    broken: "模型没有就绪，这一轮只按黑白名单判断。",
    off: "还没配置模型，只按黑白名单判断，名单里没有的窗口不会管。",
    check: "去检查",
    configure: "配置模型",
  },

  start: {
    titleFresh: "换成什么任务？",
    title: "这一轮做什么？",
    task: "任务",
    taskPlaceholder: "比如：给采样模块写单元测试，覆盖空闲检测和截图去重",
    duration: "时长",
    allowed: "额外允许",
    remove: (label: string) => `移除 ${label}`,
    clickToRemove: "点击移除",
    add: "添加",
    appOrDomain: "App 名或域名",
    allowedHint: "这些 App 和网站直接算专注。下一轮沿用；点「换个任务」时清空。",
    startFocus: "开始专注",
    doneTodayEnter: (n: number) => `今天已完成 ${n} 个番茄 · 回车直接开始`,
    enter: "回车直接开始",
    breakFirst: (n: number) => `先休息 ${n} 分钟`,
    back: "返回",
  },

  judge: {
    firstLook: "正在看第一眼…",
    off: "看起来跑偏了",
    on: "在任务上",
    unsure: "拿不准，不计入",
    allowPageTitle: "只放行这个页面，这个网站的其他页面照旧",
    allowTargetTitle: (target: string) => `这一轮不再提醒 ${target}`,
    pageCounts: "这页算任务",
    counts: "这算任务",
    why: "判断依据",
  },

  end: {
    ask: (elapsed: string) => `这一轮到这里？已经过去 ${elapsed}。`,
    taskDone: "任务做完了",
    countsHint: "记为一个番茄，照常休息。",
    notHalfHint: "时间记进回顾，照常休息；还没过半，不算一个番茄。",
    giveUp: "放弃这一轮",
    keepFocusing: "接着专注",
    end: "结束…",
  },

  /** The n-th pomodoro of the day, counting the one under way. */
  nthToday: (n: number) => `今天第 ${n} 个`,
  doneToday: (n: number) => `今天已完成 ${n} 个`,
  left: (time: string) => `还剩 ${time}`,

  focus: {
    pill: "专注中",
    remainingOf: (min: number) => `剩余 · 共 ${min} 分钟`,
    currentTask: "当前任务",
    noTask: "（没写任务）",
    pauseFor: (min: number) => `暂停 ${min} 分钟`,
  },

  paused: {
    pillAway: "已暂停 · 你离开了",
    pillBack: "你回来了",
    pill: "已暂停",
    resumesIn: (time: string) => `${time} 后自动继续`,
    waiting: "等你继续",
    holding: "停在这里",
    awayNote: (idle: string) => (
      <>
        键鼠 {idle}没有操作，计时自动停了。
        <br />
        离开的时间不算分心。
      </>
    ),
    backNote: (stopped: string) => (
      <>
        离开期间计时停了 {stopped}，不算分心。
        <br />
        其实一直在看资料？点「我没离开」把这段时间算回来。
      </>
    ),
    timedNote: () => (
      <>
        先歇一下，到点自动接着这一轮。
        <br />
        暂停期间不看你在做什么。
      </>
    ),
    heldNote: () => (
      <>
        计时停着，这一轮还在。
        <br />
        暂停期间不看你在做什么。
      </>
    ),
    awayFor: "已离开",
    pausedFor: "已暂停",
    autoResume: "回来后自动继续",
    stillHere: "我没离开",
    restMore: (min: number) => `再歇 ${min} 分钟`,
    resume: "继续专注",
  },

  returned: {
    pill: "欢迎回来",
    note: (stopped: string) => `离开期间计时停了 ${stopped}，现在接着走。离开的时间不算分心。`,
    countItTitle: "把停掉的时间算回这一轮，这一轮里也更晚才判你离开",
    countIt: "我没离开，算上",
    ok: "好",
  },

  rest: {
    longPill: "长休息",
    pill: "休息中",
    breakOf: (min: number) => `休息 · 共 ${min} 分钟`,
    note: () => (
      <>
        站起来走走，看看远处。
        <br />
        到点会叫你，休息期间不看你在做什么。
      </>
    ),
    review: "专注回顾",
    skip: "不休息了，开始",
  },

  done: {
    early: "提前做完",
    pill: "番茄完成",
    roundOver: "这一轮结束了",
    statFocus: (min: number) => `专注 ${min} 分钟`,
    statDistracted: (min: number) => `分心 ${min} 分钟`,
    statCalm: "没怎么分心",
    statAlerts: (n: number) => `提醒 ${n} 次`,
    overtime: (over: string) => `已经超出 ${over}，这次休息还没开始。`,
    takeBreak: (min: number) => `休息 ${min} 分钟`,
    again: "不休息，再来一轮",
    changeTask: "换个任务",
    reviewRound: "看这一轮的回顾",
  },

  breakOver: {
    pill: "休息结束",
    continueTask: (quoted: string) => `接着做${quoted}？`,
    next: "开始下一轮？",
    same: (min: number) => `还是 ${min} 分钟，允许列表照旧。`,
    overtime: (over: string) => `休息已经超出 ${over}。`,
    startFocus: "开始专注",
    stop: "先不做了",
  },

  nudge: {
    otherWindow: "别的窗口",
    title: "走神了吗？",
    /** `task` is already quoted; null when no task was written. */
    body: (where: ReactNode, task: string | null) => (
      <>
        你在 <strong>{where}</strong> 待了一会儿，看起来和{task ?? "当前任务"}没有关系。
      </>
    ),
    backToTask: "回到任务",
    pagePart: "这个页面算任务",
    part: "这算任务的一部分",
    restFor: (min: number) => `先歇 ${min} 分钟`,
    pageHint: (host: string) => `切回去干活，这条提醒会自己消失。${host} 在黑名单里，所以只放行当前这个页面，其他页面照旧提醒。`,
    allowHint: (target: string) => `切回去干活，这条提醒会自己消失。选「这算任务的一部分」会在这个任务里放行 ${target}，之后不再提醒。`,
    noUrlHint: "切回去干活，这条提醒会自己消失。读不到这个网页的网址，没法只放行它。",
    allowUrls: "允许读取浏览器网址",
    why: "为什么提醒我",
    distracted: "分心",
    focused: "专注",
    unknown: "—",
    thresholdHint: (n: number) => `连续 ${n} 次都判为分心才会提醒。还不回来的话，隔 1 分钟、2 分钟、5 分钟再提醒一次。`,
  },
};

export type Dict = typeof zhHans;
export default zhHans;
