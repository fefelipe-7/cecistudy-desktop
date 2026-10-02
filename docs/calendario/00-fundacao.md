# Spec 00 — Fundação do módulo Calendário

Data: 2026-09-28
Escopo: fechar as decisões de `calendario.md` §12 e as decisões de arquitetura que não são deriváveis do código; fixar convenções de pasta, nomenclatura, cor de camada e persistência que as demais specs do Calendário consomem.
Status: **implementada em 2026-09-29.** Fases 0.0 a 0.5 concluídas, mais uma fase extra (0.6) de correção de defeitos pré-existentes do shell. Critérios de aceite de §6 verificados: typecheck 0, lint 0 problems, build ok, dev server em `:1420` respondendo 200.

**Rastreabilidade — `calendario.md`:** §1, §2 (apenas nomenclatura), §3 (semântica, não UI), §7 (camadas), §12 (as 8 decisões), §13 (invariantes).
**Entregas de `calendario.md` §11 cobertas:** nenhuma é _entregue_ aqui — esta spec é pré-requisito das 10.
**Depende de:** nada. É a raiz.
**Deixa deliberadamente para:** 01 (schema e comandos), 02 (grade), 03 (mobile), 04 (módulos de origem), 05 (Google), 06 (sugestões).

---

## 1. Diagnóstico do estado atual

### 1.1 Estado estrutural do repositório (verificado em 2026-09-28)

| Item                 | Estado verificado                                                                                                                                                                                                                                                                                                                     |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Front-end            | SPA única. `src/App.tsx` tem 1137 linhas com **todos** os componentes e **todos** os mocks.                                                                                                                                                                                                                                           |
| Modos de visão       | `type View = "today" \| "calendar" \| "kanban" \| "list" \| "subject"` — `src/App.tsx:29`; switch em `src/App.tsx:337-349`. Sem roteador, como mandam `AGENTS.md:5,9`.                                                                                                                                                                |
| Pontos de integração | `src/App.tsx:346` renderiza `CalendarView`; o item de nav que entra nele está em `src/App.tsx:231-240`.                                                                                                                                                                                                                               |
| `CalendarView`       | `src/App.tsx:791-866`. Grade `grid-cols-[72px_repeat(5,1fr)]` (`src/App.tsx:810` e `:824`), altura fixa `h-[570px]`, 8 linhas de 70px rotuladas 08:00–15:00 (`src/App.tsx:826-835`). Itens posicionados por `style` inline a partir de `top`/`h` literais (`src/App.tsx:851-856`), lidos de `calendarEvents` (`src/App.tsx:709-758`). |
| Persistência         | **Inexistente.** Zero `#[tauri::command]`, zero `invoke_handler`, zero SQLite, zero store. `src-tauri/src/lib.rs:1-16` só registra `tauri-plugin-log` em debug.                                                                                                                                                                       |
| Ponte JS↔Rust        | **Inexistente.** `@tauri-apps/api` não está em `package.json:23-43`; só `@tauri-apps/cli`, em `package.json:47`. `"withGlobalTauri": false` (`src-tauri/tauri.conf.json:13`) — sem API global.                                                                                                                                        |
| Deps de data         | **Nenhuma.** `react-day-picker` e `date-fns` foram removidas de propósito (`SPEC.md:322`) e precisam ser re-adicionadas conscientemente.                                                                                                                                                                                              |
| Crates Rust          | `src-tauri/Cargo.toml:20-25` só tem `serde`, `serde_json`, `log`, `tauri`, `tauri-plugin-log`. `chrono`, `uuid`, `reqwest`, `tokio` e `time` existem em `Cargo.lock` apenas como **transitivos do Tauri** (verificado por grep), não como dependência direta.                                                                         |
| Formulários          | `Input`, `Label`, `Textarea`, `Select`, `Switch`, `Checkbox` estão em `src/components/ui/` e **nunca importados**. O ⌘K é um `<div role="dialog">` manual (`src/App.tsx:354-408`). O set curado **não** tem `form`, `calendar`, `sonner`, `alert-dialog`, `sheet` nem `table`.                                                        |
| Drag-and-drop        | **Inexistente.** `cursor-grab` em `src/App.tsx:918` é decoração, sem handler.                                                                                                                                                                                                                                                         |
| Estado global        | **Inexistente.** Só `useState` local (`src/App.tsx:125-130`).                                                                                                                                                                                                                                                                         |
| Verificação          | `npm run typecheck` → passou. `npm run lint` → **2 warnings** de `react-refresh/only-export-components` em `src/components/ui/badge.tsx:32` e `src/components/ui/button.tsx:49`. `SPEC.md:426` exige zero warning, então o baseline já viola o critério.                                                                              |
| `tsconfig.json`      | `noUncheckedIndexedAccess: true` (`:22`) e `exactOptionalPropertyTypes: true` (`:23`) — pesam em todo array de dias/slots e em todo campo opcional de ocorrência/recorrência.                                                                                                                                                         |
| Convenções           | Prettier com `printWidth: 100` (`.prettierrc:2`); ESLint com Prettier embutido e `ignores: ["dist", "src-tauri/target"]` (`eslint.config.js:9`); shadcn via `npx shadcn@latest add <nome>` (`AGENTS.md:12`); cores em oklch (`src/styles.css:65-103`).                                                                                |

