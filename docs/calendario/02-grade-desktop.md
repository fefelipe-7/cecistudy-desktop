# Spec 02 — Visão semanal do desktop

Data: 2026-09-28
Escopo: substituir a grade estática de `src/App.tsx:791-866` por uma grade temporal real, com camadas por origem, faixa de dia inteiro, arrastar/redimensionar, painel contextual persistente, e as visões diária, mensal e de agenda.
Status: **parcial em 2026-09-29** — a estrutura de cinco modos (D15bis) e a geometria da grade (D15, `domain/grid-scale.ts` com 15 testes) já estão implementadas. Faltam arraste, redimensionamento, criação e edição, que exigem o app de pé.

**Rastreabilidade — `calendario.md`:** §1 (o Calendário como dono da distribuição temporal; a Home continua dona das pendências), §4 (cancelar/remarcar por ocorrência — a **interface**), §5 (exceções por ocorrência na interface), §6 (painel mostra planejamento e realidade), §7 (integral), §13 (o painel separa marcado / a fazer / aconteceu).
**Entregas de `calendario.md` §11 cobertas:** **2** (visão semanal desktop com arrastar/redimensionar e painel contextual) e **7** (camadas visuais por origem dentro do mesmo calendário). A entrega **4** (aulas recorrentes com cancelamento e remarcação por ocorrência) fica **completa** aqui, porque o motor de recorrência vem pronto da spec 01 e esta spec entrega a interface dela.
**Depende de:** spec 00 (A4, A7, A8, D4, D6, D7) e spec 01 (domínio, repositórios, comandos).
**Deixa deliberadamente para:**

- spec 03 — agenda mobile, edição rápida por ações (sem ponteiro), timer;
- spec 04 — o painel contextual ganha abas de vínculo com o módulo de origem; aqui a aba existe e mostra só "Origem: Faculdade (referência)";
- spec 05 — o painel ganha a aba de sincronização, conflito e exclusão cruzada; aqui a aba mostra apenas "Não sincronizado";
- spec 06 — o painel ganha a seção de sugestão de bloco; aqui o slot está reservado e vazio.

---

## 1. Diagnóstico do estado atual

| Requisito                       | Onde encosta hoje                                                             | Situação                                                                                                                     |
| ------------------------------- | ----------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| §7 grade dias × horários        | `src/App.tsx:810` (`grid-cols-[72px_repeat(5,1fr)]`), `:824` (`h-[570px]`)    | Parcial — 5 dias fixos, altura fixa, sem rolagem                                                                             |
| §7 eixos de horário             | `src/App.tsx:826-835`                                                         | Parcial — 8 rótulos literais 08:00–15:00, 70px/hora, sem meia hora                                                           |
| §7 posicionamento               | `src/App.tsx:851-856` (`style` inline com `top`, `height`, `left: calc(...)`) | Parcial — a matemática existe mas é literal, sem escala de tempo                                                             |
| §7 dia inteiro                  | —                                                                             | Ausente                                                                                                                      |
| §7 arrastar/redimensionar       | `src/App.tsx:918` `cursor-grab` decorativo                                    | Ausente                                                                                                                      |
| §7 camadas                      | `src/App.tsx:33-59` `subjects[].dot`/`.soft`, cores por **disciplina**        | Ausente como camada — a cor está presa à disciplina, e o `CalendarView` recebe `setView` e nada mais (`src/App.tsx:791-792`) |
| §7 alternância dia/mês/agenda   | `src/App.tsx:760-789` `ViewSwitch` (Grade/Kanban/Lista)                       | Ausente — `ViewSwitch` troca o modo do **dashboard**, não a granularidade do Calendário                                      |
| §7 painel contextual            | —                                                                             | Ausente                                                                                                                      |
| §5 cancelar/remarcar ocorrência | —                                                                             | Ausente                                                                                                                      |
| §4 estados                      | —                                                                             | Ausente na interface                                                                                                         |
| §6 planejamento × realidade     | —                                                                             | Ausente na interface                                                                                                         |

Restrições a respeitar:

- `CalendarView` hoje recebe **apenas** `setView` (`src/App.tsx:792`). A spec 01 já o removeu do `App.tsx` e o substituiu por `CalendarScreen`; esta spec implementa a tela dentro de `src/features/calendar/ui/`.
- `AGENTS.md:5,9` — sem roteador. As visões (semana/dia/mês/agenda) são `useState` **dentro** de `CalendarScreen`, não rotas, e não tocam em `type View` (`src/App.tsx:29`).
- `minWidth: 1024` (`src-tauri/tauri.conf.json:22`) e `minHeight: 640` (`:23`) — a grade precisa caber sem scroll horizontal nessa largura; abaixo de 768px quem manda é a spec 03.
- `DESIGN.md:21` — controles de 32–36px, cantos 6–10px; `DESIGN.md:25` — transições de 150–200ms, sem animação de entrada decorativa; `styles.css:168-176` já desliga animação sob `prefers-reduced-motion`.

---

## 2. Decisões fechadas

### 2.1 Herdadas

A4 (`useGridDrag` próprio) · D4 (tons de camada) · D6 (UTC + IANA, sem ajuste automático ao viajar) · D7 (sobreposição permitida, conflito derivado) · INV-1..INV-7.

### 2.2 Novas

**D15bis — Cinco modos numa tela só, e o painel do item não é uma delas.**
Decisão: o Calendário é **um destino** com cinco modos — `agenda`, `semana`, `mes`, `lista`,
`planejamento`. `semana` é o modo inicial no desktop. O painel do item selecionado é uma
**região** da tela do Calendário, não um sexto modo nem uma rota: aparece à direita quando há
seleção e some quando não há.
Justificativa: `AGENTS.md` proíbe roteador e a spec 01 já fixou o Calendário como "workspace
estudado stateful, uma única rota". As cinco visões não são destinos diferentes — são cinco
leituras do mesmo conjunto de dados, e trocar de leitura não pode custar um carregamento.
O painel separado em tela cheia quebraria justamente a comparação entre planejado e real, que
é o motivo de o item abrir **ao lado** da grade.
Alternativa rejeitada: cinco entradas no menu principal (espalha o mesmo dado por cinco
destinos e faz o menu_gt grows sem critério); painel em modal (esconde a grade, e a usuária
perde a referência de onde o item está).
Consequência: `CalendarScreen.tsx` (spec 01 Fase 1.8) já é o dono dos cinco modos e do painel;
esta spec entrega a **geometria** de cada um, não a estrutura. O que cada modo mostra:

| Modo           | Eixo / forma               | Pergunta que responde                                     |
| -------------- | -------------------------- | --------------------------------------------------------- |
| `semana`       | 7 colunas × horas, Seg–Dom | como este dia está ocupado (leitura principal do desktop) |
| `agenda`       | lista do dia selecionado   | o que acontece hoje, item a item                          |
| `mes`          | calendário de 35 células   | o que vem no mês, e onde estão os prazos                  |
| `lista`        | "Hoje" e "Próximos dias"   | o que precisa ser feito agora (Atraso é derivado — D11)   |
| `planejamento` | métricas planejado × real  | quanto foi reservado e quanto saiu (§6)                   |

O painel do item carrega, nesta ordem: título e data, disciplina/camada, importância, etapas,
ações `Concluir` e `Reagendar`; e as abas de detalhe, origem, planejamento, execução, histórico
e sincronização. "Detalhe" e "Histórico" são a mesma leitura do mesmo item — o histórico é o
append-only de D12, então a aba é uma lista de registros, não um estado.

**D15 — Geometria da grade: uma função de escala única.**
Decisão: `timeToY(instant, view): number` e `yToTime(y, view): Instant` são funções puras em `domain/grid-scale.ts`, com parâmetros `{ dayStartHour, hourHeight, snapMinutes, tz }`. A grade renderiza com **uma** unidade: `top` e `height` em pixels derivados de `timeToY`. O snap default é **15 minutos**.
Justificativa: o código atual posiciona por literais (`src/App.tsx:851-856`). Separar a matemática do DOM é o que permite testar arraste sem browser e garante que arraste, redimensionamento, criação e leitura de item usem **exatamente** a mesma escala — sem drift entre o que se vê e o que se grava.
Alternativa rejeitada: CSS Grid com `grid-row: span N` (não dá arraste livre nem encaixe de meia hora; e o eixo horário é irregular).
Consequência: `domain/grid-scale.test.ts` cobre `timeToY`/`yToTime` como inversas, o arredondamento por `snapMinutes`, e o caso de item que **atravessa a meia-noite** (quebra em duas faixas).

