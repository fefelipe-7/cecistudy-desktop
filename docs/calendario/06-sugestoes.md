# Spec 06 — Sugestões de blocos recomendados pela Ceci

Data: 2026-09-28
Escopo: transformar a entrega 10 de `calendario.md` §11 em comportamento: a Ceci sugere blocos de tempo para responsabilidades que precisam de tempo, e a usuária aceita, ajusta ou recusa **sempre** por ação explícita.
Status: proposta fechada, aguardando implementação.

**Rastreabilidade — `calendario.md`:** §2 (o bloco "não cria uma obrigação nova; apenas reserva tempo para uma existente"), §3 (Recomendado e Opcional "podem ser sugeridos/reservados automaticamente? Sim, quando a Ceci aceitar a sugestão"; e "nunca deve transformar silenciosamente um item recomendado ou opcional em obrigação"), §6 (a sugestão respeita o planejamento, não a realidade), §7 (a sugestão aparece no painel e, no mobile, como um cartão na agenda), §12.8 (decisão D8 da spec 00), §13.
**Entregas de `calendario.md` §11 cobertas:** **10**.
**Depende de:** spec 00 (A8, D4, D7, D8), spec 01 (domínio, tabela `suggestion`, `INV-1`) e spec 02 (painel, slots reservados, grade com conflito sinalizado).
**Deixa deliberadamente para:**

- specs 03 e 04 — a sugestão aparece no mobile e a origem pode sugerir, mas o **gerador** é desta spec e o contrato de comando é o `CalendarCommandPort` da spec 04, que **não** muda;
- spec 05 — a §5 sincroniza o que foi aceito, nunca o que está sugerido. Uma sugestão recusada **nunca** chega ao Google; uma aceita entra no fluxo normal de `create_event` e, se o item tiver vínculo, no `payload.rs` da spec 05 sem código novo.

---

## 1. Diagnóstico do estado atual

| Requisito                                              | Onde encosta hoje                                                       | Situação                                                                                                                                                  |
| ------------------------------------------------------ | ----------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| §3 sugestão automática de bloco                        | —                                                                       | Ausente                                                                                                                                                   |
| §3 "nunca transformar silenciosamente em obrigação"    | —                                                                       | Ausente — não há nada que sugira, mas também não há nada que impeça (a garantia precisa ser construída)                                                   |
| §12.8 aceitar / ajustar / recusar                      | —                                                                       | Ausente                                                                                                                                                   |
| §2 bloco não cria obrigação                            | —                                                                       | Ausente                                                                                                                                                   |
| §6 sugestão respeita planejamento                      | —                                                                       | Ausente                                                                                                                                                   |
| §7 a sugestão aparece onde                             | —                                                                       | Ausente                                                                                                                                                   |
| Motor de sugestão                                      | —                                                                       | Ausente                                                                                                                                                   |
| Tabela `suggestion`                                    | criada vazia na spec 01                                                 | Schema pronto, sem uso                                                                                                                                    |
| `Sparkles` / `Zap` (ícones de sugestão que já existem) | `src/App.tsx:17` (`Sparkles`), `:19` (`Zap`), usados em `:501` e `:684` | Presentes no set de ícones, sem uso semântico de sugestão                                                                                                 |
| "12h 40min" e "68% da meta semanal de 18h"             | `src/App.tsx:689`, `:698`                                               | Mocks de métrica que **não** podem virar o critério da sugestão: um motor que optimize para bater meta vira punição, e §6 proíbe usar o desvio para punir |

---

## 2. Decisões fechadas

### 2.1 Herdadas

A8 (INV-1, INV-4, INV-7) · D4 · D7 (sobreposição permitida, conflito **derivado** e sinalizado) · D8 (três ações explícitas, nenhum caminho automático) · INV-1 (`plan_block.responsibility_id NOT NULL`).

### 2.2 Novas