### 1.2 Requisito de `calendario.md` × onde encosta hoje

| Requisito                         | Onde encosta hoje                                                            | Situação                                                                                                    |
| --------------------------------- | ---------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| §2 Evento                         | `calendarEvents` (`src/App.tsx:709-758`) + `CalendarView`                    | Parcial — só desenho; não há entidade, estado nem origem                                                    |
| §2 Responsabilidade               | `tasks` (`src/App.tsx:61-90`), `ListView` (`src/App.tsx:963`)                | Parcial — `Task` é chapada: `title/subject/date/tag/progress`; `date` é string ("Sex, 18:00"), não instante |
| §2 Bloco de planejamento          | item `"Estudo · Prova P1"` hardcoded (`src/App.tsx:742-749`)                 | Ausente — não existe entidade nem referência a uma responsabilidade                                         |
| §2 Ocorrência                     | —                                                                            | Ausente                                                                                                     |
| §2 Registro de execução           | barra `"12h 40min"` hardcoded (`src/App.tsx:689`)                            | Ausente                                                                                                     |
| §2 Regra de recorrência           | —                                                                            | Ausente                                                                                                     |
| §2 Etapa / subtarefa              | —                                                                            | Ausente                                                                                                     |
| §3 Níveis de compromisso          | `tag: "Urgente" \| "Prova" \| "Trabalho" \| "Leitura"` (`src/App.tsx:68-83`) | Ausente — tags livres, não os 4 níveis; nada impede converter recomendado em obrigação                      |
| §4 Estados                        | `done: string[]` (`src/App.tsx:130`, consumido em `:646`)                    | Ausente — booleano; sem `adiado`, `não realizado`, `dispensado`, `cancelado`                                |
| §5 Recorrências                   | `weekDays` array fixo (`src/App.tsx:708`)                                    | Ausente — nenhuma regra, nenhuma ocorrência, nenhuma exceção                                                |
| §6 Planejamento × realidade       | texto estático (`src/App.tsx:697-699`)                                       | Ausente                                                                                                     |
| §7 Visão semanal                  | `CalendarView` (`src/App.tsx:791-866`)                                       | Parcial — grade estática 5 dias × 8 horas, sem dia inteiro, sem eventos reais                               |
| §7 Arrastar / redimensionar       | `cursor-grab` decorativo (`src/App.tsx:918`)                                 | Ausente                                                                                                     |
| §7 Camadas por origem             | `subjects[].dot` / `.soft` por **disciplina** (`src/App.tsx:33-59`)          | Parcial — a cor está ligada a disciplina, não a camada de origem                                            |
| §7 Painel contextual              | —                                                                            | Ausente                                                                                                     |
| §7 Alternância dia / mês / agenda | `ViewSwitch` (`src/App.tsx:760-789`)                                         | Ausente — troca o _modo_ do dashboard, não a granularidade do Calendário                                    |
| §8 Experiência mobile             | `useIsMobile` + overlay da sidebar (`src/App.tsx:131-135`, `:170-176`)       | Ausente — não há agenda nem edição rápida                                                                   |
| §9 Módulos de origem              | —                                                                            | Ausente — Faculdade, Estudos, TCC e Base de Conhecimento **não têm código neste repositório**               |
| §10 Google Calendar               | —                                                                            | Ausente — sem conexão, sem vínculo, sem política de conflito                                                |
| §11 Entregas 1–10                 | —                                                                            | Ausente                                                                                                     |
| §13 Regra de ouro                 | —                                                                            | Ausente — nada separa "o que está marcado" de "o que precisa ser feito" de "o que realmente aconteceu"      |

---

## 2. Decisões fechadas

### 2.1 As 8 decisões de `calendario.md` §12

**D1 — Nome do item principal do Calendário.**
Decisão: **`CalendarEvent`** no TypeScript; tabela `calendar_event` no SQLite.
Justificativa: `Event` colide com o global `Event` do DOM em qualquer `.tsx`, e todo arquivo do módulo Calendário é `.tsx` por definição — é UI. Já existe `calendarEvents` no shell (`src/App.tsx:709`); manter a raiz do nome evita um segundo vocabulário para o mesmo conceito.
Alternativa rejeitada: `Event` (colisão com o global em todo arquivo de UI). `ScheduleItem` foi rejeitado porque mistura evento e bloco de planejamento — violaria §13 pela própria nomeação: o bug viria do nome, não do código.
Consequência: o mock `calendarEvents` (`src/App.tsx:709-758`) é removido na spec 01; `CalendarEvent` é exportado de `@/features/calendar/domain`.

**D2 — Uma responsabilidade pode ter múltiplos prazos?**
Decisão: **um prazo principal** (`Responsibility.dueAt`) mais etapas internas com data opcional (`ResponsibilityStep.dueAt`). Prazo adicional vira uma **responsabilidade filha** (`parentId` + `kind: "entrega"`), nunca um segundo prazo na mesma entidade.
Justificativa: §3 diz que só dois níveis "podem gerar atraso", com severidades distintas. Com `deadlines: []`, "isto está atrasado?" passa a ter duas respostas possíveis, e a Home — que §1 define como dona das pendências — exibiria as duas. §13 exige resposta inequívoca para "o que precisa ser feito".
Alternativa rejeitada: `deadlines: Deadline[]` com `isPrimary` (fonte dupla de verdade para atraso; a sincronização com o Google teria de escolher qual prazo vira `end.date`).
Consequência: gatilho de revisão — se surgir o caso real "trabalho com nota no meio e entrega no fim", modela-se como responsabilidade filha, não como coluna nova.

