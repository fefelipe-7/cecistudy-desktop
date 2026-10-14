# Spec 04 — Fronteira com os módulos de origem

Data: 2026-09-28
Escopo: definir o contrato entre o Calendário e os módulos que ele projeta (Faculdade, Estudos, TCC, Estágio, Base de Conhecimento), garantir que editar no Calendário **envie um comando ao módulo dono** em vez de criar cópia, e entregar um adaptador de referência que rode sem que nenhum desses módulos exista.
Status: proposta fechada, **parcialmente bloqueada** — os adaptadores reais dependem de código que não está neste repositório.

**Rastreabilidade — `calendario.md`:** §2 (bloco não cria obrigação; vínculo com a origem), §3 (a origem propõe, a usuária aceita), §7 (a camada é um filtro, não um calendário separado), §9 (integral), §10 (a 3ª linha da tabela — responsabilidade ligada a evento externo sem assumir propriedade), §13.
**Entregas de `calendario.md` §11 cobertas:** **5** (provas, apresentações, tarefas e prazos vindos de Faculdade) — **parcialmente**: a spec entrega o contrato, o adaptador de referência e o caminho de comando de volta; o adaptador de verdade da Faculdade depende de código que não existe (§4 BLOQUEIO).
**Depende de:** spec 00 (A7, A8, D2, D4, D13, D14) e spec 01 (domínio, `owner_ref`, `origin`).
**Deixa deliberadamente para:**

- spec 05 — Google é uma **origem** como as outras quanto ao contrato, mas tem dono externo, então ganha spec própria; aqui só fica registrado que `google` **não** é um módulo de origem e **não** implementa `CalendarCommandPort`;
- spec 06 — a origem sugere um bloco; o contrato de sugestão é da spec 06, e a `CalendarCommandPort` aceita a chamada, mas o gerador é dela;
- spec 02/03 — a interface (painel, ações) é delas; aqui só o contrato e um painel de diagnóstico em modo dev.

---

## 1. Diagnóstico do estado atual

| Requisito                                                         | Onde encosta hoje                                                             | Situação                                                      |
| ----------------------------------------------------------------- | ----------------------------------------------------------------------------- | ------------------------------------------------------------- |
| §9 Faculdade: aulas, provas, apresentações, trabalhos, prazos     | —                                                                             | Ausente — **o módulo Faculdade não existe neste repositório** |
| §9 Estudos: blocos de foco, revisão, leitura, questões, registros | —                                                                             | Ausente — módulo não existe                                   |
| §9 TCC: marcos, etapas, reuniões, entregas, blocos de escrita     | —                                                                             | Ausente — módulo não existe                                   |
| §9 Conhecimento: sugestões, `.cectx`                              | —                                                                             | Ausente — módulo não existe                                   |
| §9 Estágio: supervisão                                            | `CalendarView` mostra "Eng. de Software" e "Sala 306" (`src/App.tsx:750-757`) | Ausente como camada; só um item de mock sem dono              |
| §9 "edite no Calendário → comando ao módulo de origem"            | —                                                                             | Ausente                                                       |
| §9 "não cria cópia independente"                                  | —                                                                             | Ausente                                                       |
| §7 camadas como filtro, não calendários                           | `subjects[].dot` por disciplina (`src/App.tsx:33-59`)                         | Parcial — cor por disciplina, sem noção de camada             |
| §2 vínculo com a origem                                           | —                                                                             | Ausente                                                       |

**Fato verificado e decisivo:** `src/` contém apenas `App.tsx`, `main.tsx`, `styles.css`, `components/ErrorBoundary.tsx`, `hooks/use-mobile.tsx`, `lib/utils.ts` e `components/ui/` (18 arquivos). Não há pasta, arquivo, tipo ou constante que pertença a Faculdade, Estudos, TCC, Estágio ou Base de Conhecimento. Nada em `PRODUCT.md`, `DESIGN.md` ou `roadmap.md` promete esses módulos em curto prazo.

Consequência: **esta spec não pode entregar a integração real.** O que ela entrega, e o que é a parte difícil de qualquer jeito, é o **contrato bidirecional** e a prova de que o Calendário não duplica conteúdo. Os adaptadores reais são implementações desse contrato em repos que ainda não existem.