**D40 — O motor é uma função pura e determinística: `suggestBlocks(state, now)`. Sem modelo, sem aleatoriedade, sem chamada de rede.**
Decisão: o motor recebe o estado do Calendário (responsabilidades, eventos, ocorrências, blocos existentes) e o instante atual, e devolve `BlockSuggestion[]` ordenadas por `score`. É a mesma forma de `isOverdue` (spec 01 §3.6) e de `findOverlaps` — pura, testável com `node --test`, sem dependência. Nenhum modelo de linguagem, nenhuma chamada externa.
Justificativa: §11.10 é "sugestões de blocos recomendados pela ceci", e §3 define **quando** a sugestão pode existir ("quando a Ceci aceitar a sugestão"). Um motor determinístico é auditável — dá para provar que ele nunca produziu uma obrigação — e é o único jeito de a garantia de §3 ser verificável. Um modelo probabilístico tornaria INV-7 impossível de provar e traria custo e latência a uma ação que a usuária faz no meio do dia.
Alternativa rejeitada: LLM local/remoto gerando horários (não verificável, não testável, contradiz §3); heurística dentro de um componente React (mesma lógica, mas sem teste, e §11 é uma entrega de domínio).
Consequência: `src/features/calendar/domain/suggest.ts` + `.test.ts` com dezenas de casos.

**D41 — O motor só sugere para `commitment ∈ {recomendado, opcional}` com prazo próximo, e para nenhuma outra.**
Decisão: os candidatos são responsabilidades com `state = planejado`, `dueAt` dentro da janela, `plannedDuration` definido, `commitment` `recomendado` ou `opcional`, e **sem** `PlanBlock` já alocado. Uma responsabilidade `obrigatorio` ou `importante` **nunca** recebe sugestão automática: ela precisa de tempo, mas reservar esse tempo é uma decisão dela, não do sistema (§3: "Pode ser sugerido/reservado automaticamente? Não sem ação explícita da usuária" para os dois primeiros níveis).
Justificativa: essa é a leitura literal da tabela de §3. Inverter a regra transformaria o motor, silenciosamente, no mecanismo de obrigação que §3 proíbe.
Alternativa rejeitada: sugerir também para obrigatórias, mas sempre exigindo aceite (violaria a própria coluna "Não sem ação explícita" — sem sugestão, não há ação explícita a exigir).
Consequência: `node --test` com um caso por nível, verificando que obrigatorio/importante **nunca** produzem sugestão.

**D42 — Janela de antecedência proporcional ao prazo, com teto.**
Decisão: a sugestão só aparece para prazos entre **hoje e 14 dias** à frente. A janela de tempo oferecida vai de 1 a 8 dias antes do prazo. Em uma semana com 7 dias, a preferência é por blocos longos (2h+). Passo de 15 min, igual à grade (D15 da spec 02), e a sugestão passa por `grid-scale.ts` para ser renderizada **na mesma escala** que um bloco aceito.
Justificativa: sugerir para algo que vence em três meses é ruído; `PRODUCT.md:21` pune excesso. E renderizar pela mesma função de escala evita a duplicação de matemática entre "sugestão" e "realidade" na grade.
Alternativa rejeitada: horizonte infinito (sugestão sempre visível, vira ruído); blocos de duração fixa de 1h (ignora `plannedDuration`, que existe no schema).
Consequência: uma sugestão tem sempre `startsAt`, `endsAt` **e** um `endsAt − startsAt` igual a `plannedDuration`; teste garante.

**D43 — Colisão com evento existente não cancela a sugestão: desloca.**
Decisão: o motor tenta o horário desejado; se colidir com um `CalendarEvent` **obrigatório ou importante**, tenta +30 min, +60 min, na semana seguinte, e assim por diante. Nunca devolve um horário que colida. Se a colisão for com outro **bloco de estudo**, ele **agrupa**: propõe o bloco ao lado, na mesma faixa, e diz isso na justificativa.
Justificativa: D7 diz que a sobreposição é permitida e **sinalizada**; a sinalização é do produto, não do motor. Um motor que respeita colisão dura é um planejador; um motor que respeita só obrigação dura e é honesto com o resto. §3 distingue os níveis justamente para isto.
Alternativa rejeitada: devolver a sugestão colidindo e confiar na sinalização (joga na usuária um conflito que ela não causou); nunca sugerir em horário ocupado (nunca sugere nada, que é o oposto de §11.10).
Consequência: `suggest.test.ts` cobre: colisão com aula → deslocada; colisão com bloco de estudo → agrupada; sem horário livre em 7 dias → **nenhuma sugestão** (o motor não inventa).