**D16 — Densidade: 7 dias, `hourHeight` de 56px, rolagem vertical, dia inteiro numa faixa fixa.**
Decisão: eixo X = 7 dias (Seg–Dom), eixo Y = 24 horas com rolagem; `dayStartHour` acompanha a primeira aula da janela carregada, com piso às 06:00; `hourHeight` = 56px no desktop; faixa de dia inteiro com altura variável acima do cabeçalho de dias. Densidades alternativas (24px "compacta", 84px "ampliada") via um seletor de três estados.
Justificativa: §7 pede dias no eixo horizontal e horários no vertical, e uma faixa separada para dia inteiro. O atual é 5×8 a 70px (`src/App.tsx:810,824,830`) — 5 dias esconde o fim de semana, que é onde está a maior parte dos blocos de estudo. 56px cabe 4 linhas de 70px-equivalente num painel de 820px de janela (`src-tauri/tauri.conf.json:18`) com comfortable ~13 horas visíveis.
Alternativa rejeitada: manter 5 dias (perde sábado/domingo); altura total fixa `h-[570px]` (não há como ver 07:00–22:00 sem cortar).
Consequência: a matemática atual de `left: calc(72px + day * ((100% - 72px) / 5) + 5px)` (`src/App.tsx:852`) é substituída por colunas CSS `grid-cols-7` reais; o eixo de horas passa a ser `position: sticky` à esquerda dentro de um container de scroll.

**D17 — Arrasto por ponteiro, e somente para `origin !== "google"`.**
Decisão: `origin === "google"` e `state ∈ {cancelado, dispensado}` são **não arrastáveis**: o cursor é `not-allowed` e a alça de resize não aparece. O restante é arrastável. Arrastar aplica **otimismo local** (o item segue o ponteiro) e persiste ao soltar; se o comando falhar, o item volta e a grade mostra um aviso inline.
Justificativa: §10 diz que um evento importado do Google não é editável no cecistudy. Permitir o arraste e bloquear só no `persist` seria uma mentira de interface (INV-6: o bloqueio tem de ser visível antes do gesto). `cancelado` e `dispensado` significam que a decisão já foi tomada; mover um item dispensado reabriria um estado que a usuária fechou.
Alternativa rejeitada: bloquear só no backend (o usuário só descobperia a impossibilidade depois de arrastar).
Consequência: o teste de aceite de UI é observável — `cursor` do item e presença/ausência da alça.

**D18 — O painel contextual é uma coluna, não um modal, e sobrevive à navegação na semana.**
Decisão: `CalendarScreen` fica em `grid-cols-[minmax(0,1fr)_360px]` no desktop; o painel é a segunda coluna e **permanece montado** enquanto a Ceci navega entre semanas. No `max-lg` (abaixo de 1024px, o `minWidth` do app) o painel vira um `Sheet`.
Justificativa: §7 diz explicitamente "a decisão estrutural é que o painel contextual possa permanecer aberto enquanto a Ceci navega pela semana". Modal derrota isso. 360px é a largura que o layout atual já usa para colunas laterais (`src/App.tsx:505` usa `minmax(300px,0.75fr)`).
Alternativa rejeitada: modal `Dialog` (perde o contexto da semana, que é o ponto do requisito); popover ancorado (não comporta a quantidade de informação de §7).
Consequência: `npx shadcn@latest add sheet` na Fase 2.2 (o set curado não tem `sheet` — verificado).

