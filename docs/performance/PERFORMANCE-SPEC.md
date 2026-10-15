# Specs de performance, transição e fluidez

> **Arquivado em 2026-10-15. Não é fonte da verdade.**
>
> Três fatos, e nenhum é opinião:
>
> 1. **As citações de código daqui não resolvem mais.** Elas apontam para
>    `commands.rs:NNN`, e esse arquivo não existe desde 2026-10-15 — o caminho de
>    escrita foi para `<dono>/comandos.rs`, um diretório por dono
>    ([`SPEC-D-015`](../../../docs/specs/desktop/SPEC-D-015-modulos-do-backend-rust.md),
>    `D96..D100`). São 33 citações em três arquivos.
> 2. **Nenhum item destas specs foi medido.** A própria spec admite, no índice:
>    *"Todas as medições citadas nas specs 01 e 03 continuam sendo estimativas
>    derivadas da leitura do código… a Fase P0 não rodou, e nenhuma delas foi
>    substituída por número medido."* São 461 KB de prosa sobre custo de render
>    sem uma medição de custo de render.
> 3. **Descrevem um estado que já foi.** O `C-06` da spec 03 afirma que `origin`
>    está ausente do wire; `GridItem` tem `origin` e `allDay` desde antes
>    ([`types.ts:60,66`](../../src/features/calendar/domain/types.ts)). Item que
>    descreve bug corrigido não é pendência: é distração.
>
> **O que vale.** As decisões `D*` de [`docs/calendario/`](../calendario/) e as
> specs indexadas em [`docs/specs/`](../../../docs/specs/README.md). Onde este
> arquivo e o código divergirem, vale o código.
>
> **Por que arquivado em vez de corrigido.** Corrigir as 33 citações seria
> manter 461 KB de especificação não verificada por gate — o mesmo trabalho que a
> SPEC-D-015 acabou de pagar no backend, aplicado a um documento que a própria
> spec declara não medido. O que falta aqui não é remendo: é **rodar a Fase P0**
> e re-derivar o que for medido. Isso é decisão de fase, não de escrita — ver
> [`docs/todo/TO-DO-DESKTOP.md`](../todo/TO-DO-DESKTOP.md).
>
> O conteúdo abaixo fica **como registro do que foi levantado em 2026-09-29**,
> e é útil exatamente nisso: é uma lista de hipóteses que ninguém mediu, e uma
> hipótese não medida é um bom começo de fase.


Três specs de implementação derivadas de uma auditoria do código atual do `cecistudy-desktop` (Tauri v2 + React 19 + Vite 8 + WebView2). Cobre os três eixos pedidos: **performance** (custo de render, IPC e arranque), **transições/animações** (sistema de motion) e **fluidez** (interação, arraste, input).

Data: 2026-09-29
Status: **especificação.** Esta linha descreve 2026-09-29, quando nada tinha sido implementado. **Em 2026-10-03, 15 das 36 decisões existem no código** — ver §"Status por item, medido contra o código (2026-10-03)", que é a fonte da verdade para o que está feito. Todas as medições citadas nas specs 01 e 03 continuam sendo **estimativas derivadas da leitura do código**, marcadas como tal: a Fase P0 não rodou, e nenhuma delas foi substituída por número medido.

**As decisões de `docs/calendario/` continuam valendo.** Nenhuma destas specs reabre D1..D46 nem INV-1..INV-7. Onde performance e domínio parecessem conflitar, o domínio vence e o custo se resolve por outro caminho.

**Numeração.** `D*` é das specs de `docs/calendario/`. `DP*` (performance), `DM*` (motion) e `DF*` (fluidez) são exclusivos de cada spec e não renumeram nenhum `D`.

---

## As três specs

