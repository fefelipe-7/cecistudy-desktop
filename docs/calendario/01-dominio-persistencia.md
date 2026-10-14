# Spec 01 — Domínio do Calendário e persistência

Data: 2026-09-28
Escopo: materializar o modelo central de `calendario.md` §2, os níveis de §3, a máquina de estados de §4 e a recorrência de §5 como TypeScript puro, e persistir tudo em SQLite através de comandos Rust.
Status: **Fases 1.1 a 1.9 implementadas** (2026-09-29). As Fases 1.4 a 1.9 não foram compiladas nem executadas — a validação do Rust depende do bloqueio de toolchain abaixo (§4).

**Rastreabilidade — `calendario.md`:** §2 (integral), §3 (integral), §4 (integral), §5 (motor e persistência; a UI por ocorrência é spec 02/03), §6 (a **meia dados**: planejamento e realidade; o timer é spec 03), §13 (INV-1 a INV-7).
**Entregas de `calendario.md` §11 cobertas:** **1** (novo domínio de eventos, responsabilidades, blocos, ocorrências, etapas e registros de execução) — base obrigatória de 2 a 10. Pré-requisito de 4 e 6.
**Depende de:** spec 00 (A1–A3, A5, A7, A8, D1, D2, D4, D6, D7).
**Deixa deliberadamente para:**

- spec 02 — renderização, arrasto, redimensionamento, painel contextual;
- spec 03 — timer em execução e edição rápida mobile;
- spec 04 — comandos de retorno aos módulos de origem (aqui só existe a coluna `origin` e a chave estrangeira de vínculo, que fica nullable até lá);
- spec 05 — toda a layer de Google (aqui só existe `external_link` como schema vazio);
- spec 06 — a regra que **gera** `BlockSuggestion` (aqui só existe a entidade, para que o esquema não mude depois).

---

## 1. Diagnóstico do estado atual

O diagnóstico estrutural completo está na spec 00 §1. O que é próprio desta spec:

| Requisito                       | Onde encosta hoje                                                                                  | Situação     |
| ------------------------------- | -------------------------------------------------------------------------------------------------- | ------------ |
| §2 — 7 entidades                | nenhuma; só literais (`src/App.tsx:33-90`, `:709-758`)                                             | Ausente      |
| §3 — 4 níveis                   | `tag` textual em `src/App.tsx:68-83`                                                               | Ausente      |
| §4 — 7 estados                  | `done: string[]` (`src/App.tsx:130`)                                                               | Ausente      |
| §5 — regra, ocorrência, exceção | —                                                                                                  | Ausente      |
| §6 — planejado × real           | `"12h 40min"` estático (`src/App.tsx:689`)                                                         | Ausente      |
| Persistência                    | `src-tauri/src/lib.rs:1-16` sem `invoke_handler`; `src-tauri/Cargo.toml:20-25` sem driver de banco | Ausente      |
| Ponte                           | `src/lib/ipc.ts` (spec 00) sem nenhum comando                                                      | Ausente      |
| Pontos de integração do App     | `src/App.tsx:346` renderiza `CalendarView`; `src/App.tsx:125-130` é onde hoje mora todo o estado   | A substituir |

Restrições do compilador que moldam o desenho:

- `noUncheckedIndexedAccess: true` (`tsconfig.json:22`) — `weekDays[0]`, `slots[i]` e `parts[n]` retornam `T | undefined`. Todo acesso por índice no domínio precisa de guarda explícita; o tipo do retorno de função precisa refletir isso.
- `exactOptionalPropertyTypes: true` (`tsconfig.json:23`) — `type P = { note?: string }` **não** aceita `{ note: undefined }`. Todo campo opcional de ocorrência (`movedTo`, `cancelReason`) é `string | undefined` **explícito** ou ausente — nunca `undefined` atribuído.

---

## 2. Decisões fechadas

### 2.1 Herdadas da spec 00 (não repetidas)

A1 (verdade em TypeScript) · A2 (SQLite via `rusqlite`, comandos de domínio) · A3 (`rrule` em JS) · A5 (ponte `invoke`) · D1 (`CalendarEvent`) · D2 (um prazo principal) · D4 (tons de camada) · D6 (UTC + `timeZone` IANA) · D7 (conflito derivado) · INV-1..INV-7.

### 2.2 Novas decisões desta spec

**D9 — Como o tempo é representado no TypeScript.**
Decisão: todo instante é um `number` (epoch **milissegundos**, UTC) brandado como `type Instant = number & { readonly __brand: "Instant" }`. Datas de dia inteiro são `type PlainDate = string & { … }` no formato `YYYY-MM-DD`. Durações são `Minutes = number`. `Date` nativo só aparece dentro de `domain/time.ts`, que recebe um `Clock` injetado (`() => Instant`).
Justificativa: §5 exige fuso por regra e §6 exige comparar planejado × real. `Date` não carrega fuso e é mutável; espalhar `new Date()` pelo domínio torna todo teste dependente do relógio do sistema. O _brand_ impede misturar `Instant` com `Minutes` em tempo de compilação.
Alternativa rejeitada: `Temporal` nativo (ainda não disponível no WebView2 de forma confiável em 2026); strings ISO em todo lugar (a comparação lexicográfica funciona, mas não distingue `Instant` de `PlainDate`).
Consequência: `domain/time.ts` exporta `Instant.now(clock)`, `toPlainDate(instant, tz)`, `addMinutes`, `startOfDay`, `minutesBetween`. Toda a matemática de janela mora aqui.

