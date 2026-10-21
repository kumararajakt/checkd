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
- `src/types.ts` — shared data model (`List`, `Task`, `Repeat`, `MonthRepeat`).
- `src/settings.ts` — app settings (theme, close-to-tray), persisted in `localStorage` under `checkd.settings`.
- `src-tauri/` — Rust crate. Real entrypoint is `src-tauri/src/lib.rs` (`run()`); `main.rs` just calls `checkd_lib::run()`.
- `src-tauri/gen/` — generated Tauri schemas; never hand-edit.
- `src-tauri/capabilities/default.json` — Tauri v2 permission grants per window.
- `checkd.svg` (repo root) — the source app icon; `public/checkd.svg` is a copy used as the web favicon — keep both in sync.

## Tauri 2 gotchas

- The lib crate is intentionally named `checkd_lib` (avoids bin/lib name clash, mainly on Windows). Don't rename it; `main.rs` depends on it.
- New Rust commands are dead code until registered in `tauri::generate_handler![...]` in `src-tauri/src/lib.rs`.
- Frontend JS APIs are capability-gated: invoking a plugin command requires adding its permission in `src-tauri/capabilities/default.json` (e.g. `opener:default`, `dialog:default`).
- Vite runs on fixed port **1420** with `strictPort: true` — `tauri dev` fails if that port is busy. Vite ignores `src-tauri/` for file watching on purpose; Rust changes are picked up by the Tauri CLI instead.
- Rust commands in `src-tauri/src/lib.rs`: `set_close_to_tray(enabled)` (toggles hide-to-tray on window close) and `export_data(path, contents)` (writes a file, creating parent dirs). The tray icon is built in `setup()` and uses `tauri::include_image!("icons/32x32.png")`.
- `tauri` is built with the **`tray-icon`** feature (required for the tray); the dialog plugin (`tauri-plugin-dialog` + `@tauri-apps/plugin-dialog`) powers the export file picker.

## Styling & icons

- Tailwind CSS **v4** via the `@tailwindcss/vite` plugin in `vite.config.ts` — no `tailwind.config.js`, no PostCSS config; utilities come from `@import "tailwindcss"` in `src/index.css`.
- Dark mode is **class-based**, not media-query-based: `src/index.css` declares `@custom-variant dark (&:where(.dark, .dark *))` and `src/settings.ts` toggles the `dark` class on `<html>`. Always pair `dark:` variants with the light classes when adding new UI.
- After changing `checkd.svg`, regenerate platform icons with `bun run tauri icon checkd.svg` (overwrites `src-tauri/icons/`).

## Conventions

- `src-tauri/tauri.conf.json` hardcodes `bun run dev` / `bun run build` for Tauri's before-dev/before-build hooks — keep bun as the package manager.