| Spec | Arquivo                                                    | Eixo                                                                                       | Decisões    | Fases    | Linhas |
| ---- | ---------------------------------------------------------- | ------------------------------------------------------------------------------------------ | ----------- | -------- | ------ |
| 01   | [`01-performance-render.md`](01-performance-render.md)     | Custo de render, IPC/Rust/SQLite, bundle, arranque, paint                                  | `DP1..DP10` | `P0..P7` | 479    |
| 02   | [`02-transicoes-animacoes.md`](02-transicoes-animacoes.md) | Sistema de motion: tokens, curvas, catálogo de gestos, o que nunca anima                   | `DM1..DM12` | `M0..M5` | 849    |
| 03   | [`03-fluidez-interacao.md`](03-fluidez-interacao.md)       | Arraste, resize, criação por gesto, teclado, scroll, cancelamento, acessibilidade do gesto | `DF1..DF14` | `F0..F7` | 764    |

## Status por item, medido contra o código (2026-10-03)

Item 5.2 da F5 (débito C12). **O cabeçalho deste arquivo diz "nada foi
implementado". Isso era verdade em 2026-09-29 e está errado hoje** — 15 das 36
decisões existem no código. O status abaixo foi medido por leitura, não copiado
das specs; `caminho:linha` é a evidência.

Placar: **15 implementadas · 4 parciais · 17 propostas · 0 obsoletas.** (`DP1`,
`DP10` e `DP11` saíram de "parcial" para "implementado" depois da ligação do
cache; ver a data da tabela.)

> Regra do grupo: **contagem literal sem o comando que a produz envelhece e vira
> mentira.** A tabela abaixo é um ponto no tempo. Ao reexecutar, o comando que
> reproduz é `npm run verify` mais a leitura por `caminho:linha`; se o número
> divergir, o código ganhou e a tabela está velha.

### `01-performance-render.md` — `DP1..DP10`

| # | Decisão | Status | Evidência |
|---|---|---|---|
| DP1 | Cache por janela, invalidado por escrita | **implementado** | `data/calendar-store.ts:58-167` + singleton `:171-190`; a tela lê por `store.read` (`ui/CalendarScreen.tsx:196-245`). Ver `DP11` |
| DP2 | `Intl` é infraestrutura, singleton de módulo | **implementado** | `ui/format.ts:21-27`, `domain/time.ts:97-102` |
| DP3 | Relógio de 60s não propaga para a grade | **proposto** | o oposto acontece: `ui/CalendarScreen.tsx:119` → `:327` → `WeekView` recalcula em `:387-389` |
| DP4 | Ponte assíncrona, materialização sob demanda | **proposto** | zero `async fn`; `commands.rs:535` é `pub fn`, materialização dentro da leitura em `:538-539` |
| DP5 | Sem virtualização agora; o gate decide depois | **parcial** | a metade negativa vale (sem `@tanstack/react-virtual`), mas o gate nunca rodou: sem `scripts/measure-bundle.mjs` |
| DP6 | A fronteira de estado é `items`, não o estado da tela | **implementado** | `CalendarScreen.tsx:202,244-245`; `DayColumn.tsx:38` e `TimedItem.tsx:38` em `memo` |
| DP7 | `backdrop-filter` só onde o conteúdo rola | **implementado** | `App.tsx:200-211` — largura por corte, conteúdo em `transition-transform` |
| DP8 | Métrica de aceite é número, não adjetivo | **proposto** | sem instrumentação alguma: `performance.mark`, `PerformanceObserver` e `web-vitals` não aparecem |
| DP9 | `sideEffects: false` fica; split só com fronteira real | **parcial** | `package.json:5` ok; sem `manualChunks` nem `React.lazy`, e fonte continua vindo de `fonts.googleapis.com` (`index.html:10-14`) |
| DP10 | Cache tem dono, forma e invalidação; não é `useMemo` | **implementado** | API em `calendar-store.ts:58-66`, singleton em `:171-190`, e **10 testes** em `data/calendar-store.test.ts` (eram zero) |
| **DP11** | **O cache é ligado à tela, e a guarda de ordenação é por geração, não por boolean** | **implementado** | `01-performance-render.md` §3.2. Fecha o débito **C17** |

### `02-transicoes-animacoes.md` — `DM1..DM12`