**D10 — Formato das ocorrências: materializadas, não calculadas na leitura.**
Decisão: `event_occurrence` é uma **tabela real**, populada sob demanda para a janela visível (±1 semana em torno do intervalo pedido), com `UNIQUE(event_id, original_start)`. As instâncias com override (cancelada, remarcada, com registro) **sempre** existem, mesmo fora da janela.
Justificativa: §5 pede que uma ocorrência remarcada "mantenha referência à regra e à data original, permitindo histórico e sincronização correta com o Google Calendar". Se a ocorrência fosse calculada na leitura, o histórico da remarcação seria um evento separado e a identidade da instância se perderia — que é exatamente o par `recurringEventId` / `originalStartTime` que o Google usa. Materializar dá também um lugar para o `ExecutionRecord` se pendurar.
Alternativa rejeitada: expandir a RRULE a cada leitura e manter exceções em memória (a cada renderização da grade o semestre inteiro seria reexpandido; e o custo cresce com o histórico).
Consequência: comando `ensure_occurrences(window)` roda antes de `list_events_in_window`; é idempotente e roda dentro de uma transação.

**D11 — Onde mora o "atraso".**
Decisão: **nada é persistido.** `isOverdue(item, now)` é uma função pura em `domain/overdue.ts` que combina `commitment`, `state` e o prazo (INV-4). A Home e o Calendário chamam a mesma função.
Justificativa: §3 muda a semântica de atraso conforme o nível, e §4 diz que `cancelado` não pune. Persistir um booleano `overdue` congelaria o passado e violaria §13 — o item não está "atrasado", está em um estado que, combinado ao prazo, é lido como atraso agora.
Alternativa rejeitada: coluna `overdue INTEGER` mantida por job (duas fontes de verdade).
Consequência: única fonte é `domain/overdue.ts`; nada mais importa `now` para decidir atraso.

**D12 — `ExecutionRecord` é append-only.**
Decisão: `execution_record` só sofre `INSERT` e correção explícita (`corrected_by` apontando para o registro anterior). Nunca é apagado. `calendar_event.state` / `responsibility.state` refletem o **mais recente**.
Justificativa: §13 separa "o que realmente aconteceu" de "o que está marcado como feito". Se o registro fosse apagado ao corrigir, o histórico do que aconteceu também sumiria — e §6 diz que a diferença entre planejado e realizado alimenta histórico, sem punir.
Alternativa rejeitada: `execution_record` 1:1 com estado, com `UPDATE` in-place (perde o histórico).
Consequência: painel contextual (spec 02) mostra a cadeia; Home mostra só o último.

**D13 — `origin` é uma coluna, não um type discriminant.**
Decisão: `origin TEXT NOT NULL CHECK (origin IN ('cecistudy','faculdade','estudos','tcc','estagio','conhecimento','google'))`, presente em `calendar_event`, `responsibility` e `plan_block`. O tipo TypeScript tem `origin: Origin` em todos os três.
Justificativa: §9 diz que o Calendário "não cria cópia independente" — a autoria precisa ser consultável em query, não apenas em tempo de compilação. E `google` precisa ser distinguível de `cecistudy` **no banco** para que INV-6 seja garantida por uma restrição, não só por disciplina do código.
Alternativa rejeitada: união discriminada `type CalendarEvent = CecistudyEvent | GoogleEvent` (o dado persistido ainda precisa de coluna; e a união discriminada atrapalha a materialização de ocorrências).
Consequência: `INV-6` vira, no Rust, um `CHECK`/guard no caminho de escrita; a camada de binding nunca envia `origin: "google"` em payload de escrita.

**D14 — Ligação com módulo de origem: coluna `owner_ref` genérica.**
Decisão: `owner_type TEXT NULL` + `owner_id TEXT NULL`, com `CHECK ((owner_type IS NULL) = (owner_id IS NULL))`. Preenchidos pela spec 04. Sem FK, porque o módulo dono ainda não existe.
Justificativa: §9 exige que editar no Calendário **envie um comando ao módulo de origem** e não crie cópia. Isso precisa de uma referência estável agora e de um dispatcher depois; a FK não pode ser declarada antes de a tabela existir.
Alternativa rejeitada: criar aqui uma tabela `faculdade_disciplina` especulativa (seria código de outro módulo neste repositório, contra `AGENTS.md:5`).
Consequência: `domain/owner.ts` expõe `OwnerRef` e o contrato `CalendarCommandPort`, mas o dispatcher é da spec 04.

---

## 3. Modelo de domínio

### 3.1 Tipos (`src/features/calendar/domain/types.ts`)

```ts
export type Instant = number & { readonly __brand: "Instant" };
export type PlainDate = string & { readonly __brand: "PlainDate" };
export type Minutes = number & { readonly __brand: "Minutes" };
export type IanaTimeZone = string & { readonly __brand: "IanaTimeZone" };

export type Origin =
  "cecistudy" | "faculdade" | "estudos" | "tcc" | "estagio" | "conhecimento" | "google";

export type Commitment = "obrigatorio" | "importante" | "recomendado" | "opcional";

export type ItemState =
  | "planejado"
  | "em_andamento"
  | "concluido"
  | "adiado"
  | "nao_realizado"
  | "cancelado"
  | "dispensado";
```

Transições permitidas (a seta "→" significa "permitido"; nada mais é):

