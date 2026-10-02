# Specs de performance, transição e fluidez

Três specs de implementação derivadas de uma auditoria do código atual do `cecistudy-desktop` (Tauri v2 + React 19 + Vite 8 + WebView2). Cobre os três eixos pedidos: **performance** (custo de render, IPC e arranque), **transições/animações** (sistema de motion) e **fluidez** (interação, arraste, input).

Data: 2026-09-29
Status: **especificação — nada foi implementado.** Todas as medições citadas nas specs 01 e 03 são **estimativas derivadas da leitura do código**, marcadas como tal. A Fase P0 da spec 01 existe justamente para substituí-las por número medido.

**As decisões de `docs/calendario/` continuam valendo.** Nenhuma destas specs reabre D1..D46 nem INV-1..INV-7. Onde performance e domínio parecessem conflitar, o domínio vence e o custo se resolve por outro caminho.

**Numeração.** `D*` é das specs de `docs/calendario/`. `DP*` (performance), `DM*` (motion) e `DF*` (fluidez) são exclusivos de cada spec e não renumeram nenhum `D`.

---

## As três specs

| Spec | Arquivo                                                    | Eixo                                                                                       | Decisões    | Fases    | Linhas |
| ---- | ---------------------------------------------------------- | ------------------------------------------------------------------------------------------ | ----------- | -------- | ------ |
| 01   | [`01-performance-render.md`](01-performance-render.md)     | Custo de render, IPC/Rust/SQLite, bundle, arranque, paint                                  | `DP1..DP10` | `P0..P7` | 479    |
| 02   | [`02-transicoes-animacoes.md`](02-transicoes-animacoes.md) | Sistema de motion: tokens, curvas, catálogo de gestos, o que nunca anima                   | `DM1..DM12` | `M0..M5` | 849    |
| 03   | [`03-fluidez-interacao.md`](03-fluidez-interacao.md)       | Arraste, resize, criação por gesto, teclado, scroll, cancelamento, acessibilidade do gesto | `DF1..DF14` | `F0..F7` | 764    |

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
| A-01 | Nenhum comando Tauri é `async` — 21 `#[tauri::command]` rodam na thread principal                                                                                                                          | `src-tauri/src/commands.rs` (21 ocorrências, `rg 'async fn'` → vazio)   | 01 §1, 03 §1.7                    |
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