---

## 2. Decisões fechadas

### 2.1 Herdadas

A7 · A8 (INV-1, INV-2, INV-7) · D2 (um prazo principal) · D4 (`LayerId` fixo) · D13 (`origin` é coluna) · D14 (`owner_type` + `owner_id`).

### 2.2 Novas

**D27 — O contrato é uma porta de entrada, registrada em um registro de adaptadores; não há importação cruzada entre módulos.**
Decisão: `src/features/calendar/domain/owner.ts` define a interface `CalendarCommandPort` e o registro `registerAdapter(layerId, adapter)`. O Calendário **nunca** importa Faculdade, Estudos, TCC ou Conhecimento. A direção é invertida: cada módulo, ao existir, importa o contrato e se registra.
Justificativa: §9 diz que cada módulo é dono do seu conteúdo e que o Calendário envia um comando ao dono. Isso só é possível se o contrato morar **no Calendário** (que é quem precisa dele) e a dependência apontar no sentido contrário. Importar os módulos para o Calendário criaria o acoplamento que §9 quer evitar e colocaria código de quatro módulos dentro de `src/features/calendar/`.
Alternativa rejeitada: um barramento de eventos global (torna o efeito de uma edição de prova **indireto** e não testável; a §9 pede comando, não evento). Um `Map<LayerId, Adapter>` importando os módulos (acopla).
Consequência: `src/features/calendar/domain/owner.ts` não importa nada além dos tipos. `rg "faculdade|estudos|tcc|conhecimento" src/features/calendar/domain` → apenas como valores de `LayerId` e `Origin`, nunca como import.

**D28 — Projeção (módulo → Calendário) é por adição, e é idempotente por `owner_ref`.**
Decisão: cada adaptador expõe `project(cursor: string | null): Promise<ProjectionPage>` devolvendo itens com `ownerRef`, e o Calendário faz `upsert` por `owner_ref` dentro de uma transação. Rodar a projeção duas vezes com o mesmo cursor não muda nada. `cursor` é opaco para o Calendário e guardado em `setting` com a chave `projection:<layerId>`.
Justificativa: §9 lista "entrada automática **ou vinculável**". Um `upsert` por chave idempotente cobre os dois casos com uma só mecânica e torna re-projeção segura, inclusive depois de uma falha no meio.
Alternativa rejeitada: espelhar tudo para uma tabela local e comparar (duplica dados e quebra §9 linha 2 — "não cria cópia independente").
Consequência: a projeção nunca apaga item local; para despublicar, o adaptador devolve `withdrawn: true` e o Calendário marca `owner_state = "retirado"` **sem** apagar `execution_record` (D12, spec 01).

**D29 — Comando de volta é always um comando nomeado, com escopo explícito, e falha alta.**
Decisão: `CalendarCommandPort` tem exatamente três métodos: `move(id, when, scope): Promise<void>`, `patch(id, fields, scope): Promise<void>`, `cancel(id, scope): Promise<void>`, onde `scope ∈ {ocorrencia, serie}`. Não há método genérico `apply(command: unknown)`.
Justificativa: §9 dá exatamente três operações ("Mover uma prova atualiza a prova; alterar uma etapa do TCC atualiza o marco ou etapa; cancelar uma aula altera somente a ocorrência"). `scope` é o que garante a §4 — "a alteração deve ocorrer na ocorrência específica por padrão; a regra recorrente permanece intacta". Um método genérico devolveria o controle do `scope` a cada implementação, e alguém erraria.
Alternativa rejeitada: `apply(command)` genérico (perde o `scope` garantido; impossível de testar INV-5 no nível do contrato).
Consequência: `scope = "serie"` **só** é aceito por adaptadores que declaram `supportsSeries: true`; caso contrário, o retorno é erro e a UI oferece só "esta ocorrência".