| De              | Para                                                                                 |
| --------------- | ------------------------------------------------------------------------------------ |
| `planejado`     | `em_andamento`, `concluido`, `adiado`, `nao_realizado`, `cancelado`, `dispensado`    |
| `em_andamento`  | `concluido`, `adiado`, `nao_realizado`, `cancelado`, `dispensado`                    |
| `adiado`        | `planejado`, `em_andamento`, `concluido`, `nao_realizado`, `cancelado`, `dispensado` |
| `nao_realizado` | `planejado`, `em_andamento`, `concluido`, `adiado`, `cancelado`, `dispensado`        |
| `concluido`     | `em_andamento` (reabrir)                                                             |
| `cancelado`     | `planejado` (reativar)                                                               |
| `dispensado`    | `planejado` (reconsiderar)                                                           |

Não há transição **de/para** lugar nenhum a partir de um estado que não está nesta lista. `cancelado` e `dispensado` não são finais: §4 diz que `cancelado` é "alteração externa" e `dispensado` é uma decisão da usuária que ela pode rever. §4 também implica que não há caminho de volta para um item cuja **regra** foi removida — isso é exclusão, não transição.

**Aceite e teste:** `npm run typecheck` → 0; teste unitário `assert(transitionAllowed("planejado","concluido"))` e `assert(!transitionAllowed("concluido","cancelado"))`; `rg "from:" src/features/calendar/domain/state-machine.ts` retorna exatamente as 7 linhas da tabela.

### 3.2 Entidades

| Entidade             | Campos próprios (além de `id`, `createdAt`, `updatedAt`)                                                                                                                                                                                                                                                                                                                                         |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `CalendarEvent`      | `layerId`, `title`, `notes?`, `startsAt: Instant`, `endsAt: Instant`, `allDay: boolean` \| `plainDate?: PlainDate`, `timeZone: IanaTimeZone`, `location?`, `commitment`, `state`, `origin`, `ownerRef?`                                                                                                                                                                                          |
| `RecurrenceRule`     | `eventId`, `rrule: string`, `dtstartTz: IanaTimeZone`, `until?: Instant`, `count?: number`, `exdates: Instant[]`                                                                                                                                                                                                                                                                                 |
| `Occurrence`         | `eventId`, `ruleId: CalendarEventId \| null` (null = evento avulso), `originalStart: Instant`, `startsAt: Instant` (pode diferir de `originalStart` se remarcada — §5), `endsAt: Instant`, `state`, `override: "none" \| "cancelled" \| "moved"`, `cancelReason?`                                                                                                                                |
| `Responsibility`     | `layerId`, `title`, `notes?`, `kind: "tarefa" \| "leitura" \| "pesquisa" \| "revisao" \| "escrita" \| "preparacao"`, `commitment`, `state`, `dueAt: Instant \| null` (D2), `plannedDuration: Minutes \| null`, `objective?`, `origin`, `ownerRef?`, `parentId: ResponsibilityId \| null` (prazo adicional → filha, D2)                                                                           |
| `ResponsibilityStep` | `responsibilityId`, `title`, `position: number`, `state: "pendente" \| "em_andamento" \| "concluida"`, `dueAt: Instant \| null`, `completedAt: Instant \| null`                                                                                                                                                                                                                                  |
| `PlanBlock`          | `responsibilityId` (**NOT NULL** — INV-1), `layerId`, `startsAt`, `endsAt`, `timeZone`, `plannedDuration: Minutes`, `state: "planejado" \| "em_andamento" \| "concluido" \| "nao_realizado"`, `origin`, `timerState: "parado" \| "rodando" \| "pausado"`, `accumulated: Minutes` (usado pelo timer na spec 03)                                                                                   |
| `ExecutionRecord`    | `target: { kind: "event" \| "occurrence" \| "responsibility" \| "block"; id: string }`, `startedAt: Instant`, `finishedAt: Instant`, `actualDuration: Minutes`, `result?: "concluido" \| "parcial"`, `notes?`, `source: "manual" \| "timer"`, `correctedBy: ExecutionRecordId \| null` (D12)                                                                                                     |
| `Layer`              | `id: LayerId`, `label`, `tone` (D4), `icon`, `visible: boolean`                                                                                                                                                                                                                                                                                                                                  |
| `ExternalLink`       | `entityType`, `entityId`, `system: "google"`, `remoteCalendarId`, `remoteEventId`, `remoteEtag?`, `remoteIcalUid?`, `lastSyncedAt`, `lastSeenRemoteUpdated?`, `lastChangeOrigin: "cecistudy" \| "google"`, `syncState: "sincronizado" \| "pendente" \| "conflito" \| "removido_remoto"`, `syncFields: string[]`, `conflictLocal?`, `conflictRemote?` (schema criado aqui, preenchido na spec 05) |
| `BlockSuggestion`    | `responsibilityId`, `layerId`, `startsAt`, `endsAt`, `rationale`, `score: number`, `state: "pendente" \| "aceita" \| "recusada" \| "expirada"`, `createdAt`, `resolvedAt: Instant \| null`, `producedBy: string` (schema criado aqui, preenchido na spec 06)                                                                                                                                     |

### 3.3 Schema SQLite (`src-tauri/migrations/0001_init.sql`)

Chaves: `id TEXT PRIMARY KEY` em todas (UUID v4 gerado em Rust, `uuid` com feature `v4`).
Horas: `INTEGER` epoch ms. `TEXT` para IANA. `INTEGER` 0/1 para booleanos.

