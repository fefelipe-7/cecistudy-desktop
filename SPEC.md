# Spec — Migração para React + Vite + npm + Tauri

Data: 2026-09-28
Escopo: remover rastros do Lovable, trocar Bun por npm, substituir TanStack Start/Router/Query por React + Vite (SPA), podar código morto e envelopar como app desktop com Tauri v2.

## Status (2026-09-28)

Fases 0 a 6 implementadas e commitadas. `typecheck`, `lint` e `build` verdes; front-end validado no Chrome (dev e produção).

**Pendente:** a compilação Rust está bloqueada nesta máquina por falta de um toolchain C capaz de **linkar**. Diagnóstico completo, verificado em 2026-09-29:

1. `rustup default` é `stable-x86_64-pc-windows-msvc`; não há MSVC Build Tools, então `link.exe not found`.
2. `cargo check` **também** falha, e não apenas `cargo build`. Build scripts e crates de proc-macro são compilados **e linkados para o host**, então o linker do host é obrigatório mesmo em `check`.
3. Instalar só o _target_ `x86_64-pc-windows-gnu` não resolve, porque os build scripts continuam sendo linkados no host `msvc`.
4. Instalar o toolchain **host** `stable-x86_64-pc-windows-gnu` resolve o item 3, mas esbarra em outro problema: o único compilador C presente é o **llvm-mingw** (em `~/AppData/Local/llvm-mingw-*`), que usa `libunwind` e **não** fornece `libgcc`/`libgcc_eh`, exigidos pelo `rustc` do alvo `windows-gnu` (`lld: error: unable to find library -lgcc_eh`).
5. Piso verificado: um crate sem dependências, sem proc-macro e sem build script **passa** em `cargo check` sem nenhum linker. A barreira é, portanto, o conjunto de crates com build script — que inclui `libsqlite3-sys` (usado por `rusqlite`) e todo o `serde` derive.

**Como destravar** (qualquer uma das duas):

- instalar o _Visual Studio Build Tools 2022_ com o workload "Desktop development with C++" — recomendado, porque é o alvo que o Tauri suporta melhor e mantém `rustup default` em `msvc`; ou
- instalar um **mingw-w64 completo** (MSYS2 GCC ou WinLibs) e passar a usar o toolchain `stable-x86_64-pc-windows-gnu` como host — o llvm-mingw não serve, por não trazer `libgcc`.

Enquanto isso, o backend Rust da spec 01 foi escrito e revisado, e o schema foi **validado contra um engine SQLite real** pelo `node:sqlite` do Node 26 — ver `docs/calendario/01-dominio-persistencia.md` §4.

---

## 1. Diagnóstico do estado atual

### 1.1 Stack existente

| Camada       | Hoje                                                                        |
| ------------ | --------------------------------------------------------------------------- |
| Runtime / PM | Bun (`bun.lock` 140 KB, `bunfig.toml`)                                      |
| Framework    | TanStack Start 1.168 (SSR, Nitro 3, server entry)                           |
| Roteador     | TanStack Router 1.170 (file-based) + `@tanstack/router-plugin`              |
| Data         | TanStack Query 5.101                                                        |
| Build        | Vite 8 + `@lovable.dev/vite-tanstack-config` 2.24 (wrapper com tudo acima)  |
| UI           | Tailwind 4 + shadcn/ui "new-york" (49 arquivos em `src/components/ui/`)     |
| Rotas        | 1 rota real (`src/routes/index.tsx`, 570 linhas) + `__root.tsx` (shell SSR) |

### 1.2 Achados — rastros do Lovable (grep `lovable`, case-insensitive)

