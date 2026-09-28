# Campus

## Project architecture

- Keep the study workspace as a single stateful dashboard route; its main views are modes of one workflow, not separate destinations.

## Stack and tooling

- Front-end is a Vite SPA (`index.html` + `src/main.tsx`). There is no server, no SSR and no router — the view modes in `src/App.tsx` are `useState`, not routes.
- Tauri v2 owns the desktop shell in `src-tauri/`. The dev server runs on `http://localhost:1420` (`server.port` in `vite.config.ts` must stay in sync with `build.devUrl` in `src-tauri/tauri.conf.json`).
- npm is the package manager. Do not use bun, yarn or pnpm — `package-lock.json` is the lockfile of record.
- `src/components/ui/` holds a curated shadcn set. Add new components with `npx shadcn@latest add <name>` rather than hand-copying from the old template.

## Commands

- `npm run dev` — front-end only
- `npm run build` — typecheck + production build
- `npm run tauri:dev` / `npm run tauri:build` — desktop app
- `npm run lint`, `npm run typecheck`, `npm run format`