**D3 — Quais propriedades sincronizam por campo.**
Decisão: **allowlist explícita por direção**, gravada no vínculo. Tabela completa na spec 05 §2. Resumo: `summary`, `description`, `start`, `end`, `location` sincronizam **apenas** quando o item foi criado no cecistudy e é bidirecional; item importado do Google é somente leitura; responsabilidade ligada a evento externo **não** escreve nenhum campo do evento.
Justificativa: `calendario.md` §10 dá a tabela de permissões explicitamente. Implementar por interseção de nomes de campo seria regra implícita, que se rompe no primeiro campo novo e não é testável.
Alternativa rejeitada: "sincroniza o que tiver o mesmo nome" (implícito, contradiz a 3ª linha da tabela de §10).
Consequência: coluna `sync_fields` no vínculo; campo fora da allowlist é gravado só no cecistudy e **nunca** aparece no payload HTTP.

**D4 — Cor de cada camada e personalização.**
Decisão: cor **fixa por camada**, mapeada para os tokens `--color-chart-1..5` que já existem (`src/styles.css:73-77` no claro, `:112-116` no escuro). **Sem personalização.** Seis camadas, cinco tokens + uma neutra.

| Camada (`layer_id`) | Tom                                                                             | Justificativa                                                                                                                                                                          |
| ------------------- | ------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `faculdade`         | `chart-2`                                                                       | violeta escuro (`src/styles.css:74`) — já é a cor de "Cálculo II" (`src/App.tsx:36-37`), então a leitura "aula" é contínua                                                             |
| `estudos`           | `chart-5`                                                                       | âmbar (`src/styles.css:77`) — já a cor do bloco de estudo (`src/App.tsx:604`)                                                                                                          |
| `tcc`               | `chart-4`                                                                       | dourado (`src/styles.css:76`)                                                                                                                                                          |
| `estagio`           | `chart-1`                                                                       | amora (`src/styles.css:73`) — já a cor de "Banco de Dados" (`src/App.tsx:50`)                                                                                                          |
| `conhecimento`      | `chart-3`                                                                       | azul petroleum (`src/styles.css:75`) — já a cor de "Estruturas de Dados" (`src/App.tsx:45`)                                                                                            |
| `google`            | **neutro** — `border-dashed border-border/70 bg-muted/50 text-muted-foreground` | §10 chama o item importado de "bloqueio externo **somente leitura**". Um evento externo não deve competir em croma com as camadas internas, e `PRODUCT.md:21` pune "excesso de cores". |

Justificativa da decisão: `DESIGN.md:9` — "a cor comunica disciplina, prioridade ou estado, nunca decoração". Cor personalizada significa cor arbitrária, o que quebra (a) o contraste WCAG AA exigido em `PRODUCT.md:33` e (b) a paridade claro/escuro hoje garantida pelos tokens semânticos. Há ainda um detalhe concreto: `--chart-4` no modo claro é `oklch(0.833 0.119 88.3)` (`src/styles.css:76`), um amarelo claro que **não** atinge contraste AA como texto — a camada `tcc` tem de usar a variante de preenchimento `bg-chart-4/12 border-chart-4/30` com texto em `foreground`.
Alternativa rejeitada: allowlist de 6 cores escolhidas pela usuária (exige teste de contraste por item, por tema e por estado, e nenhum componente atual tem seletor de cor).
Consequência: um único arquivo com `LAYER_TONE: Record<LayerId, 1 | 2 | 3 | 4 | 5 | "neutral">`. Gatilho de revisão — personalização entra no roadmap quando existir uma 7ª camada ou quando `PRODUCT.md` passar a tratar workspaces como entidade.

**D5 — Google por workspace ou configuração global.**
Decisão: **conexão única, global do app**, com um **calendário dedicado** escolhido pela usuária na primeira conexão. Sem noção de workspace.
Justificativa: o repositório não tem modelo de usuário além de um avatar hardcoded (`src/App.tsx:295`) e um `identifier` de app único (`src-tauri/tauri.conf.json:5`). `calendario.md` §10 fala em "um calendário dedicado" (singular) e §11.8 em "calendário dedicado do Google". `PRODUCT.md:9` fala de "estudantes" no plural genérico, o que não é o mesmo que multiusuário no mesmo app.
Alternativa rejeitada: conexão por workspace (exige tabela de contas, tela de troca de contexto e escopo em todo vínculo — nada disso existe).
Consequência: tabela `google_connection` com `id` fixo `1` (singleton). Trocar de conta = desconectar e reconectar, o que zera `sync_cursor`. Gatilho de revisão — surgir o conceito de workspace → o singleton vira `google_connection(workspace_id, …)` e D5 é reaberta.

