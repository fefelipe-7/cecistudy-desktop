# cecistudy (desktop)

## Project architecture

- The study workspace is a **shell** with the nine modules of [`SPEC-D-006`](../docs/specs/desktop/SPEC-D-006-shell-navegacao-e-workspace-academico.md). `src/shell/navigation.ts` is the single navigation table (`MODULOS`, `WORKSPACES`, `UTILITARIOS`, `CANDIDATOS_RECUSADOS`); the sidebar, the title and the search read from it, and each entry declares its data owner and its state (`pronto` or `em-construcao`). Four modules are implemented (`pronto`) — **Calendário**, **Faculdade**, **Estudos** and **TCC & Projetos**; the other five are declared and render an honest empty state. The rule of D62: "modes of one workflow, not separate destinations" holds **inside** a module (the five Calendar views are modes) and **not between** modules — switching modules is a navigation destination, and the destination lives in the table, not in a component's `useState`. The rule is written in [`docs/espacos/00-shell-e-navegacao.md`](docs/espacos/00-shell-e-navegacao.md).

## Stack and tooling

- Front-end is a Vite SPA (`index.html` + `src/main.tsx`). There is no server, no SSR and no router — the shell keeps its three navigation states (active module, sidebar, overlay) as `useState` in `src/App.tsx`, and, **inside** a module, the views are modes, not routes. A router stays rejected until module deep-links exist (`SPEC-D-006` D62, alternative b).
- Tauri v2 owns the desktop shell in `src-tauri/`. The dev server runs on `http://localhost:1420` (`server.port` in `vite.config.ts` must stay in sync with `build.devUrl` in `src-tauri/tauri.conf.json`).
- npm is the package manager. Do not use bun, yarn or pnpm — `package-lock.json` is the lockfile of record.
- `src/components/ui/` holds a curated shadcn set. Add new components with `npx shadcn@latest add <name>` rather than hand-copying from the old template.
- Features live in `src/features/calendar/` (one folder per feature), split into `domain/`, `data/`, `ui/` and `sync/` (external service mapping). `domain/` never imports from `ui/` or `data/`.
- **Rust is the backend, and the domain truth lives there.** `src-tauri/` implements the domain rules — state machine, recurrence expansion, derived overlap/overdue, invariants — on top of SQLite, Tauri commands and OS integration. TypeScript is the bridge between the UI and that backend: it holds the types and the pure functions the UI needs to render, and it never re-derives a rule the backend already owns.
- A rule that must hold may be enforced in exactly one place, and that place is Rust. The TS mirror exists for rendering and for tests that must run without the Rust toolchain; when the two disagree, Rust is right and the TS mirror is the bug.
- **The state machine and the recurrence engine are in `src-tauri/`, not in `domain/`.** The pure rules live in `src-tauri/src/calendario/dominio.rs` (`TRANSITIONS`, `transition_allowed`, `mark_overlaps`, `is_overdue`) and the recurrence expansion lives in `commands.rs::expand_rule`, which delegates to the **`rrule` crate (RFC 5545)**. The four files in `src/features/calendar/domain/` are **mirrors**, and `golden/*.jsonl` is the contract that proves the two agree: 49 transition cases, 10 overlap cases, 84 overdue cases, 12 recurrence cases. Both sides read the same files. Since D1 (2026-10-09) the mirrors are **test-only**: `ui/`, `data/` and `sync/` do not import them, and the read path hands the UI the derived `GridItem.allowedTransitions` (from `domain::destinations_of`) and `Responsibility.isOverdue`; concluding goes through `complete_item`, which writes the `execution_record` (INV-3). Rule 6 of `check-boundaries.mjs` fails if a production file imports one of the four again.
- The rules are enforced **in the write path**, not only written: `exigir_transicao` refuses an illegal transition in `set_occurrence_state`, in `update_event` (a patch carrying `state`) and in `insert_execution`.
- **There is no hand-written RRULE parser.** `expand_rule` builds an `rrule::RRule` and lets the crate apply the RFC. The six-key `HashMap` parser, `weekday_index`, `matches_freq`, `expand_monthly` and the `max_steps` cap were all removed on 2026-10-03 — see [`SPEC-D-002`](../docs/specs/desktop/SPEC-D-002-uma-verdade-de-recorrencia.md) D50 and its §13. Concretely, that removal fixed three bugs: `FREQ=MONTHLY` fell through to the `_` arm and became **weekly**; an invalid `FREQ` became `unwrap_or("WEEKLY")`; and `FREQ=WEEKLY` without `BYDAY` produced one occurrence **per day**.
- `chrono` is a **direct** dependency, not a transitive one reached through `rrule`: `calendario/comandos.rs` converts epoch-ms ↔ `DateTime<Tz>` itself, and depending on another crate's transitive dependency is borrowing an API you do not own.
- **The Rust backend is four modules, one directory each, and the write path lives inside the owner** (SPEC-D-015, `D96..D100`). `src-tauri/src/` is `nucleo/` (no data owner: `Db`, `DbError`, `new_id`, `now_ms`) plus `calendario/`, `faculdade/`, `estudos/` and `tcc/`. Each module directory holds its engine (`dominio.rs`, `media.rs`, `fsrs.rs`, `documento.rs`…), its `comandos.rs` (the only place that knows SQL) and its `testes.rs`. **The directory name is the owner id**: rule 8 of `check-boundaries.mjs` takes the owner from the path, so a file that writes outside its module is a gate failure, and a module directory that is not in `DONOS_RUST` is a gate failure. `commands.rs` — 6537 lines and 53% of the crate — no longer exists; it was the write path of all four owners at once, and `tcc/docx.rs` (1124 lines) is now the largest file, which is coherent: it is the DOCX round-trip, not a write path.
- Derived values are computed on read and never persisted: `GridItem.has_conflict` (D7), `GridItem.allowed_transitions` (D1.3) and `Responsibility.is_overdue` (INV-4). There is no delay column in the schema, and nothing else compares `due_at` with the clock.
- `data/` talks to the backend through `src/lib/ipc.ts` and knows the schema, not the SQL. It may not re-implement persistence, recurrence or any other domain rule.
- **The screen reads through `data/calendar-store.ts` and not around it.** `calendarStore` is a module singleton wired to the real bridge, so the cache cannot be bypassed. It keys events by `<from>:<to>` — **not** `<mode>:<from>:<to>`, which is what `DP1` originally wrote: the mode decides *whether* responsibilities are read, not *under which key* events are stored.
- **A cache is not a concurrency guard.** `inFlightEvents` dedupes one window key; switching windows produces different keys and two genuinely concurrent reads, so the newest-wins rule is a `generation` counter in `CalendarScreen`, not a boolean. `calendar-store.test.ts` has a passing test that documents this limit — it describes the boundary on purpose.
- The dev seed is treated as a **write**: `seedIfEmpty()` returning `true` invalidates before re-reading. Without that the screen would show an empty database until the next window change, and the bug would be invisible because an empty grid is the state the user expected anyway.
- `domain/` in TypeScript stays pure — types, state machine, recurrence, derived rules; no React, no `invoke`, no direct `Date`, with an injected `Clock`. The toolchain is no longer the reason: it is what makes the mirrors testable **without** the Rust build, so a contributor can change a rule and see both sides at once.

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