**D44 — A sugestão é um registro próprio, com prazo de validade, e "recusar" é persistente.**
Decisão: `BlockSuggestion` nasce `pendente` com `createdAt` e **expira em 7 dias** (`state = "expirada"`). "Recusar" grava `resolvedAt` e o motor **não** volta a sugerir aquele par (responsabilidade, dia) por 14 dias. "Aceitar" grava `state = "aceita"` e cria o `PlanBlock`; "Ajustar" abre o editor com os valores sugeridos e grava o bloco **editado** — o `BlockSuggestion` original fica `aceita` com os valores finais registrados, para o histórico.
Justificativa: §3 fala em "aceitar, ajustar ou recusar" e em "não por padrão" para opcional. Uma sugestão permanente é uma obrigação disfarçada; um "recusar" que volta em uma semana é ruído, e a usuária não tem como dizer "não" de forma útil.
Alternativa rejeitada: recusa sem registro (voltaria amanhã); guardar a sugestão indefinidamente (vira inbox).
Consequência: `node --test` com relógio injetado (D9 da spec 01) cobrindo expiração e a janela de recusa.

**D45 — A sugestão é sempre `commitment = "recomendado"`, nunca "obrigatório" — nem depois de aceita.**
Justificativa: §3 — Recomendado "pode gerar atraso? Não". Se aceitar uma sugestão transformasse o bloco em obrigação, o ato de aceitar seria a obrigação silenciosa que §3 proíbe. A palavra de §3 é "sugerido **ou** reservado automaticamente", e a reserva é a intenção; a obrigação continua sendo a da **responsabilidade**, não a do bloco.
Alternativa rejeitada: "aceitar" = "assumir compromisso" (misturaria §2 e §3: bloco não cria obrigação nova).
Consequência: `INV-7` verificado por teste: nenhum caminho de código leva `commitment` de `"recomendado"` a `"obrigatorio"`.

**D46 — A visibilidade da sugestão é limitada e honesta: um selo na agenda, um slot fixo no painel, e nada mais.**
Decisão: na grade, uma sugestão pendente aparece como um contorno tracejado na cor da **camada da responsabilidade**, com rótulo "Sugestão"; não aparece no `ContextPanel` a não ser que um item esteja selecionado **ou** a responsável esteja selecionada; no mobile, vira **um** cartão no topo da agenda do dia, nunca uma pilha. Nenhuma notificação, nenhum badge global, nenhum som.
Justificativa: §7 diz que o painel pode mostrar "ações" e a §8 permite "resumos e indicadores para que a usuária saiba que existem informações mais completas". §3 exige que a sugestão seja recusável — logo ela tem de ser **visível**. Mas §11.10 é a última entrega e §3 é a regra de ouro: muita visibilidade seria pressão, que é o oposto de "nunca transformar silenciosamente em obrigação".
Alternativa rejeitada: toast/push a cada sugestão (pressão); cartão de sugestão no topo de todo dia (pilha, ruído).
Consequência: `rg "suggestion" src/features/calendar/ui` mostra **um** componente `SuggestionCard.tsx` e **um** `SuggestionSlot` no painel.

---

## 3. O motor

```ts
// src/features/calendar/domain/suggest.ts — puro, determinístico, sem I/O.
export interface SuggestInput {
  responsabilidades: ReadonlyArray<Responsibility>;
  eventos: ReadonlyArray<CalendarEvent>;
  blocos: ReadonlyArray<PlanBlock>;
  sugestoesPassadas: ReadonlyArray<BlockSuggestion>;  // para a janela de recusa
  agora: Instant;
  tz: IanaTimeZone;
}
export function suggestBlocks(in: SuggestInput): BlockSuggestion[];
```

Ordem do algoritmo, cada passo com um teste:

1. **Filtrar** (§D41): `state === "planejado"`, `dueAt` em `[agora, agora + 14d]`, `plannedDuration != null`, `commitment ∈ {recomendado, opcional}`, sem `PlanBlock` ativo, e o par (responsabilidade, dia) não está na janela de recusa (D44).
2. **Escolher a camada**: a `layerId` da responsabilidade — a sugestão **não** escolhe cor, cor é D4.
3. **Encontrar horário** (§D42, §D43): a partir de `max(agora + 24h, dueAt − duraçãoDeReserva)`, passos de 30 min, dias de segunda a sábado, **dentro** da janela de dias úteis de D42; pula slots que colidam com `obrigatorio`/`importante`; se a colisão for com um bloco de estudo, aceita e registra `colideComBloco`.
4. **Ordenar** por `score = urgência × ajuste` — a urgência é `1 / dias até o prazo` e o ajuste é `1 − (deslocamento aplicado / 7 dias)`. Duas componentes puras, sem pesos ocultos: `urgency()` e `fit()` em funções separadas, ambas testadas.
5. **Limitar** a 3 sugestões por dia-alvo e a 10 no total, para não inundar.
6. **Montar** `rationale` em pt-BR a partir de componentes fixas: "Para **{título}**, que vence {data}, há {faixa} livre antes do prazo." A justificativa **nunca** menciona produtividade, meta ou atraso — é a _localização_ da sugestão, não uma cobrança (D25 da spec 03).

