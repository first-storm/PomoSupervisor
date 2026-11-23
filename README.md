# PomoSupervisor

带“分心监督”的番茄钟（Tauri 2 + React + rig-core）。专注期间每几秒采样一次前台窗口，按决策链判断你是否分心：

```
采样（App · 标题 · URL · 空闲）
 → 空闲超过阈值？ → 人已离开：暂停计时，不算分心
 → 规则匹配（应用 / 域名 / 标题关键词 黑白名单）→ 命中即定
 → 文本模型（标题 + URL + 任务描述）→ 有把握即定
 → 截图给视觉模型（pHash 去重后才发，默认关闭）
 → 平滑器：连续 N 次分心才提醒（系统通知 + 提示音 + 记入统计）
```

## 运行

```sh
pnpm install
pnpm tauri dev        # 开发
pnpm tauri build      # 打包 .app
cd src-tauri && cargo test
```

## 权限（macOS）

- **屏幕录制**：读取其他应用的窗口标题、截图。没有它只能按应用名判断。
- **自动化**：读取 Safari / Chrome / Arc / Edge / Brave 当前标签页的 URL（每个浏览器首次会单独弹窗）。Firefox 不支持，只按标题判断。
- `pnpm tauri dev` 时权限授予的是启动它的终端；打包后的 .app 需要单独授权。

## 模型

在“设置 → 模型”中配置，通过 rig-core 调用，不配置也能用（仅空闲 + 规则）：

| 服务商 | Base URL |
| --- | --- |
| OpenAI 兼容 | 留空为官方；其他服务需带 `/v1`，如 `https://api.deepseek.com/v1` |
| Anthropic | 留空为官方 |
| Ollama | 留空为 `http://localhost:11434` |

API Key 存在 macOS 钥匙串（服务名 `PomoSupervisor`）。设置与统计数据在 `~/Library/Application Support/dev.aoba.pomosupervisor/`。

## 代码结构

- `src-tauri/src/app.rs` — 状态、1 秒主循环、一次采样走完决策链
- `pipeline.rs` 决策链执行器 · `rules.rs` 规则 · `llm.rs` rig 模型与提示词 · `vision.rs` 截图与 pHash · `smoother.rs` 平滑器 · `timer.rs` 计时状态机 · `store.rs` SQLite 统计
- `src/` — React 前端（计时 / 统计 / 设置）