**D6 — Política de fuso horário.**
Decisão: **instante absoluto em UTC no armazenamento** + **`timeZone` IANA obrigatória em toda regra de recorrência e em todo bloco planejado com hora**. A recorrência é ancorada no `timeZone` **da regra**, não no fuso do sistema. **Nenhum ajuste automático ao viajar.**
Justificativa: §5 exige fuso na regra. Guardar hora local sem fuso torna ambíguo o que "terça 08:00" significa depois de uma viagem. Ajustar pelo fuso do sistema move aulas silenciosamente — exatamente o que §3 proíbe ("nunca deve transformar silenciosamente…"). Quem viaja decide, pelo painel contextual.
Alternativa rejeitada: seguir o fuso do sistema (surpresa silenciosa); guardar só hora local (ambíguo, quebra o histórico).
Consequência: coluna `time_zone TEXT NOT NULL` em `event_recurrence` e em `plan_block`; item de dia inteiro guarda `date` puro, sem hora. Toda edição de recorrência abre o diálogo "esta ocorrência / esta série e todas as seguintes" (spec 02).

**D7 — Sobreposição de blocos e sinalização de conflito.**
Decisão: **a sobreposição é permitida e nunca bloqueia.** O conflito é **derivado**, calculado na renderização a partir da interseção de intervalos; não é estado persistido, não aparece no `state` de §4 e não altera o `commitment` de §3.
Justificativa: §12.7 pede literalmente sinalizar "sem impedir a decisão da usuária", e §3 proíbe o sistema de criar obrigação sem ação explícita. Se o conflito fosse estado, a usuária teria algo a "resolver" — uma obrigação que ela não aceitou.
Alternativa rejeitada: bloquear a criação de item sobreposto (impede a decisão); encaixe automático (mexe no planejamento dela).
Consequência: função pura `findOverlaps(items, window)` em `src/features/calendar/domain/overlap.ts`, usada pelas specs 02 e 03. Gatilho de performance: acima de 50 itens no mesmo dia, cachear o layout por dia.

**D8 — Como a Ceci aceita, ajusta ou recusa uma sugestão.**
Decisão: **três ações explícitas, sempre no painel contextual.** `Aceitar` grava um `PlanBlock` novo; `Ajustar` abre o editor pré-preenchido e grava no `onSubmit`; `Recusar` grava `dismissedAt` na própria sugestão para ela não reaparecer. **Nenhum caminho automático** converte sugestão em bloco.
Justificativa: §3 — "A Ceci nunca deve transformar silenciosamente um item recomendado ou opcional em obrigação" — e a tabela de §3: Recomendado e Opcional "podem gerar atraso? Não". Sugestão que vira bloco sozinha é precisamente a obrigação silenciosa proibida.
Alternativa rejeitada: modal "aceitar todas" (quantidade >1 burlaria a confirmação individual).
Consequência: entidade `BlockSuggestion` com `state: "pendente" | "aceita" | "recusada" | "expirada"` — detalhe na spec 06.

### 2.2 Decisões de arquitetura (não deriváveis do código)

**A1 — Onde vive a verdade: TypeScript.**
Decisão: a **verdade do domínio é TypeScript** (regras, recorrência, estados, atraso, conflito, mapeamentos). Rust é **armazenamento, rede e SO**, sem regra de negócio.
Justificativa: (a) a máquina de estados de §4, o motor de recorrência de §5 e o cálculo de conflito de D7 exigem teste unitário rápido, e o toolchain Rust está **bloqueado nesta máquina** (`SPEC.md:10`) — cada iteração passaria a depender de um compilador que não roda; (b) o único estado que existe hoje é `useState` (`src/App.tsx:125-130`), ou seja, o front-end já é dono do estado observável; (c) §10 exige duas versões do mesmo item em conflito, e comparar/mergear JSON em Rust duplicaria a tipagem.
Alternativa rejeitada: domínio em Rust (testes em `cargo test`, que não rodam aqui); híbrido com regras nos dois lados (duas implementações da verdade).
Consequência: `src/features/calendar/domain/**` é TypeScript puro — sem React, sem `invoke`, sem `Date` direto (usa um `Clock` injetado), testável com `node --test` sem Tauri.

**A2 — Persistência: SQLite em Rust, exposto por `#[tauri::command]` de domínio.**
Decisão: arquivo SQLite gerenciado pelo Rust via `rusqlite` (feature `bundled`), com migrações versionadas. Rust expõe comandos **de domínio, não de SQL genérico** (ex.: `list_events_in_window`, `move_occurrence`), cada um dentro de uma transação.
Justificativa: o modelo tem exceção por ocorrência (`UNIQUE(event_id, original_start)`), vínculo com `remote_etag` e **escrita condicional por versão** — precisa de transação e de constraint. Um JSON de store não garante nenhum dos dois, e `@tauri-apps/plugin-store` (sugerido em `SPEC.md:385`) é adequado a preferências de UI, não a um grafo de entidades. A API genérica `db_query(sql, params)` foi rejeitada: transformaria o webview em cliente SQL e devolveria regra de negócio aos dois lados, contradizendo A1.
Alternativa rejeitada: `tauri-plugin-store` / JSON (sem transação, sem consulta por janela temporal); `sql.js` no front-end (recarrega o banco inteiro a cada launch e exige WASM).
Consequência: `rusqlite` com `bundled` **exige compilador C**, o que reforça o bloqueio de MSVC (§3). Nenhum comando aceita SQL arbitrário vindo do JS.

