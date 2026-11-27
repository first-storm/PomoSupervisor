# Development

## Setup

You need macOS 12+, Rust 1.97.1+, Node 22.12+, pnpm and the Xcode Command Line Tools. With Nix the toolchain is in `flake.nix`: `nix develop` (or `direnv allow`) provides it, and the commands below can be run as `nix develop -c pnpm …`.

```sh
pnpm install
pnpm tauri dev        # develop
pnpm tauri build      # bundle the .app / .dmg
```

Under `pnpm tauri dev` the system permissions belong to the terminal that started it; the bundled .app is granted its own.

## Checks

```sh
pnpm check            # run this before committing: everything below in turn, and what CI runs
pnpm lint             # Biome: formatting and lint for the frontend
pnpm typecheck        # tsc
pnpm check:rust       # cargo fmt --check, clippy (warnings fail), cargo test; needs dist/ (pnpm build) first
pnpm fix              # auto-fix: biome check --write + cargo fmt
```

- The tools decide formatting: `src-tauri/rustfmt.toml` for Rust, `biome.jsonc` for the frontend (each lint rule that is switched off says why). `src/styles.css` keeps one rule per line and is not formatted.
- Names that are stored on disk or cross a process boundary must not change casually: the fields of `settings.json`, the SQLite tables and the values of `stage` / `result`, the keychain service name, and the Tauri command and event names. A test pins the `stage` / `result` values.

## How a debug build differs

- **API keys stay out of the keychain.** A debug build is signed afresh on every compile, and the keychain would ask for the login password each time. So keys are plain files under the system temp directory, in `pomosupervisor-dev/keys/` (readable only by you, separate from the installed app's, and possibly cleared on reboot). Set `POMO_KEYCHAIN=1` to make a debug build use the keychain. The logic is in `src-tauri/src/keys.rs`.
- **Settings and statistics are still shared with the installed app**, in `~/Library/Application Support/dev.aoba.pomosupervisor/`.
- `POMO_LANG=ja pnpm tauri dev` sets the language; `POMO_OPEN=main|review|settings` opens that window at launch.

## Previewing the UI without Tauri

Run `pnpm dev` and open in a browser:

- `http://localhost:1420/?state=start|focus|drift|away|back|returned|rest|done|early|late|break|breakover|nudge` for each state of the menu-bar popover; add `&ai=off|broken`, `&url=none` or `&site=blocked`
- `?view=review` and `?view=settings` (add `&pane=general|chain|lists|models|privacy`) for the two windows
- `&lang=ja` and the like set the language on any of them

The data comes from `src/lib/mock.ts`, which is only loaded in development. The screenshots in the README (`docs/screenshots/`) were taken from these previews, with the Chinese fixture text swapped for English.

## Code layout

Backend (`src-tauri/src/`):

- `app/` holds the app state and the actions on a round (`mod.rs`). Split out of it: `ticker.rs`, the 1-second loop; `chain.rs`, one sample's trip through the decision chain and what follows; `waiting.rs`, when an unanswered card is put away, chased and let go; `tray.rs`, the menu-bar icon, sounds and notifications.
- `commands.rs` has the Tauri commands for the frontend; `lib.rs` has startup, the tray menu and window events.
- `pipeline.rs` runs the decision chain. `rules.rs` is the rules, `llm.rs` the rig models and prompts, `decision.rs` the decision models (System One / OpenAI Decisions), `vision.rs` screenshots and pHash, `smoother.rs` the smoother, `timer.rs` the timer state machine, `store.rs` the SQLite statistics, `keys.rs` the API key store.
- `sampler.rs` reads the frontmost window and idle time; `ax.rs` reads window text through Accessibility.
- `windows.rs` places the menu-bar popover and hides it on blur, and opens the Focus Review and Settings windows.
- `i18n/` has the strings and prompts per language: one `Strings` table each, and a missing entry fails the build.

Frontend (`src/`):

- `popover/` is the menu-bar popover (before starting, focusing, paused or away, pomodoro done, break, break over, the reminder); `windows/` is Focus Review and Settings.
- `i18n/` has the dictionaries: `zh-Hans` is the source and the other languages `satisfies` its type.
- `lib/api.ts` has the types and calls shared with the backend.

Elsewhere:

- `src-tauri/l10n/` has the text of the system permission prompts.
- `src-tauri/examples/` has native helpers to run by hand: `cargo run --example smoke` (frontmost window, idle time, screenshot), `ax_dump` (what Accessibility can read) and `shot_window` (a screenshot of one of this app's windows).
- `icon/` has the icon sources (the Icon Composer project and the composed SVGs); see [icon/README.md](../icon/README.md).

## A Nix pitfall

The shell in `flake.nix` deliberately has no C toolchain (`mkShellNoCC`): the app has to link with the system clang against the installed macOS SDK. A `mkShell` with nixpkgs' own SDK links against an older one, and AppKit decides how the app looks and behaves from the SDK it was linked with.