**Aceite e teste:** `node --test src/features/calendar/domain/suggest.test.ts` verde com, no mínimo: (a) `obrigatorio` e `importante` nunca sugerem; (b) `recomendado` e `opcional` sugerem; (c) prazo a 20 dias não sugere; (d) prazo a 3 dias sugere; (e) colisão com aula desloca; (f) colisão com bloco agrupa; (g) sem horário livre em 7 dias não sugere; (h) `state = "concluido"` não sugere; (i) recusa há 3 dias não volta; (j) recusa há 15 dias volta; (k) `now` + 8 dias expira a pendência; (l) o resultado é determinístico — duas chamadas com a mesma entrada dão arrays iguais; (m) nenhuma sugestão tem `commitment = "obrigatorio"`.

---

## 4. Bloqueios

- [ ] **BLOQUEIO — a spec 04 está bloqueada, e a sugestão herda o bloqueio.** A §9 permite que qualquer módulo de origem sugira um bloco, mas os módulos **não existem** (spec 04 §4). Nesta spec, a sugestão só vem do motor determinístico com `producedBy = "motor-local"`. A extensão "o módulo X sugere" entra como `producedBy = "<modulo>"` usando o `CalendarCommandPort` da spec 04 — que **não** precisa mudar, e por isso esta spec não o bloqueia.
- [ ] **BLOQUEIO — MSVC ausente** (`SPEC.md:10`) para verificar o caminho de gravação dentro do app. _Mitigação:_ Fase 6.1 é TypeScript puro e `node --test` cobre o motor inteiro.
- [ ] _(Sem bloqueio)_ Eventos importados do Google são `commitment = "recomendado"` (spec 05, Fase 5.3) e, portanto, **são candidatos a sugestão** — uma leitura que a usuária não pediu. Mas o motor **não** sugere bloco para um `CalendarEvent`; só para `Responsibility` (INV-1: bloco sempre pertence a uma responsabilidade). Logo, um evento externo sozinho nunca gera sugestão. Quando a usuária cria uma responsabilidade ligada a ele (spec 04, D31), essa responsabilidade é candidata normal.
- [ ] _(Pergunta de produto, não bloqueia)_ A meta semanal "18h" que aparece em `src/App.tsx:698` é mock. **Esta spec deliberadamente não a usa** como sinal do motor: otimizar para uma meta é transformar desvvio em cobrança, e §6 proíbe punir pelo desvio. Se a meta virar um dado real, ela é **informativa** no painel e **não** entra no `score` da sugestão.

---

## 5. Fases de implementação

### Fase 6.1 — Motor puro

| Ação  | Detalhe                                                                                 |
| ----- | --------------------------------------------------------------------------------------- |
| Criar | `src/features/calendar/domain/suggest.ts` + `.test.ts` (D40)                            |
| Criar | `src/features/calendar/domain/scoring.ts` — `urgency()` e `fit()` puras, com `.test.ts` |

**Aceite e teste:** `node --test src/features/calendar/domain/suggest.test.ts` verde nos 13 casos de §3; `npm run typecheck` → 0; `npm run lint` → `0 problems`; `rg "Math.random|Date.now|new Date" src/features/calendar/domain/suggest.ts` → **vazio** (determinismo e relógio injetado — D9 da spec 01); `rg "fetch|invoke" src/features/calendar/domain/suggest.ts` → **vazio**.

### Fase 6.2 — Persistência da sugestão

| Ação  | Detalhe                                                                                      |
| ----- | -------------------------------------------------------------------------------------------- |
| Criar | `src-tauri/src/commands.rs` — `list_suggestions`, `dismiss_suggestion`, `expire_suggestions` |
| Criar | `src/features/calendar/data/suggestion-repository.ts`                                        |