| Arquivo                              | Ocorrência                                                                | Ação              |
| ------------------------------------ | ------------------------------------------------------------------------- | ----------------- |
| `.lovable/project.json`              | template `tanstack_start_ts_current`                                      | deletar diretório |
| `package.json:73`                    | `@lovable.dev/vite-tanstack-config`                                       | remover dep       |
| `bunfig.toml:7`                      | `minimumReleaseAgeExcludes` com 4 pacotes `@lovable.dev/*`                | deletar arquivo   |
| `vite.config.ts:1-7`                 | import + comentário de 6 linhas explicando os plugins do Lovable          | reescrever        |
| `AGENTS.md:1-10`                     | bloco `<!-- LOVABLE:BEGIN -->`                                            | remover bloco     |
| `README.md:1-11`                     | "Welcome to your Lovable project" / "Build with Lovable"                  | reescrever        |
| `src/lib/lovable-error-reporting.ts` | 59 linhas, `window.__lovableEvents`, `window.__lovableReportRuntimeError` | deletar           |
| `src/routes/__root.tsx:13,41`        | `reportLovableError(...)` no ErrorComponent                               | remover chamada   |

Não há outros rastros (nada em `public/`, `DESIGN.md`, `PRODUCT.md`, `roadmap.md`).

### 1.3 Achados — código morto / inutileis

**Arquivos que existem só por causa do TanStack Start (SSR):**

- `src/server.ts` (61) — server entry, normaliza respostas 500 do h3
- `src/start.ts` (29) — `createStart`, middleware de erro + CSRF
- `src/router.tsx` (16) — `createRouter` + `QueryClient`
- `src/routeTree.gen.ts` (69) — gerado, aponta para `start.ts`/`router.tsx`
- `src/lib/error-capture.ts` (81) — monkey-patch de `console.error` para recuperar stack engolido pelo h3
- `src/lib/error-page.ts` (30) — HTML inline de fallback 500
- `src/lib/lovable-error-reporting.ts` (59) — telemetria do editor
- `src/routes/README.md` (21) — documentação de file-based routing

Nada disso é usado pelo app em si: `src/routes/index.tsx` não tem loader, server function nem fetch — é 100% estado local do React (`useState` para `view`, `sidebar`, `searchOpen`, `done`).

**Componentes shadcn não usados:** o app importa apenas `button` e `tooltip` (`src/routes/index.tsx:8-9`). Os outros 47 arquivos existem e ninguém os importa.

**Dependências órfãs** (nenhuma importada fora de `src/components/ui/`):
`recharts`, `embla-carousel-react`, `cmdk`, `vaul`, `input-otp`, `react-day-picker`, `date-fns`, `react-hook-form`, `@hookform/resolvers`, `react-resizable-panels`, `sonner`, `zod`, `nitro`, os 4 pacotes `@tanstack/*`, `@lovable.dev/vite-tanstack-config`.
Radix não usados: `accordion`, `alert-dialog`, `aspect-ratio`, `avatar`, `collapsible`, `context-menu`, `hover-card`, `menubar`, `navigation-menu`, `radio-group`, `slider`, `toggle`, `toggle-group`.

**Misc:**

- `package.json` `"name": "tanstack_start_ts"` — nome do template.
- `src/routes/__root.tsx:107` — `<html lang="en">` num produto 100% em pt-BR.
- `public/robots.txt` — inútil num app desktop (não é servido por servidor web).
- `roadmap.md`, `DESIGN.md`, `PRODUCT.md` — conteúdo real do projeto (em pt-BR, específico). **Manter.**
- Diretório **não é um repositório git** (`git rev-parse` falha). Não existe histórico a preservar — o aviso do AGENTS.md sobre force-push é moot.

### 1.4 Ambiente (verificado nesta máquina)

| Item                  | Estado                                                                          |
| --------------------- | ------------------------------------------------------------------------------- |
| Node                  | v26.3.1                                                                         |
| npm                   | 12.0.2 (registry: `registry.npmmirror.com`)                                     |
| bun                   | instalado, mas deixa de ser usado                                               |
| `node_modules/`       | **não existe** — o install será limpo                                           |
| Rust                  | 1.97.0 presente, mas **toolchain default = `stable-x86_64-pc-windows-gnullvm`** |
| Toolchain msvc        | instalado (`stable-x86_64-pc-windows-msvc`, 1.98.0)                             |
| MSVC Build Tools / VS | **AUSENTE** (sem `Microsoft Visual Studio`, sem `vswhere.exe`)                  |
| WebView2 Runtime      | instalado (154.0.4258.37) ✔                                                     |