| # | Decisão | Status | Evidência |
|---|---|---|---|
| DM1 | Motion em CSS puro e WAAPI; sem biblioteca | **implementado** | `package.json:32-52` sem `motion`/`framer-motion` |
| DM2 | Escala fechada de duração, cinco degraus | **implementado** | `styles.css:70-74` |
| DM3 | Curvas por tipo de movimento | **implementado** | `styles.css:83-89` |
| DM4 | Só `transform` e `opacity` animam posição e tamanho | **implementado** | `transition-transform`, `transition-colors`; nada de `transition-[width/height/top/left]` |
| DM5 | Política de FLIP em dois passos, WAAPI | **proposto** | sem `use-flip.ts`; `--ease-spatial` existe e **nenhum componente consome** |
| DM6 | `prefers-reduced-motion` em três níveis | **implementado** | `styles.css:230-240` e `:457-468`. O hook `src/hooks/use-reduced-motion.ts:37-49` existe **sem consumidor** |
| DM7 | O que nunca anima. Lista fechada | **implementado** | `CalendarScreen.tsx:467-468` (`nowTop` sem transição); `TimedItem.tsx:58-59` |
| DM8 | O set curado não é editado à mão; só a folha decide | **parcial** | o override de `styles.css:99-100` existe, mas sobram literais: `tabs.tsx:30`, `progress.tsx:18`, `App.tsx:208` |
| DM9 | `backdrop-filter` proibido dentro de container rolável | **implementado** | os três `sticky` usam `var(--surface-raised)`, opaco (`CalendarScreen.tsx:395,400,436`) |
| DM10 | Nenhuma animação bloqueia ou atrasa o input | **proposto** | a troca de modo remonta a subárvore inteira (`CalendarScreen.tsx:326-351`) |
| DM11 | Feedback de gravação é estado, não espetáculo | **proposto** | sem `saving`/`saved`; botões com `disabled` hardcoded (`:785-790`) |
| DM12 | Conflito e sobreposição têm sinal estático | **proposto** | `hasConflict` existe no tipo (`data/bridge.ts:67`) e **nenhum componente o lê** |

### `03-fluidez-interacao.md` — `DF1..DF14`

| # | Decisão | Status | Evidência |
|---|---|---|---|
| DF1 | Arraste com Pointer Events manuais, sem lib | **proposto** | sem `onPointerDown`/`setPointerCapture` no `src/` |
| DF2 | `getBoundingClientRect` do corpo da grade, uma vez | **proposto** | sem ocorrência |
| DF3 | Encaixe de 15 min; `Alt` como escape | **parcial** | a geometria está pronta **e testada** (`grid-scale.ts:84,102,119-120`; `grid-scale.test.ts:110-152`), mas o escape `Alt` não tem consumidor |
| DF4 | Durante o arraste o DOM muda direto | **proposto** | `TimedItem.tsx:38-67` é só `memo` + posicionamento |
| DF5 | Teclado: aritmética de 15 min e modo de resize | **proposto** | os únicos `onKeyDown` são de tablist |
| DF6 | Cancelamento: cinco rotas, um único `reset()` | **proposto** | sem `pointercancel`/`visibilitychange` |
| DF7 | Limiar de 4 px e o `click` pós-`pointerup` engolido | **proposto** | o `onClick` de `TimedItem.tsx:50` dispara sempre |
| DF8 | `GridItem` ganha `origin` e `allDay` (mudança Rust) | **proposto** | `bridge.ts:49-68` não tem os dois; o `SELECT` em `commands.rs:541-542` também não |
| DF9 | O componente não escolhe o comando; sai de `kind` | **proposto** | `data/apply-gesture.ts` não existe; `moveOccurrence` não é chamada |
| DF10 | Scroll: um container, `overscroll-behavior: contain` | **implementado** | `styles.css:305-312`, aplicado em `CalendarScreen.tsx:392` |
| DF11 | Acessibilidade do arraste: `aria-live` com debounce | **proposto** | sem `aria-live`/`aria-grabbed` |
| DF12 | `content-visibility: auto` nas colunas de dia | **implementado** | `styles.css:325-329`, aplicado em `DayColumn.tsx:58` |
| DF13 | Feedback de falha: rollback + faixa inline | **proposto** | o erro de leitura é engolido em `catch {}` (`CalendarScreen.tsx:181-183`) |
| DF14 | O que esta spec explicitamente **não** usa | **implementado** | sem `react-aria`, `react-window`, `@tanstack/*`, `dnd-kit` |