`expire_suggestions` roda no primeiro sync de cada lançamento (e no `google_sync_now`, se existir) — é ele que aplica D44 sem depender do relógio do cliente.

**Aceite e teste:** `rg "tauri::command" src-tauri/src/commands.rs` → **37**; `npm run typecheck` e `npm run lint` em 0; `node --test` verde: `dismiss_suggestion` grava `resolved_at` e a recusa bloqueia o par por 14 dias; `expire_suggestions` com relógio adiantado muda `pendente` → `expirada` e **não** apaga a linha (histórico).

### Fase 6.3 — As três ações (D8)

| Ação   | Detalhe                                                                                                                           |
| ------ | --------------------------------------------------------------------------------------------------------------------------------- |
| Criar  | `ui/SuggestionCard.tsx` — um componente, três botões, texto de justificativa (D46)                                                |
| Editar | `ui/ContextPanel.tsx` (spec 02) — `SuggestionSlot` na seção **"O que precisa ser feito"** (nunca em "Marcado" nem em "Realidade") |
| Criar  | `ui/sections/SuggestionSection.tsx`                                                                                               |

"Recusar" grava `dismissed_at`; "Ajustar" abre o `ItemEditor` da spec 02 pré-preenchido e, no `onSubmit`, grava o `PlanBlock` **editado** e marca a sugestão `aceita`; "Aceitar" grava o `PlanBlock` com os valores sugeridos. As três passam por `applyItemCommand` da spec 01 — não há caminho de escrita próprio.

**Aceite e teste:** `npm run typecheck` e `npm run lint` em 0; `node --test` verde: `applyItemCommand({ kind: "accept_suggestion" })` grava exatamente um `PlanBlock` com `responsibility_id` da sugestão (INV-1), `commitment = "recomendado"` (D45) e `state = "planejado"`; `rg "obrigatorio" src/features/calendar/ui/SuggestionCard.tsx` → vazio; comportamento observável: uma sugestão recusada **não** reaparece em 14 dias; **não** existe caminho na UI que crie bloco a partir de sugestão sem um dos três cliques (verificável por revisão de `SuggestionCard.tsx` + `item-menu.ts`).

### Fase 6.4 — Sugestão na grade e na agenda

| Ação   | Detalhe                                                                                                                         |
| ------ | ------------------------------------------------------------------------------------------------------------------------------- |
| Editar | `ui/WeekGrid.tsx` (spec 02) — camada de desenho de sugestão, **abaixo** dos itens, tracejada, não arrastável                    |
| Editar | `ui/mobile/MobileAgenda.tsx` (spec 03) — **um** cartão no topo do dia                                                           |
| Editar | `ui/ContextPanel.tsx` — estado vazio explícito: "Nenhuma sugestão. A Ceci sugere blocos para leituras e revisões recomendadas." |

A sugestão é renderizada com o **mesmo** `grid-scale.ts` e o mesmo `packOverlapping` de um bloco real, então a posição de uma sugestão e a de um bloco aceito para o mesmo horário são idênticas.

**Aceite e teste:** `npm run typecheck` e `npm run lint` em 0; `rg "grid-scale|packOverlapping" src/features/calendar/ui/SuggestionCard.tsx src/features/calendar/ui/WeekGrid.tsx` → as mesmas funções da spec 02; comportamento observável: aceitar uma sugestão **não** faz a caixa "pular" de lugar; uma sugestão não é arrastável (é o que D17 já exige para itens não persistidos, e a sugestão **não** vira item ao ser desenhada); `rg "suggestion" src/features/calendar/ui` mostra **um** `SuggestionCard.tsx` (D46).

### Fase 6.5 — Gatilho e refinamento

| Ação   | Detalhe                                                                                                                 |
| ------ | ----------------------------------------------------------------------------------------------------------------------- |
| Editar | `CalendarToolbar.tsx` (spec 02) — botão "Sugerir blocos" explícito, com `title` explicando que nada é criado sem aceite |
| Criar  | `ui/SuggestionDigest.tsx` — linha em `DEV` com "N sugeridas, M aceitas, K recusadas"                                    |