---

## 2. Decisões fechadas

1. **Stack alvo:** React 19 + Vite (SPA) + Tauri v2. Sem SSR, sem roteador, sem server functions.
2. **shadcn:** manter _curated set_ (18 componentes) e apagar o resto.
3. **Lovable:** remover todos os rastros, inclusive o bloco do AGENTS.md.
4. **bunfig.toml:** descartar o guard `minimumReleaseAge` (sem `.npmrc` substituto).

---

## 3. Bloqueios a resolver antes da Fase 5 (Tauri)

- [ ] **BLOQUEIO — MSVC toolchain ausente.** Tauri v2 no Windows linka com MSVC. Instalar _Visual Studio Build Tools 2022_ com o workload **"Desktop development with C++"** (MSVC v143 + Windows 10/11 SDK + CMake). Sem isso `tauri dev` e `tauri build` não compilam.
- [ ] **Toolchain Rust default está errada.** `rustup default stable-x86_64-pc-windows-gnullvm` não serve. Rodar `rustup default stable-x86_64-pc-windows-msvc` (ou fixar via `rust-toolchain.toml`).
- [ ] _(Opcional,Rede)_ npm usa `npmmirror`; o cargo baixa do crates.io. Se a rede estiver lenta, configurar mirror em `~/.cargo/config.toml` (fora do repo).
- [ ] _(Recomendado)_ `git init` + commit inicial **antes** de qualquer deleção, para ter ponto de retorno.

---

## 4. Fases de implementação

### Fase 0 — Baseline seguro

```sh
git init
git add -A && git commit -m "chore: baseline do template Lovable/TanStack"
```

Motivo: o diretório não é repo git e a Fase 1 apaga ~15 arquivos. `npm run build` antes de mexer não é possível (não há `node_modules` e o build atual depende de config Lovable) — o baseline é a única rede de segurança.

**Aceite:** commit criado, `git status` limpo.

---

### Fase 1 — Bun → npm

| Ação    | Detalhe                                                                                                                          |
| ------- | -------------------------------------------------------------------------------------------------------------------------------- |
| Deletar | `bun.lock`, `bunfig.toml`                                                                                                        |
| Editar  | `.prettierignore` — remover a linha `bun.lock` (linha 7); `package-lock.json` (linha 6) e `pnpm-lock.yaml` (linha 5) já estão lá |
| Editar  | `package.json` — `"name": "tanstack_start_ts"` → `"campus-desktop"`                                                              |
| Editar  | `package.json` — adicionar `"engines": { "node": ">=22" }`                                                                       |
| Editar  | `package.json` — remover o script `"build:dev"` (usa `--mode development`, herança do template)                                  |
| Criar   | `package-lock.json` via `npm install` (na Fase 4, junto da poda de deps)                                                         |

**Aceite:** `bun.lock` e `bunfig.toml` não existem mais; nenhum arquivo do repo menciona bun.

---

### Fase 2 — TanStack → React + Vite SPA

O app tem **uma única tela** com modos de visualização internos (`today | calendar | kanban | list | subject`) controlled por `useState`. Isso casa com a convenção do próprio AGENTS.md ("single stateful dashboard route; its main views are modes of one workflow"). Portanto: **nenhum roteador é necessário** — nem React Router nem TanStack Router.

**Arquivos a deletar:** `src/server.ts`, `src/start.ts`, `src/router.tsx`, `src/routeTree.gen.ts`, `src/lib/error-capture.ts`, `src/lib/error-page.ts`, `src/routes/README.md`, e o diretório `src/routes/` inteiro depois de extrair o conteúdo.

**Arquivos a criar:**

`index.html`

