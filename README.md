# PomoSupervisor

带“分心监督”的番茄钟（Tauri 2 + React + rig-core）。专注期间每几秒采样一次前台窗口，按决策链判断你是否分心：

```
采样（App · 标题 · URL · 空闲）
 → 空闲超过阈值？ → 人已离开：暂停计时，不算分心（有视频 / 通话让屏幕保持常亮时不算离开，接着往下判）
 → 规则匹配（应用 / 域名 / 标题关键词 黑白名单）→ 命中即定
 → 文本模型（标题 + URL + 任务描述）→ 有把握即定
 → 截图给视觉模型（pHash 去重后才发，默认关闭）
 → 平滑器：连续 N 次分心才提醒（菜单栏弹出提醒 + 提示音 + 记入统计）；
   回到任务提醒自动收起，一直不回来则隔 1 / 2 / 5 分钟再提醒
```

计时之外的几条规矩：

- **结束之后不沉默**：番茄或休息到点后没人理，卡片在你面前待满 30 秒才自己收起（人不在就一直等着），菜单栏改成 `+mm:ss` 往上数；休息超时 2 分钟、5 分钟各再叫一次，番茄完成后 5 分钟提醒一次还欠着休息。人不在时到点的提醒，等你回来第一下操作时补上。都看过了还是 15 分钟没人理，才算了。
- **暂停都有时限**：暂停 = 一次短休息的长度，到点自动继续，可以「再歇 N 分钟」（最多攒到 30 分钟）。关掉「回来后自动继续」时，离开回来后这一轮会等你点继续：同样 2 分钟、5 分钟各催一次，15 分钟还不接就按放弃结束。
- **提前结束分两种**：「任务做完了」照常记时间、给休息（过半算一个番茄）；「放弃这一轮」才不算。
- **离开回来会说一声**：停了多久、现在是否已继续；其实一直在看资料就点「我没离开」，把时间算回来，这一轮的空闲阈值也放宽到 3 倍。
- **放行有范围**：「这算任务」放行的是这个任务里的 App / 网站；黑名单里的网站只放行当前页面。已放行的东西也会写进给模型的任务描述里。下一轮沿用，点「换个任务」时清空。
- 眼前的窗口被判为跑偏（还没到提醒）时，菜单栏时间后面先出现一个 ⚠，回到任务或换到别的窗口就消失。
- 空闲时有 App 让屏幕保持常亮（视频、通话）不算离开；`caffeinate`、Amphetamine 这类防休眠工具不算数。

## 运行

```sh
pnpm install
pnpm tauri dev        # 开发
pnpm tauri build      # 打包 .app
cd src-tauri && cargo test
```

## 权限（macOS）

- **屏幕录制**：读取其他应用的窗口标题、截图。没有它只能按应用名判断。
- **自动化**：读取 Safari / Chrome / Arc / Edge / Brave 当前标签页的 URL（每个浏览器首次会单独弹窗）。
- **辅助功能**（默认开启，可在设置 → 隐私关掉）：规则没命中时读取前台窗口的焦点控件和部分文字（最多约 800 字，不读密码框，不保存）交给模型；Firefox 等 AppleScript 拿不到网址的浏览器也从这里补上 URL。
- `pnpm tauri dev` 时权限授予的是启动它的终端；打包后的 .app 需要单独授权。

## 语言

界面、通知、菜单栏、判定理由和给模型的提示词都跟随系统语言（启动时读取，改了要重开）：简体中文、繁體中文、English (US)、English (Australia)、日本語，其他语言用英文。首次启动的默认黑白名单也按语言给（比如日文加上 nicovideo.jp、繁中加上 dcard.tw）。开发时可以用 `POMO_LANG=ja pnpm tauri dev` 指定语言，浏览器预览加 `&lang=ja`。

## 模型

在“设置 → 模型与密钥”中配置，通过 rig-core 调用，不配置也能用（仅空闲 + 规则）：

| 服务商 | Base URL |
| --- | --- |
| OpenAI 兼容 | 留空为官方；其他服务需带 `/v1`，如 `https://api.deepseek.com/v1` |
| Anthropic | 留空为官方 |
| Ollama | 留空为 `http://localhost:11434` |
| Decision 模型（System One 兼容） | 填**完整 endpoint**；留空为 TypeSafe `https://api.typesafe.ai/v1/systemone`，OpenRouter 为 `https://openrouter.ai/api/alpha/decisions`，AIMLAPI 为 `https://api.aimlapi.com/v1/decisions` |
| Decision 模型（OpenAI） | 留空为 `https://api.openai.com/v1/decisions`（模型 `gpt-6-luna`） |

Decision 模型不写文字，直接给出“专注 / 分心”两个选项的概率，延迟低、只按输入计费；把握低于“有把握阈值”时按没把握处理。用作视觉模型时需要支持图片输入：TypeSafe 的 jev-1.14、OpenRouter 上支持图片的 decision 模型（以 `image_url` 发送）、AIMLAPI 的 jev-1.14 / liquid/d1（以 `assets` 发送）、OpenAI gpt-6-luna；Telnyx、Eden AI 目前只支持文本。视觉槽位的“测试连接”会发一张小图，能提前发现不支持图片的配置。

API Key 存在 macOS 钥匙串（服务名 `PomoSupervisor`）。设置与统计数据在 `~/Library/Application Support/dev.aoba.pomosupervisor/`。

## 代码结构

- `src-tauri/src/app.rs` — 状态、1 秒主循环、一次采样走完决策链
- `src-tauri/src/i18n/` — 后端各语言的文字与提示词（每种语言一张 `Strings` 表，缺一条编译不过）；`src/i18n/` — 前端字典（`zh-Hans` 为源，其他语言 `satisfies` 它的类型）；`src-tauri/l10n/` — 权限弹窗文案
- `pipeline.rs` 决策链执行器 · `rules.rs` 规则 · `llm.rs` rig 模型与提示词 · `decision.rs` Decision 模型（System One / OpenAI Decisions） · `vision.rs` 截图与 pHash · `smoother.rs` 平滑器 · `timer.rs` 计时状态机 · `store.rs` SQLite 统计
- `windows.rs` — 菜单栏弹窗的定位、失焦隐藏，以及“专注回顾”“设置”两个窗口
- `src/` — React 前端：`popover/` 菜单栏弹窗（开始前 · 专注中 · 暂停/离开 · 番茄完成 · 休息 · 休息结束 · 分心提醒），`windows/` 专注回顾与设置
- 不开 Tauri 预览界面：`pnpm dev` 后打开 `http://localhost:1420/?state=start|focus|drift|away|back|returned|rest|done|early|late|break|breakover|nudge`（可加 `&ai=off|broken`、`&url=none`、`&site=blocked`）、`?view=review`、`?view=settings`（数据来自 `src/lib/mock.ts`，只在开发模式加载）
- `icon/` — 图标源文件（Icon Composer 分层 + 合成后的 SVG）