**D19 — As visões são um `ViewSwitch` interno com quatro modos.**
Decisão: `CalendarMode = "semana" | "dia" | "mes" | "agenda"`, estado local de `CalendarScreen`. O `ViewSwitch` existente (`src/App.tsx:760-789`) passa a ser usado **dentro** do Calendário com esses quatro rótulos, e as três visões do dashboard (`kanban`, `list`, `subject`) continuam onde estão. Em `agenda`, item = linha; em `mes`, item = pino por dia.
Justificativa: §7 pede alternância para diária, mensal e agenda/lista. Reaproveitar o componente que já existe evita um segundo switch visualmente idêntico, e **mantém** `type View` (`src/App.tsx:29`) intocado — `AGENTS.md:5` mantém os modos do dashboard como `useState`.
Alternativa rejeitada: transformar `kanban`/`list`/`subject` em submodos do Calendário (violaria `AGENTS.md:5` — eles são modos do dashboard, não do Calendário).
Consequência: `ViewSwitch` é movido de `src/App.tsx` para `src/features/calendar/ui/ViewSwitch.tsx` e recebe `mode` genérico; `src/App.tsx:347-348` passa a importar dali.

**D20 — O painel contextual é derivado por três colunas, uma por pergunta da §13.**
Decisão: o painel tem três seções fixas, nesta ordem: **"O que está marcado"** (evento/ocorrência, regra, estado, camada), **"O que precisa ser feito"** (responsabilidade vinculada, etapas, prazo, commitment), **"O que realmente aconteceu"** (registros de execução, planejado × real, desvios) — mais **"Ações"**. Sincronização e sugestão têm slots reservados, vazios, até as specs 05 e 06.
Justificativa: §13 é a regra de ouro: "responder a três perguntas diferentes sem misturá-las". Uma coluna por pergunta torna a regra **estrutural** — não é possível misturar porque o layout não permite. Um painel com abas misturaria "cancelar esta ocorrência" (evento) com "registrar que li 40 páginas" (realidade) na mesma lista.
Alternativa rejeitada: abas (`Tabs`) por tipo de item — esconderia a resposta às três perguntas simultaneamente, que é o que §13 pede.
Consequência: um evento sem responsabilidade mostra a seção do meio com "Nenhuma responsabilidade vinculada" e um botão "Criar responsabilidade para este compromisso" (§10 pede exatamente isso para eventos externos).

---

## 3. Arquitetura de componentes

```
src/features/calendar/ui/
  CalendarScreen.tsx          # grid-cols-[1fr_360px]; dono de mode, semana, painel
  CalendarToolbar.tsx         # anterior | hoje | próximo | período | densidade | ViewSwitch | filtro de camadas
  WeekGrid.tsx                # faixa de dia inteiro + cabeçalho de dias + corpo rolável
  DayColumn.tsx               # 1 coluna; recebe itens já posicionados
  AllDayRow.tsx               # faixa separada (Itens de dia inteiro)
  TimedItem.tsx               # caixa posicionada; arraste, resize, teclado, menu
  ContextPanel.tsx            # D20 — três seções + ações
  MonthView.tsx  DayView.tsx  AgendaView.tsx
  LayerFilter.tsx             # checkboxes das 6 camadas (D4)
  ViewSwitch.tsx              # movido de src/App.tsx:760-789, genérico
  RecurrenceScopeDialog.tsx   # "esta ocorrência / esta série / esta e seguintes" (D6)
  ItemEditor.tsx              # diálogo de criação/edição (formulário real)
  hooks/use-grid-drag.ts      # A4
  hooks/use-week-window.ts    # semana ↔ janela, cache por semana
```

`domain/grid-scale.ts`, `domain/layout.ts` (colunas de itens sobrepostos) e `domain/item-menu.ts` (quais ações existem para cada combinação de `origin` × `state`) são **puros** e testados com `node --test`.

---

## 4. Bloqueios

