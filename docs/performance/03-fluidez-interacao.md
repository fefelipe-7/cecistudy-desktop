# Spec 03 — Fluidez e interação

Data: 2026-09-29
Escopo: auditar a fluidez do app (entrada, arrasto, scroll, feedback, teclado, microcopy) e especificar a implementação que transforma a grade de leitura em grade de manipulação. Esta spec **não cria** regras de domínio: ela executa as decisões que a spec 02 deixou em aberto (D17–D20) e consome a geometria que a spec 02 Fase 2.1 já entregou (`domain/grid-scale.ts`).
Status: **proposta fechada** — nada aqui foi implementado.

**Rastreabilidade — `calendario.md`:** §7 (integral: "Eventos e blocos de planejamento ocuparão posições temporais e **poderão ser arrastados e redimensionados**"), §4 (cancelar/remarcar por ocorrência, via teclado e via menu), §3 (a regra de nunca criar obrigação implicitamente — aqui: um item não pode ser criado por arrasto acidental), §13 (regra de ouro, aplicada ao painel de arraste).
**Executa:** spec 02 D15, D16, D17, D18, D19, D20 · spec 00 A4, D4, D6, D7, INV-4, INV-6 · spec 01 Fase 1.8 (a tela) e Fase 1.9 (a semente).
**Depende de:** nada novo. Não toca Rust além do widen de `GridItem` (DF8).
**Deixa deliberadamente para:**

- spec 03 (D21–D23) — a agenda mobile e o timer; aqui tudo é ponteiro, e a spec 03 substitui o arrasto por ações;
- spec 05 — resolução de conflito de sincronização; aqui o conflito é só derivado e sinalizado (D7);
- spec 06 — a sugestão de bloco; o placeholder aqui é o mesmo, mas a origem do bloco é a Ceci, não a sugestão.

---

## 1. Diagnóstico

Só fatos do código. A coluna "Evidência" é o que se vê ao ler a linha, não uma opinião.

### 1.1 A lacuna de fundo: não existe sistema de arrasto

| #    | Interação                           | Estado atual (arquivo:linha) | Gravidade          | Evidência                                                                                                                                                                                                                                                                         |
| ---- | ----------------------------------- | ---------------------------- | ------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| G-01 | Arraste de item                     | **inexistente**              | Crítica            | `rg "drag                                                                                                                                                                                                                                                                         | pointerdown | pointermove | onMouseMove | setPointerCapture" src/`→ **0 ocorrências em 45 arquivos**. Nenhum`.tsx` importa nada de ponteiro. |
| G-02 | Redimensionamento                   | **inexistente**              | Crítica            | `rg "resize\|Resize" src/` → 0. `TimedItem` (previsto em `docs/calendario/02-grade-desktop.md:128`) não existe.                                                                                                                                                                   |
| G-03 | Criação por gesto                   | **inexistente**              | Crítica            | Nenhum `onPointerDown` na coluna do dia (`CalendarScreen.tsx:430-434`) nem na célula do mês (`CalendarScreen.tsx:638-642`). Ambas são `<div>` passivos.                                                                                                                           |
| G-04 | O `cursor-grab` citado pela spec 02 | **já não existe**            | Baixa (documental) | `docs/calendario/02-grade-desktop.md:27` e `:147` citam `src/App.tsx:918 cursor-grab decorativo`. Hoje `App.tsx:918` é `</div>` do cabeçalho de "Prazos próximos". O `cursor-grab` sumiu junto com a grade estática e **não foi substituído**. Correção de fato a fazer em 02 §1. |
| G-05 | `useGridDrag` (A4)                  | **não existe**               | Crítica            | `docs/calendario/00-fundacao.md:145-149` e `02:135,189` prometem `ui/hooks/use-grid-drag.ts`. O diretório `src/features/calendar/ui/` contém exatamente um arquivo, `CalendarScreen.tsx`.                                                                                         |
| G-06 | `domain/item-menu.ts`               | **não existe**               | Alta               | `02:200` promete a tabela `origin × state → ações`. Sem ela, D17 e INV-6 não têm onde ser decididos na interface.                                                                                                                                                                 |
| G-07 | Faixa de dia inteiro                | **inexistente**              | Alta               | `calendario.md:79` exige "Itens de dia inteiro aparecerão em uma faixa separada". `WeekView` (`CalendarScreen.tsx:371-486`) não tem faixa; e `GridItem` não tem `allDay` (`bridge.ts:49-60`).                                                                                     |
| G-08 | Painel persistente (D18)            | **parcial**                  | Alta               | `CalendarScreen.tsx:335` — `{selected !== null && <ItemPanel …/>}` **desmonta** o painel quando nada está selecionado, e D18 exige que ele _permaneça montado_ ao navegar na semana. Largura `w-[280px]` (`CalendarScreen.tsx:788`), não os 360px de D18 (`02:100`).              |

### 1.2 Precisão do input e geometria

| #    | Interação                                  | Estado atual (arquivo:linha)                   | Gravidade | Evidência                                                                                                                                                                                                                                                                                                                                                                                                             |
| ---- | ------------------------------------------ | ---------------------------------------------- | --------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| P-01 | `snapY` está **errado**                    | `grid-scale.ts:95-97`                          | Crítica   | `snapY` arredonda o pixel em `snapMinutes * 4` = **60px**. Com `hourHeight: 56` (`grid-scale.ts:37`), 15 min são `56 × 15/60 = 14px`. O passo está **4,3× maior** que o desejado: um arraste "de 15 em 15 min" saltaria de ~64 em ~64 minutos. E `snapY` **ignora `hourHeight`**: na densidade ampliada de D16 (`hourHeight: 84`), 15 min são 21px, não 60px. `snapTime` (`:89-92`) está correto — os dois discordam. |
| P-02 | `snapY` não é testado nem usado            | `grid-scale.test.ts`                           | Crítica   | `snapY` não aparece em nenhum teste do arquivo e não é importado por nenhum módulo. O defeito P-01 está **latente**: só aparece quando o arraste existir. `snapTime` tem 2 testes (`:84-95`).                                                                                                                                                                                                                         |
| P-03 | `yToTime` não tem teto                     | `grid-scale.ts:76-78`                          | Média     | Só `Math.max(0, y)`. Passar `y` acima de `visibleHeight` devolve um instante depois da meia-noite do dia pedido, e `placeInDay` (`:134`) rejeita. A clamp do alto tem de existir no hook de arraste, ou nasce um item em `01:00` do dia seguinte ao arrastar para baixo do último pixel.                                                                                                                              |
| P-04 | Custo de `formatToParts` no caminho quente | `grid-scale.ts:64-73` + `time.ts:116-134`      | Crítica   | `yInDay` chama `toWallClock` (1 `formatToParts`) e `toPlainDate` **duas** vezes (mais 2) = **3 `formatToParts` por chamada**. `placeInDay` chama `yInDay` duas vezes (`:137-138`) e `addDaysInTz` duas vezes (`:130-131`) = **8 `formatToParts` por item por coluna**. Com 100 ocorrências × 2 colunas = ~1 600 `formatToParts` por re-render. A 60 Hz isso é ~96 000/s.                                              |
| P-05 | O cache de formatter existe e não é usado  | `time.ts:97-114` vs `CalendarScreen.tsx:90-96` | Média     | `time.ts` tem `formatterCache`. `formatClock` (`CalendarScreen.tsx:89-96`) constrói um `Intl.DateTimeFormat` **novo a cada chamada**, e é chamada por item visível (`:466`), por linha da agenda (`:544`) e por `title` (`:452`). O mesmo ocorre em `:246`, `:251`, `:385` (dentro de `days.map`), `:520`, `:610`.                                                                                                    |
| P-06 | Escala única existe e está certa           | `grid-scale.ts:53-97`                          | Positivo  | `timeToY`/`yToTime` são inversas, com teste de DST real (`:76-82` do `.test.ts`). Esta spec **não toca** nessa matemática; consome.                                                                                                                                                                                                                                                                                   |

### 1.3 Handlers, listeners e render

| #    | Interação                                            | Estado atual (arquivo:linha)     | Gravidade                | Evidência                                                                                                                                                                                                                                    |
| ---- | ---------------------------------------------------- | -------------------------------- | ------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| H-01 | `formatClock` sem cache dentro do `map`              | `CalendarScreen.tsx:466,544,452` | Alta                     | Constrói um `Intl.DateTimeFormat` por item, por render. Com a grade inteira visível são ~100 construções por passe.                                                                                                                          |
| H-02 | `ordered` e `lanes` recalculados sempre              | `CalendarScreen.tsx:426-427`     | Alta                     | `[...items].sort(…)` e `columnize(…)` rodam **dentro do `.map` dos dias**, a cada render, sem `useMemo`. O comentário em `grid-scale.ts:177-179` exige entrada ordenada; a ordem está sendo garantida a cada quadro.                         |
| H-03 | Item de grade é um `<button>` inline, sem componente | `CalendarScreen.tsx:448-469`     | Crítica                  | Não há `TimedItem`. Cada re-render reconcilia N botões e N styles. Com React 19 não há memo implícito: um `setState` por quadro durante o arraste re-renderiza **toda** a grade (7 colunas × 18 células de hora + todos os itens).           |
| H-04 | Relógio de 1 min re-renderiza tudo                   | `CalendarScreen.tsx:120-123`     | Média                    | `setInterval(…, 60_000)` chama `setNow`, que invalida `WeekView` inteiro e re-roda 426-427 em todas as colunas, só para mover uma linha de 1px (`:477-485`).                                                                                 |
| H-05 | `new Intl.DateTimeFormat` dentro de `days.map`       | `CalendarScreen.tsx:385-387`     | Média                    | 7 construções por render, dentro do map do cabeçalho.                                                                                                                                                                                        |
| H-06 | Único listener global                                | `App.tsx:149-160`                | Baixa                    | `window.addEventListener("keydown", …)` com cleanup correto (`:159`). `preventDefault()` em `:154` é legítimo (⌘K). Não há `{ passive: … }` porque `keydown` não é cancelável-por-scroll — **não há o que corrigir aqui**.                   |
| H-07 | Sem `pointercancel` / `blur` / `visibilitychange`    | —                                | Crítica (ao implementar) | Não existe porque não existe arraste. Ver DF6: é o que impede o item preso ao cursor.                                                                                                                                                        |
| H-08 | Scrim móvel é um `<button>` sobre a tela             | `App.tsx:184-189`                | Média                    | `fixed inset-0 z-30` acima do conteúdo. Num gesto em janela estreita, um `pointerup` fora do alvo pode cair no scrim e ser lido como clique. Não afeta o desktop (`isMobile` falso), mas afeta a Fase que testar com a janela no `minWidth`. |
| H-09 | Sidebar anima `width` com transição                  | `App.tsx:192-193`                | Média                    | `transition-[width] duration-200`. Um ⌘\ ou clique no `PanelLeft` (`:322-327`) durante um arraste muda a largura disponível e **remapeia a largura das colunas da grade** no meio do gesto. Ver DF2.                                         |

### 1.4 Scroll