**D30 — Sem adaptador registrado, a edição é bloqueada na interface com uma explicação, não silenciosamente aceita.**
Decisão: `item-menu.ts` (spec 02) recebe `hasAdapter: boolean` no contexto. Item com `origin ≠ "cecistudy"` e **sem** adaptador registrado mostra os campos em somente leitura e um aviso: "Este item pertence a {rótulo da camada}. O módulo ainda não está conectado no Campus." Nenhum botão de mover/editar aparece.
Justificativa: §9 e §13. Prometer uma edição que vai ser descartada é pior que não oferecer. E §12.7/§3 exigem que o sistema não crie obrigação nem mudança sem ação explícita da usuária — aceitar a edição e perder em silêncio violaria isso.
Alternativa rejeitada: gravar localmente e tentar sincronizar depois (é exatamente a "cópia independente" que §9 proíbe).
Consequência: `rg "hasAdapter" src/features/calendar/ui` → presente; o aviso é observável.

**D31 — Responsabilidade ligada a item externo é uma entidade nova do cecistudy, nunca uma edição do item externo.**
Decisão: `ownerRef` pode apontar para um `CalendarEvent` externo (só para `google`, ver spec 05) **ou** para uma entidade de módulo. Quando o alvo é externo, "criar responsabilidade para este compromisso" cria uma `Responsibility` nova com `ownerRef` = o item, e **não** escreve nada no item. Quando o alvo é interno de um módulo, a responsabilidade pode ser criada **no módulo**, por comando.
Justificativa: §10, 3ª linha e §9: "não cria uma cópia independente" vale para o _conteúdo_ do módulo, mas §10 pede explicitamente "o cecistudy cria uma responsabilidade própria ligada ao evento, sem modificar o evento do Google". São duas coisas diferentes, e confundi-las destrói a propriedade de edição.
Alternativa rejeitada: sempre delegar ao módulo dono (impossível para Google, que não é um módulo nosso).
Consequência: `create_responsibility` da spec 01 aceita `ownerRef` opcional; o comando `move` **nunca** é enviado ao Google (D27 — Google não registra adaptador).

---

## 3. O contrato

```ts
// src/features/calendar/domain/owner.ts — TypeScript puro, sem React, sem ipc.

export type AdapterScope = "ocorrencia" | "serie";

export interface CalendarCommandPort {
  readonly layerId: LayerId;
  readonly label: string;
  /** Se o dono sabe aplicar mudanças a uma série inteira. */
  readonly supportsSeries: boolean;
  move(id: string, startsAt: Instant, endsAt: Instant, scope: AdapterScope): Promise<void>;
  patch(id: string, fields: Readonly<Record<string, unknown>>, scope: AdapterScope): Promise<void>;
  cancel(id: string, scope: AdapterScope): Promise<void>;
}

export interface ProjectedItem {
  ownerRef: { type: string; id: string };
  kind: "evento" | "responsabilidade" | "etapa" | "bloco";
  payload: CalendarEventInput | ResponsibilityInput;
  withdrawn?: boolean;
}

export interface OriginAdapter {
  readonly layerId: LayerId;
  project(cursor: string | null): Promise<{ items: ProjectedItem[]; nextCursor: string }>;
  command: CalendarCommandPort;
}

export const adapterRegistry: Map<LayerId, OriginAdapter>;
export function registerAdapter(a: OriginAdapter): void;
export function hasAdapter(layerId: LayerId): boolean;
```

O registro é um `Map` do ES nativo — sem store global, sem contexto, sem provider. `src/features/calendar/ui` é quem chama `registerAdapter` no `useEffect` de montagem, e quem faz `adapterRegistry.get(layerId)`.

### 3.1 Adaptador de referência (dados locais, substitui os 4 módulos ausentes)

`src/features/calendar/data/reference-adapter.ts` implementa `OriginAdapter` para as 5 camadas internas (`faculdade`, `estudos`, `tcc`, `estagio`, `conhecimento`) sobre uma lista de literais em memória — o mesmo lugar onde hoje vivem `subjects` e `tasks` (`src/App.tsx:33-90`).

Ele existe por três motivos:

1. **Verificar D27, D28, D29 e D30 de verdade**, sem esperar os módulos;
2. Dar à spec 02 algo para mostrar além de `cecistudy` — a camada `google` continua vazia até a spec 05;
3. Ser a referência de implementação que os módulos reais vão copiar.