- [ ] **BLOQUEIO — MSVC ausente** (`SPEC.md:10`). Sem ele, o app não abre e a Fase 2.6 não tem como ser verificada em execução. _Mitigação:_ as Fases 2.1 a 2.5 são TypeScript puro e verificáveis com `npm run typecheck`, `npm run lint` e `node --test`.
- [ ] **BLOQUEIO — `useGridDrag` precisa de ponteiro de verdade.** Arrastar, redimensionar e o comportamento responsivo só se provam no WebView2 (ou no Chrome em `npm run dev`, que é aceitável para o gesto). Verificação por `npm run dev` + DevTools, sem depender do Tauri.
- [ ] _(Sem bloqueio)_ Componentes shadcn necessários, ausentes do set curado: `sheet` (D18), `form` (ItemEditor, que usa `react-hook-form` + `zod`), `alert-dialog` (confirmação de exclusão e de exclusão cruzada em §10, esta última só na spec 05). Todos por `npx shadcn@latest add <nome>`, conforme `AGENTS.md:12`.

---

## 5. Fases de implementação

### Fase 2.1 — Escala de tempo e layout

| Ação  | Detalhe                                                                                      |
| ----- | -------------------------------------------------------------------------------------------- |
| Criar | `src/features/calendar/domain/grid-scale.ts` (D15) + `.test.ts`                              |
| Criar | `src/features/calendar/domain/layout.ts` — `packOverlapping(items, minMinutes)` + `.test.ts` |

`packOverlapping` é a parte de layout **não** derivada de D7: D7 diz que a sobreposição é permitida e sinalizada; isto é só como os itens dividem a largura da coluna.

**Aceite e teste:** `node --test src/features/calendar/domain/` verde; `timeToY(t)` e `yToTime(timeToY(t))` são inversas com snap de 15 min; `packOverlapping([a 08–10, b 09–11])` devolve 2 colunas de 50% cada; `packOverlapping([a 08–10, b 10–12])` devolve **uma** coluna só (intervalos que se tocam empilham, igual Google Calendar).

### Fase 2.2 — Componentes shadcn necessários

| Ação  | Detalhe                                                                                             |
| ----- | --------------------------------------------------------------------------------------------------- |
| Rodar | `npx shadcn@latest add sheet` · `npx shadcn@latest add form` · `npx shadcn@latest add alert-dialog` |
| Rodar | `npm install`                                                                                       |

**Aceite e teste:** `npm run typecheck` → 0; `npm run lint` → `0 problems` (os três componentes novos usam `Dialog` e não reexportam nada, então não reintroduzem o warning de `react-refresh`); `rg "sheet|alert-dialog" package.json` mostra as deps; `rg "react-day-picker" package.json` → **vazio** (a visão mensal desta spec **não** usa o `Calendar` do shadcn — D16 e §2.1 da Fase 2.3: mês é uma grade própria, sem `react-day-picker`; re-adicionar essa dep fica para a spec que precisar de um date picker em diálogo).

### Fase 2.3 — WeekGrid, DayColumn, AllDayRow, TimedItem (estático)

| Ação   | Detalhe                                                                                                     |
| ------ | ----------------------------------------------------------------------------------------------------------- |
| Criar  | `WeekGrid.tsx`, `DayColumn.tsx`, `AllDayRow.tsx`, `TimedItem.tsx`, `CalendarToolbar.tsx`, `LayerFilter.tsx` |
| Criar  | `hooks/use-week-window.ts`                                                                                  |
| Editar | `src/App.tsx` — remover `ViewSwitch` (`:760-789`) e os usos em `:347-348`                                   |

A grade usa `timeToY`/`yToTime`; nenhum literal de posição sobrevive. `TimedItem` recebe `origin`, `state` e `overlapped` e decide a classe por `LAYER_TONE` (D4) e por `isOverdue` (INV-4, da spec 01).

**Aceite e teste:** `npm run typecheck` → 0; `npm run lint` → `0 problems`; `rg "top: [0-9]|h-\[570px\]|repeat\(5,1fr\)" src/features/calendar/ui` → **vazio** (nenhum literal de geometria sobreviveu do `src/App.tsx:709-866`); `rg "bg-chart-4/12" src/features/calendar/ui` → presente na camada `tcc`; `rg "text-chart-4" src` → vazio; `npm run dev` mostra 7 colunas com rolagem, faixa de dia inteiro e as 6 camadas legendadas.