```html
<!doctype html>
<html lang="pt-BR">
  <head>
    <meta charset="UTF-8" />
    <link rel="icon" type="image/x-icon" href="/favicon.ico" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Campus — Organização acadêmica</title>
    <meta name="description" content="Organize aulas, tarefas, provas e notas em um só lugar." />
    <link rel="preconnect" href="https://fonts.googleapis.com" />
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
    <link
      rel="stylesheet"
      href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap"
    />
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

> `lang="pt-BR"` corrige o `lang="en"` do shell atual. Os `<meta>`/`<title>` acima migram do bloco `head()` de `__root.tsx:76-98`; os `og:*` saem (sem servidor, não há preview de crawler).

`src/main.tsx`

```tsx
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./styles.css";

const root = document.getElementById("root");
if (!root) throw new Error("Elemento #root não encontrado");

createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
```

> O import da stylesheet vira `import "./styles.css"` (antes era `appCss from "../styles.css?url"` para SSR). Manter o `<link>` de fontes no `index.html`.

`src/App.tsx` — mover o corpo de `src/routes/index.tsx` com 3 alterações:

1. remover `import { createFileRoute } from "@tanstack/react-router";`
2. remover o export `Route = createFileRoute("/")({ head: ..., component: StudyApp })` e o bloco `head()` (linhas 32-44)
3. `export default StudyApp;` no lugar de `component: StudyApp`

O resto (570 linhas: sidebar, header, `TodayView`, `CalendarView`, `KanbanView`, `ListView`, `SubjectView`, busca ⌘K) vai **literal**, sem alteração de lógica. `@/` e os imports de `lucide-react`, `@/components/ui/button`, `@/components/ui/tooltip`, `@/hooks/use-mobile`, `@/lib/utils` continuam válidos.

**`vite.config.ts` (reescrever do zero)**

```ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import tsconfigPaths from "vite-tsconfig-paths";

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss(), tsconfigPaths()],
  clearScreen: false,
  server: {
    port: 1420, // fixo: o Tauri assume essa porta em dev (ver tauri.conf.json)
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    target: "es2022",
    minify: "esbuild",
    sourcemap: false,
  },
});
```

**`tsconfig.json`**

- `include`: `["src/**/*.ts", "src/**/*.tsx", "vite.config.ts", "eslint.config.js"]` (manter)
- `types: ["vite/client"]` (manter) — cobre `import.meta.env`
- remover `allowImportingTsExtensions: true` (convenção de import sem extensão; só o `routeTree.gen.ts` usava, e ele foi deletado)
- manter o resto das flags estritas, inclusive `noUncheckedIndexedAccess` e `exactOptionalPropertyTypes`
- adicionar `"scripts": { "typecheck": "tsc --noEmit" }` no package.json

**`package.json` — scripts finais**

```json
"dev": "vite dev",
"build": "tsc --noEmit && vite build",
"preview": "vite preview",
"typecheck": "tsc --noEmit",
"lint": "eslint .",
"format": "prettier --write .",
"tauri": "tauri"
```

(A Fase 5 acrescenta `tauri:dev` e `tauri:build`.)

**Aceite:** `npm run dev` serve a tela em `http://localhost:1420` com o dashboard idêntico ao atual; `npm run build` gera `dist/`; nenhum import de `@tanstack/*` no repo.

---

### Fase 3 — Remover rastros do Lovable

| Arquivo                              | Alteração                                                                                                                                                                    |
| ------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `.lovable/project.json`              | deletar diretório `.lovable/`                                                                                                                                                |
| `AGENTS.md`                          | remover linhas 1-10 (bloco `<!-- LOVABLE:BEGIN --> … <!-- LOVABLE:END -->`). Manter a seção "Project architecture" (linhas 12-14) e acrescentar as convenções da nova stack. |
| `README.md`                          | reescrever (ver §4.3)                                                                                                                                                        |
| `src/lib/lovable-error-reporting.ts` | deletar                                                                                                                                                                      |
| `public/robots.txt`                  | deletar (não há servidor web)                                                                                                                                                |

> `src/routes/__root.tsx` (único consumidor da telemetria) já foi deletado na Fase 2, então o `reportLovableError` sai junto sem etapa extra.

**`README.md` novo**

````md
# Campus

App desktop de organização acadêmica: aulas, entregas, provas e notas em uma
interface densa e rápida de operar.

## Stack

