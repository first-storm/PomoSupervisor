import type { ReactNode } from "react";
import type { Dict } from "./zh-Hans";

const mins = (n: number) => (n === 1 ? "1 minute" : `${n} minutes`);

const enUS = {
  more: "More",
  quote: (s: string) => `“${s}”`,

  perm: {
    axTitle: "Accessibility access needed",
    axBody: "Lets PomoSupervisor read the text in windows, so the model can judge what a title alone can't, and fills in Firefox URLs. You can turn it off in Settings → Privacy.",
    grant: "Grant access",
    screenTitle: "Screen Recording access needed",
    screenBody: "Without it, other apps' window titles can't be read and screenshots can't be taken, so only app names are checked. Restart the app after granting access.",
    accessibility: "Accessibility",
    browserUrls: "Browser URLs",
  },

  sup: {
    ready: "AI supervision is ready. Windows not on either list are checked against the task above.",
    noTask: "No task yet. The AI doesn't know what you're working on, so its calls will be rough.",
    broken: "The model isn't ready, so this round only uses the block and allow lists.",
    off: "No model set up yet. Only the block and allow lists are used, and windows on neither list are ignored.",
    check: "Check",
    configure: "Set up models",
  },

  start: {
    titleFresh: "What's the new task?",
    title: "What are you working on?",
    task: "Task",
    taskPlaceholder: "e.g. Write unit tests for the sampler, covering idle detection and screenshot dedup",
    duration: "Duration",
    allowed: "Also allowed",
    remove: (label: string) => `Remove ${label}`,
    clickToRemove: "Click to remove",
    add: "Add",
    appOrDomain: "App or domain",
    allowedHint: "These apps and sites always count as focus. They carry over to the next round and clear when you choose “Change task”.",
    startFocus: "Start focusing",
    doneTodayEnter: (n: number) => `${n === 1 ? "1 pomodoro" : `${n} pomodoros`} done today · Press Return to start`,
    enter: "Press Return to start",
    breakFirst: (n: number) => `Take a ${n}-minute break first`,
    back: "Back",
  },

  judge: {
    firstLook: "Taking a first look…",
    off: "Looks off task",
    on: "On task",
    unsure: "Not sure, not counted",
    allowPageTitle: "Allows just this page. Other pages on this site are still checked.",
    allowTargetTitle: (target: string) => `No more reminders about ${target} this round`,
    pageCounts: "This page counts",
    counts: "This counts",
    why: "Why",
  },

  end: {
    ask: (elapsed: string) => `End this round? ${elapsed} so far.`,
    taskDone: "Task's done",
    countsHint: "Counts as a pomodoro. Your break starts as usual.",
    notHalfHint: "The time goes into the review and your break starts as usual. It's not past halfway, so it doesn't count as a pomodoro.",
    giveUp: "Give up this round",
    keepFocusing: "Keep focusing",
    end: "End…",
  },

  nthToday: (n: number) => `#${n} today`,
  doneToday: (n: number) => `${n} done today`,
  left: (time: string) => `${time} left`,

  focus: {
    pill: "Focusing",
    remainingOf: (min: number) => `left · ${min} min total`,
    currentTask: "Current task",
    noTask: "(No task)",
    pauseFor: (min: number) => `Pause for ${min} min`,
  },

  paused: {
    pillAway: "Paused · You're away",
    pillBack: "You're back",
    pill: "Paused",
    resumesIn: (time: string) => `Resumes in ${time}`,
    waiting: "Waiting for you",
    holding: "On hold",
    awayNote: (idle: string) => (
      <>
        No keyboard or mouse input for {idle}, so the timer stopped.
        <br />
        Time away doesn't count as distracted.
      </>
    ),
    backNote: (stopped: string) => (
      <>
        The timer stopped for {stopped} while you were away. It doesn't count as distracted.
        <br />
        Were you reading the whole time? Click “I was here” to count it back.
      </>
    ),
    timedNote: () => (
      <>
        Take a breather. The round picks up again when time's up.
        <br />
        What you do while paused isn't checked.
      </>
    ),
    heldNote: () => (
      <>
        The timer is stopped, but the round is still on.
        <br />
        What you do while paused isn't checked.
      </>
    ),
    awayFor: "Away for",
    pausedFor: "Paused for",
    autoResume: "Resume automatically when I'm back",
    stillHere: "I was here",
    restMore: (min: number) => `${min} more min`,
    resume: "Resume",
  },

  returned: {
    pill: "Welcome back",
    note: (stopped: string) => `The timer stopped for ${stopped} while you were away and is running again. Time away doesn't count as distracted.`,
    countItTitle: "Adds the stopped time back to this round, and waits longer before marking you away for the rest of it",
    countIt: "I was here, count it",
    ok: "OK",
  },

  rest: {
    longPill: "Long break",
    pill: "On break",
    breakOf: (min: number) => `break · ${min} min total`,
    note: () => (
      <>
        Get up, walk around, look at something far away.
        <br />
        You'll be called back when it's time. Nothing is checked during the break.
      </>
    ),
    review: "Focus Review",
    skip: "Skip break and start",
  },

  done: {
    early: "Done early",
    pill: "Pomodoro done",
    roundOver: "Round over",
    statFocus: (min: number) => `Focused ${mins(min)}`,
    statDistracted: (min: number) => `distracted ${mins(min)}`,
    statCalm: "barely distracted",
    statAlerts: (n: number) => (n === 1 ? "1 reminder" : `${n} reminders`),
    overtime: (over: string) => `${over} over, and your break hasn't started yet.`,
    takeBreak: (min: number) => `Take a ${min}-minute break`,
    again: "Skip break, go again",
    changeTask: "Change task",
    reviewRound: "Review this round",
  },

  breakOver: {
    pill: "Break over",
    continueTask: (quoted: string) => `Keep going with ${quoted}?`,
    next: "Start the next round?",
    same: (min: number) => `Same ${mins(min)}, same allow list.`,
    overtime: (over: string) => `Your break ran ${over} over.`,
    startFocus: "Start focusing",
    stop: "Not now",
  },

  nudge: {
    otherWindow: "another window",
    title: "Drifting off?",
    body: (where: ReactNode, task: string | null) => (
      <>
        You've been in <strong>{where}</strong> for a while, and it doesn't look related to {task ?? "your current task"}.
      </>
    ),
    backToTask: "Back to task",
    pagePart: "This page is part of the task",
    part: "This is part of the task",
    restFor: (min: number) => `Take ${min} min`,
    pageHint: (host: string) => `Switch back to work and this reminder goes away on its own. ${host} is on the block list, so only this page is allowed. Other pages there still get reminders.`,
    allowHint: (target: string) => `Switch back to work and this reminder goes away on its own. Choosing “This is part of the task” allows ${target} for this task, with no more reminders.`,
    noUrlHint: "Switch back to work and this reminder goes away on its own. This page's URL can't be read, so it can't be allowed on its own.",
    allowUrls: "Allow reading browser URLs",
    why: "Why am I seeing this?",
    distracted: "Distracted",
    focused: "Focused",
    unknown: "—",
    thresholdHint: (n: number) => `You're only reminded after ${n === 1 ? "1 check" : `${n} checks in a row`} ${n === 1 ? "finds" : "find"} you distracted. If you don't come back, you're reminded again after 1, 2 and 5 minutes.`,
  },
} satisfies Dict;

export default enUS;
