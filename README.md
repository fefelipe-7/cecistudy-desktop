# Campus

App desktop de organização acadêmica: aulas, entregas, provas e notas em uma
interface densa e rápida de operar.

## Stack

React 19 · Vite 8 · TypeScript · Tailwind CSS 4 · shadcn/ui · Tauri 2

## Desenvolvimento

Requisitos: Node >= 22, npm, Rust (toolchain `x86_64-pc-windows-msvc`) e Visual
Studio Build Tools com o workload "Desktop development with C++".

```sh
npm install
npm run dev         # front-end em http://localhost:1420
npm run tauri:dev   # app desktop
```

## Scripts

| Script                | O que faz                                        |
| --------------------- | ------------------------------------------------ |
| `npm run dev`         | Vite em http://localhost:1420 (só o front-end)   |
| `npm run build`       | typecheck + build de produção em `dist/`         |
| `npm run tauri:dev`   | app desktop em desenvolvimento                   |
| `npm run tauri:build` | instalável em `src-tauri/target/release/bundle/` |
| `npm run lint`        | ESLint                                           |
| `npm run typecheck`   | `tsc --noEmit`                                   |
| `npm run format`      | Prettier                                         |

## Estrutura

- `index.html` — shell do front-end
- `src/main.tsx` — bootstrap React
- `src/App.tsx` — dashboard único e seus modos de visualização
- `src/components/ui/` — componentes shadcn
- `src-tauri/` — shell Rust (janela, menu, permissões)
- `DESIGN.md` / `PRODUCT.md` — direção de design e de produto