```
calendar_event       (id, layer_id, title, notes, starts_at, ends_at, all_day, plain_date,
                      time_zone, location, commitment, state, origin, owner_type, owner_id,
                      created_at, updated_at)
  CHECK ((all_day = 1 AND plain_date IS NOT NULL) OR (all_day = 0 AND plain_date IS NULL))
  CHECK (ends_at >= starts_at)
  INDEX (starts_at, ends_at)
  INDEX (origin)

event_recurrence     (id, event_id UNIQUE, rrule, dtstart_tz, until_at, count, exdates_json)
event_occurrence     (id, event_id, rule_id, original_start, starts_at, ends_at, state,
                      override, cancel_reason, execution_id, created_at, updated_at)
  UNIQUE (event_id, original_start)
  INDEX (starts_at)

responsibility       (id, layer_id, title, notes, kind, commitment, state, due_at,
                      planned_duration_min, objective, origin, owner_type, owner_id,
                      parent_id, created_at, updated_at)
  INDEX (due_at, state)
responsibility_step  (id, responsibility_id, title, position, state, due_at, completed_at)
  UNIQUE (responsibility_id, position)
plan_block           (id, responsibility_id NOT NULL, layer_id, starts_at, ends_at, time_zone,
                      planned_duration_min, state, origin, timer_state, accumulated_min,
                      created_at, updated_at)
  CHECK (ends_at >= starts_at)                       -- INV-1 garantido pelo NOT NULL
execution_record     (id, target_kind, target_id, started_at, finished_at,
                      actual_duration_min, result, notes, source, corrected_by, created_at)
layer                (id, title, tone, icon, visible, position)
external_link        (id, entity_type, entity_id, system, remote_calendar_id, remote_event_id,
                      remote_etag, remote_ical_uid, last_synced_at, last_seen_remote_updated,
                      last_change_origin, sync_state, sync_fields_json,
                      conflict_local_json, conflict_remote_json)
  UNIQUE (system, remote_calendar_id, remote_event_id)
suggestion           (id, responsibility_id, layer_id, starts_at, ends_at, rationale, score,
                      state, produced_by, created_at, resolved_at)
setting              (key PRIMARY KEY, value_json)
sync_cursor          (system, calendar_id, sync_token, last_full_sync_at, PRIMARY KEY (system, calendar_id))
```

`seed` (Fase 1.4) insere as 6 linhas de `layer` de D4 e um usuário-âncora não é criado (o app é mono-usuário — D5).

**Aceite e teste:** `rg "responsibility_id TEXT NOT NULL" src-tauri/migrations/0001_init.sql` → 1 ocorrência; `rg "responsibility" src-tauri/migrations/0001_init.sql` **na tabela `calendar_event`** → nenhuma (INV-2); `rg "CREATE TABLE" src-tauri/migrations/0001_init.sql` → **12**.

### 3.4 Comandos Rust (`src-tauri/src/commands.rs`)

| Comando                                | Assinatura (TypeScript em `src/lib/ipc.ts`)                                                                             | Transação                                                                  |
| -------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| `migrate`                              | `(): Promise<{ version: number }>`                                                                                      | sim                                                                        |
| `list_events_in_window`                | `(w: Window): Promise<GridItem[]>`                                                                                      | sim — **materializa com `expand_rule` e lê na mesma transação** (D10)      |
| `list_responsibilities`                | `(f: { dueBefore?: Instant \| null; state?: ItemState \| null; layerId?: LayerId \| null }): Promise<Responsibility[]>` | não                                                                        |
| `create_event`                         | `(d: UpsertEvent): Promise<CalendarEvent>`                                                                              | sim                                                                        |
| `update_event`                         | `(d: { id; patch: Partial<UpsertEvent> }): Promise<CalendarEvent>`                                                      | sim                                                                        |
| `delete_event`                         | `({ id }): Promise<void>`                                                                                               | sim (cascateia ocorrência e regra; **não** apaga `execution_record` — D12) |
| `set_occurrence_state`                 | `({ id; state; reason? }): Promise<Occurrence>`                                                                         | sim (INV-3 e INV-5)                                                        |
| `move_occurrence`                      | `({ id; startsAt; endsAt }): Promise<Occurrence>`                                                                       | sim (grava `override = "moved"`, preserva `original_start` — §5)           |
| `upsert_recurrence`                    | `(d: { eventId; rrule; dtstartTz; untilAt?; count?; exdates? }): Promise<void>`                                         | sim (recusa `origin = "google"`; `until_at`/`count` vencem a string)       |
| `update_recurrence`                    | `({ ruleId; rrule; scope: "serie" \| "esta_ocorrencia" \| "esta_e_seguintes"; occurrenceId? }): Promise<void>`          | sim (INV-5: `scope ≠ "serie"` nunca toca `event_recurrence`)               |
| `create_responsibility`                | `(d: UpsertResponsibility): Promise<Responsibility>`                                                                    | sim                                                                        |
| `set_step_state`                       | `({ id; state }): Promise<ResponsibilityStep>`                                                                          | sim                                                                        |
| `plan_block`                           | `(d: UpsertBlock): Promise<PlanBlock>`                                                                                  | sim (INV-1)                                                                |
| `reschedule_block`                     | `({ id; startsAt; endsAt }): Promise<PlanBlock>`                                                                        | sim                                                                        |
| `complete_item`                        | `({ target; record }): Promise<void>`                                                                                   | sim — **exige `record`**, aplica INV-3                                     |
| `record_execution`                     | `({ target; record }): Promise<ExecutionRecord>`                                                                        | sim                                                                        |
| `list_layers` / `set_layer_visibility` | `(): Promise<Layer[]>` / `({ id; visible })`                                                                            | não                                                                        |
| `get_setting` / `set_setting`          | `({ key })` / `({ key; value })`                                                                                        | não                                                                        |