### Fase 2.4 — Arraste, redimensionamento e teclado

| Ação   | Detalhe                                                                                           |
| ------ | ------------------------------------------------------------------------------------------------- |
| Criar  | `ui/hooks/use-grid-drag.ts` (A4) — `pointerdown/move/up` + `setPointerCapture` + `Escape` cancela |
| Editar | `TimedItem.tsx` — alças superior e inferior; arraste no corpo; foco e teclado                     |

Comportamento: arrastar move em passos de 15 min; as alças mudam a duração em passos de 15 min com piso de 15 min; `Escape` durante o arraste devolve o item à posição original; soltar persistE via `reschedule_block` (bloco) ou `move_occurrence` (ocorrência) ou `update_event` (evento avulso) — **a escolha é derivada de `kind`, nunca escolhida pelo componente**.

**Aceite e teste:** `npm run typecheck` e `npm run lint` em 0; `node --test src/features/calendar/domain/grid-scale.test.ts` verde; comportamento observável em `npm run dev`: arrastar um item 40px para baixo move-o exatamente 15 min (snap), não 40px; arrastar para cima até o topo da grade não deixa `startsAt` antes do início do dia; um item `origin: "google"` não tem `cursor-grab` e não exibe alça; com `Escape` pressionado no meio do arraste o item volta e nenhum comando é chamado (verificável na aba Network do IPC ou por `console.log` no repositório); mover com o teclado: `Tab` foca o item, `←/→` movem 15 min, `↑/↓` redimensionam 15 min, `Shift+←/→` redimensiona.

### Fase 2.5 — Painel contextual

| Ação  | Detalhe                                                                          |
| ----- | -------------------------------------------------------------------------------- |
| Criar | `ContextPanel.tsx` (D20), `ItemEditor.tsx`, `domain/item-menu.ts` com `.test.ts` |
| Criar | `ui/sections/{MarkedSection,ToDoSection,RealitySection,ActionsSection}.tsx`      |

`item-menu.ts` é a tabela única que decide quais ações existem: given `origin` × `state`, devolve a lista de ações. Testes: item `google` → sem "editar", sem "arrastar", **com** "criar responsabilidade para este compromisso" (§10, 3ª linha); item `cancelado` → sem "editar", com "reativar"; ocorrência com regra → sempre traz o `RecurrenceScopeDialog` antes de qualquer mudança (D6, §4).

**Aceite e teste:** `npm run typecheck` e `npm run lint` em 0; `node --test src/features/calendar/domain/item-menu.test.ts` verde; comportamento observável: selecionar um item abre o painel à direita **sem fechar a semana**; navegar `←` `→` mantém o painel montado e apenas a data no cabeçalho muda; as três seções da §13 aparecem sempre, na mesma ordem, mesmo quando vazias (com texto de estado vazio); em `max-lg` o painel vira `Sheet`.

### Fase 2.6 — Visões dia, mês e agenda; subscritor de recorrência

| Ação   | Detalhe                                                                 |
| ------ | ----------------------------------------------------------------------- |
| Criar  | `DayView.tsx`, `MonthView.tsx`, `AgendaView.tsx`                        |
| Criar  | `RecurrenceScopeDialog.tsx`                                             |
| Editar | `CalendarScreen.tsx` — `mode` alterna; `ViewSwitch` com 4 rótulos (D19) |

`MonthView` é uma grade de 6×7 com um pino por dia (até 3 pinos + "+N"), **sem** `react-day-picker`. `AgendaView` é a lista do §7 "agenda/lista" ordenada por instante, agrupada por dia.

**Aceite e teste:** `npm run typecheck` e `npm run lint` em 0; comportamento observável: alternar para "Mês" mantém o item selecionado e o painel; "Cancelar ocorrência" num item recorrente abre `RecurrenceScopeDialog` com as três opções, e escolher "Esta ocorrência" cancela **só** aquela (INV-5: `rg` no log do repositório mostra `update_recurrence` não chamado); escolher "Esta série" altera a `RecurrenceRule` e regenera as ocorrências futuras; `rg "react-day-picker" package.json` continua vazio.