React 19 · Vite 8 · TypeScript · Tailwind CSS 4 · shadcn/ui · Tauri 2

## Desenvolvimento

Requisitos: Node >= 22, npm, Rust (toolchain msvc) e Visual Studio Build Tools
com o workload "Desktop development with C++".

```sh
npm install
npm run tauri:dev   # abre a janela do app
```
````

## Scripts

| Script                                                  | O que faz                                                 |
| ------------------------------------------------------- | --------------------------------------------------------- |
| `npm run dev`                                           | Vite em http://localhost:1420 (só o front-end)            |
| `npm run build`                                         | typecheck + build de produção em `dist/`                  |
| `npm run tauri:dev`                                     | app desktop em desenvolvimento                            |
| `npm run tauri:build`                                   | artefato instalável em `src-tauri/target/release/bundle/` |
| `npm run lint` / `npm run typecheck` / `npm run format` | qualidade                                                 |

## Estrutura

- `index.html` — shell do front-end
- `src/main.tsx` — bootstrap React
- `src/App.tsx` — dashboard único e seus modos de visualização
- `src/components/ui/` — componentes shadcn
- `src-tauri/` — shell Rust (janela, wmenu, permissões)
- `DESIGN.md` / `PRODUCT.md` — direção de design e produto

````

**Aceite:** `rg -i lovable .` não retorna nada (fora de `node_modules`, `.git`, `SPEC.md`).

---

### Fase 4 — Poda de código e dependências

**4.1 Curated set do shadcn — manter 18, apagar 31**

Manter: `button`, `tooltip`, `card`, `badge`, `skeleton`, `input`, `textarea`, `label`, `separator`, `dialog`, `dropdown-menu`, `popover`, `select`, `checkbox`, `switch`, `tabs`, `progress`, `scroll-area`.

Apagar: `accordion`, `alert`, `alert-dialog`, `aspect-ratio`, `avatar`, `breadcrumb`, `calendar`, `carousel`, `chart`, `collapsible`, `command`, `context-menu`, `drawer`, `form`, `hover-card`, `input-otp`, `menubar`, `navigation-menu`, `pagination`, `radio-group`, `resizable`, `sheet`, `sidebar`, `slider`, `sonner`, `table`, `toggle`, `toggle-group`.

> `form` fica de fora de propósito: puxa `react-hook-form` + `@hookform/resolvers` + `zod` (3 deps) e nada no app tem formulário ainda. Quando o botão "Nova tarefa" virar diálogo com formulário, rodar `npx shadcn@latest add form` — ele já traz o que falta.

**4.2 Deps a remover do `package.json`**

Remover: `@tanstack/react-start`, `@tanstack/react-router`, `@tanstack/router-plugin`, `@tanstack/react-query`, `@lovable.dev/vite-tanstack-config`, `nitro`, `recharts`, `embla-carousel-react`, `cmdk`, `vaul`, `input-otp`, `react-day-picker`, `date-fns`, `react-hook-form`, `@hookform/resolvers`, `react-resizable-panels`, `sonner`, `zod`, e os 13 pacotes `@radix-ui/*` não listados em 4.1.

Deps que **sobram** (curated set): `react`, `react-dom`, `@radix-ui/react-{dialog,dropdown-menu,popover,select,checkbox,switch,tabs,label,progress,scroll-area,separator,tooltip,slot}`, `class-variance-authority`, `clsx`, `tailwind-merge`, `lucide-react`, `tailwindcss`, `@tailwindcss/vite`, `tw-animate-css`, `zod` ❌ (remover, ver 4.1).
DevDeps que sobram: `@eslint/js`, `@types/node`, `@types/react`, `@types/react-dom`, `@vitejs/plugin-react`, `eslint`, `eslint-config-prettier`, `eslint-plugin-prettier`, `eslint-plugin-react-hooks`, `eslint-plugin-react-refresh`, `globals`, `prettier`, `typescript`, `typescript-eslint`, `vite`, `vite-tsconfig-paths`, `overrides.rolldown`.
> `overrides: { "rolldown": "1.2.1" }` foi pinagem do template Lovable. Testar build sem ele; se o Vite 8 resolver uma versão com o mesmo comportamento, remover.

