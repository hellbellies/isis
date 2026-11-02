# isis

Tauri v2 desktop app · Vue 3 (Composition API, `<script setup>`) · TypeScript · Nuxt UI v4 (Tailwind CSS v4, Reka UI) · Pinia · pnpm

## Commands

- `pnpm tauri dev` — run the desktop app with HMR
- `pnpm tauri build` — production bundle
- `pnpm typecheck` — `vue-tsc --noEmit`
- `pnpm test:rust` — Rust unit tests
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`

## Architecture rules

### 1. Strict IPC boundary
- All native OS, filesystem, database and system-level work lives in Rust. The Vue frontend is presentation only.
- The frontend talks to Rust **only** through `call()` in `src/lib/ipc.ts` (a typed wrapper around `invoke()`) or Tauri events. Do not call `invoke()` directly from components or stores.
- Do not add Tauri JS plugins that bypass this (e.g. `@tauri-apps/plugin-fs`) — expose a Rust command instead.

### 2. State & data flow
- Frontend state lives in Pinia setup stores: `src/stores/` (global) or `src/features/<feature>/stores/`.
- `tauri::State` is only for native lifecycle resources (DB pools, background workers, handles). Not for app/UI state.
- Every Rust struct crossing IPC lives in `src-tauri/src/models/` and is mirrored by a TypeScript type in `src/types/`. Rust uses `#[serde(rename_all = "camelCase")]` so field names match TS. Never use `any`.
- Commands return `AppResult<T>` (`src-tauri/src/error.rs`); errors reach the frontend as `AppError { kind, message }`.

### 3. Layout
```
src/
  components/             app-specific shared components composed from Nuxt UI
  components.d.ts         generated types for auto-registered U* components (commit it)
  features/<feature>/     components/, stores/, composables/, index.ts (public API)
  lib/ipc.ts              typed command registry + call()
  types/                  TS mirrors of Rust models
src-tauri/src/
  commands/<feature>.rs   thin #[tauri::command] handlers
  services/<feature>.rs   domain logic, IPC-agnostic, unit-tested
  models/<feature>.rs     serde structs shared over IPC
  error.rs                AppError / AppResult
  lib.rs                  builder + generate_handler! registration
```
- Command handlers stay thin: resolve inputs (paths, state), call a service, return its result.
- Import from other features via their `index.ts`, not deep paths.

### 4. UI
- Nuxt UI (`@nuxt/ui`, plain-Vue mode via `@nuxt/ui/vite`) is the component library. Reach for a `U*` component (UButton, UCard, UModal, UTable, …) before writing markup by hand. Docs: https://ui.nuxt.com/docs/components
- `U*` components are auto-registered; `src/components.d.ts` is regenerated on `pnpm dev`/`pnpm build`. Composables are **not** auto-imported — import them explicitly (`@nuxt/ui/composables`, `@vueuse/core`).
- The root component is wrapped in `<UApp>` (needed for toasts, tooltips, overlays).
- Theme colors are set in `vite.config.ts` (`ui.colors`). Style with Tailwind utilities and Nuxt UI semantic classes (`text-muted`, `text-highlighted`, `bg-elevated`, …) rather than raw palette colors, so dark mode works automatically. Use the `ui` prop for per-instance overrides.
- Dark mode is driven by VueUse `useColorMode`, kept in sync with the Rust-persisted setting by `useThemeSync`. Don't toggle the `dark` class manually.
- Icons: Lucide (`i-lucide-*`), bundled locally from `@iconify-json/lucide` by scanning source files. Always write icon names as full string literals so the scanner finds them. The CSP blocks the Iconify API, so the app works offline.
- Put a component in `src/components/` only when it is shared across features; otherwise keep it in its feature folder.

## Adding a command (checklist)
1. Model in `src-tauri/src/models/<feature>.rs` (if new types).
2. Logic + tests in `src-tauri/src/services/<feature>.rs`.
3. Handler in `src-tauri/src/commands/<feature>.rs`; register in `lib.rs` `generate_handler!`.
4. Mirror types in `src/types/<feature>.ts`, re-export from `src/types/index.ts`.
5. Add the entry to the `Commands` interface in `src/lib/ipc.ts`.
6. Call it from a Pinia store action, never from a component directly.