### O que essa medição revela

**O que foi implementado não é o que as specs esperavam.** As specs 02 e 03
foram executadas como **fundação** — superfície, tokens, contenção, tablist — e
não como **gesto**. Das 14 decisões de arrastre, nenhuma passou de "proposto",
exceto `DF3`, cuja geometria está isolada em `grid-scale.ts` **e testada**, e
`DF12`, entregue sem o arraste.

O caso mais agudo é **`DP1`/`DP10`: `calendar-store.ts` é código morto, e a
docstring dele afirma que o cache existe para remover um defeito.** Não há um
único importador, não há um único teste, e a tela continua usando o `cancelled`
que a docstring diz que "some junto com o cache"
(`CalendarScreen.tsx:189-191` contra `calendar-store.ts:85`). Isto é o débito C10
de novo, com outra roupa: **código morto com docstring que afirma servir.**
O [`AGENTS.md`](../../../AGENTS.md) do grupo já lista "código morto (nem com
teste vivo)" como algo que não se porta; aqui ele também não deveria ficar.

O mesmo padrão, mais brando, vale para motion. **Atualizado em 2026-10-05.**
Os tokens de `styles.css` continuam completos e corretos (`DM2`, `DM3`, `DM6`,
`DM9` plenos), mas a afirmação anterior — "o vocabulário existe; a animação
não" — era **meia verdade**, e a contagem é o que diz qual metade:

```
rg -c --glob '!src/styles.css' "motion-(state|spatial|item|cross-fade|surface|gesture)" src
```

| Utility | Consumidor | Medido em 2026-10-05 |
|---|---|---|
| `motion-state` | `src/shell/SidebarNav.tsx` (2), `src/components/ui/checkbox.tsx` (2) | tem |
| `motion-spatial` | `src/shell/SidebarNav.tsx` (1) — o chevron do ramo | tem |
| `motion-item` | — | **sem** |
| `motion-cross-fade` | — | **sem** |
| `motion-surface`, `motion-surface-closed`, `motion-gesture` | — | **sem** |

Ou seja: `motion-state` e `motion-spatial` ganharam consumidor com a barra
lateral compacta (`SPEC-D-006` D63), e continuam sem consumidor `motion-item`,
`motion-cross-fade`, `motion-surface` e `motion-gesture`. `DM5` e `DM6` seguem
"implementado" e "sem uso" ao mesmo tempo — e é exatamente o que a skill
[`verificacao-por-contrato`](../../../.opencode/skill/verificacao-por-contrato/SKILL.md)
chama de regra escrita e não conectada. A barra lateral adicionou ainda
`motion-rail`, `motion-rail-icon` e `motion-rail-in`, que já nascem com
consumidor; ver
[`02-transicoes-animacoes.md`](02-transicoes-animacoes.md) §2.

### Três incoerências internas que o código resolveu pelo lado certo

| Onde | O que a spec diz | O que o código faz | Efeito |
|---|---|---|---|
| spec 02 `:752` (X19) | `--ease-spatial` = `0.27, 1.06, 0.18, 1.00` (overshoot) | `cubic-bezier(0.2, 0, 0, 1)`, monótono — `styles.css:88` | **corrige**: a linha contradiz o bloco normativo de `DM3` (`:246`) |
| spec 02 `:224` e `:561` | `--duration-large` é para "densidade (D16)" | não existe controle de densidade; `hourHeight` é constante de módulo (`grid-scale.ts:36-37`) | **consumidor nomeado não existe** |
| spec 03 `H-09` e spec 02 `:712` | `App.tsx:192-193` com `transition-[width] duration-200` | essa linha hoje é um comentário explicando a **remoção**; o custo foi para `transform` (`App.tsx:208`) | **resolvido**: `H-09` e a "Exceção 1" de `DM4` perderam o racional |