**Nenhum comando aceita SQL, nome de tabela ou string de consulta.** Todos os parâmetros são structs `serde` com campos nomeados.

**Aceite e teste:** `rg "fn " src-tauri/src/commands.rs` lista exatamente os 19 comandos acima e nada que aceite `&str` de SQL; `rg "invoke_handler" src-tauri/src/lib.rs` mostra o `generate_handler!` com os mesmos nomes; `rg "SELECT|INSERT|UPDATE|DELETE" src/lib/ipc.ts` → vazio.

### 3.5 Recorrência (`domain/recurrence.ts`)

`expandRule(rule: RecurrenceRule, window: { from: Instant; to: Instant }): OccurrenceDraft[]`, usando `rrule` com `RRule.parseString(rule.rrule)` e `dtstart` derivado de `dtstartTz`. Exclusões: `exdates` são aplicadas como `exdate` na-options do `rrule`, não como filtro posterior — assim o índice de ocorrência original nunca desloca.

`ensureOccurrences(event, rule, window)` grava as instâncias faltantes com `override = "none"`, `state = "planejado"`, `rule_id = rule.id`. É idempotente pelo `UNIQUE(event_id, original_start)`.

**Aceite e teste:** teste unitário com `RRULE:FREQ=WEEKLY;BYDAY=TU,TH;COUNT=10` numa janela de 3 semanas retorna 6 ocorrências; reexecutar `ensureOccurrences` sobre a mesma janela **não** duplica (o `UNIQUE` segura e o teste TS simula com um stub que respeita a constraint); um `EXDATE` no meio produz 9, não 10, e **não** renumera as seguintes.

### 3.6 Atraso (`domain/overdue.ts`) — INV-4

```ts
export function isOverdue(
  c: { commitment: Commitment; state: ItemState; dueAt: Instant | null },
  now: Instant,
): boolean;
```

Retorna `true` **somente** se `dueAt !== null`, `dueAt < now`, `commitment` ∈ {`obrigatorio`, `importante`} e `state` ∈ {`planejado`, `em_andamento`, `adiado`, `nao_realizado`}. `cancelado`, `dispensado` e `concluido` nunca atrasam, mesmo com prazo vencido.

**Aceite e teste:** tabela-driven, 7 estados × 4 níveis × `dueAt` vencido/nulo = 56 asserções; `rg "isOverdue" src` → só `domain/overdue.ts` e quem consome; nenhum outro arquivo compara `dueAt < now`.

### 3.7 Sobreposição (`domain/overlap.ts`) — D7

`findOverlaps<T extends { startsAt: Instant; endsAt: Instant }>(items: T[]): Array<[T, T[]]>` agrupa pares que se intersectam. É derivado, não persistido, e não bloqueia nada.

**Aceite e teste:** `[a(08–10), b(09–11), c(11–12)]` → um grupo `{a,b}` e `c` isolado; `[a(08–10), b(10–12)]` → **nenhum** grupo (intervalos que só se tocam não conflitam).

---

## 4. Bloqueios

### 4.1 Toolchain Rust (diagnóstico verificado em 2026-09-29)

O bloqueio original desta spec dizia apenas "MSVC ausente" e sugeria que `rusqlite` (C) era o
único problema. Isso está incompleto. O que a máquina mostra:

1. `rustup default` = `stable-x86_64-pc-windows-msvc`, sem MSVC Build Tools → `link.exe not found`.
2. **`cargo check` também falha.** Build scripts e crates de proc-macro são compilados **e
   linkados para o host**, então o linker do host é necessário mesmo em `check` — não há
   verificação possível sem um compilador C funcional.
3. Adicionar o _target_ `x86_64-pc-windows-gnu` **não** resolve, porque os build scripts
   continuam sendo linkados no host `msvc`.
4. Instalar o toolchain **host** `stable-x86_64-pc-windows-gnu` resolve (3), mas esbarra em
   `lld: error: unable to find library -lgcc_eh`: o único compilador C presente é o
   **llvm-mingw**, que fornece `libunwind` e não `libgcc`/`libgcc_eh`.
5. Piso medido: crate sem dependências, sem proc-macro e sem build script **passa** em
   `cargo check` sem linker. A barreira é o conjunto de crates com build script — o que inclui
   `libsqlite3-sys` e o `serde` derive.

**Destrava com:** (a) _Visual Studio Build Tools 2022_ + "Desktop development with C++" — o
caminho recomendado, é o alvo que o Tauri suporta melhor; ou (b) um mingw-w64 **completo**
(MSYS2 GCC ou WinLibs) com o toolchain `stable-x86_64-pc-windows-gnu` como host. O llvm-mingw
não serve.

- [x] **Fases 1.1 a 1.3** — verificadas com `node --test` e `tsc`, sem Rust.
- [x] **Fases 1.4 a 1.9** — código escrito; SQL validado contra engine real (ver 4.2). Rust
      **não compilado**: os testes de `db.rs` e `commands.rs` (22 testes) só rodam depois do
      destravamento.