| #    | Interação                                     | Estado atual (arquivo:linha)                        | Gravidade | Evidência                                                                                                                                                                                                                                                                                                                                                                                              |
| ---- | --------------------------------------------- | --------------------------------------------------- | --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| S-01 | Scroll aninhado                               | `App.tsx:349` + `CalendarScreen.tsx:371`            | Crítica   | O modo semana tem **dois** containers de scroll: `overflow-auto` em `App.tsx:349` e `overflow-auto` em `CalendarScreen.tsx:371`. Rolando a grade até o fim, o evento **encadeia** para o container externo (não há `overscroll-behavior`), e a página "pula" atrás da grade.                                                                                                                           |
| S-02 | Sem `overscroll-behavior` em lugar nenhum     | `styles.css`                                        | Alta      | `rg "overscroll" src/` → 0. `styles.css:191-205` estiliza a barra (`scrollbar-slim`) e nada mais.                                                                                                                                                                                                                                                                                                      |
| S-03 | Cabeçalho de dias **não** é `sticky left`     | `CalendarScreen.tsx:372-373` vs `:406`              | Crítica   | O eixo de horas do corpo é `sticky left-0` (`:406`), mas a primeira célula do cabeçalho é um `<div aria-hidden>` comum (`:373`). Rolando horizontalmente, o cabeçalho de dias sai de vista e um dia fica **acima** do eixo de horas fixo: as colunas se dessincronizam.                                                                                                                                |
| S-04 | Grade não cabe no `minWidth`                  | `tauri.conf.json:20` + `CalendarScreen.tsx:372,402` | Crítica   | A coluna mínima é `52px + 7 × 112px = 836px`. A `1024px`: `1024 − 248` (sidebar, `App.tsx:193`) `− 32` (`px-4`, `App.tsx:320`) `− 32` (`p-4`, `CalendarScreen.tsx:259`) `= 712px`, e `420px` com o painel aberto (`w-[280px]` + gap, `CalendarScreen.tsx:788`). A spec 02 §39 promete "caber sem scroll horizontal nessa largura" — hoje ela **não cabe**, e com painel aberto não cabe nem em 1280px. |
| S-05 | Nenhum "scroll into view" ao mudar de período | `CalendarScreen.tsx:266-280`                        | Alta      | `step(±7)` e `goToday()` só mudam `anchor`. A posição de scroll do `overflow-auto` de `:371` **persiste**, então trocar de semana abre no mesmo deslocamento: ir de uma semana cheia para uma semana vazia abre em 14:00 num vazio.                                                                                                                                                                    |
| S-06 | Sem `scroll-margin-top` no cabeçalho `sticky` | `CalendarScreen.tsx:372`                            | Média     | Qualquer `scrollIntoView` numa célula colide com o cabeçalho de 44px.                                                                                                                                                                                                                                                                                                                                  |
| S-07 | Altura visível < altura do dia                | `grid-scale.ts:42-44` vs `tauri.conf.json:19`       | Média     | `visibleHeight` = `18 × 56 = 1008px`. Janela de 820px menos cabeçalho 52px (`App.tsx:320`), paddings e a linha de dias ≈ 660px visíveis. **1,5 ecrãs** de rolagem por dia — daí S-05 ser obrigatória, não cosmética.                                                                                                                                                                                   |
| S-08 | `ScrollArea` do shadcn existe e está fora     | `scroll-area.tsx`                                   | Baixa     | Nenhum import em `src/`. Se fosse adotado na grade, o `Viewport` do Radix criaria um container extra entre o `sticky` e o scroller. Ver DF10.                                                                                                                                                                                                                                                          |

### 1.5 Feedback, estados e acessibilidade

| #    | Interação                                     | Estado atual (arquivo:linha)             | Gravidade                | Evidência                                                                                                                                                                                                                                                                                                                           |
| ---- | --------------------------------------------- | ---------------------------------------- | ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| F-01 | **Não há estado de carregamento**             | `CalendarScreen.tsx:113,154-194,217-221` | Crítica                  | Enquanto `loaded === false`, `empty` é `false` (`:217-221`) e a tela renderiza a grade **vazia** — 7 colunas de 18 horas, sem nada. Não há skeleton: `skeleton.tsx` existe e **não é importado** em lugar nenhum. Pior: trocar de semana (`:266-280`) refaz o efeito (`:194`) e a grade pisca para o estado vazio a cada navegação. |
| F-02 | Erro de leitura é engolido                    | `CalendarScreen.tsx:183-185`             | Crítica                  | `catch { }` vazio. O comentário diz "o erro aparece quando a usuária tenta agir" — **não existe nenhum caminho de erro na interface**: nenhum toast, nenhuma faixa, nenhum estado. `IpcUnavailableError` (`ipc.ts:13-18`) tem uma mensagem boa que nunca chega à usuária.                                                           |
| F-03 | Botões inertes sem `disabled`                 | `CalendarScreen.tsx:813-818`             | Alta                     | "Concluir" e "Reagendar" **não têm `onClick` nem `disabled`**. A spec 00 Fase 0.6 (`00-fundacao.md:283-297`) já corrigiu 12 controles inertes exatamente assim; dois escaparam. Um botão que aceita clique e não faz nada é pior que um desabilitado.                                                                               |
| F-04 | "Criar evento" não cria evento                | `CalendarScreen.tsx:836-839`             | Alta                     | O botão do estado vazio chama `onOpenList`, que em `App.tsx:359` é `() => setView("list")` — leva ao modo **Lista** do dashboard, com outro conjunto de dados. O rótulo promete uma criação que não existe.                                                                                                                         |
| F-05 | `role="tablist"` sem `tabpanel`               | `CalendarScreen.tsx:287-300`             | Alta                     | Cinco `<button role="tab" aria-selected>` sem `aria-controls`, sem `tabpanel`, sem `tabIndex` rodante, todos alcançáveis por `Tab`. Um leitor de tela anuncia "aba" e nunca diz o que a aba contém. `App.tsx:987-1019` (`SubjectView`) faz **certinho** — o padrão correto já está no repositório, a 900 linhas de distância.       |
| F-06 | `role="tablist"` morto no `App.tsx`           | `App.tsx:733-762`                        | Média                    | `ViewSwitch` (Grade/Kanban/Lista) ainda existe com `role="tablist"` (`:741`) e `role="tab"` (`:748`), sem painel. A spec 02 Fase 2.7 (`:219-226`) prometeu removê-lo; só `CalendarScreen` o substituiu.                                                                                                                             |
| F-07 | Sem `aria-live` em lugar nenhum               | `rg "aria-live" src/` → 0                | Crítica (ao implementar) | Um arraste sem anúncio é inoperável por leitor de tela. Ver DF11.                                                                                                                                                                                                                                                                   |
| F-08 | Item de grade é `<button>` com `title` nativo | `CalendarScreen.tsx:448-452`             | Média                    | `title=` (`:452`) gera um tooltip do SO que **atrasa ~1s** e não é estilizável, convivendo com o `TooltipProvider delayDuration={250}` de `App.tsx:181`. Dois sistemas de dica para o mesmo dado.                                                                                                                                   |
| F-09 | D7 sem superfície                             | `overlap.ts:21`                          | Média                    | `findOverlaps` **não é importado por nenhum arquivo de UI** — só por `overlap.test.ts`. A sobreposição é permitida e derivada (D7), mas hoje nada **sinaliza**: o usuário só descobre a sobreposição porque as colunas estreitaram (`grid-scale.ts:180-226`). Falta a marca visual.                                                 |
| F-10 | Cancelamento visual                           | `CalendarScreen.tsx:455`                 | Baixa                    | `opacity-50 line-through` para `overrideKind === "cancelled"`. É legível, mas o item continua occupying a coluna e disputando largura com os ativos.                                                                                                                                                                                |

### 1.6 Microcopy e affordances

| #    | Interação                                                | Estado atual (arquivo:linha) | Gravidade | Evidência                                                                                                                                                                                                                                                                                      |
| ---- | -------------------------------------------------------- | ---------------------------- | --------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| M-01 | Nada comunica que se pode arrastar                       | `CalendarScreen.tsx:448-469` | Crítica   | Sem `cursor`, sem `aria-grabbed`, sem dica, sem tooltip de instrução. E o `cursor-grab` que comunicava (removido, ver G-04) já não está. Hoje a **única** affordance é o `hover:bg-accent/40` que existe na Agenda (`:541`) e **não** na Semana (`:453`). Inconsistência entre modos.          |
| M-02 | Sem cursor `not-allowed` para item não arrastável        | `CalendarScreen.tsx:453-455` | Alta      | D17 (`02:93-97`) exige: item `google` ou `state ∈ {cancelado, dispensado}` é **não arrastável**, com `cursor: not-allowed` **visível antes do gesto** (INV-6). Nada disso existe.                                                                                                              |
| M-03 | Sem `cursor: ns-resize` nem alças                        | `CalendarScreen.tsx:448`     | Crítica   | `cursor-grab` no corpo, `ns-resize` nas alças — nenhum dos três existe.                                                                                                                                                                                                                        |
| M-04 | Sem placeholder nem badge de horário                     | —                            | Crítica   | Enquanto o arraste corre, nada mostra **onde o item vai cair** nem **que horas serão gravadas**. Google Calendar, FullCalendar (`fullcalendar.io/docs/event-dragging-resizing`, `snapDuration`) e Notion (`CalendarEvent.Resize edge="start"\|"end"`, `Escape cancels a gesture`) têm os três. |
| M-05 | Sem eixo de "agora" pulsante nem estado de foco na grade | `CalendarScreen.tsx:477-485` | Baixa     | A linha do momento existe (`z-10`, 1px, `bg-primary`) mas não é `pointer-events-none` por acidente — só por não estar no fluxo de clique. Ao virar item arrastável, precisa ser explicitamente `pointer-events-none` **antes** do DF1.                                                         |

### 1.7 Casos de força

| #    | Caso                                          | Estado atual                             | Gravidade | Evidência                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| ---- | --------------------------------------------- | ---------------------------------------- | --------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C-01 | Item que começa **antes** da janela carregada | `CalendarScreen.tsx:201-210`             | Crítica   | `byDay` agrupa por `toPlainDate(item.startsAt, ZONE)`. O SQL devolve o item (`commands.rs:491`, `starts_at < to AND ends_at > from`), mas se ele começou 3 dias antes da janela, cai numa chave de dia **fora** das 7 colunas renderizadas (`:421`) e **desaparece**. O comentário de `:127-133` promete resolver com "um dia de folga em cada ponta" — a folga é de 1 dia (`window` em `:151`), e o bug precisa de 3. Um evento de ter à noite que atravessa a semana desaparece. |
| C-02 | Item na meia-noite                            | `grid-scale.ts:64-73,124-155`            | Média     | A matemática está **correta** e testada (`grid-scale.test.ts:107-125,135-144`). Mas o desenho (`:444`) toma só `placeInDay(...)[0]` e nunca renderiza o pedaço do dia seguinte na coluna correta quando o `byDay` do dia seguinte não existe.                                                                                                                                                                                                                                      |
| C-03 | Item de dia inteiro                           | ausente                                  | Crítica   | `GridItem` sem `allDay` (`bridge.ts:49-60`); SQL sem `e.all_day` (`commands.rs:487-492`); nenhuma faixa na grade. Bloqueia `calendario.md:79` e o "resize-to-absorb" do Notion.                                                                                                                                                                                                                                                                                                    |
| C-04 | Bloco de planejamento na grade                | ausente                                  | Crítica   | `list_events_in_window` só faz `event_occurrence JOIN calendar_event` (`commands.rs:487-492`). **`plan_block` nunca entra.** A spec 02 Fase 2.4 (`:192`) manda persistir o arraste de um bloco por `reschedule_block` — um comando que a grade nunca pode acionar, porque o bloco nunca chega nela.                                                                                                                                                                                |
| C-05 | Muitas ocorrências sobrepostas                | `grid-scale.ts:180-226`                  | Média     | `columnize` é **O(n)** por cluster e devolve `columns` corretas. Mas `App.tsx:803` mostra que o padrão do repo (Kanban) usa `min-w-[900px] grid-cols-4` — a 8 faixas de 50% numa coluna de 130px dão **16px de largura** e nenhum texto. Falta um piso (`min-w` por coluna, ou `--columns` limitado) e um agregador "+N".                                                                                                                                                          |
| C-06 | `origin` ausente no wire                      | `bridge.ts:49-60`, `commands.rs:487-492` | Crítica   | D17, INV-6 e `layers.ts:57-59` (`layerIsEditable`) **não podem ser aplicados**: o `GridItem` que a UI recebe não tem `origin`. Bloqueio duro de D17.                                                                                                                                                                                                                                                                                                                               |
| C-07 | Cancelar/remarcar por ocorrência              | ausente                                  | Alta      | `RecurrenceScopeDialog` (prometido em `02:208-217`) não existe. `ItemPanel` mostra os badges (`CalendarScreen.tsx:808-809`) mas não tem ação. Sem INV-5 garantido na interface.                                                                                                                                                                                                                                                                                                    |
| C-08 | `allDay` nunca lido do Rust                   | `types.ts:60` existe, `GridItem` não tem | Média     | `CalendarEvent.allDay` está no tipo de domínio (`types.ts:60`) mas some no tipo da grade. Inconsistência de mapeamento entre `calendar_event` e `GridItem`.                                                                                                                                                                                                                                                                                                                        |

---

## 2. Decisões fechadas

### 2.1 Herdadas

A4 (`useGridDrag` próprio) · D4 (tons de camada) · D6 (UTC + IANA, sem ajuste ao viajar) · D7 (sobreposição permitida, conflito derivado) · D15 (uma escala) · D16 (densidade) · D17 (arrasto só para `origin !== "google"`) · D18 (painel é coluna, sobrevive à navegação) · D19/D20 · INV-1..INV-7.

### 2.2 Novas

**DF1 — Tecnologia de arrasto: Pointer Events manuais, sem biblioteca.**