### Gate

Um comando. Roda de `src-tauri/` por dentro, porque o override do rustup é por
diretório.

```
npm run verify
```

A lista, na ordem, e a §`Commands` tem de bater com ela:

| Script | O que prova | Como medir |
|---|---|---|
| `typecheck` | `tsc --noEmit` | o próprio script |
| `lint` | `eslint .` (inclui prettier) | o próprio script |
| `test` | `node --test` nos testes de `domain/`, `data/`, `ui/` e `shell/` | `npm run test` |
| `test:golden` | os espelhos TS concordam com `golden/*.jsonl` | `npm run test:golden` |
| `check:boundaries` | **10** regras de fronteira como código: sete de TypeScript, e três do Rust — a 8ª cruza bridge × tabela por dono e tira o dono do **caminho** do arquivo (D67, D96, D97), a 9ª cruza registro × função em `lib.rs` (D97, I4), a 10ª põe teto de 2000 linhas em `<módulo>/comandos.rs` (D100, I5) | `npm run check:boundaries` |
| `test:gate` | **verificação negativa** do gate: injeta seis violações conhecidas e exige que cada uma seja acusada, inclusive num `fn` privado | `npm run test:gate` |
| `verify:rust:domain` | `cargo test --lib --no-default-features` | o próprio script |
| `verify:rust:clippy` | `cargo clippy --all-targets --all-features -- -D warnings` | o próprio script |
| `verify:rust:fmt` | `cargo fmt --all -- --check` | o próprio script |
| `verify:rust:shell` | `cargo check --all-targets` | o próprio script |

**Contagem sem o comando que a produz é mentira.** Esta tabela **não** guarda número:
cada linha diz **como medir**, e um snapshot datado mora em [`roadmap.md`](./roadmap.md)
§"Estado verificado". O número do dia sai do comando.

O CI em `.github/workflows/ci.yml` roda exatamente `npm run verify`, em
`ubuntu-latest`, onde o Rust linka sem o bloqueio de compilador C desta máquina
e o `--no-default-features` continua obrigatório pela regra do WebView2.

## Golden é o contrato, não uma cópia

`golden/*.jsonl` é lido pelo Rust **e** pelo espelho TypeScript. Divergência entre
os dois lados é falha, e a falha nomeia o caso. O golden é dado: entrada e saída
esperada, nunca algoritmo — regra dentro do golden seria uma terceira
implementação.