- [x] **Aceite da Fase 1.9** — a guarda de `DEV` aparece uma vez, e nenhum vestígio da semente
      (`seedIfEmpty`, dados do evento de exemplo) chega a `dist/assets`.
- [ ] **Aceite §7.8** — exige `npm run tauri:dev` de pé.

### 4.2 O que foi verificado sem Rust

O `node:sqlite` do Node 26 é um engine SQLite real (3.53.1 nesta máquina). Ele foi usado para
executar as duas migrations e checar, por asserção, o que normalmente só se veria subindo o app:

- 12 tabelas, 15 índices, e as migrations rodam em sequência sem erro;
- INV-1 — `plan_block` sem `responsibilityId` é rejeitado;
- INV-2 — `calendar_event` não tem coluna de responsabilidade;
- D10 — `UNIQUE (event_id, original_start)` segura a idempotência da materialização;
- D12 — apagar o evento **não** apaga `execution_record`;
- D13 — `origin` fora da lista é rejeitado; `google` é recusado no caminho de escrita Rust;
- D14 — `owner_type` sem `owner_id` é rejeitado;
- §5 — remarcar preserva `original_start` e a duração;
- `layer.tone` só aceita `1..5` ou `neutral`, que é o que `domain/layers.ts` usa.

### 4.3 Imprecisões encontradas na própria spec

Duas correções de aceitação, encontradas implementando:

1. **`update_recurrence` não consegue identificar a ocorrência.** A assinatura de §3.4 é
   `({ ruleId; rrule; scope })`, mas "esta ocorrência" só tem sentido com a ocorrência em mãos.
   Acrescentado `occurrenceId`, obrigatório nos escopos locais — o Rust recusa em vez de adivinhar.
2. **As contagens de aceite com `rg` estão erradas.** `responsibility_id TEXT NOT NULL` aparece
   **3** vezes, não 1: `responsibility_step`, `plan_block` e `suggestion` — e nos três é a decisão
   certa. O critério deve mirar o bloco de `plan_block`. E `bundled` aparece 2 vezes em
   `Cargo.toml` porque o comentário menciona a feature; a dependência é 1.
3. **A recorrência não tinha comando de criação, e a materialização estava no lado errado.**
   A tabela só tinha `update_recurrence`, então uma série não podia _nascer_ — e a Fase 1.9 pede
   justamente "1 regra semanal". Pior: `§3.4` atribuía `ensure_occurrences` ao comando de
   leitura, mas `domain/recurrence.ts` é a decisão de domínio e o comando Rust só tem onde
   gravá-la. As duas coisas foram resolvidas juntas: `upsert_recurrence` entra como 20º comando, e
   a materialização (`materialize_occurrences` + `expand_rule`) passa a viver em `commands.rs`,
   dentro da transação da leitura. Isso é coerente com `AGENTS.md`, que agora determina que a
   verdade de domínio é o Rust e que o espelho TypeScript existe para render e para testar sem o
   toolchain — divergindo, o Rust está certo.

- [x] **Clientes npm.** `rrule` e `date-fns` instalados.
- [ ] `Cargo.lock` precisa ser regravado com `rusqlite` e `uuid`, o que exige o item 4.1.

---

## 5. Fases de implementação

### Fase 1.1 — Vocabulário de tempo

| Ação  | Detalhe                                                                                                  |
| ----- | -------------------------------------------------------------------------------------------------------- |
| Criar | `src/features/calendar/domain/time.ts` — `Instant`, `PlainDate`, `Minutes`, `IanaTimeZone`, `Clock` (D9) |
| Criar | `src/features/calendar/domain/time.test.ts`                                                              |

Funções: `now(clock)`, `toPlainDate(instant, tz)`, `startOfDay(instant, tz)`, `addMinutes`, `minutesBetween`, `addDaysInTz`. Usa `date-fns` + `Intl.DateTimeFormat` (já disponível no WebView2) para a conversão com fuso — **não** `date-fns-tz`, que seria uma dependência extra.

**Aceite e teste:** `npm run typecheck` → 0; `node --test src/features/calendar/domain/time.test.ts` verde, cobrindo: `toPlainDate` de um instante às 23:30 em `America/Sao_Paulo` retorna o dia **anterior** se o instante já passou da meia-noite UTC; `minutesBetween(10:00, 08:00)` = 120 (sinal, não absoluto).

### Fase 1.2 — Tipos, estados e níveis

| Ação  | Detalhe                                                                           |
| ----- | --------------------------------------------------------------------------------- |
| Criar | `src/features/calendar/domain/types.ts` (§3.2)                                    |
| Criar | `src/features/calendar/domain/state-machine.ts` com `transitionAllowed(from, to)` |
| Criar | `src/features/calendar/domain/overdue.ts` (INV-4) + `.test.ts`                    |
| Criar | `src/features/calendar/domain/overlap.ts` (D7) + `.test.ts`                       |

**Aceite e teste:** `npm run typecheck` → 0 (com `noUncheckedIndexedAccess` e `exactOptionalPropertyTypes` ativos, sem `!` não justificado); `npm run lint` → `0 problems`; `node --test` verde em `overdue.test.ts` (56 casos) e `overlap.test.ts`.

### Fase 1.3 — Recorrência e materialização

| Ação  | Detalhe                                                                                   |
| ----- | ----------------------------------------------------------------------------------------- |
| Criar | `src/features/calendar/domain/recurrence.ts` com `expandRule` e `ensureOccurrences` (D10) |
| Criar | `src/features/calendar/domain/recurrence.test.ts`                                         |