Nenhuma delas é um bug do app. São três places onde a spec descreve um estado
intermediário que o código passou. É o débito C12 em forma menor, e a correção
é a data: **estas três linhas descrevem 2026-09-29.**

`grid-scale.ts` e `hourHeight` **não** mudaram — `hourHeight: 56`,
`dayStartHour: 6` e `visibleHeight` de 1008 px continuam, o que confirma as
premissas de `S-07` e de `DF12`.

### O que a Fase P0 violaria se fosse feita agora

`P0` (baseline e instrumentação) é pré-requisito de `DP8`, e `DP8` é pré-requisito
de `DP5`. **Nenhuma das três existe.** Ou seja: o caminho crítico declarado na
tabela de ordem de implementação começa por uma fase que não rodou, e duas das
decisões mais caras (`DP3`, `DP4`) não têm como ser avaliadas sem `P0`. Isso é
um bloqueio de **sequência**, não de ambiente — e por isso é trabalho de roadmap,
não de instalação.

---

## Ordem de implementação

```
P0  Baseline e instrumentação (nenhum comportamento muda)
 └─o─ M1  Tokens de motion em @theme (nenhum componente tocado)
      └─o─ F0  Correções de base em TypeScript puro (snap, origin, byDay)
           └─o─ P2  Intl como infraestrutura
                └─o─ P3  Fronteira de estado e memoização
                     ├─o─ M2  Custo de paint: sidebar e backdrop-blur
                     ├─o─ F1  Contrato do gesto (testável sem DOM)
                     └─o─ P1  Cache por janela  ·  P4  Backend async
                          └─o─ F2  useGridDrag ← caminho crítico de latência
                               └─o─ M3  Troca de modo, navegação, enter/exit
                                    └─o─ F3  Criar, redimensionar, teclado
                                         └─o─ M4  Carga, erro inline, gravação
                                              └─o─ F5  Painel e grade  ·  F6  Meses e dia inteiro
                                                   └─o─ P5  Paint  ·  P6  Arranque  ·  P7  Fechamento
```

**Regras de encadeamento:**

- **Nenhuma fase começa antes de P0 fechar.** Nenhuma otimização entra no repositório antes de existir número medido.
- **M1 não toca componente nenhum.** Tokens em `@theme` é a única fase de motion que pode rodar antes de qualquer medição, porque não muda comportamento — ela só cria o vocabulário.
- **F0 e F1 são TypeScript puro e não precisam do app de pé.** São as fases que destravam o resto da spec 03 e podem ser executadas num ambiente onde o toolchain Rust não linka.
- **P4 toca Rust.** É a fase mais arriscada (20 comandos viram `async`) e não deve dividir commit com nenhuma outra.

---

## Caminho crítico

**F2 (`useGridDrag`) é a fase que define se o app "flui" ou não.** Tudo o que vem depois dela (criar, redimensionar, teclado, animação do arraste) é correção de um gesto que já responde. Se F2 falhar o critério de aceite — _100 ocorrências a 60 fps com o React Profiler parado_ — as fases seguintes não-meaningfully avançam.

**P3 é a fase que destrava a memoização.** DP3 (o relógio de 60 s não propaga para a grade) e DP6 (a fronteira de estado é `items`, não o estado da tela) vêm **antes** dos `memo`, porque `memo` aplicado antes disso nunca acerta e é custo sem benefício.

**M2 é a fase de motion mais urgente** e a mais barata: remove a única transição de _layout_ viva do app (`transition-[width]` da sidebar) e os quatro `backdrop-filter` sobre conteúdo que rola.

---

## Achados que atravessam as três specs