**Exceção de Faculdade e Estudos (D74/D76):** `media.jsonl` (10 casos) e
`fsrs.jsonl` (8 casos) são lidos **só pelo Rust** — D74 rejeitou o recálculo no
TypeScript, então não há espelho para divergir. No de média os valores são
aritmética de mão; no de FSRS vêm da própria crate (`fsrs 6.6.2`), porque
hand-rolar o modelo é a alternativa rejeitada de D76 — o pin é contra bump
silencioso da dependência, e regenerar é o teste `#[ignore]`
`regenerar_o_golden_do_fsrs` (ver o cabeçalho do arquivo).

Os goldens foram gerados por implementações independentes de propósito, e o
recorrência é o caso onde isso pagou duas vezes.

A primeira versão do golden de recorrência foi gerada por aritmética de data
escrita à mão, no Node — deliberadamente, para não depender do motor que se
queria provar. Foi essa independência que revelou `FREQ=WEEKLY` sem `BYDAY`
produzindo uma ocorrência por dia, e depois que a aritmética própria estava
errada de um jeito que nenhum teste escrito contra ela pegaria: ela **limitava**
o dia 31 ao último dia do mês, quando a RFC 5545 §3.3.10 manda **ignorar** a
ocorrência inválida e não contá-la.

Hoje o golden é gerado pelo **pacote npm `rrule`** e o lado Rust usa o **crate
Rust `rrule`**. São duas implementações independentes da mesma RFC, e o golden é
o contrato entre elas — o espelho TS exercita `domain/recurrence.ts` real, não
uma reimplementação. Por isso `rrule` é dependência de `dependencies` e não de
`devDependencies`: ele está no caminho de teste do espelho.

Regra que decorre disso, e que vale para qualquer motor com RFC: **não escreva
o gerador do golden com o mesmo tipo de raciocínio do motor que ele prova.** Um
gerador manual não é neutro, é uma terceira implementação com o mesmo viés.

### Por que `--no-default-features` no gate de domínio

A feature `tauri-shell` (ligada por padrão) guarda as 48 cascas
`#[tauri::command]` (Calendário + Faculdade + Estudos + TCC) e `lib.rs::run`.
O número se mede com
`Select-String -Path src-tauri\src\lib.rs -Pattern "comandos::" -Encoding UTF8 | Measure-Object` —
o padrão mudou de `commands::` para `<módulo>::comandos::` em F4, quando
`lib.rs` passou a registrar com o caminho do dono. Sem ela o binário
de teste não linka
`WebView2Loader.dll` nem a pilha de GUI (`user32`, `gdi32`, `dwmapi`,
`WebView2Loader`), e o loader do Windows falha com `STATUS_ENTRYPOINT_NOT_FOUND`
em máquina sem o runtime do WebView2. Ver
[`SPEC-D-005`](../docs/specs/desktop/SPEC-D-005-executar-o-backend-rust.md) D84.

`verify:rust:shell` continua **com** a feature padrão, e é ele que impede a
separação de quebrar o `tauri build`.

**Não** use `tauri` como dependência `optional` para resolver isso:
`tauri-build::is_dev()` aborta sem `DEP_TAURI_DEV`, que só existe se o build
script do crate `tauri` rodar, e build script não pode ser opcional.

### Toolchain

O toolchain do host de desenvolvimento é `stable-x86_64-pc-windows-gnu`, com
MinGW-w64 portátil. `rustup override set stable-x86_64-pc-windows-gnu` é aplicado
em `src-tauri/` e é estado local do rustup: **não há `rust-toolchain.toml`
versionado**, porque fixar um canal específico de host quebraria o CI em Linux.

Se `cargo` reclamar de `link.exe not found`, o override não está aplicado **ou o
comando rodou de fora de `src-tauri/`** — o override não vale para a raiz do app.
Se reclamar de `-lgcc_eh`, falta um `MinGW-w64` com `libgcc_eh.a`; o
`llvm-mingw` não serve, e isso já foi medido.

Nesta máquina, `C:\Users\fjferreira\tools\msys64\ucrt64\bin` (com `libgcc_eh.a`) e o
`llvm-mingw` (sem) estavam **os dois** no PATH de **usuário**, na ordem errada: o
`llvm-mingw` vinha antes, então `npm run verify` falhava no passo Rust. **Resolvido em
2026-10-13** só com a **reordenação** do PATH de usuário (o `msys64\ucrt64` primeiro;
nenhuma instalação, 18 entradas → 18). Medido com `npm run verify`: `EXIT 0`.
**Terminais abertos antes da correção ainda herdam a ordem antiga** — abra um novo.

`src-tauri/rustfmt.toml` fixa 2 espaços, que é o estilo que o projeto já usava.
O default do rustfmt é 4 e reformata o repositório inteiro sem mudar regra.

### O que ainda não roda

`npm run tauri:dev` e `npm run tauri:build` **continuam bloqueados**: em
`windows-gnu` o empacotamento exige DLL de runtime do MinGW junto do binário, e
isso não foi resolvido. Não reportar teste Rust como passando sem ter rodado o
`verify` na frente dele.
