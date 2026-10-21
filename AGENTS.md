# AGENTS.md

Desktop app: **Tauri 2** (Rust backend) + **React 19** + **TypeScript**, built with Vite. Package manager is **bun** (`bun.lock`; no npm/yarn/pnpm lockfiles).

## Commands

- `bun run tauri dev` — run the full app (starts Vite, then opens the Tauri window).
- `bun run dev` — frontend only, no desktop window.
- `bun run tauri build` — release build/bundle (runs `tsc && vite build` first).
- `tsc` (via `bun run build`) is the **only** typecheck step. There is no linter, formatter, test runner, or CI config — don't hunt for one; add tests only if the task requires it.

## Layout

- `src/` — React frontend; entrypoint `src/main.tsx`. Styles in `src/index.css` (`@import "tailwindcss"`).
- `src/components/` — `Sidebar`, `NavItem` (one list row), `TaskList`, `TaskItem` (one task row), `ColorModal`, `RepeatModal`, `SettingsDialog`.
- `src/types.ts` — shared data model (`List`, `Task`, `Repeat`, `MonthRepeat`, `AppData`).
- `src/db.ts` — typed wrappers around the SQLite commands (`db.load()`, `db.createList()`, …).
- `src/settings.ts` — app settings (theme, close-to-tray), persisted in `localStorage` under `checkd.settings`.
- `src-tauri/` — Rust crate. Real entrypoint is `src-tauri/src/lib.rs` (`run()`); `main.rs` just calls `checkd_lib::run()`.
- `src-tauri/src/db.rs` — SQLite layer: schema, models (mirror `src/types.ts`), and all CRUD.
- `src-tauri/src/repeat.rs` — occurrence generator for repeat rules (pure functions over `chrono` dates).
- `src-tauri/src/notify.rs` — the reminder scheduler: a background thread that fires desktop notifications.
- `src-tauri/gen/` — generated Tauri schemas; never hand-edit.
- `src-tauri/capabilities/default.json` — Tauri v2 permission grants per window.
- `checkd.svg` (repo root) — the source app icon; `public/checkd.svg` is a copy used as the web favicon — keep both in sync.

## Data & persistence

- Lists and tasks live in SQLite (`rusqlite` with the `bundled` feature — no system SQLite needed). The DB file is `checkd.db` in the platform app-data dir (`~/.local/share/com.kumar.checkd/` on Linux).
- Schema (see `src-tauri/src/db.rs`): `lists(id, name, color, position)` and `tasks(id, list_id → lists ON DELETE CASCADE, title, description, due_date, due_time, repeat_json, position, completed, next_occurrence, occurrence_index, notified_key)`. `position` keeps the display order; the repeat rule is stored as JSON text.
- New columns are added by `ALTER TABLE` in `init_schema()` — old database files migrate in place when they are opened.
- The `Repeat` / `MonthRepeat` / `RepeatEnd` Rust types use `serde` tags that mirror the TS union shapes exactly (`{"type":"last-day"}`, `{"type":"weekday","weekday":1,"week":2|"last"}`). **Changing one side means changing the other.**
- The UI is optimistic: `App.tsx` updates React state first, then calls `src/db.ts`. If a write fails, `persist()` shows an error banner and reloads from the DB so the UI never drifts from disk.
- Data commands (all in `src-tauri/src/lib.rs`): `load_data`, `create_list`, `rename_list`, `delete_list`, `set_list_color`, `create_task`, `rename_task`, `set_task_due`, `set_task_description`, `set_task_repeat`, `set_task_completed`, `delete_task` — plus `set_close_to_tray` and `export_data`.
- `set_task_repeat` and `set_task_completed` return the updated `Task` — the database owns the repeat series (it anchors the rule to an occurrence and advances it), so the frontend takes the saved task back instead of guessing.
- The Rust crate has unit tests (`cargo test` in `src-tauri/`) — the JS side has no test runner. The IPC tests in `src-tauri/src/lib.rs` drive the commands through Tauri's mock runtime using the exact payloads `src/db.ts` sends.
- `cargo run --example dump` prints the live database as the frontend would receive it — handy for inspecting data without the UI.

## Reminders (notifications)

- `tauri-plugin-notification` (Rust + JS) sends the notifications; the JS side only asks for permission (macOS/Windows), Linux needs none. Permission: `notification:default` in `capabilities/default.json`.
- `src-tauri/src/notify.rs` runs a background thread for the whole life of the app — reminders keep firing while the window is hidden in the tray. It sleeps until the next due moment (capped at 60s so new tasks are picked up quickly).
- A task is due at `due_date` + `due_time`; tasks without a time count as due at **09:00**. A repeating task is due at its active occurrence (`next_occurrence`) plus the rule's time.
- `notified_key` stores the occurrence a reminder was already sent for (`due:<date>T<time>` or `occ:<date>`) — that is what keeps reminders from repeating.
- Reminders are **per session**: the scheduler clears `notified_key` when the app starts, so every launch mentions everything that is overdue again — however long ago it was due — and nothing repeats while the app keeps running. Overdue reminders read `Overdue · <list> · <date> · <time>`.
- Repeat series are *virtual*: one task row plus a pointer (`next_occurrence`, `occurrence_index`). Completing an occurrence advances the pointer; a missed occurrence is dropped once the following one is due, so the series keeps reminding. When the series runs out (`end: after N` or `end: on <date>`) the rule is cleared and the task stays completed.
- The scheduler emits the `data-changed` event when it moves a series on — the frontend listens and reloads (`src/App.tsx`).

## Tauri 2 gotchas

- The lib crate is intentionally named `checkd_lib` (avoids bin/lib name clash, mainly on Windows). Don't rename it; `main.rs` depends on it.
- New Rust commands are dead code until registered in `tauri::generate_handler![...]` in `src-tauri/src/lib.rs`.
- Frontend JS APIs are capability-gated: invoking a plugin command requires adding its permission in `src-tauri/capabilities/default.json` (e.g. `opener:default`, `dialog:default`, `notification:default`).
- Vite runs on fixed port **1420** with `strictPort: true` — `tauri dev` fails if that port is busy. Vite ignores `src-tauri/` for file watching on purpose; Rust changes are picked up by the Tauri CLI instead.
- Rust commands in `src-tauri/src/lib.rs`: `set_close_to_tray(enabled)` (toggles hide-to-tray on window close) and `export_data(path, contents)` (writes a file, creating parent dirs). The tray icon is built in `setup()` and uses `tauri::include_image!("icons/32x32.png")`.
- `tauri` is built with the **`tray-icon`** feature (required for the tray); the dialog plugin (`tauri-plugin-dialog` + `@tauri-apps/plugin-dialog`) powers the export file picker.

## Styling & icons

- Tailwind CSS **v4** via the `@tailwindcss/vite` plugin in `vite.config.ts` — no `tailwind.config.js`, no PostCSS config; utilities come from `@import "tailwindcss"` in `src/index.css`.
- Dark mode is **class-based**, not media-query-based: `src/index.css` declares `@custom-variant dark (&:where(.dark, .dark *))` and `src/settings.ts` toggles the `dark` class on `<html>`. Always pair `dark:` variants with the light classes when adding new UI.
- After changing `checkd.svg`, regenerate platform icons with `bun run tauri icon checkd.svg` (overwrites `src-tauri/icons/`).

## Conventions

- `src-tauri/tauri.conf.json` hardcodes `bun run dev` / `bun run build` for Tauri's before-dev/before-build hooks — keep bun as the package manager.