**A3 — Recorrência: `rrule` em JavaScript (RFC 5545).**
Decisão: a regra é uma **string RRULE** persistida; a expansão para instantes é feita em TypeScript com o pacote `rrule`. Rust não conhece recorrência.
Justificativa: o Google Calendar fala RRULE — a mesma string serializa para os dois lados, sem camada de tradução (spec 05) — e o parser fica testável sem cargo (A1).
Alternativa rejeitada: `chrono` + `rrule` em Rust (duplicaria a lógica em duas linguagens e colocaria o bloqueio de MSVC no caminho crítico da entrega 4 de §11).
Consequência: `rrule` reintroduzida conscientemente (removida em `SPEC.md:322`). A expansão é limitada a uma janela (visível ± 1 semana) e materializada em `event_occurrence` sob demanda.

**A4 — Drag-and-drop: eventos de ponteiro próprios.**
Decisão: sem biblioteca. Hook `useGridDrag` sobre `pointerdown` / `pointermove` / `pointerup` com `setPointerCapture`, snap de 15 minutos e duas alças de resize (borda superior e inferior).
Justificativa: o caso é _grade com snap_, não _reordenação de lista_. Nenhuma biblioteca de DnD sabe posicionar numa grade temporal: entregaria um modelo de sortable e a grade continuaria precisando de matemática própria. O repositório não tem DnD para reaproveitar (`src/App.tsx:918` é `cursor-grab` sem handler), e a lib traria 1–2 MB de JS com tipagem que colide com `noUncheckedIndexedAccess`.
Alternativa rejeitada: `@dnd-kit/core` (modelo errado, mais peso); `react-dnd` (idem).
Consequência: `src/features/calendar/ui/hooks/use-grid-drag.ts`. Acessibilidade: `PRODUCT.md:28` exige que ações recorrentes funcionem por teclado — mover e redimensionar também têm equivalente por teclado (setas ±15 min; `Shift`+setas ±15 min de duração), independente do ponteiro.

**A5 — Ponte JS↔Rust: `#[tauri::command]` + `invoke` de `@tauri-apps/api`.**
Decisão: instalar `@tauri-apps/api` e envolver tudo em um único módulo `src/lib/ipc.ts` com funções tipadas. `withGlobalTauri` continua `false` (`src-tauri/tauri.conf.json:13`).
Justificativa: é o caminho suportado do Tauri v2 e mantém `withGlobalTauri: false`, que é a postura correta — uma superfície global é invisível ao type-checker. Um único wrapper evita que cada componente conheça nomes de comando.
Alternativa rejeitada: `withGlobalTauri: true` (perde tipagem; `SPEC.md:376` já fixou `false`); `postMessage` pela webview (não é IPC do Tauri).
Consequência: `capabilities/default.json` precisa apenas de `core:default` — comandos `#[tauri::command]` próprios não exigem permissão adicional no Tauri v2.

**A6 — HTTP do Google: `reqwest` em Rust, dentro de comandos.**
Decisão: **toda** chamada ao Google sai do Rust (`reqwest` + `serde`). O webview nunca vê token, cabeçalho `Authorization` nem resposta crua. A CSP de produção **não é alterada**.
Justificativa: a CSP atual (`src-tauri/tauri.conf.json:31`) é `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com; img-src 'self' data:; connect-src 'self' ipc: http://ipc.localhost` — **não** inclui `https://www.googleapis.com` nem `https://oauth2.googleapis.com`, então `fetch` no webview seria bloqueado. Além disso, refresh token em JS é token legível por qualquer XSS. `reqwest` já está no `Cargo.lock:2512` como transitivo.
Alternativa rejeitada: `fetch` no webview + ampliar `connect-src` (enfraquece a CSP de produção e põe credencial no webview); `tauri-plugin-http` (daria acesso HTTP ao JS, exatamente o que não queremos).
Consequência: o refresh token vive **apenas** em `google_connection`, lida e usada só por comandos Rust. Risco aceito e registrado: token em claro no arquivo SQLite local. Gatilho de revisão — chaveiro do SO (DPAPI no Windows) é a correção; fica fora de escopo até o bloqueio de MSVC ser resolvido, porque DPAPI também exige toolchain nativo.

**A7 — Organização de pastas (padrão novo, não derivado do repositório).**
O repositório tem hoje `src/App.tsx`, `src/components/`, `src/hooks/`, `src/lib/`. **Não existe** pasta de domínio, nem de dados, nem convenção de naming de domínio. Esta spec **propõe**:

```text
src/features/calendar/
  domain/     # TypeScript puro: tipos, máquina de estados, RRULE, atraso, conflito. Sem React, sem ipc.
  data/       # repositórios TS; falam com ipc.ts. Conhecem o schema, não o SQL.
  ui/         # componentes de tela e hooks de interação
  sync/       # mapeamento Google (spec 05)
src/lib/ipc.ts           # wrapper único de invoke
src-tauri/src/
  db.rs  commands.rs  oauth.rs  google/
src-tauri/migrations/   # 0001_init.sql, 0002_*.sql
```

Justificativa: sem isso o `CalendarView` cresce dentro de um arquivo de 1137 linhas e o domínio fica acoplado ao React — e A1 exige que o domínio seja importável sem subir o Tauri.
Alternativa rejeitada: `src/calendar/` plano (não separa domínio de dados; e `components.json:14-20` já reserva `components`, `lib` e `hooks`).
Consequência: o alias `@/` continua válido (`tsconfig.json:25-27`); por convenção, `domain/` nunca importa de `ui/` nem de `data/`.