Decisão: `pointerdown` / `pointermove` / `pointerup` sobre `setPointerCapture`, com `pointercancel`, `lostpointercapture`, `blur` e `visibilitychange` como rede de segurança. Nenhuma dependência nova em `package.json`.

Justificativa — o caso é **grade temporal com encaixe**, não reordenação de lista, e as três candidatas falham em coisas diferentes:

| Opção                                   | Peso                                                                                                                                                   | React 19                                                                       | Modelo                                                                                                                                                                                                                                                     | Veredito                                                                                                                                                                                                                                                                                                                                                                                                                               |
| --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Pointer Events manuais**              | 0 kB                                                                                                                                                   | nativo                                                                         | o que a grade precisa                                                                                                                                                                                                                                      | **Escolhida**                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `@dnd-kit/react` (o sucessor do `core`) | `@dnd-kit/core` = 18,9 kB gzip, 5 deps, unpacked 1,1 MB, 18,1M downloads/semana ([devpick.co/pkg/@dnd-kit/core](https://devpick.co/pkg/@dnd-kit/core)) | o novo `@dnd-kit/react` é uma camada fina sobre um core agnóstico de framework | sensor → colisão → plugins de `transform`. É _sortable-first_: não tem "encaixe numa escala temporal" em lugar nenhum; a matemática continuaria 100% nossa dentro de um framework de colisão que nunca entenderia `timeToY`                                | Rejeitada                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `@formkit/drag-and-drop`                | ~4 kB gzip, 108 925 downloads/semana ([npm](https://www.npmjs.com/package/@formkit/drag-and-drop))                                                     | sim                                                                            | data-first **de listas**: reordena o array, com _drop zones_ e _insert indicators_                                                                                                                                                                         | Rejeitada — é uma lib de lista; o item da grade não é uma posição num array                                                                                                                                                                                                                                                                                                                                                            |
| `react-aria` `@react-aria/dnd`          | o pacote completo é grande; `useDrop`/`useDraggable` são tree-shakeable                                                                                | sim                                                                            | o **melhor** modelo de acessibilidade que existe: `Enter` entra em modo arrasto, `Tab` cicla entre alvos válidos, `Enter` solta, `Escape` cancela, com _live region_ e anúncios localizados ([react-aria.adobe.com/dnd](https://react-aria.adobe.com/dnd)) | Rejeitada por um motivo só: `useDraggable` **usa a API nativa de drag-and-drop do navegador** ("mouse and touch drag and drop interactions utilize the native browser APIs" — na documentação da própria Adobe). Em WebView2 o arrasto nativo **não pinta um ghost** e o evento `dragover` não dá controle fino de posição por quadro — que é justamente o que a grade precisa. Acessibilidade é reaproveitada pelo DF11, não pela lib |

O `Pointer Events` é **Baseline "widely available"** ([MDN](https://developer.mozilla.org/en-US/docs/Web/API/Element/setPointerCapture)) e o WebView2 é Chromium Evergreen, que se atualiza sozinho ([Tauri — Webview Versions](https://v2.tauri.app/reference/webview-versions/)) — ou seja, `setPointerCapture` funciona em toda máquina Windows 11 sem detecção de feature. A dependência de runtime é zero.

Alternativa rejeitada: HTML5 drag-and-drop nativo (`draggable="true"`). É famously quebrado dentro de WebView/shadow DOM e não oferece controle por quadro.

Consequência: `src/features/calendar/ui/hooks/use-grid-drag.ts` (~200 linhas), testável por máquina de estados, sem `package.json` tocado. Gatilho de revisão: se um dia o app precisar de arrasto **entre** telas (uma lista reordenável, uma barra lateral reordenável), reavaliar `@dnd-kit/react` **para aquele caso**, mantendo a grade manual.

**DF2 — Coordenadas: `getBoundingClientRect` do corpo da grade, capturada uma vez, em CSS px. `devicePixelRatio` nunca entra.**

Decisão: no `pointerdown` (estado `armed`) grava-se um único `origin: DOMRect` do **corpo rolável** (não da coluna, não da grade inteira) mais `scrollTop`/`scrollLeft` do mesmo elemento. De `armed` em diante, a posição é `clientX/Y − origin.left/top`, e o `top` do item é `rawY + originRect.top − bodyRect.top + scrollTop`. Nunca `devicePixelRatio`, nunca `offsetX/offsetY` (que são relativos ao alvo e mudam com `pointer capture`), nunca um segundo `getBoundingClientRect` por quadro.

Justificativa:

1. `getBoundingClientRect` **já é em CSS px** e já é independente de `devicePixelRatio` e do zoom do SO — o WebView2 aplica o DPI ao viewport antes de entregar coordenadas CSS. Multiplicar por `devicePixelRatio` é o erro clássico que quebra o arraste em telas 150%/175%.
2. Ler o `DOMRect` **uma vez** no `armed` é o que dá **imunidade ao scroll durante o gesto**: se a usuária rolar a grade com a roda enquanto arrasta, `clientY` continua correto e o `+ scrollTop` reposiciona. Um `getBoundingClientRect()` por quadro continuaria correto, mas força layout a 60 Hz — que é o caminho para o _layout thrashing_ que o Chrome documenta exatamente para grades de dados ([modern-web-guidance — interactions in complex layouts](https://github.com/GoogleChrome/modern-web-guidance-src/blob/main/guides/performance/interactions-in-complex-layouts/guide.md)).
3. `setPointerCapture` faz o `pointermove` ir para o elemento capturado, então `e.target` deixa de ser a coluna embaixo do cursor — `elementFromPoint` não pode ser a fonte da coluna de destino. **A coluna vem da aritmética** `⌊(x − gutter) / colWidth⌋`, que é a mesma que o CSS grid usou para desenhar, e que pode divergir do `elementFromPoint` quando há colunas de largura variável.

Consequência: o hook recebe `bodyRef` + `gutterWidth` + `columnCount` e expõe `columnAt(clientX)`, `yToSlot(clientY)`. A troca de densidade (D16) durante um arraste é **bloqueada** (DF6) porque muda `columnCount` e invalidaria o `DOMRect` capturado. O toggle da sidebar (`App.tsx:192`, transição de 200 ms em `width`) **também** fica bloqueado durante o arraste — ver H-09.

**DF3 — Encaixe: passo único, 15 min, corrigido em pixel pela escala; `Alt` como escape.**

Decisão: o passo em pixel é `snapMinutes / 60 × hourHeight` — **14px** com os valores de `DEFAULT_SCALE` (`grid-scale.ts:35-39`), **não** os 60px de `snapY` hoje (`grid-scale.ts:96`, ver P-01). `snapY` é corrigido e coberto por teste antes de qualquer uso. Segurar `Alt` **durante o arraste** desliga o encaixe (passo de 1 minuto) para alinhar em meia hora exata, em 09:07, no limite de uma aula. O `snapMinutes` continua vindo de `GridScale` — a escala é a fonte, como D15 exige.

Justificativa: o encaixe de 15 min é o que torna a grade **digitável** — arrastar 40px para baixo tem de dar 15 min, não 40px (critério de aceite da spec 02 Fase 2.4, `02:194`). Mas `calendario.md` não pede só encaixe: uma aluna que bloqueia revisão 09:00–09:30 precisa poder **criar** esse item, e 09:07 existe. Um escape de 1 minuto é o custo de uma tecla e o preço da liberdade — Google Calendar e FullCalendar resolvem com o mesmo par conceitual (`snapDuration` + tolerância). `Alt` foi escolhido sobre "encaixe só quando perto da linha" porque o segundo exige saber **qual** linha e falha em grade vazia.

Alternativa rejeitada: arrasto **livre** com gravação só no `pointerup` encaixado. O item segue o ponteiro em passos de 1 min e "salta" ao soltar — é o padrão do Google e é péssimo com `hourHeight` de 56px, porque a diferença entre "onde soltei" e "onde caiu" chega a meio snapping e o usuário não confia.

Consequência: **DF3 é um bug fix bloqueante da Fase F1**, não parte do arraste. `grid-scale.test.ts` ganha: `snapY` é inverso de `yToTime` dentro de meio passo; `snapY` respeita `hourHeight` (84px → passo 21px); `snapY` é monotônico.

**DF4 — Durante o arraste, o DOM muda direto; o React só vê as transições.**

Decisão: enquanto o ponteiro se move, **nenhum `setState`**. O ghost, o placeholder e a linha de encaixe escrevem **CSS custom properties** em elementos já montados, uma vez por quadro de animação:

```
ghost.style.setProperty("--grid-top",    `${top}px`)
ghost.style.setProperty("--grid-height", `${height}px`)
ghost.style.setProperty("--drag-dx",     `${dx}px`)
ghost.style.setProperty("--drag-dy",     `${dy}px`)
```

com CSS `.dragging { transform: translate3d(var(--drag-dx, 0), var(--drag-dy, 0), 0) }`. O React é chamado **só** nas transições de estado: `idle → armed → dragging → committing → idle` (ou `→ error`).

Justificativa — este é o ponto que concilia o "otimismo local" da D17 com o React:

- D17 pede que **o item siga o ponteiro** e que a persistência só ocorra ao soltar. Se o otimismo é estado React (`useState({top, height})`), cada quadro re-renderiza a grade inteira: H-03 (item inline, sem componente), H-02 (`columnize` e `sort` sem memo) e P-04 (8 `formatToParts` por item) somam ~100 itens × ~1 600 `formatToParts` = um passe que **estoura os 16,6 ms** só em `formatToParts`. Com `setState` a 60 Hz o arraste seria um slideshow.
- `transform` e as custom properties não disparam **layout** nem **paint**: só composite. É a propriedade que o compositor pode resolver fora da main thread.
- Manter o estado de arraste **fora** do React tem um custo: o React não sabe onde o item está. A conciliação é explícita — no `commit`, o hook **escreve o valor final uma última vez** e só então chama o commit de domínio; se o comando falhar, o React recebe o item original de volta e o ghost desaparece. Não existe estado intermediário que precise ser reconciliado.

Alternativa rejeitada: estado React com `startTransition`. `useTransition` marca a atualização como não urgente, mas **o `pointermove` continua gerando um `setState` por evento** e o React ainda agenda trabalho por quadro. Ajuda a prioridade, não a contagem.

Consequência: o hook **não** devolve posição para o React. Quem precisa da posição para announces é o callback `onAnnounce` (§3.6), alimentado por `requestAnimationFrame` com debounce (DF11). Fase F2 é onde isso é medido.

**DF5 — Política de teclado: aritmética de 15 min com três modificadores, e um modo de redimensionamento explícito.**

Decisão, com o item focado:

| Tecla                 | Efeito                                                                                                             |
| --------------------- | ------------------------------------------------------------------------------------------------------------------ |
| `←` / `→`             | mover **15 min**                                                                                                   |
| `↑` / `↓`             | mover **1 dia**, preservando a hora                                                                                |
| `Shift` + `←`/`→`     | encurtar/alongar **15 min** na borda de `endsAt`                                                                   |
| `Shift` + `↑`/`↓`     | encurtar/alongar **1 dia**                                                                                         |
| `Alt` + qualquer seta | passo de **1 minuto** (equivale ao escape de encaixe do DF3)                                                       |
| `Enter`               | abre o editor no slot atual                                                                                        |
| `Escape`              | desfaz o último ajuste de teclado (pilha de 1 nível, ver DF6)                                                      |
| `F2`                  | entra em **modo redimensionamento**: `←`/`→` movem `startsAt`, `Ctrl`+`←`/`→` movem `endsAt`, `Escape` sai do modo |

Justificativa: a spec 00 A4 (`00-fundacao.md:149`) e a Fase 2.4 da spec 02 (`02:194`) já prometeram "setas ±15 min; `Shift`+setas ±15 min de duração". O que elas **não** dizem — e é a lacuna real — é como se **redimensiona pela borda de cima** pelo teclado. Setas em uma grade de duas dimensões não têm como ser "mover" e "redimensionar" ao mesmo tempo. O modo `F2` é o `role="application"` do padrão Primer/GitHub, que existe exatamente porque `↑`/`↓` estão normalmente reservations dos leitores de tela ([primer.style — Drag and Drop](https://primer.style/accessibility/patterns/drag-and-drop/)).

Alternativa rejeitada: `Ctrl`+seta redimensiona e seta pura move. Colide com "ctrl + seta = mover cursor/palavra" em leitores de tela e é indistinguível de um atalho do sistema.

Consequência: o item de grade recebe `aria-keyshortcuts` e um texto de ajuda em `aria-describedby` apontando para a **primeira vez** que ele recebe foco (não todo frame).

**DF6 — Cancelamento: cinco rotas, um único `reset()`, e nenhum item pode ficar preso ao cursor.**

Decisão. O `reset(original)` é **idempotente** e chamado por qualquer um de:

1. `Escape` pressionado (`keydown` em `window`, registrado no `armed`);
2. `pointercancel` — cancelamento do SO (gesto de toque interrompido, palm rejection, troca de app);
3. `lostpointercapture` — a captura foi roubada ou o elemento desmontou;
4. `blur` de `window` — a janela do Tauri perdeu o foco (outro monitor, alt-tab, UAC);
5. `visibilitychange → hidden` — a aba ficou oculta.

Além disso:

- `releasePointerCapture` é chamado em todo caminho de saída e **guardado por `hasPointerCapture`**, porque `releasePointerCapture` lança `NotFoundError` se o ponteiro já não estiver capturado ([MDN](https://developer.mozilla.org/en-US/docs/Web/API/Element/setPointerCapture)) — e um `try`/`catch` em release não é opcional, é obrigatório.
- Mudar de semana (`step`), trocar de modo (`setMode`), abrir/fechar o painel (`selected`), ou mudar a densidade **durante um `dragging`** é **bloqueado**: ou o gesto é cancelado antes (a UI esconde os controles), ou o gesto termina. Racional: qualquer um desses muda o `DOMRect` capturado (DF2) e transformaria o arraste em um salto.
- Cada `pointerdown` **descarta** o `reset` pendente anterior. Sem isso, dois `pointerup` followed de um `reset` atrasado reposicionariam o item errado.
- **Zero chamadas de domínio** em qualquer rota de cancelamento (critério de aceite de `02:194`).

Justificativa: as rotas 2 e 3 são o contrato do Pointer Events; a rota 4 é a que **só** o Tauri tem. Uma janela Win32 que perde o foco não emite `pointerup` de forma confiável — sem a rota 4, soltar o botão fora da janela deixa o item colado ao cursor e a grade num estado impossível. O `pointercancel` é `bubbles: false` na spec ([Pointer Events 4](https://www.w3.org/TR/pointerevents4/)) — quem escuta tem de estar no elemento capturado ou no `window` em fase de captura, o que `setPointerCapture` sobre o item garante.

**DF7 — Limiar de 4 px e o `click` depois do `pointerup` é engolido.**

Decisão: `pointermove` abaixo de **4px** acumulados deixa o gesto ser um clique (seleciona o item e abre o painel). Acima de 4px, vira arrasto. No `pointerup` que fecha um arrasto, um **token de supressão de clique** é posto; o `click` seguinte é `preventDefault()` + `stopPropagation()` e o token é limpo num microtask.

Justificativa: sem isso, todo arraste termina abrindo o editor — o pior defeito de UX possível numa grade. O limiar de 4px é o que Both Notion e o calendário do `forceCalendar` usam como padrão documentado; abaixo disso a intenção do usuário é clicar, acima disso é mover. 4px e não 5 ou 8 porque o item da coluna já é pequeno e um limiar alto faz "arrastar" parecer "não responde".

Consequência: o `<button>` do item (`CalendarScreen.tsx:448`) precisa de `onClick` que consulte o token. `preventDefault()` aqui é seguro e **não** afeta scroll: o `click` de um ponteiro **já** não é o que rola a página em WebView2, que usa o compositor.

**DF8 — `GridItem` ganha `origin` e `allDay`. É uma mudança de Rust, e é pré-requisito de D17.**

Decisão: `GridItem` (`bridge.ts:49-60`) passa a ter `origin: Origin`, `allDay: boolean`, `commitment: Commitment` e `timeZone: IanaTimeZone`, e o `SELECT` de `list_events_in_window_core` (`commands.rs:487-492`) passa a trazê-los (`e.origin`, `e.all_day`, `e.commitment`, `e.time_zone`).

Justificativa: **D17 não é implementável hoje.** `layerIsEditable` (`layers.ts:57-59`) existe e é testável, mas `ui/` não tem como chamar, porque o `GridItem` que chega não tem `origin`. A INV-6 (`00-fundacao.md:192`, "nenhum comando de escrita toca campo de item com `origin = 'google'`") é garantida no Rust — mas D17 exige que **o bloqueio seja visível antes do gesto**, e um bloqueio invisível que o usuário só descobre depois de arrastar é exatamente a "mentira de interface" que `02:95` recusa. Não há caminho around: ou o `origin` vem pelo wire, ou D17 vira `cursor-grab` que dá num erro 2 segundos depois.

Alternativa rejeitada: um comando separado `list_item_permissions(ids)`. Duas idas ao IPC porInteraction e uma fonte de verdade duplicada; `origin` é **coluna**, não permissão derivada.

Consequência: é o único ponto desta spec que toca `src-tauri/`. É uma widening de payload, não uma regra — nenhuma regra nova no Rust.

**DF9 — O componente **não escolhe** o comando. A escolha sai de `kind`, e a reconciliação é assíncrona e silenciosa.**

Decisão: no `commit`, o hook entrega `{ item, nextStartsAt, nextEndsAt }` a uma função de `data/` chamada `applyGesture(item, range)`. Essa função **deriva** o comando do tipo do item — ocorrência com `ruleId !== null` → `move_occurrence` (bridge.ts:210); bloco → `reschedule_block` (`:323`); evento avulso → `update_event` (`:192`) — e **nunca** recebe o nome do comando como argumento. `ui/` não conhece nenhum dos três.

Justificativa: a spec 02 Fase 2.4 (`:192`) diz "a escolha é derivada de `kind`, nunca escolhida pelo componente", e é a forma de INV-5 não se perder: se `ui/` escolhesse, bastaria um `if` errado para uma recorrência virar edição de série. A derivação fica em `data/`, que é a camada que conhece o schema.

Além disso:

- Só **uma** chamada de IPC por gesto, no `pointerup`. **Nunca** por quadro. Um arraste a 60 Hz × 1000ms = 60 `invoke`s serializados por JSON-RPC ([Tauri — IPC](https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/concept/Inter-Process%20Communication/index.mdx)) seria 60 serializações + 60 desserializações na main thread, e o custo escala com tamanho × frequência.
- A reconciliação é **otimista e silenciosa**: o item já está no lugar certo na tela (foi o DF4 que o pôs lá); a leitura da janela **não** é recarregada. Só um `catch` faz o rollback.
- O `catch` é **não-fatal e visível**: rollback para a posição original + uma faixa inline na grade (DF13). Nada de `alert`, nada de modal.

**DF10 — Scroll: um container, `overscroll-behavior: contain`, e nenhuma biblioteca de scroll.**

Decisão:

- No modo `semana`, o `overflow-auto` de `CalendarScreen.tsx:371` é o **único** scroller vertical. O de `App.tsx:349` recebe `overscroll-behavior-y: contain` nesse modo e `overflow: hidden` na área da grade — a rolagem que chega no fim **para**, e não arrasta a página (S-01, S-02).
- **`ScrollArea` do Radix não é adotada na grade.** Ela está no repositório (`scroll-area.tsx`) e não é importada em lugar nenhum (S-08). Motivo: o `Viewport` do Radix é um elemento com `overflow: hidden` entre o conteúdo e o scroller, o que empurra o `sticky top-0` do cabeçalho de dias (`:372`) para dentro do `Viewport` e dá ao `sticky left-0` do eixo de horas (`:406`) uma referência de scroll errada. Uma grade temporal com dois eixos `sticky` cruzados precisa de _um_ scroller nativo.
- **`overscroll-behavior: contain`** em `.grid-scroll` (novo utility em `styles.css`) e **`scroll-margin-top: 44px`** no corpo, para que o `scrollIntoView` do DF-none/generated não fique debaixo do cabeçalho.
- A largura mínima por coluna cai de 112px para **96px**, e a coluna do eixo de horas de 52px para 48px: `48 + 7 × 96 = 720px`, que **cabe** nos 712px disponíveis no `minWidth` com sidebar recolhida, e com a sidebar aberta no padrão de 1280 (968px) sobra folga para o painel. Isso conserta S-04 e faz a promessa de `02:39` ser verdadeira.
- O cabeçalho de dias ganha `sticky left-0` **na primeira célula** (S-03), para que ele acompanhe o eixo de horas.

Alternativa rejeitada: `react-window`/`@tanstack/virtual` na grade. Virtualizar um eixo Y de 18 horas com altura **fixa e conhecida** (`visibleHeight`, `grid-scale.ts:42`) é trabalho sem retorno: o conteúdo são 7 colunas e ~100 itens. `content-visibility` (DF12) dá 90% do ganho por 3 linhas de CSS e **mantém** a árvore de acessibilidade, que a virtualização remove.

**DF11 — Acessibilidade do arraste: `aria-live` com debounce, `aria-grabbed` recusado.**

Decisão:

1. Uma `aria-live="polite"` (não `assertive`) na raiz do Calendário, **debounced a 120 ms**, anuncia: início do arraste ("Cálculo II movido, grabbed"), mudança de slot ("quinta-feira, 14:30"), fim ("movido para quinta-feira 14:30"), cancelamento ("movimento cancelado"), falha ("não foi possível mover"). O conteúdo da live region muda **em texto**, nunca em atributo.
2. `aria-grabbed` **não é usado**. Foi marcado para depreciação pelo grupo de trabalho ARIA em 2016 ([ACTION-1672](https://www.w3.org/WAI/ARIA/track/actions/1672)) e tem suporte ruim — o NVDA e o JAWS anunciam em parte, mas o VoiceOver no iOS não anuncia o estado de _grabbed_ ao navegar pelo item ([darins.page — screen readers and drag-and-drop](http://www.darins.page/articles/screen-readers-drag-drop-2)). O padrão que funciona é texto estático no acessível + live region.
3. O `120 ms` não é arbitrário: o Primer documenta exatamente o mesmo número, com a mesma razão — "positional updates may occur rapidly, sometimes even before the previous update has been announced, and potentially cause confusion and disruption" ([primer.style](https://primer.style/accessibility/patterns/drag-and-drop/)).
4. `role="application"` **apenas** no item enquanto ele está em modo arraste por teclado (`F2`, DF5), para que `←`/`→` parem de ser atalhos do leitor de tela naquele instante. Fora do modo, o item é um `button` comum.
5. Todo item arrastável tem `aria-keyshortcuts` e um `aria-describedby` apontando para um texto de ajuda ("Use as setas para mover, F2 para redimensionar") exibido **na primeira vez** que o item recebe foco na sessão.

Justificativa: F-07 mostra que hoje **não há `aria-live` nenhuma**, e um arraste de calendário é, por definição, um gesto cujo resultado só existe na posição final. Sem anúncio, a informação é puramente visual.

**DF12 — `content-visibility: auto` nas colunas de dia, com `contain-intrinsic-size` por média medida.**

Decisão: `.day-column { content-visibility: auto; contain-intrinsic-size: auto <largura> 1008px; }`, com fallback `@supports not (content-visibility: auto) { contain: layout style paint; }`.

Justificativa: é **Baseline "widely available"** e o WebView2 reporta suporte ([caniwebview.com/features/web-feature-content-visibility](https://caniwebview.com/features/web-feature-content-visibility/), atualizado em 26/09/2026). E o ganho aqui não é só de _offscreen_ (a grade inteira está na tela): o ganho real é que `content-visibility: auto` aplica `contain: layout style paint` **mesmo para o elemento visível**, e essa contenção **isola o reflow**. O guia do Chrome para grades complexas é literal sobre isso — "the browser will only reflow this specific column, not the entire board layout" ([interactions in complex layouts](https://github.com/GoogleChrome/modern-web-guidance-src/blob/main/guides/performance/interactions-in-complex-layouts/guide.md)). Hoje o `pointermove` de um item invalida a grade inteira (H-03) porque `left: calc(…% …)` depende de uma `%` que resolve contra o pai.

Cuidado registrado: `content-visibility` **tem** um custo de avaliação de fronteira de visibilidade, e o guia manda **não** aplicar em conteúdo no primeiro _fold_. Numa grade de dia inteiro de altura fixa o ganho de contenção domina; a Fase F6 mede antes de aceitar.

**DF13 — Feedback de falha: rollback mais uma faixa inline na grade. Nada de toast, nada de modal.**

Decisão: se `applyGesture` (DF9) rejeitar, o `reset(original)` roda, o item volta com uma transição de 150 ms (`DESIGN.md:25` — "transições rápidas de 150–200ms apenas para mudanças de estado") e uma faixa `role="status"` de 4 segundos aparece **no topo da área da grade** com o motivo vindo do `DbError::Domain` do Rust (`:440`, "evento inexistente") ou uma mensagem genérica honesta quando não há motivo. A faixa não bloqueia nada e some sozinha.

Justificativa: D17 pede "se o comando falhar, o item volta e a grade mostra um aviso inline". Um `alert`/`confirm` bloquearia o gesto seguinte e a grade inteira; um toast flutuante sai da grade e o usuário não sabe **qual** item falhou.

Consequência: corrige também F-02 — o mesmo componente de faixa serve para o erro de leitura do efeito de carga (F-01), que hoje é engolido em `CalendarScreen.tsx:183-185`.

**DF14 — O que esta spec explicitamente NÃO usa.**

Recusa, com o motivo:

| Recusado                                 | Motivo                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| ---------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `pointerrawupdate`                       | Disponível no Chromium desde o Chrome 77 e, portanto, no WebView2 — **mas a própria spec adverte**: "Adding listeners for the `pointerrawupdate` event might negatively impact the performance of the web page. A `pointerrawupdate` listener should only be added if JavaScript needs high frequency events and can handle them just as fast" ([Pointer Events 4, §4.2.5](https://www.w3.org/TR/pointerevents4/)). Não Firefox, não Safari. E com encaixe de 15 min (**DF3**) nós **precisamente não** queremos resolução sub-frame — o último evento coalescido é a posição correta. |
| `getCoalescedEvents()`                   | Mesmo motivo. A coalescência do `pointermove` é **o que protege** o rAF loop: o Chrome adia o despacho de `pointermove` até logo antes do próximo quadro ([inside-browser part 4](https://developer.chrome.com/blog/inside-browser-part4)). Para uma grade com snap, perder o ponto intermediário é o comportamento correto.                                                                                                                                                                                                                                                           |
| `will-change` permanente                 | `will-change: transform` num item que fica no DOM o tempo todo cria uma camada de composição por item e o浏览ador mantém memória. O ghost é montado **só** durante o gesto e desmontado no fim — a layer morre com ele.                                                                                                                                                                                                                                                                                                                                                                |
| `animation-timeline: scroll()`           | Tem suporte no WebView2 Evergreen ([caniwebview — scroll-driven animations](https://caniwebview.com/features/web-feature-scroll-driven-animations/)), mas resolve um problema que não temos: animações **ligadas ao scroll**. Nossas animações são ligadas a **estado** (150 ms, `DESIGN.md:25`), e `transition` já faz isso sem custo de timeline. Guardar para quando existir algo rolando.                                                                                                                                                                                          |
| `passive: true` nos listeners do arraste | O DF1 usa `pointerdown`/`pointermove` em fase de captura com `preventDefault` **deliberado** (impedir seleção de texto e arrasto nativo de imagem durante o gesto). `passive` é para listeners que **não** bloqueiam scroll ([Chrome — passive listeners](https://developer.chrome.com/blog/scrolling-intervention-2)). O que precisa ser `passive` é o `wheel` do auto-scroll (Fase F5), se existir.                                                                                                                                                                                  |

---

## 3. Máquina de estados do arraste

### 3.1 Estados

```
        ┌──────────────────────────────────────────────────────────────┐
        │                                                              │
        v                                                              │
    ┌───────┐  pointerdown(item,allowed)   ┌────────┐                 │
    │ idle  │ ───────────────────────────► │ armed  │                 │
    └───────┘                              └────────┘                 │
        ^                                    │   │                    │
        │   reset(), sem commit              │   │ dist > 4px          │
        │                                    │   v                     │
        │      ┌──────────────┐        ┌─────────────┐               │
        └──────┤ cancelamento ├───────►│  dragging   │               │
        │      │ (DF6, 5 vias)│        └─────────────┘               │
        │      └──────────────┘             │      │                  │
        │                                    │      │ pointerup        │
        │                                    │      v                  │
        │                              ┌──────────────┐               │
        │                              │  committing   │──── ok ─────►│
        │                              └──────────────┘               │
        │                                    │      │ erro             │
        └────────────────────────────────────┘      v                  │
                                            ┌──────────┐              │
                                            │ rollback │              │
                                            └──────────┘              │
                                                                       │
    pointerup sem passar de 4px ──► idle (seleciona o item, abre painel)
```

### 3.2 O que acontece em cada estado

**`idle`** — nenhum listener global registrado, nenhum elemento com `pointer-events` alterado, nenhum custom property escrito. O item é o `<button>` de `CalendarScreen.tsx:448`, com `cursor` conforme DF2/M-02 (D17).

**`armed`** — disparado por `pointerdown` **sobre um item arrastável** (`origin !== "google"`, `state ∉ {cancelado, dispensado}` — D17/INV-6), **dentro da área da grade** (`aria-agendario`, dia × hora).

- `setPointerCapture(e.pointerId)` no elemento.
- Grava-se o `DOMRect` do corpo da grade e o `scrollTop`/`scrollLeft` **uma única vez** (DF2).
- Grava-se o **grab offset**: `grabDy = yDoPonteiro − topDoItem`, `grabDx = xDoPonteiro − esquerdaDoItem`. Sem isto, o item **pula** de modo que o canto do ponteiro vire o canto do item no primeiro quadro, e a usuária perde a noção de onde pegou — a falha mais comum de grade temporal.
- `preventDefault()` no `pointerdown` — impede seleção de texto e o arrasto nativo de `<img>`/`<button>`.
- Registram-se, **uma vez**, os listeners de `pointermove`/`pointerup`/`pointercancel`/`lostpointercapture` no próprio elemento capturado (não em `window` — o capture garante que chegam) e os de `keydown`/`blur`/`visibilitychange` em `window`.
- **Nada é desenhado ainda.** O limiar de 4px ainda não foi cruzado; um clique comum não pode ter custo de arraste.

**`dragging`** — disparado quando a distância acumulada passa de 4px (DF7). Ainda dentro dele existem dois sub-estados, porque o encaixe (DF3) não é o mesmo para mover e para redimensionar:

- `dragging.move` — deslocamento livre; a posição é `startY + grabDy`.
- `dragging.resize-start` / `dragging.resize-end` — só a borda arrastada se move; a outra é pino.

O que **renderiza**, por quadro de animação (DF4):

| Elemento                        | Onde                                                                                       | O que é                                                                                                                                                                     |
| ------------------------------- | ------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Ghost**                       | `position: fixed`, `transform: translate3d()` a partir de `--drag-dx/dy`                   | Cópia do item a 70% de opacidade, `pointer-events: none`, `shadow-window`, largura **igual à do slot de destino** (não à do item original — em modo estreito a coluna muda) |
| **Placeholder**                 | dentro do container da grade, `position: absolute`, escrito por `--grid-top/--grid-height` | Retângulo tracejado no **slot final, já encaixado**. É a resposta à pergunta "onde vai cair"                                                                                |
| **Snap line**                   | dentro do container da grade                                                               | Traço de 1px em `bg-primary` na borda do placeholder, espelhando a linha do "agora" (`CalendarScreen.tsx:477-485`)                                                          |
| **Badge de horário**            | `position: fixed`, perto do ponteiro, `pointer-events: none`                               | `"qui, 14:30 – 15:45"` em `tabular-nums`, com o **intervalo**, não só o início — é o número que confirma a duração                                                          |
| **Linha do agora**              | `CalendarScreen.tsx:477-485`                                                               | Ganha `pointer-events: none` **explícito** (M-05) e opacidade reduzida durante o gesto                                                                                      |
| **Colunas de origem e destino** | —                                                                                          | Realce de 1px na coluna de destino em `border-primary`                                                                                                                      |

O que **não** renderiza, deliberadamente: nenhum modal, nenhum tooltip do SO, nenhum som, nenhuma vibração.

Sobre feedback sonoro e háptico: **nenhum**. É desktop, em WebView2, sem API de háptico no Windows; um `aria-live` (DF11) é o canal honesto. Um "ding" de sistema seria mais barulho que sinal.

**`committing`** — disparado por `pointerup`. Sequência, sem `await` antes da escrita otimista:

1. `reset` visual local: o ghost e o badge saem, o item real assume a posição final do placeholder **já escrita no DOM** (a mutação já aconteceu; o React é quem está atrasado).
2. Escreve-se a posição final no item real e marca-se `data-committing`, que dá uma opacidade levemente reduzida — o feedback de "está gravando" durante os ~2–10 ms do IPC.
3. `releasePointerCapture` (guardado por `hasPointerCapture`, DF6).
4. `applyGesture(item, range)` (DF9) — **uma** chamada de IPC.
5. `onResolve`: o item real sai de `data-committing`. **Nenhuma recarga da janela** — o estado local já é o correto.
6. `onReject`: `reset(original)` + faixa inline (DF13).

**`rollback`** — sub-estado transitório de `committing`. O item recebe a transform do placeholder de volta até a posição original, com transição de 150 ms, e o `ghost` reaparece por **um** quadro antes de sumir, para que o olho acompanhe o caminho de volta.

### 3.3 Onde o "otimismo local" da D17 mora

D17 diz: _"Arrastar aplica otimismo local (o item segue o ponteiro) e persiste ao soltar; se o comando falhar, o item volta e a grade mostra um aviso inline."_

O otimismo tem **duas metades** e elas vivem em lugares diferentes:

- **A metade visual** (o item segue o ponteiro) é o ghost + o placeholder do `dragging`, movidos por CSS custom properties, fora do React (DF4). Ela é **totalmente reversível**: basta apagar o `transform`.
- **A metade de dados** (o item está no lugar certo) só existe **depois** do `pointerup`, e só é gravada se o comando passar (DF9). Antes disso, `items` em `CalendarScreen` **não mudou**.

Essa separação é o que torna o rollback barato: nenhuma referência a guardar, nenhuma lista a restaurar, nenhum estado a invalidar. O "estado real" só é tocado uma vez e só no fim.

### 3.4 Regras de invariante durante o gesto

- `findOverlaps` (`overlap.ts:21`) é rodado **contra o placeholder**, não só no commit, e produz uma marca visual não-bloqueante (D7): uma borda tracejada `border-destructive/40` no placeholder quando ele colide. **Nunca** impede o `pointerup`. Esta é a superfície que falta hoje (F-09).
- O item arrastado **não** re-participa de `columnize` durante o gesto — o layout de colunas é recalculado **no commit**, uma vez. Durante o gesto o ghost flutua por cima e o placeholder ocupa a coluna de destino.
- O `grab offset` é **sempre** subtraído, em `move` e em `resize`. Em `resize`, o offset é em relação à **borda** arrastada, não ao item.

### 3.5 Diagrama de responsabilidades

```
ui/TimedItem.tsx            ui/hooks/use-grid-drag.ts        data/apply-gesture.ts
──────────────────           ─────────────────────────         ────────────────────
pointerdown ──────────────►  armed: capture, DOMRect, grab     (não entra)
onPointerDown(handle)        offset, listeners
                             │
                             ▼
  (nada renderiza)          dragging: rAF → CSS custom props   (não entra)
                             │   + placeholder + snap line
                             │   + badge + findOverlaps (só sinal)
                             ▼
onPointerUp  ─────────────►  committing: 1 invoke ────────────►  deriva de `kind`:
                             │                                  move_occurrence |
                             │ ok                                reschedule_block |
                             ▼                                                      update_event
  item real na posição final   error → rollback + faixa inline
```

---

## 4. Tela de criação e edição por gesto

### 4.1 Criar arrastando numa célula vazia

`pointerdown` no **fundo** de uma coluna de dia (`aria-agendario`, `layer: dia`), **não** sobre um item:

- `pointerdown` → `armed` com `mode: "create"`, grab offset zero.
- Arrastar para baixo/acima cria um **placeholder** com a mesma linguagem visual do arraste: tracejado, com snap line e badge mostrando `"qui, 14:00 – 15:00"`. Arrastar só 4px e soltar cria o **slot padrão de 60 min** encaixado (é o que Google Calendar e o `forceCalendar` fazem, e é o que evita "criou um item de 1 minuto" por clique acidental).
- No `pointerup`, o item é criado **otimista** (aparece na grade imediatamente) e o `ItemEditor` (spec 02 Fase 2.5) abre **com o intervalo já preenchido**, focando o título. O título é o único campo obrigatório.
- `Escape` no `armed`/`dragging` de criação: o placeholder some, **nada é criado** (DF6) — criação por arrasto acidental é a caminho mais fácil de poluir o banco, e `dev-seed.ts:53-62` já trata banco não-vazio como sacred).
- Criação por arrasto **não** cria responsabilidade, não promove `commitment` e não marca nada como `obrigatorio` (INV-7). O `commitment` do item novo é escolhido no editor, com o padrão visível.

### 4.2 Editar ao soltar em outro slot

Arrastar um item existente e soltar em outro slot **move** e **não** abre o editor. Abrir o editor é `click` (DF7) ou `Enter` (DF5). Essa separação é deliberada: abrir um modal a cada arraste transforma o gesto mais frequente do produto no gesto mais interrompido.

Quando o item **é** recorrente (`isRecurring`), o `pointerup` **não** move sozinho: ele abre o `RecurrenceScopeDialog` da spec 02 Fase 2.6 com as três opções (`bridge.ts:218`, `"serie" | "esta_ocorrencia" | "esta_e_seguintes"`). "Esta ocorrência" chama `move_occurrence` e **não** toca `event_recurrence` — INV-5. O ghost fica no lugar e a caixa de escolha aparece abaixo dele, sem bloquear o resto da grade.

### 4.3 Redimensionar pelas duas bordas

Duas alças de 6px de alvo, **centradas 2px para dentro** da borda, não sobre ela. Isso não é preciosismo: um relato de QA real documenta exatamente essa falha — "handles centered exactly ON the span's boundary read as the NEIGHBORING row — a 1px drag jumped a whole day. They now sit a step inside the fragment" ([snomiao/rgui commit `3358a41`](https://github.com/snomiao/rgui/commit/3358a41ff8873c81def220cb90dc4bfabfb50ffb)). Com `hourHeight: 56` e coluna de 96px, uma alça de 6px centrada na borda seria ambígua entre dois itens vizinhos.

- Piso de **15 min** (uma grade de 56px/hora dá 14px de passo; menos que isso não é redimensionamento, é ruído).
- Ao chegar no piso, a alça **trava visualmente** (muda para `opacity-60`) em vez de empurrar a outra borda.
- Ao esticar para fora da janela visível (abaixo de `24:00` ou acima de `06:00`), o snap é **relaxado**: o item pode ocupar `05:30 → 07:00`, e a faixa invisível é indicada por uma **serrilha** no topo ou na base da coluna, nunca por um item fantasma no lugar errado. Isso é o que `grid-scale.ts:139-141` já protege no desenho (`placeInDay` devolve `[]`).
- `state = "concluido"` **não** redimensiona: a duração realizada é registro de realidade (INV-3, D12), não campo editável. O cursor vira `not-allowed` e a alça some.

### 4.4 Resize-to-absorb

Quando o arraste de uma borda **encosta** na borda de outro item, **não** há repulsão nem encaixe automático (D7 rejeita explicitamente: "encaixe automático (mexe no planejamento dela)", `00-fundacao.md:116`). O que existe:

- O placeholder do item arrastado ganha uma borda tracejada `border-destructive/40` e a faixa inline mostra "Sobrepõe com 2 itens".
- O `pointerup` **persiste normalmente**.
- O painel do item (D18/D20) ganha a seção de sobreposição com as duas leitunas lado a lado e três ações explícitas, herdadas do diálogo de conflito da spec 05 quando ela existir.

Absorção real (empurrar o outro item) fica **fora de escopo** e registrada como backlog: exigiria uma transação de duas escritas no Rust e é uma decisão de produto, não de interação.

### 4.5 Mover entre granularidades

| De → Para                 | O que acontece                                                                                                                                                                                                                                                                                                                                                                               |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| dia inteiro → dia inteiro | move de dia em dia, snap de 1 dia. Redimensionar muda a **contagem** de dias, com piso de 1 dia                                                                                                                                                                                                                                                                                              |
| dia inteiro → com hora    | arrastar para dentro do corpo da grade **converte**. Regra do Notion, que é a referência de mercado aqui: "A three-day all-day event dropped at 10:00 ends at 11:00 on its third day" ([notion-ui — Calendar](https://notion-ui.vercel.app/docs/blocks/calendar)). A conversão abre `ItemEditor` com o novo formato, porque `allDay` e `startsAt` são **colunas diferentes** (`commands.rs`) |
| com hora → dia inteiro    | arrastar para a faixa superior converte para dia inteiro, preservando as datas e **descartando a hora** (mesma regra do Notion). Também abre o editor                                                                                                                                                                                                                                        |
| `mes` → `mes`             | move de dia em dia; a célula alvo ganha realce; `+N` some para o dia de origem                                                                                                                                                                                                                                                                                                               |
| `agenda` → `agenda`       | **sem arrasto.** A lista ordenada (`:535-561`) é reordenação, não temporal — arrastar aí seria reordenação de lista, que é o caso que a spec 00 A4 diz explicitamente que **não** estamos resolvendo. A spec 03 (D23) cobre a edição por ações aqui                                                                                                                                          |

Toda conversão **passa por `ItemEditor`**, porque muda o schema e não pode ser um COMMIT cego. Isso é coerente com INV-6: um item `google` nunca chega aqui, porque não é arrastável (D17/DF8).

---

## 5. Camada de eventos

### 5.1 Os três hooks

**`usePointerIntent`** (`ui/hooks/use-pointer-intent.ts`) — a **intenção**. Um único `onPointerDown` no corpo da grade decide o que vai acontecer, olhando para o alvo:

```
pointerdown no <body da grade>
  ├─ alvo tem [data-drag-role="handle-start"]  → resize-start
  ├─ alvo tem [data-drag-role="handle-end"]    → resize-end
  ├─ alvo tem [data-drag-role="item"]          → move (ou click, DF7)
  ├─ alvo é [data-drag-role="day-surface"]     → create
  └─ nada                                     → ignora
```

Um listener só, não um por item. Com 100 itens isso é a diferença entre 100 closures e 1. É também o que mantém a camada de reaproveitamento do compositor pequena: `pointerdown` num container não marca a **página inteira** como região não-fast-scrollable, ao contrário do que o Chrome descreve para delegação em `document.body` ([inside-browser part 4](https://developer.chrome.com/blog/inside-browser-part4)).

**`useGridDrag`** (`ui/hooks/use-grid-drag.ts`) — a máquina de estados. Recebe a intenção, o `DOMRect` (DF2) e a escala; devolve `{ phase, ghostStyle, placeholderStyle, snapLineStyle, badgeStyle, announce }`. **Nunca** devolve posição para o `CalendarScreen`.

**`useRafThrottle`** (`ui/hooks/use-raf-throttle.ts`) — o laço. Trinta linhas:

- `pointermove` grava a última coordenada e **agenda** um `requestAnimationFrame` se não houver um pendente.
- O `rAF` escreve as custom properties e **chama o callback `onFrame`**.
- `cancelAnimationFrame` no `reset`.

Sem throttle temporal. Um `setTimeout(…, 16)` introduz **lag de um quadro** em cima do `rAF` e acumula com a taxa de atualização do monitor (144 Hz) — o sintoma clássico de "arraste que gruda". O `rAF` **é** o agendador certo: o Chrome já adia o despacho de `pointermove` para logo antes do próximo quadro, então o `rAF` fica com no máximo um evento por quadro, sem atraso artificial.

### 5.2 Por que isso é melhor que uma lib — e onde é pior

**Melhor**, neste caso específico:

1. **A grade não é um drop target.** Nenhuma lib de DnD sabe o que é `timeToY`. Toda a matemática — grab offset, snap de 14px, clamp ao dia, duração mínima, relaxamento na borda visível — seria nossa de qualquer jeito, escrita por cima do modelo da lib.
2. **Controle por quadro.** `pointerrawupdate` e `getCoalescedEvents` estão disponíveis (WebView2 é Chromium), mas a lib captura a posição **em `pointermove`** e a processa no seu próprio ciclo. Com o encaixe de 15 min nós queremos deliberadamente a **última** amostra, não todas.
3. **Zero peso, zero risco de colisão de tipos.** A4 (`00-fundacao.md:147`) jáINVocou `noUncheckedIndexedAccess` e `exactOptionalPropertyTypes` (`tsconfig.json:22-23`) como motivos; hoje `npm run build` produz **358,90 kB / 112,43 kB gzip** (spec 00 §Verificação). Uma lib de 18,9 kB gzip seria **+17%** do bundle gzip por um modelo que não usamos.
4. **Testabilidade.** A máquina de estados é uma função pura de `(estado, evento) → estado`. Testa-se com `node --test`, sem browser — o mesmo motivo de `domain/` ser puro (`AGENTS.md:17`).

**Pior, e é honesto dizer:**

1. **Não há acessibilidade de graça.** A React Aria resolve teclado e leitor de tela num modelo testado com NVDA, JAWS e VoiceOver ([react-aria.adobe.com/blog/drag-and-drop](https://react-aria.adobe.com/blog/drag-and-drop)). Escrevemos isso do zero no DF11 e no DF5, e **não** vamos ter a mesma confiança de campo. Mitigação: o padrão é o mesmo que o Primer e o Darin Senneff documentam como o que funciona — texto estático + live region + `role="application"` no modo.
2. **Sem sensores prontos.** Lib de DnD tem sensor de distância, de atraso, de toque longo, de teclado. O nosso tem o limiar de 4px (DF7) e nada mais — e é o suficiente, porque a spec 03 cobre toque por ações, não por arrasto.
3. **Um滚动 aninhado para acertar.** O DF10 precisa mexer em `overscroll-behavior` e na largura mínima das colunas. Uma lib não teria esse problema, porque não sabe que existe um container externo.

### 5.3 Ciclo de vida e cleanup

Uma única regra, e ela é o critério de aceite mais importante da Fase F2: **toda saída de `armed`/`dragging` passa por `reset()`**, e `reset()` é idempotente e faz, nesta ordem:

1. `cancelAnimationFrame` se pendente;
2. remover os 5 listeners registrados no `armed` (`pointermove`, `pointerup`, `pointercancel`, `lostpointercapture`, `keydown`, `blur`, `visibilitychange`);
3. `releasePointerCapture` se `hasPointerCapture`;
4. limpar as custom properties dos elementos ghost/placeholder/badge;
5. desmontar ghost, placeholder e badge;
6. limpar o token de supressão de clique (DF7) **depois** de um microtask.

`StrictMode` está ligado (`main.tsx:12`), o que significa que todo `useEffect` monta, desmonta e remonta em desenvolvimento. `reset()` idempotente é o que faz isso não custar dois listeners por gesto em dev e um em produção.

---

## 6. Fases de implementação

### Fase F0 — Correções de base, sem gesto (TypeScript puro)

Sem uma linha de arraste. Tudo verificável com `npm run typecheck` e `node --test`.

| Ação                                                                                                                                                     | Arquivo                                              |
| -------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------- |
| Corrigir `snapY` para `snapMinutes / 60 × hourHeight` e torná-lo o **único** caminho de encaixe em pixel                                                 | `domain/grid-scale.ts:95-97`                         |
| Adicionar teto em `yToTime` (`Math.min(y, visibleHeight)`) ou exportar um `clampY`                                                                       | `domain/grid-scale.ts:76-78`                         |
| Usar o `formatterCache` de `time.ts:97-114` em `formatClock`                                                                                             | `ui/CalendarScreen.tsx:89-96`                        |
| `useMemo` em `ordered`/`lanes` por dia                                                                                                                   | `ui/CalendarScreen.tsx:426-427`                      |
| Extrair `TimedItem` como componente memoizado                                                                                                            | `ui/CalendarScreen.tsx:448-469` → `ui/TimedItem.tsx` |
| `min-w-[96px]`, eixo de horas 48px, `sticky left-0` na 1ª célula do cabeçalho                                                                            | `ui/CalendarScreen.tsx:372-373,402,406`              |
| `overscroll-behavior: contain` + `scroll-margin-top` no utility `.grid-scroll`                                                                           | `styles.css`                                         |
| `content-visibility: auto` + `contain-intrinsic-size` em `.day-column`, com fallback `contain`                                                           | `styles.css` + `CalendarScreen.tsx:430-434`          |
| `disabled` em "Concluir"/"Reagendar"                                                                                                                     | `ui/CalendarScreen.tsx:813-818`                      |
| Trocar o `role="tablist"` de modos por um `tablist` de verdade (roving `tabIndex`, `aria-controls`, `tabpanel`), copiando o padrão de `App.tsx:985-1019` | `ui/CalendarScreen.tsx:287-300`                      |
| Remover o `ViewSwitch` morto de `App.tsx`                                                                                                                | `App.tsx:733-762` + `801,871`                        |
| Corrigir o "Criar evento" do estado vazio (ou rotular para o que ele faz)                                                                                | `ui/CalendarScreen.tsx:836-839`                      |
| Corrigir o `anchor` inicial, que usa a data **UTC** (`toISOString`) em vez da data no fuso da grade                                                      | `ui/CalendarScreen.tsx:107-109`                      |

**Aceite:** `node --test src/features/calendar/domain/grid-scale.test.ts` verde **e** gains: `snapY(0) === 0`, `snapY(7) === 0`, `snapY(8) === 14` com `hourHeight: 56`; com `hourHeight: 84`, passo de 21; `snapY` é monotônico; `yToTime(yToTime(t))`-style round-trip. `rg "snapMinutes \* 4" src` → vazio. `rg "new Intl.DateTimeFormat" ui/CalendarScreen.tsx` → **no máximo 4** (um por modo, não dentro de `map`). `rg "cursor-grab" docs/calendario/02-grade-desktop.md:27,147` → corrigido para "removido". `npm run build` sem crescimento de bundle (o `Intl` cache e o memo **devem** reduzir).

### Fase F1 — O contrato do gesto (TypeScript puro, sem DOM)

| Ação                                                                                                                                                   | Arquivo |
| ------------------------------------------------------------------------------------------------------------------------------------------------------ | ------- |
| `domain/gesture.ts` — tipos puros: `GesturePhase`, `GestureIntent`, `GestureRange`, `snapRangeToScale(range, scale)`, `clampRangeToDay`, `minDuration` | novo    |
| `domain/gesture.test.ts`                                                                                                                               | novo    |
| `domain/item-menu.ts` + `.test.ts` — `origin × state → ações` (prometido em `02:200`): decide `draggable`, `resizable`, `editable`                     | novo    |

`domain/gesture.ts` **não importa React, não importa DOM, não importa `Date`** — as mesmas regras de `AGENTS.md:17`. Ele pega um `GestureRange` cru e devolve o `GestureRange` final. Isso é o que torna a Fase F2 verificável sem browser.

**Aceite:** `node --test src/features/calendar/domain/` verde. Casos: mover 40px para baixo dá **exatamente** 15 min (critério de `02:194`); arrastar para cima até o topo não deixa `startsAt` antes do início do dia; `resize` nunca produz duração < 15 min; `clampRangeToDay` de `23:00 + 3h` dá `[23:00, 24:00]` **no mesmo dia**, e não `01:00` do dia seguinte (P-03); `item-menu` diz: `origin: "google"` → `draggable: false`; `state: "cancelado"` → `draggable: false`; `state: "concluido"` → `resizable: false`.

### Fase F2 — `useGridDrag` e a máquina de estados (o caminho crítico de latência)

| Ação                                                                                       | Arquivo                                       |
| ------------------------------------------------------------------------------------------ | --------------------------------------------- |
| `ui/hooks/use-raf-throttle.ts`                                                             | novo                                          |
| `ui/hooks/use-grid-drag.ts` — os 7 estados de §3, DF1–DF7                                  | novo                                          |
| `ui/hooks/use-pointer-intent.ts`                                                           | novo                                          |
| `ui/DragLayer.tsx` — ghost, placeholder, snap line, badge; **fora** do fluxo do grid       | novo                                          |
| `ui/WeekGrid.tsx` — a grade como **um** scroller (DF10), cabeçalho e corpo no mesmo `grid` | novo (extrai de `CalendarScreen.tsx:352-489`) |
| `ui/CalendarScreen.tsx` — monta `WeekGrid` + `DragLayer` + a live region                   | editar                                        |

**Aceite — e este é o critério que decide se a Fase passou:**

1. Arrastar **100 ocorrências** numa semana mantém **60 fps** no painel Performance do DevTools, com **zero** `setState` React por quadro (verificável: o contador "renders" do React Profiler **não se move** entre dois quadros de arraste).
2. O `pointermove` produz **no máximo uma** escrita por quadro de animação (o `useRafThrottle` descarta; confirmar em `PerformanceEventTiming`/`performance.measure`).
3. O total de chamadas a `formatToParts` durante um arraste de 2 s é **0** (o caminho do arraste não chama `timeToY`/`placeInDay`/`toPlainDate`; ele só faz aritmética de pixel a partir do `DOMRect` capturado — DF2).
4. `Escape` no meio do arraste devolve o item ao lugar, remove o ghost e **chama zero** comandos.
5. Arrastar para fora da janela e soltar **deixa o item no lugar** (rota 4 do DF6) — testar alt-tabando com o botão pressionado.
6. `touch-action` no item é `none` **durante** o gesto e `manipulation` fora dele; o `styles.css:156-161` continua global como está.
7. `rg "devicePixelRatio" src/features/calendar/ui` → **vazio**.

### Fase F3 — Criar, redimensionar e teclado

| Ação                                                                                                 | Arquivo |
| ---------------------------------------------------------------------------------------------------- | ------- |
| `usePointerIntent` ganha a intenção `create` no fundo da coluna                                      | editar  |
| Alças superior e inferior em `TimedItem.tsx`, com `data-drag-role`                                   | editar  |
| `data-committing` e o estado `rollback` de §3.2                                                      | editar  |
| Política de teclado (DF5) + `role="application"` no modo `F2`                                        | editar  |
| `aria-live` com debounce de 120 ms (DF11) + `aria-keyshortcuts` + `aria-describedby` de primeira vez | editar  |

**Aceite:**

1. Arrastar numa célula vazia por 5 min e soltar cria um item de **60 min**, com o editor aberto e o título focado.
2. `Escape` no arasto de criação não cria nada — `rg` no log do repositório mostra zero `create_event`.
3. Redimensionar pela borda de cima muda `startsAt` e mantém `endsAt`; pela de baixo, o inverso. Com 6 itens empilhados, a alça da borda superior **não** redimensiona o vizinho (a lição de `rgui 3358a41`).
4. Com o item focado, `Tab` alcança o item, `←` move 15 min, `Shift`+`←` encurta 15 min, `F2` entra no modo resize, `Escape` sai, `Escape` de novo desfaz o último ajuste.
5. Com NVDA: "Cálculo II movido, quinta-feira, 14:30" é anunciado em modo polito, **não** uma vez por quadro.

### Fase F4 — Persistência, escopo de recorrência e falha

| Ação                                                                           | Arquivo            |
| ------------------------------------------------------------------------------ | ------------------ |
| `data/apply-gesture.ts` — deriva de `kind` (DF9), um `invoke` por gesto        | novo               |
| `RecurrenceScopeDialog.tsx` no `pointerup` de item recorrente                  | novo (de `02:208`) |
| Faixa inline de erro e a varredura de `catch {}` (F-02, DF13)                  | editar             |
| `window resize` e toggle de densidade/sidebar bloqueados durante o gesto (DF6) | editar             |

**Aceite:**

1. Um arrasto = **uma** chamada de IPC (contar na aba Network do WebView2).
2. Item recorrente arrastado abre o diálogo de escopo; "Esta ocorrência" chama `move_occurrence` e **não** `update_recurrence` (INV-5, critério de `02:252`).
3. Item `origin: "google"` não tem `cursor-grab`, não tem alça, e `⌘`-clicar abre o item **sem** `Concluir`/`Reagendar`/`Reagendaroccurrence` (D17/INV-6, critério de `02:251`).
4. Um comando rejeitado devolve o item ao lugar **com transição de 150 ms** e mostra a faixa com o motivo do Rust.
5. `rg "catch\s*\{\s*\}" src/features/calendar` → **vazio**.

### Fase F5 — Alinhamento do painel e a grade

| Ação                                                                             | Arquivo                                     |
| -------------------------------------------------------------------------------- | ------------------------------------------- |
| Painel sempre montado, `grid-cols-[minmax(0,1fr)_360px]` (D18)                   | editar `CalendarScreen.tsx:303-336,784-825` |
| `content-visibility` no painel vira `hidden` em `max-lg` em vez de desmontar     | editar                                      |
| Rolagem automática até "agora" ao abrir a semana (S-05), com `scroll-margin-top` | editar                                      |
| `scrollIntoView` no item recém-criado                                            | editar                                      |

**Aceite:** selecione um item, navegue `←`/`→` na semana: o painel **permanece montado** e só o cabeçalho de data muda (critério de `02:250`). Ao abrir a semana, a grade rola até a linha do "agora" se ela estiver fora da vista, e **não** se já estiver visível. O painel tem 360px no desktop, não 280px.

### Fase F6 — Meses, dia inteiro e polimento

| Ação                                                                                                                      | Arquivo                        |
| ------------------------------------------------------------------------------------------------------------------------- | ------------------------------ |
| `GridItem` ganha `origin`, `allDay`, `commitment`, `timeZone` (**DF8** — exige `commands.rs:487-492` e `bridge.ts:49-60`) | `src-tauri` + `data/bridge.ts` |
| Faixa `AllDayRow` acima do cabeçalho de dias                                                                              | novo                           |
| Arraste na faixa de dia inteiro (snap de 1 dia)                                                                           | editar                         |
| Conversão dia inteiro ↔ com hora (§4.5)                                                                                   | editar                         |
| Sinalização de sobreposição via `findOverlaps` (F-09, D7) — **derivada, nunca bloqueante**                                | editar                         |
| `+N` e piso de largura por faixa em `columnize` (C-05)                                                                    | editar                         |

**Aceite:** `rg "e.origin, e.all_day" src-tauri/src/commands.rs` → 1. `rg "origin" src/features/calendar/data/bridge.ts` → presente em `GridItem`. Um item que começa 3 dias antes da janela **aparece** na coluna correta (C-01). Arrastar 8 itens simultâneos na mesma faixa não produz colunas com menos de 24px. Um placeholder sobreposto mostra a borda tracejada e **o `pointerup` persiste normalmente** (INV-7/D7: conflito nunca bloqueia).

### Fase F7 — Estados de carga e erro (o que hoje é silêncio)

| Ação                                                                                      | Arquivo           |
| ----------------------------------------------------------------------------------------- | ----------------- |
| `Skeleton` de grade para `loaded === false` (F-01), **sem** piscar a cada troca de semana | `ui/WeekGrid.tsx` |
| Faixa de erro de leitura no lugar do `catch {}` (F-02)                                    | editar            |
| Recarga diferenciada: manter a grade anterior visível com uma opacidade, em vez de limpar | editar            |

**Aceite:** trocar de semana com 3 items na janela **não** esvazia a grade por um quadro. O `skeleton.tsx` que existe e não é usado passa a ser usado. `IpcUnavailableError` (`ipc.ts:13-18`) aparece na tela quando se roda `npm run dev` sem o shell Tauri, com a mensagem que já está escrita.

---

## 7. Riscos e anti-padrões

Cada linha é uma armadilha específica desta implementação, com o sintoma observável que a denuncia.

| #    | Anti-padrão                                                                     | Sintoma                                                                                                                     | Regra                                                                                                                                                                           |
| ---- | ------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R-01 | **Throttle temporal** (`setTimeout 16ms`, `debounce 50ms`) no `pointermove`     | O ghost "gruda" ~1 quadro atrás do ponteiro, e piora em monitor de 144 Hz                                                   | Só `requestAnimationFrame`. O Chrome já adia o despacho de `pointermove` para antes do quadro ([inside-browser part 4](https://developer.chrome.com/blog/inside-browser-part4)) |
| R-02 | **`getBoundingClientRect()` por quadro**                                        | Layout thrash: escrita → leitura → escrita a 60 Hz; o Chrome documenta a queda de quadro em grades complexas                | Ler **uma** vez no `armed` (DF2)                                                                                                                                                |
| R-03 | **`devicePixelRatio` na conversão de coordenada**                               | Em tela 150%/175% o item cai ~1,5× longe do ponteiro, e o sintoma piora ao redimensionar a janela do Tauri                  | CSS px do `getBoundingClientRect`, sempre                                                                                                                                       |
| R-04 | **Recalcular `DOMRect` quando a sidebar anima** (`App.tsx:192`, 200 ms)         | O item dá um salto no meio do gesto quando o `PanelLeft` é clicado                                                          | Bloquear o toggle durante o gesto (DF6)                                                                                                                                         |
| R-05 | **Arraste que quebra com o zoom do SO**                                         | A 150%, o item vai para o lugar errado só se o código misturar coordenadas de dispositivo                                   | Ver R-03. Zoom do SO é invisível para `clientX/Y`                                                                                                                               |
| R-06 | **Snap que impede alinhar em meia hora**                                        | Não existe bloco de 09:30–10:00 sem digitar 09:30 à mão                                                                     | `Alt` = passo de 1 min (DF3)                                                                                                                                                    |
| R-07 | **`preventDefault()` no `pointerdown` do fundo da grade**                       | A grade deixa de rolar com a roda; em janela estreita o scroll externo some também                                          | `preventDefault()` **só** quando o alvo é um item, uma alça ou quando o gesto de criação já armou                                                                               |
| R-08 | **`touch-action: none` global**                                                 | O `styles.css:156-161` põe `touch-action: manipulation` em tudo; trocar por `none` globalmente mata o scroll do app inteiro | `touch-action` no item só **durante** o arraste; `manipulation` fora                                                                                                            |
| R-09 | **Conflito como bloqueio**                                                      | A usuária não consegue mover um item para cima de outro — viola D7 e `00-fundacao.md:116`                                   | Sobreposição é **sinal**, nunca bloqueio. `pointerup` persiste sempre (Fase F6)                                                                                                 |
| R-10 | **Recriação do item no gesto, o que quebra `UNIQUE(event_id, original_start)`** | "Esta ocorrência" some da série                                                                                             | Nunca recriar. Só `move_occurrence` (`commands.rs:907`), que preserva `originalStart` (`types.ts:88`)                                                                           |
| R-11 | **`Escape` cancela também o diálogo de escopo**                                 | A usuária cancela o `RecurrenceScopeDialog` e perde o arraste sem querer                                                    | `Escape` é do gesto **ou** do diálogo, nunca dos dois. Checar `event.defaultPrevented`                                                                                          |
| R-12 | **Duplo clique = dois gestos**                                                  | O `pointerup` de um arraste gera um `click` que abre o editor                                                               | Token de supressão (DF7)                                                                                                                                                        |
| R-13 | **`pointercancel` não escutado**                                                | Arraste de toque interrompido deixa o item colado                                                                           | Uma das 5 rotas do DF6                                                                                                                                                          |
| R-14 | **`reset()` não idempotente**                                                   | Em `StrictMode` (`main.tsx:12`) dois listeners por gesto em dev, e uma posição errada                                       | Regra única de cleanup em §5.3                                                                                                                                                  |
| R-15 | **`will-change: transform` permanente em `TimedItem`**                          | Uma camada de composição por item, memória sobe sem necessidade                                                             | Só no ghost, que é desmontado no fim                                                                                                                                            |
| R-16 | **`columnize` recalculado durante o gesto**                                     | 100 itens × re-sort por quadro                                                                                              | Calcular **uma vez** no commit (DF4)                                                                                                                                            |
| R-17 | **`formatClock`/`Intl` por item por quadro**                                    | Congelamento de 100+ ms na primeira passagem                                                                                | `formatterCache` (`time.ts:97`) + componente memoizado (Fase F0)                                                                                                                |
| R-18 | **Placeholder que "empurra" os vizinhos (reflow)**                              | A coluna inteira se reorganiza a cada quadro, e o item sob o ponteiro anda                                                  | O placeholder é `absolute` e **não** entra em `columnize` até o commit (§3.4)                                                                                                   |
| R-19 | **`overscroll-behavior` faltando**                                              | Rolagem encadeada: a página pula atrás da grade no fim do scroll                                                            | DF10                                                                                                                                                                            |
| R-20 | **Header de dias sem `sticky left`** na 1ª célula                               | O eixo de horas fixo fica embaixo de um dia, e as colunas mentem (S-03)                                                     | Fase F0                                                                                                                                                                         |
| R-21 | **`aria-live="assertive"` no arraste**                                          | O leitor de tela interrompe tudo a cada 16 ms                                                                               | `polite` + debounce de 120 ms (DF11)                                                                                                                                            |
| R-22 | **Rollback sem feedback**                                                       | O item volta e a usuária não sabe se salvou                                                                                 | Faixa inline (DF13)                                                                                                                                                             |
| R-23 | **Persistir no `pointermove`**                                                  | 60 `invoke` por segundo; JSON-RPC serializa e desserializa na main thread                                                   | Um `invoke`, no `pointerup` (DF9)                                                                                                                                               |
| R-24 | **`ScrollArea` do Radix na grade**                                              | O `Viewport` intermediário quebra o `sticky` cruzado (S-08)                                                                 | Scroll nativo (DF10)                                                                                                                                                            |

---

## 8. Rastreabilidade

### 8.1 O que esta spec executa da spec 02

| Decisão da 02                                           | Texto        | Onde é executada                                                                                                                           |
| ------------------------------------------------------- | ------------ | ------------------------------------------------------------------------------------------------------------------------------------------ |
| **D15** — escala única, snap de 15 min                  | `02:81-85`   | Fase F0 corrige `snapY` (`grid-scale.ts:96`) e o torna o caminho único; Fase F1 põe o snap em `domain/gesture.ts`, puro e testado          |
| **D16** — 7 dias, `hourHeight` 56, rolagem, dia inteiro | `02:87-91`   | Fase F0 ajusta a largura mínima para 96px; Fase F6 entrega a `AllDayRow` que a D16 promete e que hoje não existe                           |
| **D17** — arrasto só para `origin !== "google"`         | `02:93-97`   | **DF8** traz `origin` pelo wire; Fase F1 põe a decisão em `domain/item-menu.ts`; Fase F4 aplica `cursor: not-allowed` e a ausência de alça |
| **D18** — painel é coluna e sobrevive à navegação       | `02:99-103`  | Fase F5: painel sempre montado, 360px, `content-visibility: hidden` em vez de desmontar                                                    |
| **D19** — `ViewSwitch` interno com quatro modos         | `02:105-109` | Fase F0 conserta o `tablist` de `CalendarScreen.tsx:287-300` e remove o `ViewSwitch` morto de `App.tsx:733-762`                            |
| **D20** — painel em três colunas                        | `02:111-115` | Fora do escopo de **fliuidez**; esta spec só garante que o painel **sobrevive** ao gesto e à navegação (Fase F5)                           |
| **Fase 2.4** — `use-grid-drag.ts`, alças, `Escape`      | `02:185-194` | Fase F2 e F3, com os critérios de aceite da 02 transcritos literalmente                                                                    |
| **Fase 2.6** — `RecurrenceScopeDialog`                  | `02:207-217` | Fase F4                                                                                                                                    |
| **Fase 2.3** — `TimedItem`                              | `02:173-183` | Fase F0 (extrair e memoizar)                                                                                                               |
| **`cursor-grab` em `App.tsx:918`**                      | `02:27,147`  | **Fato obsolete** (G-04): a linha citada não tem mais `cursor-grab`. Correção documental na Fase F0                                        |

### 8.2 O que esta spec executa da spec 00

| Decisão                                                                                  | Onde                                                                                                                                                                                                                                 |
| ---------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **A4** — `useGridDrag` próprio, sem lib, `setPointerCapture`, snap de 15 min, duas alças | **DF1** (justificativa com dados de 2026), Fase F2                                                                                                                                                                                   |
| **A4/INV** — equivalentemente por teclado                                                | **DF5**, Fase F3                                                                                                                                                                                                                     |
| **D4** — cor fixa por camada                                                             | `layerClass` (`layers.ts:61-63`) já existe; a Fase F2 usa a mesma classe no ghost e no placeholder para que o tom não mude durante o gesto                                                                                           |
| **D6** — sem ajuste automático ao viajar                                                 | O `grab offset` e o `DOMRect` são de **tela**; a conversão para instante passa por `yToTime`/`snapTime` (`grid-scale.ts:76-92`), que são **relógio de parede** (`time.ts:225-233`). Uma viagem durante o arraste **não** move o item |
| **D7** — conflito derivado, nunca bloqueante                                             | §3.4 e R-09; Fase F6 põe `findOverlaps` na tela                                                                                                                                                                                      |
| **INV-4** — atraso só para `obrigatorio`/`importante`                                    | `isOverdue` (`overdue.ts`) já é usado (`CalendarScreen.tsx:693-695`); o gesto não altera `commitment`                                                                                                                                |
| **INV-6** — nada escreve campo de `google`                                               | D17 + DF8; `item-menu` decide antes do gesto                                                                                                                                                                                         |
| **INV-7** — nada vira `obrigatorio` sem ação                                             | §4.1: criação por arrasto abre o editor e não promove nada                                                                                                                                                                           |

### 8.3 O que esta spec executa da spec 01

- `data/bridge.ts` é o **único** caminho de IPC (`AGENTS.md:19`). A Fase F4 acrescenta `apply-gesture.ts` em `data/`, que usa `moveOccurrence` (`:210`), `rescheduleBlock` (`:323`) e `updateEvent` (`:192`) e **nenhum outro**.
- `domain/` continua puro e sem React (`AGENTS.md:17`): `domain/gesture.ts` e `domain/item-menu.ts` passam em `node --test` sem Rust.
- `GridItem` (`bridge.ts:49-60`) é widenado por DF8, sem regra nova no Rust — `AGENTS.md:15` continua valendo: o Rust é dono, e aqui ele só **revela** uma coluna que já tinha.
- O `catch {}` de `CalendarScreen.tsx:183-185` sai (Fase F7): falha de leitura vira estado visível.

### 8.4 O que esta spec deixa para as outras

| Spec                     | Recebe                                                                                                      |
| ------------------------ | ----------------------------------------------------------------------------------------------------------- |
| **03** (mobile, D21–D23) | A DF5 (teclado) e a DF11 (anúncios) são reutilizáveis pelas ações; o arrasto em si, não                     |
| **05** (Google)          | O item `google` chega **não arrastável** por DF8/D17, e o conflito de sync usa a mesma faixa inline do DF13 |
| **06** (sugestões)       | O placeholder da criação (§4.1) é o mesmo objeto que a sugestão aceitaria; o `commitment` nunca é promovido |

### 8.5 Critérios de aceite finais da spec

1. `npm run typecheck` → 0 e `npm run lint` → `0 problems` em **todas** as fases.
2. `node --test src/features/calendar/domain/` verde, incluindo `snapY` correto em `hourHeight` 56 **e** 84, `clampRangeToDay` na borda da meia-noite, e `item-menu` com `google`/`cancelado`/`concluido`.
3. **100 ocorrências arrastadas a 60 fps**, com **zero** `setState` React por quadro (React Profiler parado).
4. **Zero** chamadas a `formatToParts` durante o arraste.
5. `Escape` no meio do arraste: item volta, ghost some, **nenhum** comando chamado.
6. Soltar o botão fora da janela **não** prende o item ao cursor.
7. `rg "devicePixelRatio" src/features/calendar/ui` → vazio. `rg "setInterval.*16|throttle" src/features/calendar/ui` → vazio.
8. `rg "drag|pointerdown|pointermove|setPointerCapture" src` → **≠ 0** (o oposto do estado atual), e `rg "devicePixelRatio" src/features/calendar` → vazio.
9. `rg "cursor-grab" docs/calendario/02-grade-desktop.md` → corrigido para "removido em 2026-09-29".
10. Item `origin: "google"` não tem `cursor-grab`, não tem alça, não tem ação de escrita (D17/INV-6).
11. "Esta ocorrência" num item recorrente **não** altera `event_recurrence` (INV-5).
12. Conflito **nunca** impede o `pointerup` (D7).
13. `rg "catch\s*\{\s*\}" src/features/calendar` → vazio; o estado de carregamento não pisca a cada troca de semana.

---

## 9. Fora de escopo

- Absorção de um item por outro (empurrar o vizinho). Exigiria transação de duas escritas e é decisão de produto (§4.4).
- Redimensionamento por **meia-noite**: um item entre 23:00 e 01:00 ocupa duas colunas; mover a borda de cima de um pedaço move a do outro? Hoje `placeInDay` devolve `[]` e a coluna decide (C-02). A semântica precisa de decisão de domínio, não de interação — e pertence à spec 01, não a esta.
- Arraste entre **telas** (uma lista reordenável, uma barra lateral reordenável). Gatilho de revisão de A4 (§DF1).
- Zoom da grade (`Ctrl`+roda / pinça). É uma decisão de **densidade**, e D16 já a define como seletor de três estados. `Ctrl`+roda no WebView2 é reservado para zoom do navegador e não deve ser sequestrado.
- Animações ligadas ao scroll (`animation-timeline`). Suportado no WebView2, mas não há nada rolando que valha (DF14).
- Feedback sonoro ou háptico (M-04): sem canal confiável no Windows, e `aria-live` é o honesto.
- Multi-seleção e arraste múltiplo. `columnize` já mostra a sobreposição; mover 8 itens de uma vez é produto novo.