Ele **não** é a integração. Fica atrás de `import.meta.env.DEV` e some do bundle de produção (`rg "reference-adapter" dist/assets` → vazio), pelo mesmo mecanismo do `dev-seed` da spec 01.

**Aceite e teste:** `node --test src/features/calendar/domain/owner.test.ts` verde com: `registerAdapter` duas vezes com a mesma `layerId` substitui (o `Map.set` sobrescreve) e emite aviso; `hasAdapter("google")` é `false` (D31); um `command` com `scope: "serie"` num adaptador com `supportsSeries: false` retorna erro tipado (D29); `registerAdapter` não é chamado em nenhum caminho de `import` (só em `useEffect`), o que é verificável com `rg "registerAdapter" src` e conferindo que só aparece em `ui/`.

### 3.2 O que o Calendário faz com um `ProjectedItem`

| `kind`             | Entidade criada                                                 | `origin`    | `ownerRef` |
| ------------------ | --------------------------------------------------------------- | ----------- | ---------- |
| `evento`           | `CalendarEvent` (+ `RecurrenceRule` se o payload tiver `rrule`) | a `layerId` | preenchido |
| `responsabilidade` | `Responsibility` (+ `ResponsibilityStep[]` se houver)           | a `layerId` | preenchido |
| `etapa`            | `ResponsibilityStep` na responsabilidade pai                    | a `layerId` | preenchido |
| `bloco`            | `PlanBlock` — **exige** um `responsibilityId` resolvido         | a `layerId` | preenchido |

Para `kind: "bloco"`, o adaptador **tem** de indicar a responsabilidade de origem (INV-1: um bloco sempre pertence a uma responsabilidade). O Calendário rejeita um bloco órfão com erro explícito.

**Aceite e teste:** teste com repositório em memória: projetar a mesma página duas vezes deixa a contagem de linhas igual; projetar um bloco órfão retorna erro mencionando INV-1; `withdrawn: true` marca sem apagar — o `execution_record` continua lá (D12).

### 3.3 Comando de volta, do clique ao dono

```
TimedItem/QuickActions  →  item-menu (D30)  →  adapterRegistry.get(layer).command
                                                    ↓
                              applyItemCommand local (spec 01) grava owner_state="pendente"
                                                    ↓
                              command falha?  →  owner_state="erro" + aviso no painel
                              command ok?     →  owner_state="sincronizado"
```

`owner_state` é uma coluna nova em `calendar_event` e `responsibility`, com `CHECK IN ('sincronizado','pendente','erro','retirado')`. O Calendário grava **antes** de chamar o dono, e corrige depois — a UI nunca mostra uma edição como salva quando ela não foi.

**Aceite e teste:** `node --test` verde: adaptador que lança → `owner_state = "erro"` e o `updated_at` do item local **não** avança nos campos que só o dono tem; adaptador que resolve → `owner_state = "sincronizado"`. Comportamento observável: no painel contextual a aba "Origem" mostra "Sincronizado com {rótulo}" ou "Falha ao gravar em {rótulo}" com botão "Tentar de novo".

---

## 4. Bloqueios

- [ ] **BLOQUEIO — os módulos de origem não existem.** Faculdade, Estudos, TCC, Estágio e Base de Conhecimento **não têm código neste repositório**. Desbloqueia: a existência deles, ou uma spec que os defina assumindo o contrato `CalendarCommandPort`. **Sem isso, a entrega 5 de `calendario.md` §11 fica pela metade, permanentemente.**
- [ ] **BLOQUEIO — sem um `owner_ref` estável dos módulos.** Mesmo que os módulos existam, a chave de idempotência da projeção (D28) precisa ser um identificador **durável e opaco** que o módulo possua. Desbloqueia: decisão de identidade de cada módulo (UUID estável é o mínimo).
- [ ] _(Sem bloqueio)_ Um item de TCC é um documento versionado: editar o **bloco de escrita** de um capítulo muda a versão do capítulo no módulo de Conhecimento. Esse encadeamento é do módulo dono; o Calendário só emite `patch({ kind: "bloco", caputuloId, inicio, fim })` e não sabe o que acontece depois.
- [ ] _(Sem bloqueio)_ Convite/concorrência: duas janelas do mesmo app editando o mesmo item. Fora de escopo; relevante quando `execution_record` ganhar autoria.