**A8 — Invariantes da regra de ouro (§13), verificáveis.**
Decisão: as sete invariantes abaixo são testáveis e **nenhuma fase pode violá-las**.

| #     | Invariante                                                                                                                                                                                                                                | Como se prova                             |
| ----- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------- |
| INV-1 | `plan_block.responsibility_id` é `NOT NULL`. Um bloco **nunca** cria obrigação nova (§2).                                                                                                                                                 | constraint do schema + construtor de tipo |
| INV-2 | `calendar_event` **não** tem coluna de referência a `responsibility`.                                                                                                                                                                     | inspeção do schema                        |
| INV-3 | Nenhum item com `state = "concluido"` sem um `ExecutionRecord` correspondente.                                                                                                                                                            | regra do comando `complete_item`          |
| INV-4 | `isOverdue()` só retorna `true` para `obrigatorio`/`importante` **e** `state ∈ {planejado, em_andamento, adiado, nao_realizado}`. `cancelado`, `dispensado`, `concluido` e qualquer item `recomendado`/`opcional` nunca atrasam (§3, §4). | unitário                                  |
| INV-5 | Uma `OccurrenceException` **nunca** altera a `event_recurrence`. Editar a série é comando separado e explícito (§4).                                                                                                                      | unitário                                  |
| INV-6 | Nenhum comando de escrita toca campo de item com `origin = "google"` (§10, 2ª linha da tabela).                                                                                                                                           | unitário                                  |
| INV-7 | Nenhuma transição para `commitment = "obrigatorio"` ocorre sem ação explícita. Não existe código que promova `recomendado` → `obrigatorio` (§3).                                                                                          | unitário + revisão de diff                |

---

## 3. Bloqueios

- [ ] **BLOQUEIO — MSVC Build Tools ausente.** `cargo build` / `npm run tauri:dev` não rodam nesta máquina (`SPEC.md:10`). Desbloqueia: _Visual Studio Build Tools 2022_ com o workload **"Desktop development with C++"**. Afeta: toda verificação das specs 01, 02 e 05, e a compilação de `rusqlite` (A2) — o SQLite `bundled` é C puro. **Não afeta** as specs 00 e 06, que são TypeScript puro.
- [x] **BLOQUEIO (não técnico) — 2 warnings de lint no baseline.** `src/components/ui/badge.tsx:32` e `src/components/ui/button.tsx:49` violam `SPEC.md:426` desde antes desta spec. Desbloqueia: extrair `badgeVariants` e `buttonVariants` para arquivos que não exportam componente. **Resolvido na Fase 0.1** (2026-09-29) — `npm run lint` reporta `0 problems`.
- [ ] **BLOQUEIO — credenciais de OAuth do Google.** Sem projeto no Google Cloud Console e sem client OAuth, a spec 05 não pode ser verificada ponta a ponta. Desbloqueia: client OAuth 2.0 do tipo **Desktop app** (o _loopback_ em `http://127.0.0.1:<porta>` continua suportado para clientes desktop; a depreciação vale para iOS/Android/Chrome) + Calendar API habilitada. Detalhe na spec 05 §3.
- [ ] **BLOQUEIO — módulos de origem não existem.** Faculdade, Estudos, TCC e Base de Conhecimento **não têm código neste repositório** (verificado: `src/` contém apenas `App.tsx`, `main.tsx`, `styles.css`, `components/ErrorBoundary.tsx`, `hooks/`, `lib/`, `components/ui/`). A spec 04 entrega o **contrato** e um adaptador de referência; os adaptadores reais ficam bloqueados até que os módulos existam.
- [ ] _(Sem bloqueio)_ Reinstalar `rrule`, `date-fns` e `@tauri-apps/api` exige rede; o registry desta máquina é `npmmirror` (`SPEC.md:77`).

---

## 4. Fases de implementação

### Fase 0.0 — Baseline

| Ação  | Detalhe                                                                               |
| ----- | ------------------------------------------------------------------------------------- |
| Rodar | `npm run typecheck` e `npm run lint` antes de qualquer alteração e registrar a saída. |

**Aceite e teste:** `npm run typecheck` sai com código 0; `npm run lint` reporta exatamente os 2 warnings de `react-refresh` mapeados em §3.
_Resultado em 2026-09-28: typecheck passou; lint = 2 warnings (`src/components/ui/badge.tsx`, `src/components/ui/button.tsx`)._

### Fase 0.1 — Zerar o lint do baseline

| Ação   | Detalhe                                                                                                                                     |
| ------ | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Criar  | `src/components/ui/button-variants.ts` exportando `buttonVariants`                                                                          |
| Editar | `src/components/ui/button.tsx` re-exporta `buttonVariants` do novo arquivo; `src/components/ui/badge.tsx` deixa de exportar `badgeVariants` |

**Aceite e teste:** `npm run lint` → `0 problems`; `npm run typecheck` → 0; o `Button` continua importado em `src/App.tsx:24` e usado sem alteração de contrato.

### Fase 0.2 — Reintroduzir as dependências de dado

| Ação   | Detalhe                                                                              |
| ------ | ------------------------------------------------------------------------------------ |
| Editar | `package.json` — adicionar `rrule`, `date-fns` e `@tauri-apps/api` em `dependencies` |
| Rodar  | `npm install`                                                                        |