O botão explícito existe por §3: a sugestão pode ser automática, mas a usuária **precisa** poder pedir mais, e a existência do botão deixa visível que existe uma decisão por trás.

**Aceite e teste:** `npm run typecheck` e `npm run lint` em 0; `rg "Sugerir blocos" src/features/calendar/ui/CalendarToolbar.tsx` → 1; `rg "SuggestionDigest" dist/assets` → **vazio**; comportamento observável: clicar em "Sugerir blocos" roda o motor e **não** cria nenhum `PlanBlock` (contagem de blocos inalterada).

---

## 6. Resumo de arquivos

**Criar**
`src/features/calendar/domain/{suggest,scoring}.ts` + `.test.ts` · `src/features/calendar/data/suggestion-repository.ts` · `src/features/calendar/ui/{SuggestionCard,SuggestionDigest}.tsx` · `src/features/calendar/ui/sections/SuggestionSection.tsx`

**Modificar**
`src-tauri/src/commands.rs` (+3 comandos) · `src/lib/ipc.ts` · `src/features/calendar/ui/ContextPanel.tsx` (spec 02) · `src/features/calendar/ui/WeekGrid.tsx` (spec 02) · `src/features/calendar/ui/CalendarToolbar.tsx` (spec 02) · `src/features/calendar/ui/mobile/MobileAgenda.tsx` (spec 03)

**Não tocar**
`src/styles.css` — nenhum token novo; a sugestão usa a cor da camada da responsabilidade (D4) e `border-dashed` como o `google` (D4 da spec 00) · `src-tauri/tauri.conf.json` · `src-tauri/src/google/**` — a sugestão nunca chega ao Google antes de ser aceita, e depois disso é um `create_event` comum · `src/App.tsx`

---

## 7. Critérios de aceite finais

1. `npm run typecheck` → 0 e `npm run lint` → `0 problems` em todas as fases.
2. `node --test src/features/calendar/domain/suggest.test.ts` verde nos 13 casos de §3, incluindo "obrigatório e importante nunca sugerem" e "determinismo".
3. `rg "Math.random|Date.now|new Date|fetch|invoke" src/features/calendar/domain/suggest.ts` → **vazio**.
4. `rg "tauri::command" src-tauri/src/commands.rs` → **37**.
5. `rg "SuggestionCard" src/features/calendar/ui` → **um** arquivo (D46).
6. Comportamento observável: aceitar uma sugestão não move a caixa na grade; recusar esconde a sugestão por 14 dias; a sugestão nunca é arrastável; nenhuma sugestão aparece em "O que está marcado" nem em "O que realmente aconteceu" — só em "O que precisa ser feito".
7. `rg "obrigatorio" src/features/calendar/ui/SuggestionCard.tsx` → vazio (D45) e nenhum teste de `suggest.ts` produz `commitment = "obrigatorio"`.
8. INV-7 intacta: nenhum caminho de código em `src/features/calendar/` promove `commitment` de `"recomendado"` a `"obrigatorio"`.

---

## 8. Fora de escopo

- Qualquer modelo de linguagem, chamar de API, heurística aprendida ou dado de telemetria (D40).
- Sugestão para `obrigatorio`/`importante` — proibido por §3 (D41).
- Sugestão que crie, altere ou remova `Responsibility`, `CalendarEvent` ou `RecurrenceRule`. O motor **só** sugere `PlanBlock`, e `PlanBlock` sempre pertence a uma `Responsibility` (INV-1, §2).
- Uso da meta semanal, de "12h 40min" ou de qualquer métrica de produtividade no `score` (D25 da spec 03, §4).
- Notificação, toast, badge global, e-mail ou som sobre sugestão (D46).
- Ajustes automáticos de sugestão quando a grade é redimensionada ou quando o prazo muda — a sugestão é recalculada no próximo ciclo, não empurrada.
- Re-sugerir com nova redação a mesma responsabilidade depois de uma recusa (D44 — é ruído).
- Sugestões vindas de modelos externos (estilo "aprenda meu ritmo") ou de outros dispositivos.
- Blink/lista de desejos: o conjunto nunca vira obrigação; a `Responsibility` continua sendo criada à mão, ou pela spec 04 com o módulo de origem.
- A Home (`TodayView`, `src/App.tsx:476-706`) mostrar sugestões — §1 diz que a Home é dona de pendências e prioridades; a sugestão é do Calendário. Backlog.