| #    | Achado                                                                                                                                                                                                     | Onde                                                                    | Specs                             |
| ---- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- | --------------------------------- |
| A-01 | Nenhum comando Tauri é `async` — 20 `#[tauri::command]` rodam na thread principal                                                                                                                          | `src-tauri/src/commands.rs` (20 comandos; `rg 'async fn'` → vazio)   | 01 §1, 03 §1.7                    |
| A-02 | `formatClock` constrói um `Intl.DateTimeFormat` por item, por render                                                                                                                                       | `CalendarScreen.tsx:90-96`, chamada em `:452` e `:465`                  | 01 (P2)                           |
| A-03 | `placeInDay` custa ~8 `Intl.formatToParts` por item                                                                                                                                                        | `domain/time.ts:117`; `CalendarScreen.tsx:444`; `grid-scale.ts:130-138` | 01 (P2)                           |
| A-04 | O relógio de 60 s re-renderiza a tela inteira e impede qualquer `memo`                                                                                                                                     | `CalendarScreen.tsx:120-123` → `:306,309,315,323,329`                   | 01 (P3/DP3), 02 (M3)              |
| A-05 | Quatro `backdrop-filter` sobre conteúdo que rola                                                                                                                                                           | `styles.css:186-189`; `App.tsx:192,320`; `CalendarScreen.tsx:372,406`   | 01 (P5/DP7), 02 (M2/DM9)          |
| A-06 | **Não existe sistema de arraste** — `rg 'drag\|pointerdown\|pointermove\|setPointerCapture' src/` → 0 em 45 arquivos. O `cursor-grab` citado em `02-grade-desktop.md:27` **não existe mais no código**     | —                                                                       | 03 (todo o §3)                    |
| A-07 | `snapY` está **errado**: encaixa em `snapMinutes * 4` = 60px, mas 15 min a 56px/h são 14px (4,3× fora) e ignora `hourHeight`. Não é testado nem usado — só estoura quando o arraste existir                | `domain/grid-scale.ts:96`                                               | 03 (F0, bug fix bloqueante de F2) |
| A-08 | `origin` não vem pelo wire → a D17 (bloquear arraste de item do Google antes do gesto) é **inimplementável** como está                                                                                     | `data/bridge.ts:49-60`; `commands.rs:487-492`                           | 03 (DF8), 01 (Fase P4)            |
| A-09 | `byDay` agrupa por `item.startsAt` → ocorrência que começa 3 dias antes da janela é buscada pelo SQL e **desaparece**                                                                                      | `CalendarScreen.tsx:201-210`                                            | 03 (F0)                           |
| A-10 | Sem estado de carregamento nem de erro: `empty` só liga depois de `loaded` (a grade pisca vazia a cada troca de semana) e o `catch {}` engole tudo                                                         | `CalendarScreen.tsx:113,183-186,217-221`                                | 02 (M4), 03 (F7)                  |
| A-11 | Quatro curvas de easing convivendo, sem token nenhum em `@theme`                                                                                                                                           | `styles.css:21-63`; `skeleton.tsx:4`; `App.tsx:192`; defaults do Radix  | 02 (M1/DM2, DM3, DM8)             |
| A-12 | `prefers-reduced-motion` não zera `animation-iteration-count` — `animate-pulse` do skeleton vira strobe sob `reduce`. Também não cobre `prefers-reduced-transparency`, e a `vibrancy` roda em três lugares | `styles.css:168-176`; `skeleton.tsx:4`; `App.tsx:192,320,379`           | 02 (M1/DM6)                       |
| A-13 | Cabeçalho de dias sem `sticky left` → as colunas dessincronizam no scroll horizontal                                                                                                                       | `CalendarScreen.tsx:373` vs `:406`                                      | 03 (F5/DF10)                      |
| A-14 | 836px de largura mínima contra 712px disponíveis no `minWidth: 1024` — a promessa de `02-grade-desktop.md:39` é falsa                                                                                      | `src-tauri/tauri.conf.json:22`                                          | 03 (F5/DF10)                      |

---

## Decisões de tecnologia (uma por eixo, todas fechadas)