### Fase 2.7 — Conectar o App

| Ação   | Detalhe                                                                                           |
| ------ | ------------------------------------------------------------------------------------------------- |
| Editar | `src/App.tsx:346` — `<CalendarScreen onOpenList={() => setView("list")} />` (já feito na spec 01) |
| Editar | `src/App.tsx:149-158` — o `title` de `view === "calendar"` vira o modo interno, se necessário     |

**Aceite e teste:** `rg "type View = " src/App.tsx` → **inalterado** (`AGENTS.md:5,9` — sem roteador); `rg "CalendarScreen" src/App.tsx` → 1; `npm run typecheck` → 0; `npm run lint` → `0 problems`; `npm run build` gera `dist/` sem erro; visualmente: a sidebar, o ⌘K e as outras 4 vistas continuam idênticos ao estado da spec 01.

---

## 6. Resumo de arquivos

**Criar**
`src/features/calendar/ui/{CalendarScreen,CalendarToolbar,WeekGrid,DayColumn,AllDayRow,TimedItem,ContextPanel,MonthView,DayView,AgendaView,LayerFilter,ViewSwitch,RecurrenceScopeDialog,ItemEditor}.tsx` · `src/features/calendar/ui/sections/*.tsx` · `src/features/calendar/ui/hooks/{use-grid-drag,use-week-window}.ts` · `src/features/calendar/domain/{grid-scale,layout,item-menu}.ts` + `.test.ts` · componentes shadcn `sheet`, `form`, `alert-dialog`

**Modificar**
`src/App.tsx` (remoção de `ViewSwitch` em `:760-789` e ajuste de `:347-348`, `:149-158`) · `package.json` / `package-lock.json`

**Não tocar**
`src/styles.css` (nenhum token novo — D4 reutiliza `--color-chart-1..5`) · `src-tauri/**` · `tauri.conf.json` · `SPEC.md`

---

## 7. Critérios de aceite finais

1. `npm run typecheck` → 0 e `npm run lint` → `0 problems` em todas as fases.
2. `node --test src/features/calendar/domain/` verde, incluindo `timeToY`/`yToTime` como inversas, snap de 15 min e `packOverlapping` com intervalos que só se tocam.
3. `rg "top: [0-9]|repeat\(5,1fr\)|h-\[570px\]" src/features/calendar` → vazio.
4. `rg "text-chart-4" src` → vazio; `rg "bg-chart-4/12" src/features/calendar/ui` → presente.
5. `rg "type View = " src/App.tsx` → inalterado; nenhum roteador criado.
6. O painel contextual permanece montado ao navegar `←`/`→` da semana; abaixo de 1024px ele vira `Sheet`.
7. Um item `origin: "google"` não é arrastável, não tem alça de resize e não oferece "editar" no menu.
8. "Cancelar ocorrência" com item recorrente sempre abre o diálogo de escopo; com "Esta ocorrência", `event_recurrence` não muda (INV-5).
9. `npm run build` gera `dist/`; `npm run dev` funciona com o painel, arrasto e as 4 granularidades.

---

## 8. Fora de escopo

- Agenda mobile, edição por ações sem ponteiro, timer e registro de execução pela interface — spec 03.
- Comandos de retorno aos módulos de origem e a aba de vínculo do painel — spec 04.
- OAuth, sincronização, conflitos, exclusão cruzada, indicador de estado e a aba de sincronização do painel — spec 05. Aqui o painel mostra apenas "Não sincronizado".
- Sugestões de bloco e o slot correspondente no painel — spec 06.
- `react-day-picker` e o componente `Calendar` do shadcn — re-adicionados só quando um date picker em diálogo for necessário; o mês é uma grade própria.
- Notificações e lembretes de prazo.
- Vista de **resources** (uma coluna por sala/professor) — §7 não pede.
- Insights de planejamento × realidade (gráficos, médias). §6 diz que a diferença "pode alimentar histórico e aprendizagem" e **não** deve punir; transformar isso em dashboard é produto novo.
- A Home (`TodayView`) deixar de ser mock — backlog após esta spec.
