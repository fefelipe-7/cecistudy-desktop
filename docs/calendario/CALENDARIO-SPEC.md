# Specs do módulo Calendário

Conjunto de specs de implementação derivado de [`calendario.md`](../../calendario.md) (spec de produto, 13 seções, 10 entregas de MVP no §11) para o repositório `cecistudy-desktop`.

Data: 2026-09-28
Escopo: as 7 specs abaixo cobrem `calendario.md` §1 a §13 sem lacuna, e as 10 entregas do §11.
Status: **spec 00 implementada em 2026-09-29** (Fases 0.0–0.5, mais a Fase 0.6 de correção de defeitos do shell). A spec 01 está implementada em código (Fases 1.1–1.9) e a spec 02 começou pela geometria da grade; ambas aguardam o destravamento do toolchain Rust para serem executadas de ponta a ponta. As specs 03–06 continuam propostas fechadas, nenhuma implementada.

---

## Por que dividir

`calendario.md` é um documento de produto: 13 seções, 8 decisões abertas e 10 entregas que misturam domínio, persistência, duas plataformas de interface, quatro módulos que não existem e uma integração externa com credenciais. Um monólito de spec seria: (a) impossível de revisar por partes — cada reviewer teria opinions sobre OAuth e sobre `grid-scale` ao mesmo tempo; (b) sem ordem de implementação, porque as decisões se bloqueiam mutuamente; (c) impossível de abandonar sem perder tudo, e **duas** das partes (Google, módulos de origem) estão bloqueadas por coisas fora do repositório.

A divisão é por **módulo grande com fronteira verificável**. Cada spec tem um `npm run typecheck` / `node --test` que passa sem as outras, e as specs 00, 01 e 06 são verificáveis **sem** Rust — o que importa num repositório cujo toolchain Rust está quebrado (`SPEC.md:10`).

| Spec | Arquivo                                                    | Módulo                                                      | Entregas do §11                               |
| ---- | ---------------------------------------------------------- | ----------------------------------------------------------- | --------------------------------------------- |
| 00   | [`00-fundacao.md`](00-fundacao.md)                         | Decisões §12, arquitetura, convenções, tokens de camada     | — (pré-requisito) **✅ implementada**         |
| 01   | [`01-dominio-persistencia.md`](01-dominio-persistencia.md) | Domínio §2–§6 + SQLite                                      | **1** — 🟡 código pronto, sem `cargo test`    |
| 02   | [`02-grade-desktop.md`](02-grade-desktop.md)               | Grade semanal, arraste, painel, dia/mês/agenda              | **2** (geometria feita), **7**, fecha a **4** |
| 03   | [`03-mobile-execucao.md`](03-mobile-execucao.md)           | Agenda mobile, edição rápida, timer, execução               | **3**, **6**                                  |
| 04   | [`04-modulos-origem.md`](04-modulos-origem.md)             | Contrato com Faculdade, Estudos, TCC, Estágio, Conhecimento | **5** (parcial — bloqueada)                   |
| 05   | [`05-google-calendar.md`](05-google-calendar.md)           | OAuth, sync bidirecional, conflito, exclusão                | **8**, **9**                                  |
| 06   | [`06-sugestoes.md`](06-sugestoes.md)                       | Sugestões de bloco da Ceci                                  | **10**                                        |

---

## Rastreabilidade `calendario.md` → specs