| Eixo        | Decisão                                                                                        | Por quê                                                                                                                                                                                                                                                                                                                                                                                                            |
| ----------- | ---------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Performance | Sem biblioteca de estado. Cache por janela é um módulo com `Map` privado e API de três funções | `useSyncExternalStore` entra só quando a spec 05 trouxer sincronização por evento; `zustand`/`react-query` são duas dependências para o problema de uma janela (DP10)                                                                                                                                                                                                                                              |
| Performance | `@tanstack/react-virtual` **não** entra agora — é um gate, decidido por medição na Fase P5     | `hourHeight: 56` fixo (D16) é o que torna a grade não-virtualizável-com-ganho hoje. Introduzir antes de medir troca problema mensurável por não mensurável (DP5)                                                                                                                                                                                                                                                   |
| Performance | Sem `React.lazy` nem `manualChunks`                                                            | `AGENTS.md:9` — sem roteador; trocar de leitura não pode custar carregamento (D15bis), nem `invoke` (DP1/DP9)                                                                                                                                                                                                                                                                                                      |
| Motion      | **CSS puro + WAAPI. Nenhuma biblioteca de animação.**                                          | `motion` (framer) rejeitado: 34 kB não-shakeáveis e o `layout` mede `getBoundingClientRect` em ~130 nós da grade; `@formkit/auto-animate` rejeitado porque pula FLIP de elementos offscreen — o caso errado na grade; `startViewTransition` rejeitado por custo de captura, scroll não controlável e por ser all-or-nothing (o painel precisa se manter, D18). Adotados `@starting-style` + `allow-discrete` (DM1) |
| Fluidez     | **Pointer Events manuais. Zero dependência.**                                                  | `@dnd-kit/core` = 18,9 kB gzip sobre um bundle de 112 kB (+17%) para um modelo _sortable-first_ que não conhece `timeToY`; `@formkit/drag-and-drop` (4 kB) é data-first de **listas**; `react-aria` tem o melhor modelo de acessibilidade mas usa o DnD nativo do navegador, que em WebView2 não dá controle por quadro (DF1)                                                                                      |
| Fluidez     | Durante o arraste, o DOM muda direto por CSS custom properties; **zero `setState` por quadro** | É o que concilia o otimismo local da D17 com a latência de quadro. O React só vê as transições (DF4)                                                                                                                                                                                                                                                                                                               |

**Gatilhos de revisão.** `DM1` reabre com medição anexada se a grade passar de ~50 itens visíveis por dia e o FLIP manual virar gargalo mensurável, ou se surgir necessidade real de _shared element_ entre modos que o WAAPI não expresse. `DP5` reabre na Fase P5 com o perfil de paint na mão. `DF1` reabre se o `useGridDrag` passar de ~300 linhas e o custo de manutenção superar o bundle economizado.

---

## Invariantes de domínio que restringem as três specs

As três specs têm a mesma regra de ouro: **nenhuma otimização pode fabricar uma obrigação, e nenhuma animação pode fabricar um consentimento.**

| Regra                                                                                           | Onde                                                                   | Como as specs respeitam                                                                                                                                                         |
| ----------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `INV-6` / `D17` — o bloqueio de arraste de item do Google tem de ser **visível antes do gesto** | `docs/calendario/02-grade-desktop.md` D17                              | 03 §8.1 DF8 exige `origin` no wire **antes** de F2; a spec 01 (P4) muda o mesmo payload                                                                                         |
| `D7` / `INV-5` — sobreposição é derivada e só sinalizada, **nunca bloqueia**                    | `grid-scale.ts:157-165` ("isto é apenas legibilidade, nunca bloqueio") | 02 DM7 item 2: **a largura de um item na Semana nunca interpola**. Se `columnize` animar, a usuária lê a reorganização automática como "o sistema resolveu um conflito por mim" |
| `INV-7` / `D8` — `recomendado`/`opcional` nunca viram obrigação                                 | —                                                                      | 02 DM7 item 7: `Aceitar` e `Recusar` com **peso visual idêntico**; X11 rejeita animação que promova visualmente um item                                                         |
| `AGENTS.md:14-15` — Rust é a verdade do domínio; TS é a ponte                                   | —                                                                      | 01 P4 otimiza Rust **sem mover regra**; 01 §3 (DP2/DP3) dá a justificativa: memoizar `Intl` não é memoizar domínio                                                              |
| `AGENTS.md:17` — `domain/` é puro, sem React, sem `Date` direto                                 | —                                                                      | 01 DP2/DP3: cache de plataforma é singleton de módulo; nenhuma regra de domínio entra num cache de memoização, e nenhum cache entra no `domain/`                                |
| `AGENTS.md:12` — `src/components/ui/` é set curado do shadcn, não se edita à mão                | —                                                                      | 02 DM8: a folha de estilo (`--tw-duration`/`--tw-ease` no `:root`) é o **único** lugar de decisão, o que corrige os quatro arquivos Radix de uma vez sem editar o set curado    |