**4.3 Ajustes de config derivados**

- `eslint.config.js`: `ignores: ["dist", "src-tauri/target", "node_modules"]` (trocar `.output`, `.vinxi`); remover/adicionar a regra `no-restricted-imports` que hoje proíbe `server-only` citando TanStack Start (linhas 23-34) — em SPA não há `*.server.ts`.
- `.prettierignore`: trocar `.output`/`.vinxi` por `src-tauri`; remover `routeTree.gen.ts` (arquivo deletado).
- `.gitignore`: remover `.output`, `.vinxi`, `.tanstack/**`, `.nitro`, a seção "Wrangler / Cloudflare" (`.wrangler/`, `.dev.vars`); adicionar `src-tauri/target/`, `.env`, `.env.*`.
- `components.json`: manter como está (`rsc: false`, `css: src/styles.css`, aliases `@/*`) — continua válido para o shadcn CLI com Vite.

**4.4 Reinstalar**
```sh
npm install        # gera package-lock.json a partir do package.json podado
npm run lint && npm run typecheck && npm run build
````

**Aceite:** `npm ls --depth=0` sem vulnerabilidades deloating; zero arquivos em `src/components/ui/` fora do curated set; `npm run build` e `npm run typecheck` passam.

---

### Fase 5 — Tauri v2

Pré-condição: §3 resolvido (MSVC Build Tools + toolchain msvc).

```sh
npm install -D @tauri-apps/cli@^2
npx tauri init
# respostas do init:
#   app name ....: Campus
#   window title : Campus
#   frontend dir : ../dist
#   dev URL .....: http://localhost:1420
#   before dev   : npm run dev
#   before build : npm run build
npx tauri icon public/favicon.ico     # gera o set de ícones do Windows/macOS/Linux
```

**`tauri.conf.json` — ajustes sobre o default gerado:**

| Chave                                   | Valor                                                                                                                                            | Motivo                                                                                                  |
| --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------- |
| `app.windows[0].title`                  | `Campus`                                                                                                                                         |                                                                                                         |
| `app.windows[0].width` / `height`       | `1280` / `820`                                                                                                                                   | cabe a sidebar 248px + conteúdo                                                                         |
| `app.windows[0].minWidth` / `minHeight` | `1024` / `640`                                                                                                                                   | o layout usa `lg:`/`xl:` breakpoints                                                                    |
| `app.windows[0].backgroundColor`        | `#F8F6FA`                                                                                                                                        | converter de `--background: oklch(0.975 0.008 325.6)` para evitar o flash branco no load                |
| `app.windows[0].decorations`            | `true`                                                                                                                                           | mantém os controles nativos (o DESIGN.md pede remover a _falsa_ moldura desenhada em CSS, não a nativa) |
| `build.devUrl`                          | `http://localhost:1420`                                                                                                                          | casa com `server.port` da Fase 2                                                                        |
| `build.frontendDist`                    | `../dist`                                                                                                                                        |                                                                                                         |
| `build.beforeDevCommand`                | `npm run dev`                                                                                                                                    |                                                                                                         |
| `build.beforeBuildCommand`              | `npm run build`                                                                                                                                  |                                                                                                         |
| `app.security.csp`                      | `null` em dev; `'default-src 'self'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src https://fonts.gstatic.com'` em prod | o `vibrancy`/Inter vêm de fora                                                                          |
| `app.withGlobalTauri`                   | `false`                                                                                                                                          |                                                                                                         |

**Scripts a acrescentar:**

```json
"tauri:dev": "tauri dev",
"tauri:build": "tauri build"
```

**Não adicionar agora:** `@tauri-apps/api` e plugins (`fs`, `store`, `notification`, `updater`). O app ainda é 100% mock data em `useState`; quando precisar persistir, `@tauri-apps/plugin-store` é o caminho natural (o atalho ⌘K e a sidebar já justificam persistência de estado). Isso fica para uma spec de persistência.

**Riscos específicos do Tauri aqui:**