---

## 5. Fases de implementação

### Fase 4.1 — Contrato e registro

| Ação  | Detalhe                                                                                                                                        |
| ----- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Criar | `src/features/calendar/domain/owner.ts` (D27) + `.test.ts`                                                                                     |
| Criar | `src/features/calendar/domain/projection.ts` — `applyProjection(page): Promise<{ inserted; updated; withdrawn; rejected }>` (D28) + `.test.ts` |

**Aceite e teste:** `npm run typecheck` e `npm run lint` em 0; `node --test src/features/calendar/domain/owner.test.ts` e `projection.test.ts` verdes; `rg "^import" src/features/calendar/domain/owner.ts` → só tipos próprios, nenhum módulo de origem; `npm run lint` → `0 problems`.

### Fase 4.2 — Coluna `owner_state` e comando de volta

| Ação   | Detalhe                                                                                                        |
| ------ | -------------------------------------------------------------------------------------------------------------- |
| Criar  | `src-tauri/migrations/0003_owner_state.sql` — `owner_state` em `calendar_event` e `responsibility` com `CHECK` |
| Editar | `src-tauri/src/commands.rs` — `dispatch_owner_command { entity, id, op, payload, scope }`                      |
| Criar  | `src/features/calendar/data/owner-command-repository.ts`                                                       |

O comando Rust **não** conhece adaptadores TypeScript: ele grava `owner_state = "pendente"`, **emite um evento Tauri** `calendar://owner-command` e retorna. O adapter React recebe o evento, chama o dono e responde com `resolve_owner_command { token, ok, error? }`. Isso mantém INV-7 e o controle de concorrência no Rust e evita que a UI segure transação aberta.

**Aceite e teste:** `rg "owner_state" src-tauri/migrations/0003_owner_state.sql` → 2; `rg "calendar://owner-command" src-tauri/src/commands.rs` → 1; `npm run typecheck` e `npm run lint` em 0; `node --test` verde: comando com `scope: "serie"` e adaptador `supportsSeries: false` deixa `owner_state = "erro"`.

### Fase 4.3 — Adaptador de referência

| Ação   | Detalhe                                                                                                   |
| ------ | --------------------------------------------------------------------------------------------------------- |
| Criar  | `src/features/calendar/data/reference-adapter.ts` (§3.1) + `.test.ts`                                     |
| Criar  | `src/features/calendar/ui/ReferenceAdapterHost.tsx` — `useEffect` que chama `registerAdapter` só em `DEV` |
| Editar | `src/App.tsx:39-90` — `subjects` e `tasks` **saem** do App e viram os literais do adaptador de referência |

**Aceite e teste:** `npm run typecheck` e `npm run lint` em 0; `node --test` verde: a projeção idempotente, o comando de volta e a falha alta; `rg "reference-adapter" dist/assets` → **vazio** depois de `npm run build`; `npm run dev` mostra itens nas camadas `faculdade`, `estudos` e `tcc` com as cores de D4; `rg "subjects|const tasks" src/App.tsx` → **vazio**.

### Fase 4.4 — Bloqueio explícito na interface

| Ação   | Detalhe                                                                                             |
| ------ | --------------------------------------------------------------------------------------------------- |
| Editar | `ui/item-menu.ts` (spec 02) — parâmetro `hasAdapter` (D30)                                          |
| Criar  | `ui/sections/OriginSection.tsx` — aba "Origem" do painel, com rótulo, id, estado e "Tentar de novo" |
| Editar | `ui/mobile/QuickActions.tsx` (spec 03) — mesma regra                                                |

**Aceite e teste:** `npm run typecheck` e `npm run lint` em 0; `node --test src/features/calendar/domain/owner.test.ts` (spec 02) continua verde com o novo parâmetro; comportamento observável: com `hasAdapter("tcc") === false` o item de TCC aparece em somente leitura, com o aviso "Este item pertence a TCC. O módulo ainda não está conectado no Campus." e **sem** botão de mover; com o adaptador de referência registrado, o mesmo item tem "Mover" e a aba "Origem" mostra "Sincronizado com TCC".

