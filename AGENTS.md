# Campus

## Project architecture

- Keep the study workspace as a single stateful dashboard route; its main views are modes of one workflow, not separate destinations.

## Stack and tooling

- Front-end is a Vite SPA (`index.html` + `src/main.tsx`). There is no server, no SSR and no router — the view modes in `src/App.tsx` are `useState`, not routes.
- Tauri v2 owns the desktop shell in `src-tauri/`. The dev server runs on `http://localhost:1420` (`server.port` in `vite.config.ts` must stay in sync with `build.devUrl` in `src-tauri/tauri.conf.json`).
- npm is the package manager. Do not use bun, yarn or pnpm — `package-lock.json` is the lockfile of record.
- `src/components/ui/` holds a curated shadcn set. Add new components with `npx shadcn@latest add <name>` rather than hand-copying from the old template.
- Features live in `src/features/calendar/` (one folder per feature), split into `domain/`, `data/`, `ui/` and `sync/` (external service mapping). `domain/` never imports from `ui/` or `data/`.
- **Rust is the backend, and the domain truth lives there.** `src-tauri/` implements the domain rules — state machine, recurrence expansion, derived overlap/overdue, invariants — on top of SQLite, Tauri commands and OS integration. TypeScript is the bridge between the UI and that backend: it holds the types and the pure functions the UI needs to render, and it never re-derives a rule the backend already owns.
- A rule that must hold may be enforced in exactly one place, and that place is Rust. The TS mirror exists for rendering and for tests that must run without the Rust toolchain; when the two disagree, Rust is right and the TS mirror is the bug.
- `data/` talks to the backend through `src/lib/ipc.ts` and knows the schema, not the SQL. It may not re-implement persistence, recurrence or any other domain rule.
- `domain/` in TypeScript stays pure — types, state machine, recurrence, derived rules; no React, no `invoke`, no direct `Date`, with an injected `Clock` — because that is what makes those rules testable on every machine, including the ones where the Rust toolchain does not link.

- All Tauri IPC goes through `src/lib/ipc.ts`. Components never call `invoke` directly.

## Domain invariants

- The Calendar answers three separate questions and never mixes them: what is scheduled (events and occurrences), what must be done (responsibilities, steps and planning blocks), and what actually happened (execution records). A planning block never creates a new obligation; it only reserves time for an existing one.
- The Calendar must never silently turn a `recomendado` or `opcional` item into an obligation, and must never auto-adjust a recurring item's timezone when the system timezone changes.
- Conflict and overlap are derived and only signalled, never blocking.

## Commands

- `npm run dev` — front-end only
- `npm run build` — typecheck + production build
- `npm run tauri:dev` / `npm run tauri:build` — desktop app
- `npm run lint`, `npm run typecheck`, `npm run format`