| § do documento                  | Onde está coberto                                                                                                                                                 |
| ------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| §1 Propósito                    | 00 (escopo), 02 (grade), 03 (§13 na 3ª pergunta do painel). **A Home não muda** — a §1 diz que ela é dona das pendências; o repovoamento dela é backlog em 02 §8. |
| §2 Modelo central (7 entidades) | 01 §3.2 (tipos) e §3.3 (schema). INV-1 e INV-2 em 00 §2.2.                                                                                                        |
| §3 Níveis de compromisso        | 00 (D4, INV-4, INV-7), 01 (`Commitment`, `isOverdue`), 06 (D41, D45 — só sugere para `recomendado`/`opcional`).                                                   |
| §4 Estados                      | 00 (INV-3, INV-4), 01 §3.1 (tabela de transições), 02 §5 Fase 2.6 (escopo de recorrência na interface), 03 §5 Fase 3.6 (ações rápidas).                           |
| §5 Recorrências                 | 01 §3.5 (motor `rrule` + materialização, D10), 02 (interface por ocorrência, D36 na spec 05), 05 (instância remota).                                              |
| §6 Planejamento × realidade     | 01 §3.2 (`ExecutionRecord`, `PlanBlock.plannedDuration`), 03 (integral: timer, `ExecutionSheet`, desvio, D24–D26).                                                |
| §7 Experiência desktop          | 00 (D4, camadas), 02 (integral: grade, arraste, painel, granularidades, D15–D20).                                                                                 |
| §8 Experiência mobile           | 03 (integral: D21–D23).                                                                                                                                           |
| §9 Integração com módulos       | 04 (integral no contrato; **adaptadores reais bloqueados**), 01 (`owner_ref`, D14).                                                                               |
| §10 Google Calendar             | 00 (D3, D5, D6, A6), 05 (integral: D32–D39).                                                                                                                      |
| §11 MVP (10 entregas)           | Tabela acima. A **4** (recorrência) é entregada por 01 (motor) + 02 (interface). A **5** é parcial por bloqueio.                                                  |
| §12 Decisões abertas (8)        | **Todas em 00 §2.1** — D1 a D8, cada uma com decisão, justificativa, alternativa rejeitada e consequência.                                                        |
| §13 Regra de ouro               | 00 §2.2 (INV-1 a INV-7, verificáveis), 02 D20 (painel em três colunas), 03 §3 (registro de execução), 06 D45.                                                     |

### Decisões de `calendario.md` §12 — onde foram fechadas

| #   | Pergunta                                           | Decisão                                                                      | Spec               |
| --- | -------------------------------------------------- | ---------------------------------------------------------------------------- | ------------------ |
| 1   | `Event`, `CalendarEvent` ou outro nome?            | `CalendarEvent`                                                              | 00 D1              |
| 2   | Múltiplos prazos ou um prazo principal com etapas? | Um prazo principal + etapas; prazo extra = responsabilidade filha            | 00 D2              |
| 3   | Quais propriedades sincronizam por campo?          | Allowlist explícita por direção, executada em Rust                           | 00 D3 + 05 D35     |
| 4   | Cor da camada e personalização?                    | Fixa, mapeada para `--color-chart-1..5`; Google é neutro; sem personalização | 00 D4              |
| 5   | Google por workspace ou global?                    | Conexão única global, com calendário dedicado escolhido                      | 00 D5              |
| 6   | Política de fuso horário?                          | UTC no armazenamento + IANA por regra; sem ajuste automático ao viajar       | 00 D6              |
| 7   | Blocos podem se sobrepor? Como sinalizar conflito? | Podem, e o conflito é derivado e só sinalizado — nunca bloqueia              | 00 D7              |
| 8   | Como aceitar/ajustar/recusar sugestão?             | Três ações explícitas, sempre; nenhum caminho automático                     | 00 D8 + 06 D40–D46 |

---

## Ordem de implementação e dependências

```
00  Fundação
 ├─► 01 Domínio e persistência
 │    ├─► 02 Grade do desktop
 │    │    ├─► 03 Mobile e execução ──► 06 Sugestões
 │    │    └─► 04 Módulos de origem
 │    │    └─► 05 Google Calendar
 │    └─► 06 Sugestões (independe de 02, exceto pelo slot no painel)
 └─► 06 Sugestões (o motor é puro; só o slot da UI depende de 02)
```

Caminho crítico para as 10 entregas: **00 → 01 → 02 → 03**. 04 e 05 são trilhas paralelas que podem ficar bloqueadas sem travar o resto — que é exatamente o que `calendario.md:146` exige ("não deve impedir a evolução do calendário interno caso a conexão esteja indisponível").

## Bloqueios consolidados

| Bloqueio                                                                                                  | Desbloqueia                                                                                        | Afeta                                                                                                                   |
| --------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| **MSVC Build Tools ausente** (`SPEC.md:10`) — `link.exe not found`                                        | VS Build Tools 2022 com o workload "Desktop development with C++"                                  | Verificação de 01, 02, 03, 05. **00 e 06 não dependem.** `rusqlite` (`bundled`) e a chaveiro do SO exigem compilador C. |
| **Módulos de origem não existem** — Faculdade, Estudos, TCC, Estágio, Base de Conhecimento não têm código | Que os módulos existam, com um `owner_ref` estável                                                 | Entrega **5** do §11, por tempo indeterminado                                                                           |
| **Credenciais do Google** — sem projeto no Cloud Console                                                  | Client OAuth 2.0 do tipo **Desktop app** + Calendar API habilitada + tela de consentimento interna | Entregas **8** e **9** do §11                                                                                           |
| **Lint com 2 warnings no baseline** (`badge.tsx:32`, `button.tsx:49`)                                     | Fase 0.1 da spec 00                                                                                | ✅ **Resolvido** em 2026-09-29 — `npm run lint` reporta `0 problems`                                                    |
| **Produto: escopo de leitura do Google**                                                                  | Decisão de produto                                                                                 | 05 — esta spec assume **só** o calendário dedicado; ampliar é reabrir D5                                                |