---

## O que estas specs não fazem

- **Não reimplementam o motor de recorrência nem as queries.** 01 P4 mexe em `expand_rule` e `materialize_occurrences` por custo, não por correção semântica; nenhum caminho de cálculo muda.
- **Não inventam gesto novo.** 03 executa o que `02-grade-desktop.md` (D17–D20, A4) já prometeu. Onde a spec 02 do calendário cite um `cursor-grab` que não existe mais em `App.tsx:918`, esta spec registra o fato (§1.1) e segue.
- **Não tocam na spec 05 (Google) nem na spec 06 (Sugestões)** além de preparar o terreno: 01 §6 mapeia o que cada uma vai trazer (sincronização por evento → `useSyncExternalStore`, `Channel`, paginação; sugestões → painel, nunca o caminho quente da grade).
- **Não medem nada por conta própria.** Todas as estimativas de tempo nas specs 01 e 03 vêm da leitura do código e estão marcadas como tal; a Fase P0 substitui todas elas.

---

## Rastreabilidade às specs do Calendário

| Spec do Calendário           | O que estas specs fazem com ela                                                                                                                                                                                                                                                                                      |
| ---------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `00-fundacao.md`             | A4 (drag próprio) é a razão de DF1. DP3 e DP6 decidem que o alvo da escrita a 60 Hz do arraste é um `TimedItem` memoizado com estado **local** — se o arraste usar o estado da tela, cada `pointermove` re-renderiza a Semana inteira. D6 (UTC + IANA) é intocada: P4 muda o custo de `expand_rule`, nunca o caminho |
| `01-dominio-persistencia.md` | DP4 transforma 20 comandos síncronos em `async` e tira a materialização do caminho de leitura. DF8 e DF9 (payload) tocam o mesmo Rust                                                                                                                                                                                |
| `02-grade-desktop.md`        | **03 é a executora de D17–D20**, que a spec 02 do Calendário deixou em aberto. 01 é a executora de D15bis (trocar de leitura não pode custar carregamento nem `invoke`), D16 (`hourHeight: 56` é o que torna P5 um gate) e D18 (o painel de 280px + `minWidth: 1024` é o que força rolagem horizontal)               |
| `03-mobile-execucao.md`      | DP3 vira **norma**, não correção pontual: o timer de 1 s escrevendo em estado de tela é a mesma classe de defeito, em escala pior. DF5 (teclado) e DF11 (anúncios) são reutilizáveis pelas ações; o arraste em si, não                                                                                               |
| `04-modulos-origem.md`       | `origin` não viaja no wire hoje; DP1 absorve o crescimento do payload como mais uma chave de cache, não como um `filter` por render                                                                                                                                                                                  |
| `05-google-calendar.md`      | É a spec que **materializa** o risco que DP10 adia: sincronização por evento traz `useSyncExternalStore` com seletor memoizado e `getSnapshot` estável, listeners Tauri (hoje `rg 'listen' src` → vazio, sem precedente de cleanup), paginação e `Channel`                                                           |
| `06-sugestoes.md`            | Nenhuma leitura de sugestão entra no caminho de render da grade. DM7 item 7 e DM12 já restringem o que ela pode animar: sinal estático, peso visual idêntico entre aceitar e recusar                                                                                                                                 |