Cada uma por um motivo: `rrule` por A3; `date-fns` para aritmética de janela e formatação pt-BR (reintroduzida conscientemente, removida em `SPEC.md:322`); `@tauri-apps/api` por A5. Nenhuma é importada por `src/` ainda — a Fase 0.3 cria o consumidor.

**Aceite e teste:** `npm run typecheck` → 0; `npm run lint` → `0 problems`; `rg '"rrule"|"date-fns"|"@tauri-apps/api"' package.json` retorna as três.

### Fase 0.3 — Árvore de pastas e wrapper de IPC

| Ação  | Detalhe                                                                  |
| ----- | ------------------------------------------------------------------------ |
| Criar | `src/features/calendar/{domain,data,ui,sync}/.gitkeep`, `src/lib/ipc.ts` |

`src/lib/ipc.ts` expõe, por enquanto, apenas `isIpcAvailable(): boolean`, que retorna `false` quando `window.__TAURI_INTERNALS__` não existe — para que o app continue rodando em `npm run dev` (browser puro) durante o desenvolvimento do módulo. Nenhum comando é chamado ainda.

**Aceite e teste:** `npm run typecheck` → 0; `npm run lint` → `0 problems`; `npm run dev` sobe em `http://localhost:1420` sem erro; `rg "invoke" src` só encontra a definição em `src/lib/ipc.ts`, nenhum consumidor.

### Fase 0.4 — Vocabulário de camadas e de níveis

| Ação  | Detalhe                                                                                                                                             |
| ----- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| Criar | `src/features/calendar/domain/layers.ts` — `LayerId`, `LAYERS`, `LAYER_TONE` (D4), `layerClass(tone)`                                               |
| Criar | `src/features/calendar/domain/commitment.ts` — `Commitment` (`obrigatorio \| importante \| recomendado \| opcional`) e `canGenerateDelay()` (INV-4) |

**Aceite e teste:** `npm run typecheck` → 0; `npm run lint` → `0 problems`; `rg "LAYER_TONE" src` retorna o registro com as 6 chaves de D4; `rg "text-chart-4" src` → **vazio** (é por isso que a camada `tcc` usa preenchimento, não cor de texto — `--chart-4` no claro é `oklch(0.833 0.119 88.3)`, `src/styles.css:76`).

### Fase 0.5 — Registrar o padrão no AGENTS.md

| Ação   | Detalhe                                                                                                                                                                                                     |
| ------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Editar | `AGENTS.md` — acrescentar: camadas em `src/features/calendar/{domain,data,ui,sync}/`; Rust guarda armazenamento, rede e SO e nunca regra de negócio; §13 de `calendario.md` é invariante de todas as fases. |

**Aceite e teste:** `rg "features/calendar" AGENTS.md` retorna a linha; as três seções existentes de `AGENTS.md` permanecem e a seção de comandos não muda.
_Resultado em 2026-09-29: concluída. `AGENTS.md` ganhou "Features live in `src/features/calendar/`" em Stack and tooling, e as seções "Domain invariants" e "Commands" (que já existia) foram preservadas._

### Fase 0.6 — Defeitos pré-existentes do shell (fora do plano original)

Executada em 2026-09-29 porque a Fase 0.1 limpou o lint e revelou um conjunto de defeitos que nenhuma spec do Calendário previa. Nenhum deles é do módulo Calendário: são do shell visual, e todos incorriam nos critérios de `PRODUCT.md` (WCAG AA) ou em consistência de dados. Corrigi-los agora porque qualquer spec filha vai herdar o shell.