**Aceite e teste:** `node --test src/features/calendar/domain/recurrence.test.ts` verde com: (a) semanal `BYDAY` em janela parcial; (b) `COUNT`; (c) `UNTIL`; (d) `EXDATE` no meio **sem** renumerar; (e) recorrência ancorada em `America/Sao_Paulo` mantida estável quando o `Clock` está em UTC; (f) `ensureOccurrences` duas vezes → mesmo conjunto.

### Fase 1.4 — Schema e driver

| Ação   | Detalhe                                                                                                                           |
| ------ | --------------------------------------------------------------------------------------------------------------------------------- |
| Criar  | `src-tauri/migrations/0001_init.sql` (§3.3)                                                                                       |
| Criar  | `src-tauri/src/db.rs` — abre a conexão no `app_data_dir`, roda migrações versionadas em transação                                 |
| Editar | `src-tauri/Cargo.toml` — `rusqlite = { version = "0.3x", features = ["bundled"] }`, `uuid = { version = "1", features = ["v4"] }` |
| Editar | `src-tauri/src/lib.rs` — `tauri::Manager::path().app_data_dir()` → `<dir>/campus.sqlite`                                          |

**Aceite e teste:** `rg "user_version" src-tauri/src/db.rs` (controle de versão no PRAGMA); `rg "bundled" src-tauri/Cargo.toml` → 1; o arquivo `.sqlite` criado em `%APPDATA%/com.campus.desktop/campus.sqlite` tem **12 tabelas** (`calendar_event`, `event_recurrence`, `event_occurrence`, `responsibility`, `responsibility_step`, `plan_block`, `execution_record`, `layer`, `external_link`, `suggestion`, `setting`, `sync_cursor`) — **verificação manual, bloqueada sem MSVC**; `npm run typecheck` e `npm run lint` continuam em 0 (nenhum arquivo de `src/` mudou).

### Fase 1.5 — Comandos de leitura

| Ação   | Detalhe                                                                                                                          |
| ------ | -------------------------------------------------------------------------------------------------------------------------------- |
| Criar  | `src-tauri/src/commands.rs` — `migrate`, `list_layers`, `set_layer_visibility`, `list_events_in_window`, `list_responsibilities` |
| Editar | `src-tauri/src/lib.rs` — `.invoke_handler(tauri::generate_handler![…])`                                                          |
| Criar  | `src/lib/ipc.ts` — funções tipadas `migrate`, `listLayers`, `listEventsInWindow`, `listResponsibilities` + guarda de ambiente    |
| Criar  | `src/features/calendar/data/*.ts` — repositórios que chamam `ipc.ts` e traduzem snake_case ↔ camelCase                           |

**Aceite e teste:** `rg "generate_handler" src-tauri/src/lib.rs` lista 5 comandos; `rg "snake|case" src/features/calendar/data` não existe — a tradução é explícita e testável; `npm run typecheck` → 0; `npm run lint` → `0 problems`; `npm run dev` no browser puro não quebra, porque `isIpcAvailable()` devolve `false` e o repositório devolve lista vazia.

### Fase 1.6 — Comandos de escrita de evento e ocorrência

| Ação   | Detalhe                                                                                                                                      |
| ------ | -------------------------------------------------------------------------------------------------------------------------------------------- |
| Editar | `src-tauri/src/commands.rs` — `create_event`, `update_event`, `delete_event`, `set_occurrence_state`, `move_occurrence`, `update_recurrence` |
| Criar  | `domain/commands.ts` — `applyItemCommand(cmd: ItemCommand): Promise<void>`, único ponto de entrada de escrita de §4                          |

`ItemCommand` é a união discriminada de §4; cada ramo tem um teste de transição negativa.

**Aceite e teste:** teste unitário em `commands.test.ts` com repositório em memória: `applyItemCommand({ kind: "set_state", to: "concluido" })` **sem** `record` retorna erro; com `record` grava os dois (INV-3); `update_recurrence` com `scope: "esta_ocorrencia"` deixa `event_recurrence` **byte-idêntica** (INV-5); `move_occurrence` mantém `original_start` e muda `starts_at` (§5). `npm run typecheck` e `npm run lint` em 0.

### Fase 1.7 — Comandos de responsabilidade, bloco e execução

| Ação   | Detalhe                                                                                                                                        |
| ------ | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Editar | `src-tauri/src/commands.rs` — `create_responsibility`, `set_step_state`, `plan_block`, `reschedule_block`, `complete_item`, `record_execution` |
| Criar  | `domain/responsibility.ts` — `progressOf(r)`, `childDeadlines(r)`, validação de D2 (um prazo principal)                                        |

**Aceite e teste:** `npm run typecheck` → 0; teste: `create_responsibility` rejeita payload com `dueAt` **e** `deadlines` (D2 — o tipo nem compila, e o teste documenta isso); `plan_block` sem `responsibilityId` é rejeitado pelo `NOT NULL` (INV-1); `complete_item` sem `record` é rejeitado (INV-3); `execution_record` não é apagado por `delete_event` (D12); `npm run lint` → `0 problems`.

### Fase 1.8 — Ligar o App ao repositório (retirar o mock)