- `styles.css` usa `backdrop-filter` (utilitário `vibrancy`) com fundos semitransparentes. No WebView2 isso desfoca o conteúdo da própria janela — o efeito existe, mas sobre a página, não sobre o desktop. Se quiser o blur de verdade no Windows, seria `transparent: true` + `macos-private-api`-style config, o que exigepaque e traz custo de perf. **Fora do escopo desta spec** — anotar como backlog visual.
- `overflow: hidden` no `body` + `h-dvh` no App: funciona igual no WebView2, mas testar com a janela maximizada e em monitor de DPI 150%.
- Compilação Rust inicial: ~5-10 min e ~1-2 GB em `src-tauri/target/`.

**Aceite:** `npm run tauri:dev` abre a janela "Campus" com o dashboard funcional; `npm run tauri:build` produz `.msi`/`.exe` em `src-tauri/target/release/bundle/`.

---

### Fase 6 — Documentação e limpeza final

- `AGENTS.md`: bloco Lovable removido; manter "Project architecture" e adicionar: "App desktop único (`src/App.tsx`); sem roteador — os modos de visualização são `useState`", "Tauri v2 em `src-tauri/`; o front-end roda em `localhost:1420`", "npm é o package manager; não usar bun".
- `roadmap.md`: as 4 caixas estão marcadas `[x]`. Substituir por itens do backlog real (persistência via Tauri store, modo escuro toggle — as cores `.dark` já existem em `styles.css` mas nada alterna a classe, dialogs reais no lugar dos `<div role="dialog">` inline de `index.tsx:169-193`).
- Verificação final do grep de Lovable e de bun.

---

## 5. Resumo de arquivos

**Deletar (15 + 31 ui)**
`.lovable/project.json` · `bun.lock` · `bunfig.toml` · `public/robots.txt` · `src/server.ts` · `src/start.ts` · `src/router.tsx` · `src/routeTree.gen.ts` · `src/routes/` (inteiro) · `src/lib/error-capture.ts` · `src/lib/error-page.ts` · `src/lib/lovable-error-reporting.ts` · 31 arquivos de `src/components/ui/` · `SPEC.md` (ao final)

**Criar**
`index.html` · `src/main.tsx` · `src/App.tsx` · `package-lock.json` · `src-tauri/**` (gerado pelo `tauri init`) · ícones Tauri

**Modificar**
`package.json` · `vite.config.ts` · `tsconfig.json` · `eslint.config.js` · `.gitignore` · `.prettierignore` · `AGENTS.md` · `README.md` · `roadmap.md`

**Manter intacto**
`src/styles.css` · `src/hooks/use-mobile.tsx` · `src/lib/utils.ts` · `src/components/ui/{button,tooltip}.tsx` + curated set · `components.json` · `DESIGN.md` · `PRODUCT.md` · `public/favicon.ico`

---

## 6. Critérios de aceite finais

1. `rg -i "lovable|bunfig|bun.lock" .` (fora de `SPEC.md`) → vazio.
2. `rg "@tanstack" src` → vazio. `rg "@lovable.dev" package.json` → vazio.
3. `npm ci` do zero funciona e produz `package-lock.json` commitado.
4. `npm run lint` e `npm run typecheck` passam sem warning.
5. `npm run build` gera `dist/` sem erro.
6. `npm run tauri:dev` abre a janela do app com o dashboard visualmente idêntico ao atual (sidebar recolhível, ⌘K, 5 modos de view, responsivo).
7. `npm run tauri:build` gera instalador.
8. `git status` limpo no fim de cada fase.

---

## 7. Fora de escopo

- Persistência de dados (hoje tudo é `useState` + mock).
- Modo escuro toggle (o CSS já tem o bloco `.dark`; falta o controle).
- Empacotamento / code signing / auto-update do Tauri.
- Trocar `public/favicon.ico` por um ícone 1024×1024 dedicado do produto.
- Mover a Inter para self-host (hoje vem do Google Fonts — num app desktop isso é uma chamada de rede por launch).
- Qualquer trabalho visual de UI.