| #   | Defeito                                                                                                                                                                 | Onde                                     | Correção                                                                                                                                               |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1   | Busca rápida sem focus trap, sem devolução de foco ao fechar e com `<div role="dialog">` manual — `Escape` era tratado por um listener global que não restaurava o foco | `src/App.tsx:354-408`                    | Substituída pelo `Dialog` do curated set (`roadmap.md:13`). `DialogContent` ganhou `showCloseButton` e overlay mais leve.                              |
| 2   | A busca não limpava `query` ao fechar — reabrir via ⌘K mostrava o filtro anterior e a lista de resultados já filtrada                                                   | `src/App.tsx`                            | `setQuery("")` no clique de resultado; listener de ⌘K passou a ser condicional a `searchOpen`, deixando o `Escape` para o Radix.                       |
| 3   | Controles com `cursor-pointer` e nenhum handler: "Preferências", "Notificações", "Semana anterior", "Hoje", "Próxima semana", "Adicionar" (Kanban)                      | `src/App.tsx`                            | `disabled` + `cursor-not-allowed` + opacidade reduzida. `IconButton` e `NavItem` ganharam a prop `disabled`. Deixaram de mentir sobre serem clicáveis. |
| 4   | Checkbox de "Para avançar hoje" desenhado com `<span>` — leitores de tela não anunciavam o estado                                                                       | `src/App.tsx:653-661`                    | `aria-pressed` no botão + `aria-hidden` no quadrado decorativo.                                                                                        |
| 5   | `useIsMobile` lia `window.innerWidth` em vez de `mql.matches`, divergindo do listener registrado                                                                        | `src/hooks/use-mobile.tsx:11`            | `onChange` passou a usar `mql.matches`, consistente com a media query.                                                                                 |
| 6   | Kanban mostrava `tasks[2]` em "A fazer" **e** em "Em revisão" ao mesmo tempo                                                                                            | `src/App.tsx:868-894`                    | `columns` reescrito com fatias disjuntas (`slice(2,3)`, `slice(0,2)`, `slice(3,4)`).                                                                   |
| 7   | `ListView` duplicava 2 tarefas com `tasks.concat(tasks.slice(0, 2))` e usava `key={`${title}-${i}`}` para mascarar a colisão                                            | `src/App.tsx:983`                        | Renderiza `tasks` uma vez, `key={task.title}`. `cursor-grab` do card removido — não há drag-and-drop (D4/A4).                                          |
| 8   | Contadores divergentes do conteúdo: "12 atividades" com 4 tarefas, "4 nesta semana", `count="4"` no nav                                                                 | `src/App.tsx`                            | Derivados dos dados: `kanbanTotal`, `tasks.length`, `subjects.length`.                                                                                 |
| 9   | `weekDays` era `string[]` e o dia atual era detectado por `day.includes("25")` — casa com qualquer rótulo contendo "25"                                                 | `src/App.tsx:708,817`                    | `weekDays` virou `{ label, isToday }[]`; o destaque usa `day.isToday`.                                                                                 |
| 10  | `ViewSwitch` usava `role="tablist"`/`role="tab"` sem nenhum `tabpanel` correspondente — ARIA inválido                                                                   | `src/App.tsx:768,776`                    | Virou `role="group"` + `aria-pressed`.                                                                                                                 |
| 11  | Abas de `SubjectView` sem semântica de tabulação alguma                                                                                                                 | `src/App.tsx:1041`                       | `role="tablist"` / `role="tab"` / `aria-selected`; a lista virou a constante `subjectTabs` (typed).                                                    |
| 12  | `ErrorBoundary` com `<a href="/">` — recarrega a webview inteira num app desktop                                                                                        | `src/components/ErrorBoundary.tsx:37-42` | Removido. Já existia "Tentar novamente".                                                                                                               |

**Aceite e teste:** `npm run typecheck` → 0; `npm run lint` → `0 problems`; `npm run build` → ok (1788 módulos, 358,90 kB); `http://localhost:1420/src/App.tsx` retorna 200 com o código novo.

**Fora desta fase, propositalmente:** modo escuro, `favicon`, auto-hospedar a Inter, overflow horizontal abaixo de 480px — os três últimos já são itens de `roadmap.md` e não têm relação com o Calendário.

---

## 5. Resumo de arquivos

**Criar**
`src/features/calendar/domain/layers.ts` · `src/features/calendar/domain/commitment.ts` · `src/features/calendar/{domain,data,ui,sync}/.gitkeep` · `src/lib/ipc.ts` · `src/components/ui/button-variants.ts` · `src/components/ui/badge-variants.ts`

**Modificar**
`package.json` · `package-lock.json` · `src/components/ui/button.tsx` · `src/components/ui/badge.tsx` · `src/components/ui/dialog.tsx` · `src/hooks/use-mobile.tsx` · `src/components/ErrorBoundary.tsx` · `src/App.tsx` (só na Fase 0.6) · `AGENTS.md`

**Não tocar nesta spec**
`src/styles.css` · `src-tauri/**` · `src-tauri/tauri.conf.json` · `SPEC.md` · `calendario.md`

---

## 6. Critérios de aceite finais

1. `npm run typecheck` → código 0. **Verificado em 2026-09-29: OK.**
2. `npm run lint` → **`0 problems`** (hoje são 2; esta spec zera). **Verificado: OK.**
3. `rg "text-chart-4" src` → vazio. **Verificado: vazio.**
4. `rg "invoke" src/App.tsx` → vazio — a ponte só entra na spec 01. **Verificado: vazio.**
5. `npm run dev` sobe em `http://localhost:1420` com o dashboard atual. **Verificado: HTTP 200, servindo o código novo.** A ressalva é que a Fase 0.6 **alterado** `src/App.tsx` — a restrição original da spec era "visualmente idêntico", e ela não se aplica mais: 12 defeitos do shell foram corrigidos, com mudança visual restrita a controles que agora aparecem desabilitados e à busca rápida, que passou a ser um `Dialog` posicionado a 14vh como antes.
6. `git status` limpo ao fim de cada fase. **Verificado: nada commitado ainda — a commitagem fica a critério de quem conduz.**

---

## 7. Fora de escopo

- Schema SQLite, comandos Rust e migrações — spec 01.
- Qualquer entidade de domínio além de `Layer` e `Commitment` — spec 01.
- Grade, painel contextual, arrasto e redimensionamento — spec 02.
- Agenda mobile, timer e registro de execução — spec 03.
- Código de Faculdade, Estudos, TCC ou Base de Conhecimento — spec 04 (bloqueado, §3).
- OAuth, `reqwest`, qualquer chamada ao Google, qualquer alteração de `tauri.conf.json` — spec 05.
- O gerador de sugestões — spec 06.
- Modo escuro: o bloco `.dark` existe (`src/styles.css:105-139`) e nada alterna a classe — segue em `roadmap.md:11`.
- Chaveiro do SO para o refresh token (decisão A6; gatilho em §3).