| Ação   | Detalhe                                                                                                                         |
| ------ | ------------------------------------------------------------------------------------------------------------------------------- |
| Criar  | `src/features/calendar/ui/CalendarScreen.tsx` — **wrapper fino**; só escolhe provider e visões, delega às specs 02/03           |
| Editar | `src/App.tsx:346` — `<CalendarView setView={setView} />` passa a `<CalendarScreen onOpenList={() => setView("list")} />`        |
| Editar | `src/App.tsx:709-758` — remover `calendarEvents` e `weekDays`; `CalendarView` (`src/App.tsx:791-866`) é removido                |
| Criar  | `src/features/calendar/ui/CalendarScreen.stories` não existe; em vez disso um estado `vazio` com mensagem e ação "Criar evento" |

Este é o **único** ponto onde `src/App.tsx` para de ser mock no Calendário. `type View` (`src/App.tsx:29`) e o switch (`src/App.tsx:337-349`) **não mudam** — `AGENTS.md:5,9` proíbe roteador, e a spec 02 usa `View` como está.

**Aceite e teste:** `rg "calendarEvents" src/App.tsx` → vazio; `rg "CalendarView" src/App.tsx` → vazio; `rg "type View = " src/App.tsx` → **inalterado**; `npm run typecheck` → 0; `npm run lint` → `0 problems`; `npm run dev` mostra a tela do Calendário com estado vazio real (a grade visual vem na spec 02; aqui um `Panel` com texto e botão já é o aceite).

### Fase 1.9 — Semente de desenvolvimento

| Ação  | Detalhe                                                                                                                                                    |
| ----- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Criar | `src-tauri/migrations/0002_seed_layers.sql` — as 6 linhas de `layer` de D4                                                                                 |
| Criar | `src/features/calendar/data/dev-seed.ts` — cria 1 responsabilidade, 1 regra semanal e 1 bloco, **só** quando `import.meta.env.DEV` e o banco estiver vazio |

**Aceite e teste:** `rg "import.meta.env.DEV" src/features/calendar/data/dev-seed.ts` → 1; `npm run build` (que roda `tsc --noEmit && vite build`) não inclui a semente no bundle de produção — verificável com `rg "dev-seed" dist/assets` → vazio; `npm run typecheck` → 0.

---

## 6. Resumo de arquivos

**Criar**
`src/features/calendar/domain/{time,types,state-machine,overdue,overlap,recurrence,responsibility,commands,owner}.ts` + os `.test.ts` correspondentes · `src/features/calendar/data/{event-repository,responsibility-repository,block-repository,execution-repository,layer-repository,dev-seed}.ts` · `src/features/calendar/ui/CalendarScreen.tsx` · `src-tauri/migrations/{0001_init,0002_seed_layers}.sql` · `src-tauri/src/{db,commands}.rs`

**Modificar**
`src/lib/ipc.ts` · `src/App.tsx` (só `:346` e a remoção de `:708-758` + `:791-866`) · `src-tauri/src/lib.rs` · `src-tauri/Cargo.toml` · `src-tauri/Cargo.lock`

**Não tocar**
`src/styles.css` · `src-tauri/tauri.conf.json` · `AGENTS.md` (já feito na spec 00)

---

## 7. Critérios de aceite finais

1. `npm run typecheck` → 0 e `npm run lint` → `0 problems` em **todas** as fases.
2. `node --test src/features/calendar/domain/` → verde, cobrindo as 56 asserções de INV-4, a transição negativa de `concluido → cancelado`, o `EXDATE` sem renumerar e a âncora de fuso.
3. `rg "SELECT|INSERT|UPDATE|DELETE" src/lib/ipc.ts` → vazio: nenhum SQL atravessa a ponte.
4. `rg "calendarEvents|CalendarView" src/App.tsx` → vazio; `rg "type View = " src/App.tsx` → inalterado.
5. `rg "responsibility" src-tauri/migrations/0001_init.sql` dentro do bloco `CREATE TABLE calendar_event` → nada (INV-2).
6. `rg "text-chart-4" src` → vazio ( herdado da spec 00).
7. `npm run dev` funciona no browser puro sem erro de console — `isIpcAvailable()` é `false` e a tela mostra o estado vazio.
8. **[bloqueado até resolver §4]** `npm run tauri:dev` abre o app, cria `<dir>/campus.sqlite` com as 12 tabelas de §3.3, e o seed aparece na tela.

---

## 8. Fora de escopo

- Toda a UI da grade: arrasto, redimensionamento, painel contextual, alternância dia/mês/agenda — spec 02.
- Timer em execução, registro de execução pela interface, agenda mobile, edição rápida — spec 03.
- Comandos de retorno aos módulos de origem e o `CalendarCommandPort` implementado — spec 04 (aqui só a coluna `owner_ref` e o tipo).
- OAuth, `reqwest`, sincronização, resolução de conflito, preenchimento de `external_link` — spec 05 (aqui só o schema).
- A regra que produz `BlockSuggestion` e a UI aceitar/ajustar/recusar — spec 06 (aqui só o schema).
- Views diária, mensal e de agenda/lista do Calendário — spec 02.
- Home (`TodayView`, `src/App.tsx:476-706`) consumir o Calendário: §1 diz que a Home destaca pendências, mas **esta spec não muda a Home**. O repovoamento da Home com `isOverdue` é item de backlog após a spec 02.
- Busca global ⌘K (`src/App.tsx:354-408`) indexando o Calendário — backlog.
- Notificações / lembretes de prazo — `roadmap.md` não lista; fora do conjunto.
