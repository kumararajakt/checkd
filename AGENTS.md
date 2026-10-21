# AGENTS.md

Desktop app: **Tauri 2** (Rust backend) + **React 19** + **TypeScript**, built with Vite. Package manager is **bun** (`bun.lock`; no npm/yarn/pnpm lockfiles).

## Commands

- `bun run tauri dev` — run the full app (starts Vite, then opens the Tauri window).
- `bun run dev` — frontend only, no desktop window.
- `bun run tauri build` — release build/bundle (runs `tsc && vite build` first).
- `tsc` (via `bun run build`) is the **only** typecheck step. There is no linter, formatter, test runner, or CI config — don't hunt for one; add tests only if the task requires it.

## Layout

- `src/` — React frontend; entrypoint `src/main.tsx`. Styles in `src/index.css` (`@import "tailwindcss"`).
- `src-tauri/` — Rust crate. Real entrypoint is `src-tauri/src/lib.rs` (`run()`); `main.rs` just calls `checkd_lib::run()`.
- `src-tauri/gen/` — generated Tauri schemas; never hand-edit.
- `src-tauri/capabilities/default.json` — Tauri v2 permission grants per window.
- `checkd.svg` (repo root) — the source app icon; `public/checkd.svg` is a copy used as the web favicon — keep both in sync.

## Tauri 2 gotchas

- The lib crate is intentionally named `checkd_lib` (avoids bin/lib name clash, mainly on Windows). Don't rename it; `main.rs` depends on it.
- New Rust commands are dead code until registered in `tauri::generate_handler![...]` in `src-tauri/src/lib.rs` (the `invoke_handler` line was removed with the template `greet` command — re-add it when the first real command exists).
- Frontend JS APIs are capability-gated: invoking a plugin command requires adding its permission in `src-tauri/capabilities/default.json` (e.g. `opener:default`).
- Vite runs on fixed port **1420** with `strictPort: true` — `tauri dev` fails if that port is busy. Vite ignores `src-tauri/` for file watching on purpose; Rust changes are picked up by the Tauri CLI instead.

## Styling & icons

- Tailwind CSS **v4** via the `@tailwindcss/vite` plugin in `vite.config.ts` — no `tailwind.config.js`, no PostCSS config; utilities come from `@import "tailwindcss"` in `src/index.css`.
- After changing `checkd.svg`, regenerate platform icons with `bun run tauri icon checkd.svg` (overwrites `src-tauri/icons/`).

## Conventions

- `src-tauri/tauri.conf.json` hardcodes `bun run dev` / `bun run build` for Tauri's before-dev/before-build hooks — keep bun as the package manager.
