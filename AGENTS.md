# PomoSupervisor

A macOS menu-bar pomodoro timer (Tauri 2, Rust backend, React frontend). During a focus round it samples the frontmost window and reminds the user when they drift off the task. `README.md` describes the product; `docs/development.md` has setup, the file-by-file code layout and the debug-build notes.

## Commands

The toolchain comes from `flake.nix`. Run commands as `nix develop -c <cmd>` (or `direnv exec . <cmd>`).

```sh
pnpm install
pnpm check                # everything CI runs: lint, typecheck, build, then check:rust
pnpm lint                 # Biome: format + lint for the frontend
pnpm typecheck            # tsc
pnpm check:rust           # cargo fmt --check, clippy -D warnings, cargo test
pnpm fix                  # biome check --write + cargo fmt
pnpm dev                  # frontend only, in a browser, on mock data
pnpm tauri dev            # the real app (read "Verifying a change" first)
```

- A change is done when `pnpm check` passes.
- Anything that compiles the Rust crate needs `dist/` to exist (`pnpm build`): `tauri::generate_context!` embeds it at compile time.
- One Rust test: `cd src-tauri && cargo test <module>::tests::<name>`, e.g. `cargo test smoother::` or `cargo test app::waiting`.
- There are no frontend tests. Rust tests run with the language unset, which means Simplified Chinese strings (`i18n::lang`).

## Verifying a change

Pick the cheapest way that actually exercises the change:

- **Logic** (timer, smoother, rules, waiting, parsing): a Rust unit test next to the code.
- **UI**: the browser previews. `pnpm dev`, then `http://localhost:1420/` with
  - `?state=start|focus|drift|away|back|returned|rest|done|early|late|break|breakover|nudge` for the popover (add `&ai=off|broken`, `&url=none`, `&site=blocked`),
  - `?view=review`, or `?view=settings&pane=general|chain|lists|models|privacy` for the windows,
  - `&lang=` on any of them.

  The data is `src/lib/mock.ts`, which `src/lib/api.ts` swaps in outside Tauri (dev only). A new screen or state needs a fixture there.
- **Native calls** (frontmost window, idle time, Accessibility, screenshots): the helpers in `src-tauri/examples/`, e.g. `cargo run --example smoke`, `ax_dump`, `shot_window`. They need the terminal to hold the matching macOS permissions.
- **The real app** (`pnpm tauri dev`): only when the above can't show it, and ask the user first. It is a menu-bar app that appears on their screen, a copy is usually already running from `/Applications`, and a debug build shares that copy's settings and statistics (`~/Library/Application Support/dev.aoba.pomosupervisor/`). `POMO_LANG=ja` sets the language; `POMO_OPEN=main|review|settings` opens that window at launch. Don't use `POMO_OPEN=nudge`: it starts a round and writes a session to the real database.

A debug build keeps API keys as files under the temp directory (`pomosupervisor-dev/keys/`) instead of the keychain, because every rebuild is signed afresh and the keychain would ask for the login password each time; `POMO_KEYCHAIN=1` opts back in (`src-tauri/src/keys.rs`).

## Architecture

**One bundle, three windows.** `src/main.tsx` picks the root component from the window label: the menu-bar popover (`main`), Focus Review, or Settings. The frontend holds almost no state: Rust owns the timer and pushes a full `TimerView` on `timer://tick` every second, plus `supervisor://verdict` per sample. `src/lib/api.ts` mirrors the Rust types by hand, so change both sides together (and `mock.ts` with them).

**Backend state** is one `AppState` (`src-tauri/src/app/mod.rs`): `core: Mutex<Core>` (timer, smoother, caches, which card is on screen), settings, the SQLite store, and the built models. The methods on `AppState` are the actions on a round (start, pause, finish early, "I was here"...); `commands.rs` is a thin Tauri layer over them that emits a tick afterwards.

**The loop** (`app/ticker.rs`) runs once a second: it ticks the timer, chases cards nobody answered (`app/waiting.rs` decides when a card hides, is reminded, or is given up on, counting only time the person was present), and, when a sample is due and none is in flight, spawns one.

**One sample** (`app/chain.rs`) goes through the decision chain built by `build_stages` and run by `pipeline.rs`, which stops at the first stage that concludes:

1. Idle check. Away pauses the timer and is not a distraction; a display held awake by video or a call is not away.
2. This round's allow list, then the block/allow rules (`rules.rs`).
3. Text model, with a per-task LRU cache; only a confident answer concludes (`llm.rs`, or `decision.rs` for decision-model endpoints).
4. Vision model on a screenshot; when the pHash is close to the last screenshot's, the last answer is reused instead of calling the model (`vision.rs`).

`run_sample` then feeds the outcome to the `Smoother` (N distracted samples in a row before a reminder, with back-off on repeats), logs it to the store, and decides which popover to show or hide. Accessibility text (`ax.rs`) is read lazily, at most once per sample, only when a model needs it.

## Strings

User-facing strings live in one of three places, and a new string needs every language of the place it lives in. The language follows the system and is settled once at launch.

- `src-tauri/src/i18n/`: one `Strings` table per language (zh-Hans, zh-Hant, en-US, en-AU, ja), model prompts included; a missing entry fails the build.
- `src/i18n/`: the same five. `zh-Hans` is the source and the others `satisfies` its type; `en-AU` spreads `en-US` and holds only what differs.
- `src-tauri/l10n/`: the macOS permission prompts (en, ja, zh-Hans, zh-Hant).

## Constraints

- **Installed copies have data, so stored and wire names stay as they are.** That covers `settings.json` fields, SQLite tables and columns, the `stage` / `result` string values (pinned by `app::tests::stage_and_result_names_are_pinned`), the keychain service `PomoSupervisor`, the bundle identifier, and Tauri command and event names. Adding is fine: a new settings field with a default, a new command or event. Renaming or removing one, or changing an existing table (the schema is `CREATE TABLE IF NOT EXISTS`, with no migrations yet), needs a migration for existing data: raise it with the user before doing it.
- **Formatting is the tools' call**: run `pnpm fix` rather than formatting by hand. Rust is written wide (`src-tauri/rustfmt.toml`: 140 columns, Max heuristics); the frontend is Biome at 160. `src/styles.css` keeps one rule per line and is deliberately not formatted. Each lint rule switched off in `biome.jsonc` carries its reason; do the same for a new one.
- **Keep `mkShellNoCC` in `flake.nix`.** The app must link with the system clang against the installed macOS SDK; nixpkgs' own SDK is older, and AppKit picks appearance and behaviour from the SDK it was linked with.
- macOS only: the code calls Accessibility, CoreGraphics and AppKit directly.
- The default branch is `master`.