### Fase 4.5 — Painel de diagnóstico da projeção

| Ação  | Detalhe                                                                                                        |
| ----- | -------------------------------------------------------------------------------------------------------------- |
| Criar | `ui/dev/ProjectionPanel.tsx` — em `DEV`: botão "Reprojetar {rótulo}", cursor salvo, contagem por camada, erros |
| Criar | `domain/setting.ts` — `getSetting`/`setSetting` (comandos da spec 01) para `projection:<layerId>`              |

**Aceite e teste:** `npm run typecheck` e `npm run lint` em 0; `rg "ProjectionPanel" dist/assets` → **vazio**; comportamento observável: "Reprojetar Faculdade" duas vezes seguidas produz a mesma contagem (D28); um `withdrawn` remove o item da grade, mas a aba "Origem" ainda mostra o histórico de execução.

---

## 6. Resumo de arquivos

**Criar**
`src/features/calendar/domain/{owner,projection}.ts` + `.test.ts` · `src/features/calendar/data/{reference-adapter,owner-command-repository}.ts` + `.test.ts` · `src/features/calendar/ui/{ReferenceAdapterHost,ProjectionPanel,OriginSection}.tsx` · `src/features/calendar/domain/setting.ts` · `src-tauri/migrations/0003_owner_state.sql`

**Modificar**
`src-tauri/src/commands.rs` (+`dispatch_owner_command`, +`resolve_owner_command`) · `src/lib/ipc.ts` · `src/features/calendar/ui/item-menu.ts` (spec 02) · `src/features/calendar/ui/mobile/QuickActions.tsx` (spec 03) · `src/App.tsx:39-90` (retira `subjects` e `tasks`)

**Não tocar**
`src/styles.css` · `src-tauri/tauri.conf.json` · a grade e o arrasto (spec 02) · a agenda mobile e o timer (spec 03) · qualquer pasta de um módulo de origem (eles não existem)

---

## 7. Critérios de aceite finais

1. `npm run typecheck` → 0 e `npm run lint` → `0 problems` em todas as fases.
2. `rg "^import" src/features/calendar/domain/owner.ts` só referencia tipos próprios — nenhum módulo de origem é importado pelo Calendário (D27).
3. `node --test src/features/calendar/domain/` verde, incluindo: projeção idempotente, `withdrawn` sem apagar `execution_record`, bloco órfão rejeitado por INV-1, `scope: "serie"` recusado quando `supportsSeries === false`, e `hasAdapter("google") === false`.
4. `rg "subjects|const tasks" src/App.tsx` → vazio; `rg "reference-adapter" dist/assets` → vazio.
5. Sem adaptador registrado, o item de origem externa está em somente leitura com aviso e **sem** botão de mover.
6. `rg "owner_state" src-tauri/migrations/0003_owner_state.sql` → 2, com `CHECK` de 4 valores.
7. `rg "type View = " src/App.tsx` → inalterado.

---

## 8. Fora de escopo

- **A implementação de Faculdade, Estudos, TCC, Estágio e Base de Conhecimento** — não existem e não são responsabilidade do módulo Calendário (BLOQUEIO, §4).
- **O adaptador real de cada módulo** — fica para o repo de cada módulo, contra este contrato.
- Google Calendar como origem — spec 05. `google` **não** implementa `CalendarCommandPort` (D31).
- Sugestões de bloco vindas dos módulos — spec 06.
- Conflitos **entre** módulos de origem (dois módulos do cecistudy discordando sobre a mesma prova) — §10 trata de conflito com o Google; conflito interno é produto novo.
- Notificações quando um módulo externo muda.
- Escrita bidirecional em tempo real (o Calendário não empurra; quem empurra é o módulo, por `project`).
- Camadas **aninhadas** (uma sub-disciplina herdando a cor da disciplina-mãe): D4 fixa 6 camadas; sub-camadas exigiriam uma 7ª cor e reabririam D4.
- A Home (`TodayView`, `src/App.tsx:476-706`) mostrar itens vindos de módulos de origem — backlog.