---

## Fatos do prompt que precisaram de correção

Verificados no repositório em 2026-09-28, antes de citar:

- `SPEC.md:70` afirma "diretório **não é um repositório git**". **Desatualizado** — `git log` mostra 4 commits (`272599a`, `12a9207`, `1dabb94`, `b7ce6d2`). O aviso de force-push do AGENTS.md original não é mais moot.
- `SPEC.md:385` sugere `@tauri-apps/plugin-store` como caminho de persistência. **Superado** por 00 A2: o modelo tem exceção por ocorrência, vínculo com `remote_etag` e escrita condicional por versão; um JSON não dá transação nem constraint. A spec 00 justifica a troca.
- "`@tauri-apps/api` não está instalado" — **confirmado**: `package.json:23-43` não o lista; só `@tauri-apps/cli` (`:47`).
- "Zero `#[tauri::command]`, zero `invoke_handler`" — **confirmado** por `rg` em `src` e `src-tauri/src`.
- "`tsconfig.json` liga `noUncheckedIndexedAccess` e `exactOptionalPropertyTypes`" — **confirmado** (`:22-23`).
- "Tokens `--color-chart-1..5` já existem" — **confirmado** (`src/styles.css:73-77` claro, `:112-116` escuro). Detalhe novo usado em 00 D4: `--chart-4` no claro é `oklch(0.833 0.119 88.3)` e **não** serve como cor de texto.
- "`CalendarView` em `src/App.tsx:791-866`" — **confirmado**. "grade 5×8" — **confirmado**: `repeat(5,1fr)` em `:810`/`:824` e 8 linhas de 70px em `:826-835`.
- "a CSP bloqueia rede do webview" — **confirmado** (`src-tauri/tauri.conf.json:31`): `connect-src 'self' ipc: http://ipc.localhost`, sem `googleapis`.
- "Rust bloqueado nesta máquina" — **herdado** de `SPEC.md:10`; nenhuma verificação Rust foi feita aqui.
- "o ⌘K é um `role="dialog"` manual" — **confirmado** (`src/App.tsx:354-408`).
- "Nenhum formulário, diálogo real, drag-and-drop ou estado global" — **confirmado**.
- Fato **novo** relevante: `npm run lint` **já falha** no critério de `SPEC.md:426` (2 warnings), e `src/components/ErrorBoundary.tsx` (48 linhas) existe mas não estava na lista de componentes do módulo — nenhuma spec deste conjunto o altera.

## Verificação executada

| Comando                                    | Baseline (2026-09-28)                                                                                        | Após a spec 00 (2026-09-29)                       |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------ | ------------------------------------------------- |
| `npm run typecheck`                        | passou (código 0)                                                                                            | **passou** (código 0)                             |
| `npm run lint`                             | **2 warnings** de `react-refresh/only-export-components` (`src/components/ui/badge.tsx:32`, `button.tsx:49`) | **`0 problems`** (Fase 0.1)                       |
| `npm run build`                            | não executado                                                                                                | **ok** — 1788 módulos, 358,90 kB / 112,43 kB gzip |
| `npm run dev`                              | não executado                                                                                                | **HTTP 200** em `:1420`, servindo o código novo   |
| `npm run tauri:dev`, `npm run tauri:build` | bloqueado por MSVC (`SPEC.md:10`)                                                                            | segue bloqueado por MSVC (`SPEC.md:10`)           |

A spec 00 também corrigiu 12 defeitos pré-existentes do shell — busca rápida sem focus trap, controles inertes com `cursor-pointer`, checkbox sem estado acessível, tarefa duplicada em duas colunas do Kanban, contadores divergentes, ARIA inválido e `<a href="/">` no `ErrorBoundary`. O detalhamento está na spec 00 §4, Fase 0.6.

O `AGENTS.md` passou a registrar o padrão de `src/features/calendar/{domain,data,ui,sync}/`, o papel de `src/lib/ipc.ts`, a fronteira Rust/TypeScript e as invariantes de domínio de `calendario.md` §13.
