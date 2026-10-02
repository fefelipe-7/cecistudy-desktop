# Spec 02 — Transições e animações

Data: 2026-09-29
Escopo: auditar **todo** o sistema de movimento do app (o que já anima, o que não anima e o que está quebrado) e fechar a política de motion — escala de duração, curvas, tecnologia, FLIP, `prefers-reduced-motion` e a lista do que nunca pode animar — antes que a spec 02 do Calendário (arrasto, redimensionamento, painel contextual) escreva o primeiro gesto animado.
Status: **especificada, não implementada.** Nenhum arquivo de `src/` é tocado por este documento.

**Restrição herdada e inviolável:** `DESIGN.md:25` — _"Transições rápidas de 150–200ms apenas para mudanças de estado. Sem animações de entrada decorativas. Movimento reduzido é respeitado."_ Toda decisão `DM` abaixo que proponha duração fora de 150–200ms precisa justificar por que **não é uma mudança de estado** e por que a alternativa custa caro demais. As decisões que assim se justificam são exatamente duas: rearranjo de layout (DM5) e superfície em tela cheia (DM2, `--duration-xlarge`).

**Rastreabilidade — `calendario.md`:** §7 (experiência desktop: a grade, o painel contextual que permanece aberto, o arrastar/redimensionar), §3 (níveis de compromisso — `recomendado`/`opcional` nunca viram obrigação), §13 (regra de ouro: três perguntas separadas), §12.7 (conflito derivado, nunca bloqueante).
**Rastreabilidade — `docs/calendario/00-fundacao.md`:** D4 (tom de camada fixo, sem personalização), D7 (sobreposição permitida, conflito derivado — "isto é apenas legibilidade, nunca bloqueio", `domain/grid-scale.ts:157-165`), A4 (arrasto próprio, sem biblioteca), A8/INV-1..INV-7.
**Rastreabilidade — `docs/calendario/02-grade-desktop.md`:** D15 (uma escala de tempo só), D16 (7 dias, `hourHeight` 56px, densidades 24/56/84px), D17 (arrasto só para `origin !== "google"`), D18 (painel é coluna, não modal; vira `Sheet` no `max-lg`), D19 (`ViewSwitch` interno), D20 (painel em três colunas, uma por pergunta da §13). A linha `02-grade-desktop.md:40` já registra que `styles.css:168-176` desliga animação sob `prefers-reduced-motion` — **esta spec mostra que esse bloco está incompleto** (§1.6).
**Entregas de `calendario.md` §11 cobertas:** nenhuma diretamente. Esta spec **destrava** a entrega **2** (visão semanal com arrastar/redimensionar e painel contextual) e a **7** (camadas por origem) com um sistema de motion utilizável, e é pré-requisito de motion da spec 03.
**Depende de:** nada. `DESIGN.md`, `AGENTS.md`, `docs/calendario/00-fundacao.md` e `02-grade-desktop.md`.
**Deixa deliberadamente para:** spec 02 (arrasto e redimensionamento reais — os gestos estão especificados aqui, o código não), spec 03 (`Sheet`, timer, ações rápidas mobile), spec 05 (indicador de sincronização), spec 06 (sugestões — DM7 item 7 fecha como sugestão **não** se promove visualmente).

---

## 1. Diagnóstico do estado atual

### 1.1 Panorama

Verificado por leitura de `src/` inteiro em 2026-09-29.

| Métrica                                                                | Valor                                                                                                                                                                                          |
| ---------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Arquivos `.tsx`/`.ts`/`.css` em `src/`                                 | 45                                                                                                                                                                                             |
| Arquivos com alguma classe de motion                                   | 18                                                                                                                                                                                             |
| Componentes de `src/components/ui/` **efetivamente importados**        | **4** — `badge`, `button`, `dialog`, `tooltip`                                                                                                                                                 |
| Componentes de `src/components/ui/` **existentes e nunca importados**  | 15 — `card`, `checkbox`, `dropdown-menu`, `input`, `label`, `popover`, `progress`, `scroll-area`, `select`, `separator`, `skeleton`, `switch`, `tabs`, `textarea`, `button-variants` (parcial) |
| Animações de Radix que **rodam hoje**                                  | 2 — `Dialog` (overlay + conteúdo) e `Tooltip`                                                                                                                                                  |
| Animações de Radix **escritas mas nunca executadas**                   | 10 arquivos — `popover`, `select`, `dropdown-menu` (content, subcontent, 3× item), `scroll-area`, `switch` (2×), `checkbox`, `progress`, `tabs`, `input`                                       |
| Tokens de motion em `src/styles.css` (`@theme`, linhas 21–63)          | **zero**. Só `--font-sans`, `--radius-*` e `--color-*`                                                                                                                                         |
| `@keyframes` customizados no projeto                                   | **zero**. Só os de `tw-animate-css` e os do Tailwind                                                                                                                                           |
| `framer-motion` / `motion` / `@formkit/auto-animate` em `package.json` | **nenhuma** (verificado, `package.json:23-45`)                                                                                                                                                 |
| `will-change` declarado em qualquer lugar                              | **nenhum**                                                                                                                                                                                     |
| `content-visibility` / `contain`                                       | **nenhum**                                                                                                                                                                                     |
| Bloco `prefers-reduced-motion`                                         | `src/styles.css:168-176` — existe e está **incompleto** (§1.6)                                                                                                                                 |

Leitura geral: **o app anima quase nada, e o pouco que anima é por acidente de biblioteca.** Não há nenhuma decisão de motion registrada, nenhum token, nenhuma curva escolhida. O que existe é o default do Tailwind v4 (`--default-transition-duration: 150ms` e `--default-transition-timing-function: cubic-bezier(0.4, 0, 0.2, 1)` — `node_modules/tailwindcss/theme.css:492-493`) colado em classes literais, mais o default do `tw-animate-css` (`--animate-in: enter var(--tw-animation-duration, var(--tw-duration, .15s)) var(--tw-ease, ease) …` — `node_modules/tw-animate-css/dist/tw-animate.css:1`) herdado do shadcn.

### 1.2 Inventário — tudo que anima hoje

Curvas usadas no inventário, nomeadas:

- **C1** `cubic-bezier(0.4, 0, 0.2, 1)` — default de transição do Tailwind v4 (`theme.css:493`)
- **C2** `cubic-bezier(0.25, 0.1, 0.25, 1)` — a palavra-chave `ease`, default do `tw-animate-css` nos `@keyframes enter`/`exit` (`tw-animate.css:1`)
- **C3** `cubic-bezier(0, 0, 0.2, 1)` — `--ease-out` do Tailwind (`theme.css:435`)
- **C4** `cubic-bezier(0.4, 0, 0.6, 1)` — `pulse` do Tailwind (`animate-pulse`)
- **C0** — sem transição: troca instantânea

| #   | Elemento                                          | `arquivo:linha`                                       | Propriedade                                                                                            | Duração              | Curva  | Gatilho                                          | Custo               | Opinião                                                                                                                                                                                                                                                                                                                                                                                            |
| --- | ------------------------------------------------- | ----------------------------------------------------- | ------------------------------------------------------------------------------------------------------ | -------------------- | ------ | ------------------------------------------------ | ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Barra lateral (recolher/expandir)                 | `src/App.tsx:192`                                     | `width`                                                                                                | 200ms                | C3     | clique em `PanelLeft` (`:322-327`)               | **layout** — reflow | 🔴 **O pior item do inventário.** `width` em flex row: cada quadro reflow do `<main>` inteiro, que na visão Calendário contém a grade de 7 colunas (`CalendarScreen.tsx:402`). Proibido por DM4.                                                                                                                                                                                                   |
| 2   | Linha do topo da sidebar                          | `src/App.tsx:197`                                     | `color`, `background-color`                                                                            | 150ms                | C1     | hover                                            | paint               | ok                                                                                                                                                                                                                                                                                                                                                                                                 |
| 3   | Botão "Busca rápida"                              | `src/App.tsx:217`                                     | `color`, `background-color`, `border-color`                                                            | 150ms                | C1     | hover                                            | paint               | ok                                                                                                                                                                                                                                                                                                                                                                                                 |
| 4   | Item de navegação (`NavItem`)                     | `src/App.tsx:453`                                     | `color`, `background-color`                                                                            | 150ms                | C1     | hover / seleção                                  | paint               | ok — mas `hover:bg-primary` (`:456`) compete com `hover:bg-accent/60` (`:453`); o vencedor depende de ordem de Tailwind, não de intenção.                                                                                                                                                                                                                                                          |
| 5   | Linha da "Sua linha do tempo"                     | `src/App.tsx:639`                                     | `background-color`                                                                                     | 150ms                | C1     | hover                                            | paint               | ok                                                                                                                                                                                                                                                                                                                                                                                                 |
| 6   | Item de "Para avançar hoje" (linha)               | `src/App.tsx:675`                                     | `background-color`                                                                                     | 150ms                | C1     | hover                                            | paint               | ok                                                                                                                                                                                                                                                                                                                                                                                                 |
| 7   | Quadrado de conclusão (decorativo)                | `src/App.tsx:680`                                     | `border-color`, `background-color`                                                                     | 150ms                | C1     | marcar/desmarcar                                 | paint               | 🟠 Custo ok, mas **é a única animação de "concluir"** e ela não diz nada: o `Check` (`:686`) e o `line-through` (`:692`) entram de golpe. §1.5 A11.                                                                                                                                                                                                                                                |
| 8   | Aba de `ViewSwitch`                               | `src/App.tsx:752`                                     | `color`, `background-color`                                                                            | 150ms                | C1     | trocar de modo                                   | paint               | 🟡 ok                                                                                                                                                                                                                                                                                                                                                                                              |
| 9   | Card do Kanban                                    | `src/App.tsx:818`                                     | `transform` (`-translate-y-px`)                                                                        | 150ms                | C1     | hover                                            | composite           | 🟢 **o único uso correto de `transform` no app.** Mas `-1px` é subliminar: não vale os 150ms de composição por quadro em 20 cards. Preferir cor.                                                                                                                                                                                                                                                   |
| 10  | Linha da tabela de lista                          | `src/App.tsx:889`                                     | `background-color`                                                                                     | 150ms                | C1     | hover                                            | paint               | ok                                                                                                                                                                                                                                                                                                                                                                                                 |
| 11  | Aba de `SubjectView`                              | `src/App.tsx:1006`                                    | `color`, `border-color` (2px inferior)                                                                 | 150ms                | C1     | troca de aba                                     | paint               | ok                                                                                                                                                                                                                                                                                                                                                                                                 |
| 12  | Botão do `ErrorBoundary`                          | `src/components/ErrorBoundary.tsx:33`                 | `color`, `background-color`                                                                            | 150ms                | C1     | hover                                            | paint               | ok                                                                                                                                                                                                                                                                                                                                                                                                 |
| 13  | `Badge` (todas as variantes)                      | `src/components/ui/badge-variants.ts:4`               | `color`, `background-color`, `border-color`                                                            | 150ms                | C1     | hover                                            | paint               | ok                                                                                                                                                                                                                                                                                                                                                                                                 |
| 14  | `Button` (todas as variantes)                     | `src/components/ui/button-variants.ts:4`              | `color`, `background-color`, `border-color`, `fill`, `stroke`                                          | 150ms                | C1     | hover / focus / disabled                         | paint               | ok                                                                                                                                                                                                                                                                                                                                                                                                 |
| 15  | `Input`                                           | `src/components/ui/input.tsx:11`                      | `color`, `background-color`, `border-color`                                                            | 150ms                | C1     | focus / placeholder                              | paint               | 🟡 **nunca importado.** A decisão de motion é o que decide se entra.                                                                                                                                                                                                                                                                                                                               |
| 16  | `DialogOverlay` (⌘K, busca rápida)                | `src/components/ui/dialog.tsx:24`                     | `opacity` (fade)                                                                                       | **150ms**            | **C2** | abrir/fechar                                     | composite           | 🟠 Correto, mas **fora de sincronia com o conteúdo** (#17) — a camada some antes do diálogo.                                                                                                                                                                                                                                                                                                       |
| 17  | `DialogContent` (⌘K)                              | `src/components/ui/dialog.tsx:43`                     | `opacity` (fade)                                                                                       | **200ms**            | **C2** | abrir/fechar                                     | composite           | 🟠 `duration-200` literal dentro de arquivo do set curado; o overlay (#16) herda 150ms. Duas superfícies do **mesmo** diálogo com durações diferentes.                                                                                                                                                                                                                                             |
| 18  | `DialogPrimitive.Close` (×)                       | `src/components/ui/dialog.tsx:50`                     | `opacity`                                                                                              | 150ms                | C1     | hover                                            | paint               | ok — mas `data-[state=open]:bg-accent` (`:50`) entra sem transição: só `transition-opacity` está declarado.                                                                                                                                                                                                                                                                                        |
| 19  | `TooltipContent`                                  | `src/components/ui/tooltip.tsx:23`                    | `opacity`, `transform` (zoom 0.95→1, slide 8px)                                                        | 150ms                | C2     | hover após `delayDuration={250}` (`App.tsx:181`) | composite           | 🟠 O `animate-in` está **na classe base, sem `data-[state=open]`** (ao contrário do `animate-out`, que tem) — default do shadcn; funciona porque o Radix só monta quando aberto, mas é frágil. `zoom-in-95` é `transform` puro, correto.                                                                                                                                                           |
| 20  | `Dialog` × override do ⌘K                         | `src/App.tsx:379`                                     | `transform` (`translate-y-0`, `top-[14vh]`)                                                            | —                    | —      | —                                                | —                   | 🟠 Sobrescreve `translate-y-[-50%]` de `dialog.tsx:43`. Correto (o ⌘K fica ancorado ao topo), mas depende de `tailwind-merge` acertar `translate-y-0` vs `-translate-y-[-50%]`. Frágil.                                                                                                                                                                                                            |
| 21  | `Checkbox` (estado marcado)                       | `src/components/ui/checkbox.tsx:14`                   | — (`bg` por `data-[state=checked]`, **sem transição**)                                                 | **0**                | **C0** | marcar                                           | —                   | 🔴 **Inconsistência com `Switch`** (#22, #23): o switch anima, o checkbox não. Dois controles com a mesma semântica, dois comportamentos. Nunca importado.                                                                                                                                                                                                                                         |
| 22  | `Switch` — trilho                                 | `src/components/ui/switch.tsx:12`                     | `background-color`, `border-color`                                                                     | 150ms                | C1     | marcar                                           | paint               | 🟡 nunca importado                                                                                                                                                                                                                                                                                                                                                                                 |
| 23  | `Switch` — polegar                                | `src/components/ui/switch.tsx:20`                     | `transform` (`translateX(0)→16px`)                                                                     | 150ms                | C1     | marcar                                           | composite           | 🟢 **o padrão certo**: `transform`, não `left`. Nunca importado.                                                                                                                                                                                                                                                                                                                                   |
| 24  | `TabsTrigger`                                     | `src/components/ui/tabs.tsx:30`                       | **`transition-all`**: `background-color`, `color`, **`box-shadow`**, `outline-color`, `fill`, `stroke` | 150ms                | C1     | troca de aba                                     | paint               | 🟠 `transition-all` é o antipadrão (§4, X2): anima `box-shadow` (`data-[state=active]:shadow`), que é paint puro e caro. Nunca importado.                                                                                                                                                                                                                                                          |
| 25  | `Progress` — indicador                            | `src/components/ui/progress.tsx:18`                   | **`transition-all`** sobre `transform: translateX()` (`progress.tsx:19`)                               | 150ms                | C1     | mudança de `value`                               | composite           | 🟡 `transform` está certo, `transition-all` é o antipadrão. Nunca importado.                                                                                                                                                                                                                                                                                                                       |
| 26  | `ScrollArea` — barra de rolagem                   | `src/components/ui/scroll-area.tsx:32`                | `color`                                                                                                | 150ms                | C1     | hover sobre a barra                              | paint               | 🟡 nunca importado — o app usa `scrollbar-slim` (`styles.css:191-205`), não Radix.                                                                                                                                                                                                                                                                                                                 |
| 27  | `PopoverContent`                                  | `src/components/ui/popover.tsx:22`                    | `opacity`, `transform` (zoom + slide por `data-[side]`)                                                | 150ms                | C2     | abrir/fechar                                     | composite           | 🟡 nunca importado                                                                                                                                                                                                                                                                                                                                                                                 |
| 28  | `SelectContent`                                   | `src/components/ui/select.tsx:71`                     | `opacity`, `transform` (zoom + slide)                                                                  | 150ms                | C2     | abrir/fechar                                     | composite           | 🔴 **Bug real.** `select.tsx:73` aplica `data-[side=bottom]:translate-y-1` **sem gate de `data-[state=open]`**: o conteúdo fica permanentemente deslocado 4px, e o `@keyframes enter` do `tw-animate-css` (`transform: translate3d(...) scale3d(...)`) sobrescreve essa `transform` durante a animação → **salto de 4px na abertura**. Nunca importado, mas precisa ser corrigido antes de entrar. |
| 29  | `DropdownMenuContent`                             | `src/components/ui/dropdown-menu.tsx:67`              | `opacity`, `transform`                                                                                 | 150ms                | C2     | abrir/fechar                                     | composite           | 🟡 nunca importado                                                                                                                                                                                                                                                                                                                                                                                 |
| 30  | `DropdownMenuSubContent`                          | `src/components/ui/dropdown-menu.tsx:49`              | `opacity`, `transform`                                                                                 | 150ms                | C2     | abrir submenu                                    | composite           | 🟡 nunca importado                                                                                                                                                                                                                                                                                                                                                                                 |
| 31  | `DropdownMenuItem` / `CheckboxItem` / `RadioItem` | `src/components/ui/dropdown-menu.tsx:85,101,123`      | `color`, `background-color`                                                                            | 150ms                | C1     | foco do teclado / hover                          | paint               | 🟡 nunca importado                                                                                                                                                                                                                                                                                                                                                                                 |
| 32  | `Skeleton`                                        | `src/components/ui/skeleton.tsx:4`                    | `opacity` (pulso)                                                                                      | **2000ms, infinito** | C4     | enquanto carrega                                 | composite           | 🔴 **Nunca usado, e quebrado sob `prefers-reduced-motion`** — ver §1.6 R1. Também é `bg-primary/10`, um rosa a 10%: skeleton de orquídea, incompatível com `DESIGN.md:17` ("superfícies brancas com bordas sutis").                                                                                                                                                                                |
| 33  | Item da Agenda                                    | `src/features/calendar/ui/CalendarScreen.tsx:541`     | `background-color`                                                                                     | 150ms                | C1     | hover                                            | paint               | 🟢 o único hover animado do Calendário. Faltam o `TimedItem` da Semana (`:448-469`, só `title=`) e o pino do `MonthView` (`:654-663`), que **não têm nenhum**.                                                                                                                                                                                                                                     |
| 34  | Linha do "agora" na grade                         | `src/features/calendar/ui/CalendarScreen.tsx:480-484` | `top` (`style={{ top: nowTop }}`)                                                                      | **0**                | **C0** | a cada 60s (`CalendarScreen.tsx:120-123`)        | layout              | 🟡 Salto seco a cada minuto. Correto hoje (não anima); DM7 item 3 exige que **continue** assim.                                                                                                                                                                                                                                                                                                    |

### 1.3 Transições em propriedades caras

| Classe de custo   | Propriedades                                              | Ocorrências no código                                                                                                                                                                                                            | Risco                                                                                                                                                                                                                                                                                                               |
| ----------------- | --------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **L — layout**    | `width`, `height`, `top`, `left`, `margin`, `padding`     | `App.tsx:192` (`width`, animada) · `CalendarScreen.tsx:480` (`top`, sem transição)                                                                                                                                               | 🔴 `App.tsx:192` é o único **realmente** animado: 200ms × ~12 quadros × reflow de uma árvore que contém a grade. Ver DM4.                                                                                                                                                                                           |
| **P — paint**     | `background-color`, `color`, `border-color`, `box-shadow` | Itens 2–15, 18, 21, 22, 24, 26, 31, 33; `box-shadow` só em #24                                                                                                                                                                   | 🟢 Baixo no desktop, **exceto** onde a lista é longa: `ListView` (`App.tsx:889`) e a Agenda (`CalendarScreen.tsx:535-560`) pintam linha a linha.                                                                                                                                                                    |
| **C — composite** | `transform`, `opacity`                                    | Itens 9, 16, 17, 19, 23, 25, 27–30, 32                                                                                                                                                                                           | 🟢 Todos baratos. **`transform` em `App.tsx:818` promove uma camada por quadro** sem `will-change`; em 20 cards são 20 promoções.                                                                                                                                                                                   |
| **B — backdrop**  | `backdrop-filter`                                         | `styles.css:186-189` (`vibrancy`, usada em `App.tsx:192,320,379`) · `CalendarScreen.tsx:372` (cabeçalho de dias `sticky`) · `CalendarScreen.tsx:406` (eixo de horas `sticky`) · `App.tsx:186` (overlay mobile) · `dialog.tsx:39` | 🔴 **`CalendarScreen.tsx:372` e `:406` são o pior caso do arquivo.** `backdrop-filter` num elemento `position: sticky` **dentro de container rolável** (`:371`) força re-amostragem do backdrop a cada quadro de rolagem, e o conteúdo por trás é uma grade de 7 colunas × 1008px (`grid-scale.ts:42-44`). Ver DM9. |

### 1.4 Inconsistências

| #   | Problema                                                                       | Onde                                                                               | Gravidade           |
| --- | ------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------- | ------------------- |
| I1  | **Quatro curvas convivem** para o mesmo gesto de "mudar de estado"             | C1 (15 lugares), C2 (Radix, 4 arquivos), C3 (`App.tsx:192`), C4 (`skeleton.tsx:4`) | 🟡 Alta             |
| I2  | Overlay e conteúdo do **mesmo** diálogo com durações diferentes (150/200)      | `dialog.tsx:24` vs `dialog.tsx:43`                                                 | 🟡 Média            |
| I3  | `Checkbox` troca de estado instantaneamente; `Switch` troca em 150ms           | `checkbox.tsx:14` vs `switch.tsx:12,20`                                            | 🟡 Média            |
| I4  | `transition-all` em dois componentes (inclui `box-shadow`)                     | `tabs.tsx:30`, `progress.tsx:18`                                                   | 🟡 Média            |
| I5  | `Popover`, `Select`, `DropdownMenu` animam com zoom 0.95; `Dialog` só com fade | `popover.tsx:22`, `select.tsx:71`, `dropdown-menu.tsx:49,67` vs `dialog.tsx:24,43` | 🟡 Média            |
| I6  | `translate-y-0` sobrescrevendo `-translate-y-[-50%]`                           | `App.tsx:379` vs `dialog.tsx:43`                                                   | 🟡 Média            |
| I7  | `translate-y-1` estático (sem `data-[state=open]`) → salto de 4px              | `select.tsx:73`                                                                    | 🔴 Alta (se entrar) |
| I8  | `Skeleton` com pulso infinito de 2s sob `prefers-reduced-motion: reduce`       | `skeleton.tsx:4` + `styles.css:168-176`                                            | 🔴 Alta             |
| I9  | `Tooltip` declara `animate-in` na base, sem gate `data-[state=open]`           | `tooltip.tsx:23`                                                                   | 🟢 Baixa            |
| I10 | `duration-200` literal dentro de arquivo do set curado (viola `AGENTS.md:12`)  | `dialog.tsx:43`                                                                    | 🟡 Média            |

**Sobre I1.** C1 é o default do Tailwind e está em toda parte; C2 é o default do `tw-animate-css` e está em todo `animate-in`. Um `fade-in` de um `Button` e um `fade-in` de um `Popover` **não** têm a mesma curva. C3 e C4 aparecem uma vez cada, sem justificativa. Isso não é inconsistência estética: é o sintoma de que nenhuma curva foi escolhida.

### 1.5 Ausências

Verificado por leitura: nenhum item abaixo existe em `src/`.

| #   | Ausência                                                                                                                                                                                                                                                                                                 | Onde faria diferença                                                                                                                                                |
| --- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A1  | **Nenhum feedback de carregamento.** `CalendarScreen.tsx:113` tem `loaded`, e `:187` faz `finally { setLoaded(true) }` — mas `loaded` só alimenta o predicado de `empty` (`:217-221`). Entre o primeiro render e a resposta do IPC, a tela mostra **a grade vazia**, indistinguível de "não há eventos". | Toda a tela. Um esqueleto com a geometria exata da grade evita o salto de layout e o "vazio falso".                                                                 |
| A2  | **Nenhuma transição de entrada/saída de item.** `WeekView` mapeia `lanes.map(...)` (`:443-471`) direto para `<button>`; `AgendaView` idem (`:536-559`); `MonthView` idem (`:653-664`). Navegar de uma semana para outra **remove e cria** todos os nós.                                                  | Navegação `←`/`→` (`:266`, `:277`) e troca de modo (`:305-330`).                                                                                                    |
| A3  | **Nenhuma animação de rearrange (FLIP).** `columnize` (`grid-scale.ts:180-225`) devolve `column`/`columns` e `CalendarScreen.tsx:459-460` escreve `left`/`width` em `calc(% ± 2px)` **direto**. Quando dois itens se sobrepõem, a largura de todos salta de uma vez.                                     | Drag de um item sobre outro (spec 02 Fase 2.4), mudança de `hourHeight` na densidade (D16). ⚠️ Mas em `columnize` isso é **proibido** — DM7 item 2 e §1.7 R1.       |
| A4  | **Nenhuma transição estado vazio → cheio.** `empty` (`:217-221`) liga/desliga `<EmptyState>` (`:332`, `:827-841`) com montagem direta.                                                                                                                                                                   | Fim da carga IPC, criação de item.                                                                                                                                  |
| A5  | **Nenhum feedback no toggle de modo de visualização.** `CalendarScreen.tsx:288-299` só troca `variant` de `Button` (`ghost` → `secondary`); o corpo troca por cinco `&&` condicionais (`:305-330`) — **remonta a subárvore inteira**.                                                                    | Todo clique em Agenda/Semana/Mês/Lista/Planejamento. O `selected` e o `ItemPanel` (`:335`) sobrevivem; a grade não.                                                 |
| A6  | **Nenhum feedback de "salvo".** Nenhum estado `saving`/`saved` em `CalendarScreen.tsx`; `setItems` (`:169`) substitui o array inteiro sem sinal.                                                                                                                                                         | Todo `#[tauri::command]` de escrita.                                                                                                                                |
| A7  | **Nenhuma transição de expand/collapse.** `ItemPanel` (`:784-825`) monta/desmonta (`:335`); as três colunas de D20 nem existem. `SubjectView` (`App.tsx:1015-1086`) troca o painel de aba por render condicional.                                                                                        | Painel contextual (D18/D20), `Sheet` no `max-lg` (D18), abas de `SubjectView`.                                                                                      |
| A8  | **Nenhuma animação de drag.** `ui/hooks/use-grid-drag.ts` **não existe** (spec 02 Fase 2.4). `origin === "google"` precisa de feedback **antes** do gesto (D17) e hoje não há cursor nem estado.                                                                                                         | Toda a arraste/redimensionamento.                                                                                                                                   |
| A9  | **Nenhum feedback de erro inline.** `CalendarScreen.tsx:183-186` engole o erro em `catch {}` com o comentário _"Falha de leitura não pode derrubar a tela"_ — e não mostra **nada**. O `ErrorBoundary` (`ErrorBoundary.tsx:21-41`) só cobre crash de render, não falha de IPC.                           | Falha de `list_events_in_window` / `move_occurrence` / `reschedule_block`. D17 já prometeu: _"se o comando falhar, o item volta e a grade mostra um aviso inline"_. |
| A10 | **Nenhum hover no `TimedItem` da Semana** (`:448-469`, só `title=`) **nem no pino do `MonthView`** (`:654-663`). O `AgendaView` tem (`:541`).                                                                                                                                                            | Duas das cinco leituras do Calendário.                                                                                                                              |
| A11 | **Nenhum feedback ao concluir.** `ItemPanel.tsx:813` tem um `Button` "Concluir" **sem handler**; `App.tsx:675-703` só risca o texto. Não há `ExecutionRecord`, não há transição.                                                                                                                         | INV-3: concluir exige `ExecutionRecord`. O motion tem de ser **consequência**, não o sinal.                                                                         |
| A12 | **Nenhuma sinalização de sobreposição (D7).** `columnize` distribui em colunas e pronto — não há `overlapped` visual nem no `WeekView`, nem no painel. A spec 02 Fase 2.3 diz que `TimedItem` recebe `overlapped`; hoje não recebe.                                                                      | Sinalização de conflito derivada (§1.7 R1).                                                                                                                         |
| A13 | **Nenhum comportamento definido para o foco durante transição.** Todos os `focus-visible:ring-*` existem (`App.tsx:217,408,453,675,752`; `button-variants.ts:4`), mas não há regra para onde o foco vai quando o elemento **some** durante uma animação.                                                 | Fechar o ⌘K, fechar o `ItemPanel`, trocar de aba com o teclado.                                                                                                     |
| A14 | **Espaçamento sem hierarquia de movimento.** O `h3` de `App.tsx:582` tem `mt-6` dentro de cards de altura variável (`:566-585`): o título flutua 24px do topo do card sem relação com nada.                                                                                                              | `TodayView`. Não é animação, mas é o mesmo sintoma.                                                                                                                 |

### 1.6 Acessibilidade de motion

O bloco existe em `src/styles.css:168-176`:

```css
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    scroll-behavior: auto !important;
    animation-duration: 0.01ms !important;
    transition-duration: 0.01ms !important;
  }
}
```

| #   | Achado                                                                                                                                                                                                                                                                                                                                                                    | Gravidade              |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------- |
| R1  | **Falta `animation-iteration-count: 1 !important`.** O padrão consagrado para `prefers-reduced-motion` inclui essa linha. Sem ela, `animate-pulse` (`skeleton.tsx:4`, `infinite`, 2s) vira **pulso de 0,01ms em laço infinito** — strobe. O componente nunca foi importado, o defeito está latente.                                                                       | 🔴 Alta                |
| R2  | **Não cobre `prefers-reduced-transparency`.** A utility `vibrancy` (`styles.css:186-189`) aplica `backdrop-filter: saturate(180%) blur(20px)` em três lugares (`App.tsx:192,320,379`). A preferência existe no Windows 10/11 em _Configurações › Personalização › Cores › Efeitos de transparência_ e o app a ignora.                                                     | 🟡 Média               |
| R3  | **Não cobre `prefers-contrast: more`.** `DESIGN.md:17` e `PRODUCT.md:33` exigem contraste; a camada `tcc` já teve que abandonar `text-chart-4` por isso (spec 00 D4). Não há estratégia alternativa de sinalização por cor.                                                                                                                                               | 🟡 Média               |
| R4  | **`transition-duration: 0.01ms !important` não zera o `transition-delay`.** Nenhum elemento usa `transition-delay` hoje, mas `delayDuration={250}` do `TooltipProvider` (`App.tsx:181`) é um **atraso de JS**, não de CSS — o tooltip **continua atrasando 250ms** sob `reduce`. Correto (o atraso evita flicker), mas é uma decisão implícita que precisa ser explícita. | 🟢 Baixa               |
| R5  | **Só CSS.** Não há `matchMedia` em `src/` (verificado). O dia em que entrar JS — WAAPI, `startViewTransition`, drag — o reset de CSS **não alcança**.                                                                                                                                                                                                                     | 🔴 Alta (prospectiva)  |
| R6  | **Foco e eventos de animação.** A regra global corta a transição, mas `transitionend`/`animationend` continuam disparando em 0,01ms. Qualquer código que dependa de `onAnimationEnd` para desmontar (padrão do Radix `Presence`) roda **uma vez** — ok. Código que conte iterações quebra.                                                                                | 🟡 Média (prospectiva) |
| R7  | **Acessibilidade do shimmer, quando ele existir.** A prática corrente (Adrian Roselli, atualizado 2026-04-14; e o guia MFA11y) é: o shimmer **vive dentro de `@media (prefers-reduced-motion: no-preference)`** e o `reduce` recebe **preenchimento estático**, não nada. `aria-busy` sozinho não é anunciado por quase nenhum leitor de tela.                            | 🟡 Média (prospectiva) |

### 1.7 Regra de ouro do domínio × motion

`AGENTS.md:23-25` e `calendario.md` §13. Três regras que **proíbem** animação.

**R1 — Conflito e sobreposição são derivados e só sinalizados, nunca bloqueantes.**
`grid-scale.ts:157-165` já é explícito: _"Sobreposição é permitida e derivada (D7) — isto é apenas legibilidade, nunca bloqueio."_ Se a redistribuição de largura feita por `columnize` for **animada**, a usuária vê os itens se reorganizando sozinhos e lê aquilo como _"o sistema resolveu um conflito por mim"_. Isso é exatamente a obrigação silenciosa que §13 proíbe. **A largura de um item na Semana nunca interpola** (DM7 item 2).

**R2 — `recomendado`/`opcional` nunca viram obrigação (INV-7, D8).**
`commitment.ts:39` — `DELAYABLE = {obrigatorio, importante}`. A tabela de §3 diz que Recomendado e Opcional "podem gerar atraso? **Não**". Consequência de motion: qualquer animação que **promova** visualmente — _pulse_, _brilho_, _contorno pulsante_, _subida de escala_, _"chamada"_ — transforma uma sugestão em algo que a interface trata como prioritário. E a spec 06 depende inteiramente disso: as três ações (Aceitar/Ajustar/Recusar, D8) precisam ter **peso visual idêntico**. Uma sugestão que pulsa convida ao clique, e clicar nela é a única forma de ela virar obrigação — ou seja, a animação estaria fabricando o consentimento.

**R3 — Três perguntas separadas, três regiões de animação separadas.**
Um item que responde a "o que precisa ser feito?" (responsabilidade — `LISTA`/`PLANEJAMENTO`, `CalendarScreen.tsx:680-769`) **não pode** compartilhar a animação de um item que responde a "o que está marcado?" (ocorrência — `SEMANA`/`AGENDA`/`MÊS`). Se um card de atraso pulsa e o mesmo card listado não pulsa, a interface está misturando as duas perguntas. **O mesmo componente em dois modos anima igual.**

---

## 2. Decisões fechadas

### 2.1 Tecnologia

**DM1 — Motion em CSS puro e WAAPI; nenhuma biblioteca de animação.**

Decisão: o sistema de motion é **CSS** (tokens em `@theme` do Tailwind v4, classes utilitárias, `@starting-style` + `transition-behavior: allow-discrete`) com **WAAPI** (`element.animate()`) nos três pontos em que CSS não chega: o rearranjo da grade, a linha do "agora" e o arrasto. **`motion`/`framer-motion` não é adicionada. `@formkit/auto-animate` não é adicionada. `document.startViewTransition` não é usada para troca de modo.** `package.json` fica intocado.

Justificativa, opção a opção:

- **`motion` (sucessor do framer-motion).** A própria documentação admite que _"Because of its declarative, props-driven API, it's impossible for bundlers to tree shake it any smaller than **34kb**"_, e que com `LazyMotion` + `m` cai para 4,6kb no render inicial ([motion.dev/docs/react-reduce-bundle-size](https://motion.dev/docs/react-reduce-bundle-size)). O pacote está em `12.42.x` ([npm](https://www.npmjs.com/package/framer-motion)) e suporta React 19 — a ausência de bloqueio técnico é real. O problema é o **custo por quadro**: `layout` mede `getBoundingClientRect` de cada nó marcado **a cada commit de render** e escreve `transform` nele. A `WeekView` renderiza 7 colunas × 18 linhas de hora (`CalendarScreen.tsx:435-441`) + N itens + dois `sticky` com `backdrop-blur` (`:372`, `:406`). Um FLIP automático em cima disso é **pior** que o rearranjo seco que ele resolve. Pagar 34kB (ou 4,6kB + uma segunda camada de abstração) para instrumentar exatamente o componente que mais precisa de controle manual é o trade-off invertido.
- **`@formkit/auto-animate`.** Zero configuração, 0 dependências, ~5kB. Três fatos o reprovam aqui: (a) _"Animations are only triggered when immediate children of the parent element are added, removed, or moved"_ ([auto-animate.formkit.com](https://auto-animate.formkit.com/)) — a `WeekView` tem como filhos imediatos 7 `<div>` de dia, e é **dentro** de cada um que os itens se movem; (b) _"The parent element will automatically receive `position: relative`"_ — intrusive num grid que já é `relative` com `sticky`; (c) no fonte, `if (isOffscreen(el)) { … return }` — pula o FLIP de elementos fora da tela. Esse último ponto é **justamente o caso errado**: na grade, o rearrange que importa é o dos dias parcialmente visíveis, que é quase sempre offscreen por bounding box.
- **`document.startViewTransition` / View Transitions.** `startViewTransition` é Baseline desde outubro de 2025 ([MDN](https://developer.mozilla.org/en-US/docs/Web/API/Document/startViewTransition)), o que inclui o WebView2 evergreen do Tauri v2 em Windows. **Mas:** (a) ele captura o estado antigo e o novo como **imagens** e anima pseudo-elementos (`::view-transition-old/new`) — o custo é proporcional ao número de nós e à área, e a `WeekView` tem ~130 nós com `backdrop-filter`; (b) o comportamento de **scroll não é controlável** — a troca de modo rola até o topo, o que é errado para quem está comparando com a semana anterior; (c) é **all-or-nothing**: não dá para animar o painel contextual junto com a grade, e o painel **precisa** se manter (D18); (d) o modo cross-document é irrelevante — não há roteador (`AGENTS.md:9`). **Rejeitado** para troca de modo; DM7 mantém a porta aberta para o caso em que a troca de modo vire "uma imagem entra no lugar de outra".
- **`@starting-style` + `transition-behavior: allow-discrete`.** Baseline desde agosto de 2024: Chrome/Edge 117+, Firefox 129+, Safari 17.4+ ([MDN `@starting-style`](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@starting-style), [MDN `transition-behavior`](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/transition-behavior), [caniuse](https://caniuse.com/mdn-css_properties_transition-behavior_allow-discrete)). É a via **correta** para A2 e A7: entrada e saída de item em DOM, sem JS, sem `Presence`, sem listener. Adotado, com `@supports not (transition-behavior: allow-discrete)` degradando para aparecimento instantâneo — a ausência de suporte **nunca** pode custar a visibilidade do elemento.
- **WAAPI.** `Element.animate()` é universal no WebView2, roda na compositor para `transform`/`opacity`, não exige dependência, e dá `fill`, `composite` e `KeyframeEffectOptions.easing` — tudo que o FLIP manual precisa. É também o **único** caminho para o arrasto, porque A4 da spec 00 já decidiu "sem biblioteca" e D17 exige controle por quadro.

Alternativa rejeitada: `react-spring` / `gsap` — mesma família de custo que `motion`, sem a parte declarativa que compensa.

Consequência: `package.json` **sem alteração**. A Fase M1 (§5) adiciona ~60 linhas de CSS em `src/styles.css` e **zero** bytes de JavaScript de terceiros.

---

### 2.2 Escala e curvas

**DM2 — Escala fechada de duração, cinco degraus, em `@theme`.**

Decisão (especificado, não aplicado):

```css
@theme inline {
  /* ── Motion ─────────────────────────────────────────────────────────
     Regra: mudanca de estado vive em 150 ou 200 (DESIGN.md:25).
     90 e abaixo sao resposta a gesto, nao mudanca de estado.
     Acima de 200 so rearranjo de layout e superficie em tela cheia,
     e cada uso passa por revisao.                                  */
  --duration-micro: 90ms; /* resposta a gesto: hover pressionado, foco entrando */
  --duration-small: 150ms; /* PADRAO. toda mudanca de estado no lugar */
  --duration-medium: 200ms; /* PADRAO de entrada/saida de superficie e de layout local */
  --duration-large: 280ms; /* rearranjo/FLIP e painel que ocupa uma coluna */
  --duration-xlarge: 400ms; /* RESERVADO: uma superficie cobrindo a janela. 1 consumidor */
}
```

| Token               | Valor | Quando é **permitido**                                                                                                                                                                        | Quando é **proibido**                                                                    | Base                                                                                                                                                                                                                                                                          |
| ------------------- | ----- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `--duration-micro`  | 90ms  | (1) estado de `:active`/`:focus-visible`; (2) realce que acompanha o ponteiro; (3) anel de foco entrando                                                                                      | qualquer coisa visível por mais de um quadro sem intenção declarada                      | Abaixo de 150ms porque **não é mudança de estado** — é latência sentida. `DESIGN.md:25` fixa o piso de 150ms para mudanças de estado, não para resposta a gesto.                                                                                                              |
| `--duration-small`  | 150ms | **Padrão de mudança de estado**: cor, borda, opacidade, fundo, sombra, realce. Hover, foco, toggle de `Switch`/`Checkbox`, chip, badge, aba selecionada, item selecionado, barra de progresso | deslocamento espacial de elemento; entrada/saída de superfície                           | Tailwind v4 `--default-transition-duration: 150ms` (`theme.css:492`); Material **desktop**, "150ms to 200ms" ([Material 2 motion/speed](https://m2.material.io/design/motion/speed.html))                                                                                     |
| `--duration-medium` | 200ms | **Padrão de entrada/saída**: `Dialog`, `Popover`, `Select`, `DropdownMenu`, `Tooltip`; cross-fade de modo; deslocamento de coluna de painel                                                   | rearrange que cruza mais de meia tela                                                    | `DESIGN.md:25` (teto); Material 3 "standard accelerate 200ms" ([m3 easing-and-duration](https://m3.material.io/styles/motion/easing-and-duration)); já é o valor em `dialog.tsx:43`                                                                                           |
| `--duration-large`  | 280ms | **Só** rearranjo/FLIP de lista ou grade; abertura do painel contextual de 280/360px (D18); mudança de `hourHeight` na densidade (D16)                                                         | mudança de cor, hover, foco, estado                                                      | Material 3 `md.sys.motion.duration.medium1 = 250ms`, arredondado ([tokens](https://web.archive.org/web/20251225122044/https://m3.material.io/styles/motion/easing-and-duration/tokens-specs)) — não é mudança de estado, é **mudança de layout**, que `DESIGN.md:25` não rege |
| `--duration-xlarge` | 400ms | **Um único consumidor**: o `Sheet` em `max-lg` (D18, spec 02 Fase 2.2). Mais nada, jamais                                                                                                     | qualquer coisa dentro de um painel; qualquer coisa que a usuária possa reagir cancelando | Material 3 `medium4 = 400ms`, o caso "FAB into a full-screen dialog"                                                                                                                                                                                                          |

Justificativa da **forma** da escala: ela é fechada e cresce com **distância percorrida e área atravessada**, não com o nome do componente. É a regra que os guias de maior convergem: Material 2 diz literalmente _"Transitions that cover small areas of the screen have shorter durations than those that traverse larger areas"_ e que um switch leva 100ms enquanto uma bottom sheet leva 250ms. O `--duration-micro` de 90ms existe pelo mesmo motivo: um controle de 32px (`DESIGN.md:21`) que atravessa 4px não é o mesmo objeto que um painel que atravessa 300px.

Alternativa rejeitada: a escala do Material 3 verbatim (250/300/350/400/450ms). É **incompatível com `DESIGN.md:25`**, que é regra local e mais estrita. Alternativa rejeitada: manter só 150 e 200. É mais simples, mas deixa o rearranjo e o `Sheet` sem degrau — e é a ausência de degrau que produz I1 (quatro curvas improvisadas).

---

**DM3 — Quatro curvas, cada uma com um tipo de movimento; a curva é escolhida pelo tipo, nunca pelo gosto.**

```css
@theme inline {
  /* ── Curvas ─────────────────────────────────────────────────────────
     standard : mudanca de estado no lugar (cor, borda, opacidade)
     entrance : o que entra (decelera)
     exit     : o que sai (acelera)
     spatial  : o que muda de lugar (monotona, sem overshoot)
     linear   : varredura e progresso (honestidade, nao estilizacao) */
  --ease-standard: cubic-bezier(0.4, 0, 0.2, 1); /* = C1 */
  --ease-entrance: cubic-bezier(0, 0, 0.2, 1); /* = C3 */
  --ease-exit: cubic-bezier(0.4, 0, 1, 1);
  --ease-spatial: cubic-bezier(0.2, 0, 0, 1);
  --ease-linear: linear;
}
```

| Curva             | Valor                          | Tipo de movimento               | Quem usa hoje (vira)                                                                 | Base                                                                                                                                                                                                                                                                 |
| ----------------- | ------------------------------ | ------------------------------- | ------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `--ease-standard` | `cubic-bezier(0.4, 0, 0.2, 1)` | estado no lugar                 | 15 itens do inventário — **a maioria já está certa**                                 | É literalmente o default do Tailwind v4 (`theme.css:493`) e a "standard curve" do Material ([m1.material.io](https://m1.material.io/motion/duration-easing.html): `cubic-bezier(0.4, 0.0, 0.6, 1)`). Manter o que já domina o app é o menor diff e a menor surpresa. |
| `--ease-entrance` | `cubic-bezier(0, 0, 0.2, 1)`   | entrada                         | Radix `Dialog`/`Tooltip`/`Popover`/`Select`/`Menu` — hoje em `ease` (C2, **errada**) | "deceleration curve" do Material; Tailwind `--ease-out` (`theme.css:435`); M3 "standard decelerate" (250ms) usa a mesma família                                                                                                                                      |
| `--ease-exit`     | `cubic-bezier(0.4, 0, 1, 1)`   | saída                           | os mesmos cinco, no `animate-out` — hoje em `ease` (C2, **errada**)                  | "acceleration curve" do Material; Tailwind `--ease-in` (`theme.css:434`); M3 "standard accelerate" (200ms)                                                                                                                                                           |
| `--ease-spatial`  | `cubic-bezier(0.2, 0, 0, 1)`   | rearranjo/FLIP, painel, `Sheet` | **ninguém** — não existe FLIP hoje                                                   | M3 "emphasized", escolhida pela **unicidade**: a curva espacial do M3 `0.27, 1.06, 0.18, 1.00` tem _overshoot_, e overshoot numa grade com snap de 15min (`grid-scale.ts:89-97`) mostraria `14:52` antes de `15:00`. Rejeitada por D15.                              |
| `--ease-linear`   | `linear`                       | progresso e varredura           | `--` (o `animate-pulse` de `skeleton.tsx:4` usa C4, fora de ordem)                   | Uma barra de progresso com `ease-out` mente sobre a taxa. Linear é a única curva honesta.                                                                                                                                                                            |

Regras de aplicação (verificáveis):

- Transição que muda **só** cor/borda/opacidade → `--duration-small` + `--ease-standard`. Ponto.
- **Entrada** → `--duration-medium` + `--ease-entrance`. **Saída** → `--duration-small` + `--ease-exit`. A saída é **mais curta** que a entrada: Material 3 usa 400/250ms para entrar e 200ms para sair, e `DESIGN.md:25` dá o mesmo teto para os dois, então a assimetria vem da **duração**, não da curva.
- **Nunca** misturar: um elemento que entra em `--duration-medium` não sai em `--duration-medium`.
- `--ease-linear` é proibido fora de progresso e da linha do "agora".

**DM3b — Um override único corrige o Radix inteiro sem editar quatro arquivos.** O `tw-animate-css` lê `var(--tw-duration, .15s)` e `var(--tw-ease, ease)` no `@keyframes enter`/`exit` (`tw-animate.css:1`). Declarando no `:root`:

```css
:root {
  --tw-duration: var(--duration-medium);
  --tw-ease: var(--ease-entrance);
}
```

todo `animate-in`/`animate-out` do set curado passa a usar 200ms + curva de entrada **sem tocar em `popover.tsx`, `select.tsx`, `dropdown-menu.tsx`, `tooltip.tsx` ou `dialog.tsx`** — e sem violar `AGENTS.md:12` (que proíbe editar o set curado à mão). As utilities `duration-*` do Tailwind escrevem `--tw-duration` **no elemento**, por isso o `duration-200` literal de `dialog.tsx:43` continua prevalecendo ali e precisa ser removido (I10) para o valor passar a ser o token.

---

**DM4 — Só `transform` e `opacity` animam posição e tamanho. Exceções escritas.**

Decisão: nenhuma transição de `width`, `height`, `top`, `left`, `margin`, `padding`, `right` ou `bottom` em elemento cujo **tamanho ou posição no fluxo** depende do valor animado. Só `transform` (`translate`/`scale`/`rotate`) e `opacity`.

**Exceção 1 — `App.tsx:192`, a barra lateral.** É a única transição de layout viva. Custo: 200ms × ~12 quadros × **reflow do `<main>` inteiro**, que na visão Calendário contém a grade de 7 colunas (`CalendarScreen.tsx:402`) com dois `sticky` + `backdrop-blur` (`:372`, `:406`). Duas saídas, nesta ordem de preferência:

1. **Animar `transform: translateX(-180px)`** com o conteúdo interno em `width: 248px` fixo (já é — `App.tsx:196`) e o estado "rail" de 68px (`:193`) num elemento separado. `translate` é composite; o layout é calculado **uma vez**.
2. Se (1) custar mais em código do que o reflow custa em quadros: **a sidebar colapsa sem animação**, com corte seco. É uma decisão legítima e preferível a 12 quadros de reflow da grade.

A medição que escolhe entre (1) e (2) está na Fase M2.

**Exceção 2 — `CalendarScreen.tsx:480`, a linha do "agora".** `style={{ top: nowTop }}` muda a cada minuto (`CalendarScreen.tsx:120-123`). Hoje é um salto (correto). **Continua um salto.** Ver DM7 item 3.

Consequência: `rg "transition-\[(width|height|top|left|margin|padding)" src` → **vazio**, e vira critério de aceite (§6).

---

**DM5 — Política de FLIP: dois passos, WAAPI, só nós cujo rect mudou, e nunca durante o arrasto.**

Decisão: o rearranjo é implementado à mão, com WAAPI, em três passos:

1. **Antes** do commit de estado, medir `getBoundingClientRect()` dos candidatos e guardar num `Map` estável.
2. Renderizar.
3. **Depois**, para cada nó cujo rect mudou e cujo delta exceda 1px: `el.animate([{ transform: translate(dx, dy) }, { transform: "none" }], { duration: 280, easing: <--ease-spatial>, fill: "none" })`.

Regras:

- Só quando `matchMedia("(prefers-reduced-motion: no-preference)").matches` **e** a mudança veio de **dados** (nova carga IPC, mudança de dia, mudança de densidade).
- **Nunca** durante um arraste em curso. Arraste e rearrange são mutuamente exclusivos: enquanto o ponteiro está na grade, o único rearranjo permitido é o do **ghost**, e ele é 1:1 com o ponteiro, sem curva.
- **Nunca** em `columnize` (§1.7 R1): largura/coluna de item na Semana é DM7 item 2.
- **Sempre** cancelável — a nova animação de FLIP cancela a anterior do mesmo nó. Uma transição interrompida não pode deixar o item a meio caminho.

Alternativa rejeitada: `layout`/`LayoutGroup` do `motion` (DM1). Alternativa rejeitada: `Element.animate` sem medir antes — não é FLIP, é animação de entrada, e produz o salto que a medição evita.

---

**DM6 — `prefers-reduced-motion` em três níveis, com o que cada um preserva.**

Decisão: o bloco `styles.css:168-176` é reescrito. A regra não é "desligar tudo": desligar um cross-fade entre modos troca uma transição por um **flash**, que é pior para a mesma pessoa que pediu menos movimento. São três níveis:

| Nível            | O que                                                                                                                                                                       | O que acontece                                                                                                                           | Implementação                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| ---------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **1 — desliga**  | Movimento repetido ou espacial: shimmer, `animate-pulse`, spinner, `hover:-translate-y-px` (`App.tsx:818`), `zoom-in-95`, todos os `slide-in-from-*`, `scale` de item       | **Substituir por estado estático**, não por nada. Skeleton vira `bg-muted` sem animação. `hover:-translate-y-px` vira só mudança de cor. | O keyframe vive **dentro** de `@media (prefers-reduced-motion: no-preference)`. É o padrão que a prática corrente recomenda: _"the shimmer animation lives only inside a no-preference query, so reduced-motion users get the static skeleton automatically"_ ([MFA11y](https://www.modern-framework-accessibility.com/core-accessibility-principles-for-modern-frameworks/reduced-motion-and-animation-accessibility/accessible-loading-skeletons-and-spinners); [camoa](https://camoa.github.io/dev-guides/css/css-craft/skeleton-and-loading-states) idem). `animation: none` no `reduce` para o que sobrar. |
| **2 — depara**   | Movimento espacial que carrega informação de **onde** algo veio                                                                                                             | Some o deslocamento, fica `opacity` de `--duration-micro` (90ms). Tooltip/popover "aparecem" em vez de "deslizar de 8px".                | `transition: opacity 90ms linear` no `reduce`; `transform` fora do `transition-property`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| **3 — preserva** | (a) cor de hover/foco — **cor não é movimento**; (b) cross-fade entre modos — sem ele vira flash; (c) anel de foco — é o único indicador de posição do teclado (WCAG 2.4.7) | intactos                                                                                                                                 | nada a fazer; apenas **não** colocá-los nos níveis 1/2.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |

Correções obrigatórias no bloco atual:

```css
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    scroll-behavior: auto !important;
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important; /* R1: sem isto animate-pulse vira strobe */
    transition-duration: 0.01ms !important;
  }
}

@media (prefers-reduced-transparency: reduce) {
  /* R2 */
  .vibrancy {
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
  }
  /* substituir por cor opaca derivada — ver DM9 */
}

@media (prefers-contrast: more) {
  /* R3 */
  /* ver §3, interação 11 */
}
```

E, porque DM1 introduz JS (WAAPI, drag), **um hook próprio** `src/hooks/use-reduced-motion.ts` sobre `matchMedia("(prefers-reduced-motion: reduce)")` com listener de `change` — o mesmo que o `useReducedMotion` do `motion` faria, sem a dependência ([motion.dev/docs/react-use-reduced-motion](https://motion.dev/docs/react-use-reduced-motion); e a armadilha está documentada em [web.dev](https://web.dev/articles/prefers-reduced-motion): _"Animating in JavaScript with no matchMedia check, so CSS overrides do nothing"_). Ele é lido por DM5 e pela Fase M4 (drag).

---

**DM7 — O que nunca anima. Lista fechada.**

Nove proibições. Cada uma é verificável por `rg` ou por observação, e cada uma existe por uma regra do domínio ou de design já escrita.

1. **Nada que responda a uma pergunta de domínio cuja animação possa ser lida como afirmação.** Estado (`concluido`), atraso, conflito, nível de compromisso. Um item que muda de estado **troca de cor e de rótulo**; a cor interpola em `--duration-small`/`--ease-standard`, e **nada mais**. Proibidos: _pop_, _stamp_, _checkmark que se desenha_, _pulse de atraso_, _contorno pulsante_, _ring_ de urgência. (§1.7 R2, INV-4, INV-7)
2. **A largura e a coluna de um item na Semana nunca interpolam.** `columnize` reposiciona instantaneamente. A redistribuição animada de sobreposição pareceria o sistema resolvendo um conflito. (§1.7 R1, D7, `grid-scale.ts:157-165`)
3. **A linha do "agora" nunca interpola entre minutos.** No máximo um fade de `--duration-medium` quando ela **aparece** na faixa visível; o reposicionamento a cada 60s é um corte.
4. **Nada de zoom, pan ou rotação de item da grade.** Um item de 56px com `zoom-in-95` não comunica nada que a cor não comunique.
5. **Nenhuma animação de montagem do app ou de tela.** `DESIGN.md:25` — _"Sem animações de entrada decorativas"_. O `ErrorBoundary` (`ErrorBoundary.tsx:22`) e o primeiro render de `TodayView` entram sem fade. Atenção ao `StrictMode` (`main.tsx:12`), que monta duas vezes em dev: qualquer stagger apareceria duplicado.
6. **Nada que atrase o primeiro dado.** Enquanto `loaded === false`, a grade **não entra com stagger** e não há cascata por dia. A grade entra inteira e de uma vez, ou não entra.
7. **Nada que fabrique consentimento.** `Aceitar` de sugestão (D8) tem **peso visual idêntico** a `Recusar` e a qualquer outro botão. Sugestão que pulsa convida ao clique; e clicar é a única forma de virar obrigação — a animação estaria fabricando o consentimento de INV-7. (§1.7 R2, spec 06)
8. **Nada de toast que entra deslizando de fora da tela.** Confirmação de gravação é um estado **na posição final**, com fade de `--duration-small`. Um toast que entra de fora transforma uma confirmação local num evento global.
9. **Nada que anime a cor de camada (D4).** O tom de D4 (`layers.ts:36-43`) não brilha, não pulsa, não muda no hover. Item da camada `google` (neutro, `border-dashed`, `layers.ts:42`) não ganha movimento. Foco = anel, não salto.

---

**DM8 — O set curado não é editado à mão; a folha de estilo é o único lugar de decisão.**

Decisão: `AGENTS.md:12` manda adicionar componentes com `npx shadcn@latest add <nome>`. Isso significa que **qualquer ajuste de motion dentro de `src/components/ui/` é perdido na próxima atualização do componente**. Logo:

- Nenhuma classe de duração/curva nova é escrita em `src/components/ui/*.tsx`.
- A correção do I7 (`select.tsx:73`) é a **única** alteração de template admissível, e ela é um **patch de bug upstream** (translate estático sem gate de estado), não uma decisão de motion. Registrar como dívida: o próximo `shadcn add` traz o bug de volta.
- A correção do I2 (overlay 150 vs conteúdo 200) é resolvida por DM3b, **sem tocar em `dialog.tsx`**.
- Se um componente do set curado precisar de comportamento que o token não expressa, ele vai para um **wrapper** em `src/features/*/ui/`, não para o template.
- `src/styles.css` é o **único** lugar onde se decide duração e curva.

---

**DM9 — `backdrop-filter` é proibido dentro de container rolável.**

Decisão: `backdrop-filter` só onde **não há rolagem por trás** no quadro seguinte: a sidebar (`App.tsx:192`), o header do app (`App.tsx:320`), o conteúdo do ⌘K (`App.tsx:379`) e o overlay do diálogo (`dialog.tsx:39`). **Proibido** em elemento `sticky`/`fixed` dentro de um container com `overflow: auto`.

Justificativa: `CalendarScreen.tsx:372` (cabeçalho de dias, `sticky top-0` dentro de `:371` `overflow-auto`) e `:406` (eixo de horas, `sticky left-0` dentro do mesmo container) aplicam `backdrop-blur`. Cada quadro de rolagem força o WebView2 a re-amostrar o conteúdo por trás, e o conteúdo por trás é uma grade de 7 colunas × 1008px com bordas de 1px — o pior padrão possível para um filtro de borrão. Substituição: **cor opaca derivada**, `background: color-mix(in oklab, var(--popover) 98%, var(--foreground))`. A 52px de cabeçalho e 52px de eixo, a diferença visual é imperceptível e o custo por quadro vai a zero.

Consequência: `rg "backdrop-blur|backdrop-filter" src/features` → **vazio**.

---

**DM10 — Nenhuma animação bloqueia, atrasa ou retarda o input. Interromper é sempre mais barato do que esperar.**

Decisão: toda animação de entrada é **cancelável pelo gesto seguinte** e **nunca** atrasa a disponibilidade do dado.

- Trocar de modo de visualização (`CalendarScreen.tsx:288-299`) hoje remonta a subárvore inteira (`:305-330`, cinco `&&`). A Fase M3 troca por um cross-fade de `--duration-medium` na **região que muda**, sem remontar a grade. Critério: o `TimedItem` selecionado e o `ItemPanel` (`:335`) **não desmontam** durante o cross-fade.
- Clicar `←`/`→` durante o cross-fade **cancela** o cross-fade; não forma fila. Navegar é sempre mais urgente do que assistir.
- `prefers-reduced-motion: reduce` → a troca de modo mantém o cross-fade (DM6 nível 3), porque sem ele a troca vira flash; mas **sem deslocamento espacial**.
- Nunca há atraso de input por animação: um botão fica habilitado no instante do clique, mesmo que sua animação de "pressionado" ainda esteja rodando.

---

**DM11 — Feedback de gravação é estado, não espetáculo.**

Decisão: após um `#[tauri::command]` de escrita, o feedback é (a) a **própria mudança de estado** do item (badge, `line-through`, tom) e (b) um indicador `Salvando` / `Salvo` **na posição final**, com fade de `--duration-small` e desaparecimento após 2s. **Sem** toast deslizante (DM7 item 8), **sem** pulso no item (DM7 item 1), **sem** o item "pular" na grade.

Justificativa: A6 é onde esse indicador nasce, e o item que acabou de ser salvo **não pode pular na grade** — um salto de 200ms empurra os vizinhos visualmente durante a leitura da semana. `AGENTS.md:23` exige que a grade responda "o que está marcado"; a grade não pode ser tekken.

---

**DM12 — Conflito e sobreposição têm sinal **estático**, nunca animado.**

Decisão: a sinalização de D7 (sobreposição derivada, "apenas legibilidade") é um **contorno tracejado de 1px** e um **badge textual** "sobreposto", ambos com transição de cor de `--duration-small`. **Nenhum** `box-shadow` animado, nenhuma pulsação, nenhuma cor que "pisca" para chamar atenção. A cor de `--destructive` nunca anima: vermelho pulsando é o oposto de "só sinalizado, nunca bloqueante" (§1.7 R1).

---

### 2.3 Unindo: o bloco CSS

```css
/* src/styles.css — a ser aplicado na Fase M1 (§5). NÃO IMPLEMENTADO. */

@theme inline {
  /* … os tokens existentes de cor e raio (styles.css:22-63) … */

  /* Motion — duração */
  --duration-micro: 90ms;
  --duration-small: 150ms;
  --duration-medium: 200ms;
  --duration-large: 280ms;
  --duration-xlarge: 400ms;

  /* Motion — curva por tipo de movimento */
  --ease-standard: cubic-bezier(0.4, 0, 0.2, 1);
  --ease-entrance: cubic-bezier(0, 0, 0.2, 1);
  --ease-exit: cubic-bezier(0.4, 0, 1, 1);
  --ease-spatial: cubic-bezier(0.2, 0, 0, 1);
  --ease-linear: linear;

  /* Motion — keyframes compostos. Cada um é uma única entrada/saída de
     superfície; nunca anima width/height/top/left. */
  --animate-surface-in: surface-in var(--duration-medium) var(--ease-entrance);
  --animate-surface-out: surface-out var(--duration-small) var(--ease-exit);
  --animate-item-in: item-in var(--duration-small) var(--ease-entrance);
  --animate-cross-fade-in: cross-fade-in var(--duration-medium) var(--ease-entrance);
}

@keyframes surface-in {
  from {
    opacity: 0;
    transform: translateY(-4px) scale(0.98);
  }
  to {
    opacity: 1;
    transform: none;
  }
}
@keyframes surface-out {
  from {
    opacity: 1;
  }
  to {
    opacity: 0;
  }
}
@keyframes item-in {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}
@keyframes cross-fade-in {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

:root {
  /* … as cores existentes (styles.css:65-103) … */

  /* DM3b: faz TODO o Radix do set curado herdar a escala, sem editar
     popover.tsx / select.tsx / dropdown-menu.tsx / tooltip.tsx / dialog.tsx. */
  --tw-duration: var(--duration-medium);
  --tw-ease: var(--ease-entrance);

  /* Superfície opaca substitui backdrop-blur dentro de container rolável (DM9) */
  --surface-raised: color-mix(in oklab, var(--popover) 98%, var(--foreground));
}

/* Utilities — mesmo padrão @utility já usado em styles.css:178-205 */
@utility motion-surface {
  transition:
    opacity var(--duration-medium) var(--ease-entrance),
    transform var(--duration-medium) var(--ease-entrance);
}
@utility motion-surface-closed {
  transition:
    opacity var(--duration-small) var(--ease-exit),
    transform var(--duration-small) var(--ease-exit);
}
@utility motion-item {
  transition: opacity var(--duration-small) var(--ease-entrance);
}
@utility motion-cross-fade {
  transition: opacity var(--duration-medium) var(--ease-entrance);
}
@utility motion-state {
  transition:
    color var(--duration-small) var(--ease-standard),
    background-color var(--duration-small) var(--ease-standard),
    border-color var(--duration-small) var(--ease-standard);
}
@utility motion-gesture {
  transition: background-color var(--duration-micro) var(--ease-standard);
}
@utility motion-spatial {
  transition: transform var(--duration-large) var(--ease-spatial);
}
```

Dívida registrada sobre o `--tw-ease` global: ele muda a curva de **entrada** de todos os `animate-in` e `animate-out`; a assimetria entrada/saída vem da **duração** (200ms na base, 150ms no `closed`), não de curvas diferentes. Quando existir uma segunda superfície, reavaliar se `animate-out` deve ler um token próprio.

---

## 3. Catálogo de interações

Cada item é a **animação exata** de um gesto do app: o que move, por quanto tempo, com que curva, e qual invariante do domínio ela respeita. Onde a resposta é "nada", isso é uma decisão, não uma omissão.

### 3.1 Trocar de modo de visualização

`CalendarScreen.tsx:288-299` (os cinco `Button`) → `:305-330` (os cinco blocos condicionais).

| Campo               | Especificação                                                                                                                                                                                                                                                                    |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| O que move          | Nada se move no espaço. Só **opacidade**: a região que sai faz fade-out de `--duration-small` (150ms) + `--ease-exit`; a região que entra faz fade-in de `--duration-medium` (200ms) + `--ease-entrance`, começando 50ms antes do fim da saída, então nunca há quadro em branco. |
| Duração             | 200ms percebidos.                                                                                                                                                                                                                                                                |
| Curva               | `--ease-exit` na saída, `--ease-entrance` na entrada.                                                                                                                                                                                                                            |
| O que **não** muda  | O `header` (`:260-301`), o `ItemPanel` (`:335`) e o `selected` **não** são desmontados. A **grade não é remontada** — hoje é (os cinco `&&`).                                                                                                                                    |
| Reduced motion      | Cross-fade **preservado** (DM6 nível 3): sem ele a troca vira flash. Sem deslocamento espacial.                                                                                                                                                                                  |
| Interrupção         | Clicar outro modo durante a transição cancela a anterior (DM10).                                                                                                                                                                                                                 |
| INV respeitada      | D15bis: o painel é **região**, não modo — ele não pode piscar junto com a grade. `selected` sobrevive, e um item selecionado **não pode parecer concluído** por sumir e voltar.                                                                                                  |
| Anti-padrão evitado | ❌ cross-fade de tela inteira (esconde sidebar e `header`, e faz a grade piscar). ❌ stagger por dia. ❌ slide horizontal (impliciria direção que o domínio não tem — a Semana não é "depois" da Lista).                                                                         |

### 3.2 Navegar semana / mês / dia

`CalendarScreen.tsx:266` (`←`), `:277` (`→`), `:270-272` (`Hoje`).

| Campo               | Especificação                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| O que move          | **Os itens da grade**, por WAAPI-FLIP (DM5): o nó que estava na quinta às 08:00 e está agora na próxima quinta às 08:00 **desliza** de `translateY(0)` ao novo lugar em `--duration-large` (280ms) + `--ease-spatial`. Itens que saem da janela: fade-out de `--duration-small`. Itens que entram: fade-in de `--duration-small` (`item-in`). O cabeçalho de dias e o título (`:283`) **não** deslizam — trocam por fade de `--duration-small` no texto. |
| Duração             | 280ms no movimento; 150ms nos fades.                                                                                                                                                                                                                                                                                                                                                                                                                     |
| Curva               | `--ease-spatial` (monótona, sem overshoot — D15, snap de 15min).                                                                                                                                                                                                                                                                                                                                                                                         |
| O que **não** muda  | A largura/coluna dos itens **nunca** é animada (DM7 item 2). O `hourHeight` (D16) só muda por densidade, que é uma transição separada.                                                                                                                                                                                                                                                                                                                   |
| Reduced motion      | **Nível 2 (depara)**: os itens saltam para a posição nova com fade de 90ms. Nada de deslize.                                                                                                                                                                                                                                                                                                                                                             |
| Interrupção         | `←` repetido cancela o FLIP em curso e recomeça.                                                                                                                                                                                                                                                                                                                                                                                                         |
| INV respeitada      | **D6** — o fuso não muda com a navegação; a animação move pixels, nunca instantes. O `anchor` (`:107`) muda por `addDaysInTz` e a animação é puramente visual, então o que se vê durante o FLIP é sempre o que **será** gravado (`timeToY`/`yToTime`, D15).                                                                                                                                                                                              |
| Anti-padrão evitado | ❌ animar `top`/`height` dos itens (reflow por quadro, ~130 nós). ❌ `scrollTo({ behavior: "smooth" })` sob reduced motion — `scroll-behavior` já é neutralizado (`styles.css:172`).                                                                                                                                                                                                                                                                     |

### 3.3 Abrir / fechar o painel contextual

`CalendarScreen.tsx:335` → `ItemPanel` (`:784-825`). D18: **coluna, não modal**; o painel **permanece montado** ao navegar na semana.

| Campo                    | Especificação                                                                                                                                                                                                                                                                                                           |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| O que move               | **A coluna em si**: `transform: translateX(280px → 0)` com fade de opacidade, em `--duration-medium` (200ms) + `--ease-entrance`. Fechar: `translateX(0 → 280px)` com fade em `--duration-small` (150ms) + `--ease-exit`. A grade **não se move** — o painel entra por cima do `flex gap-3` (`CalendarScreen.tsx:303`). |
| Duração                  | 200ms abrir / 150ms fechar.                                                                                                                                                                                                                                                                                             |
| Curva                    | `--ease-entrance` / `--ease-exit`.                                                                                                                                                                                                                                                                                      |
| Largura real 360px (D18) | A entrada é `translateX(360px → 0)`, ainda 200ms. **Sem** `width` animada (DM4).                                                                                                                                                                                                                                        |
| `max-lg` (< 1024px, D18) | Vira `Sheet`: **o único consumidor** de `--duration-xlarge` (400ms) + `--ease-spatial`, entra de baixo.                                                                                                                                                                                                                 |
| Foco                     | Ao abrir, o foco vai para o primeiro elemento focável do painel, **após** a animação. Ao fechar, o foco **volta ao `TimedItem` que originou a seleção**; e o item **não pode ter sumido** — se o filtro de camada escondeu a camada, o foco vai para a barra de ferramentas, **nunca** para `body`.                     |
| Reduced motion           | **Nível 1 (desliga)**: o painel aparece e desaparece sem `transform`.                                                                                                                                                                                                                                                   |
| INV respeitada           | D20 — as três colunas (o que está marcado / o que precisa ser feito / o que realmente aconteceu) aparecem **na mesma ordem e as três**, mesmo vazias. Animação escalonada por coluna faria a usuária ler a ordem errada. **Nada escalonado.**                                                                           |
| Anti-padrão evitado      | ❌ modal (perde a referência da semana — D18). ❌ `width` animada. ❌ stagger entre as três colunas. ❌ abrir com _spring_ (rebote em coluna de informação é ruído; `DESIGN.md:5` — "precisa, silenciosa").                                                                                                             |

### 3.4 Concluir um item

`ItemPanel.tsx:813` (o `Button` "Concluir", hoje sem handler) e `App.tsx:671-703` (o item de "Para avançar hoje"). Regra: **INV-3** — concluir exige `ExecutionRecord`.

| Campo                 | Especificação                                                                                                                                                                                                                              |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| O que move            | **Nada se move.** O botão dá 90ms de `background-color` mais escuro (`--duration-micro`, resposta a gesto — DM6 nível 3).                                                                                                                  |
| O que muda            | O estado: o badge `Concluído` (`ITEM_STATE_LABEL`, `commitment.ts:31`) troca de texto com fade de `--duration-small`; no card do item, `bg-primary/10 → bg-muted` e a opacidade do texto caem, ambos `--duration-small`/`--ease-standard`. |
| Duração               | 150ms.                                                                                                                                                                                                                                     |
| Curva                 | `--ease-standard`.                                                                                                                                                                                                                         |
| Proibido              | ❌ pop ❌ stamp ❌ checkmark que se desenha ❌ "item que pula na grade" ❌ confete ❌ anel expandindo ❌ pulso verde. Tudo isso é DM7 item 1.                                                                                              |
| Se a conclusão falhar | O item **volta** ao estado anterior com fade de `--duration-small` **e** uma mensagem inline (§3.9). Sem shake — shake é movimento, e `DESIGN.md:5` é "silenciosa".                                                                        |
| Reduced motion        | Nível 3: só as transições de cor, que já são 150ms.                                                                                                                                                                                        |
| INV respeitada        | **INV-3** — a animação não substitui o `ExecutionRecord`; ela é consequência de um registro já gravado. **INV-4/INV-7** — concluir é ação explícita da usuária; a animação não é o convite.                                                |
| Anti-padrão evitado   | ❌ "achievement animation". ❌ mexer no `hourHeight` quando a grade comprime.                                                                                                                                                              |

### 3.5 Arrastar uma ocorrência

`ui/hooks/use-grid-drag.ts` (não existe; spec 02 Fase 2.4; A4 + D17).

| Campo                       | Especificação                                                                                                                                                                                                                                                                                                                                       |
| --------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Durante o arrasto           | O item segue o ponteiro **1:1**, por `transform: translate3d(dx, dy, 0)` no WAAPI, com `duration: 0` / `fill: "forwards"` — **sem curva**: curva em arraste é atraso. O snap de 15min (`snapY`, `grid-scale.ts:95-97`) é aplicado no `pointermove` **antes** do `translate`, então a posição já é final a cada quadro; não há "corrigir no soltar". |
| Fantasma / placeholder      | Uma linha de 1px (`--primary`, opacidade 0.35) na altura final, **sem** transição. Aparece e some com corte seco.                                                                                                                                                                                                                                   |
| Borda de tempo              | A etiqueta de horário segue o item (`x`, sem curva).                                                                                                                                                                                                                                                                                                |
| Ao soltar                   | `element.animate` de volta à posição final em `--duration-small` **apenas se** houve erro de comando (D17: _"se o comando falhar, o item volta"_). No sucesso, **sem animação de settle** — o item já está onde tem que estar, e um "pouso" seria teatro.                                                                                           |
| Enquanto arrasta            | **Nenhum rearrange** (DM5). A redistribuição de `columnize` só acontece no `pointerup`, e **sem animação** (DM7 item 2).                                                                                                                                                                                                                            |
| `origin === "google"` (D17) | Sem `cursor-grab`, sem alça, **sem** sombra de arraste. O bloqueio tem de ser visível **antes** do gesto.                                                                                                                                                                                                                                           |
| `Escape` no meio do arraste | O item volta **por corte**, sem transição: é cancelamento, não deslocamento.                                                                                                                                                                                                                                                                        |
| Reduced motion              | O arrasto **não é afetado** — é interação direta com o ponteiro, não animação, e WCAG 2.3.3 só cobre movimento não essencial. O que muda: o _snap visual_ do placeholder, que vira corte seco.                                                                                                                                                      |
| INV respeitada              | **D7** — a sobreposição é permitida e derivada: ao arrastar sobre outro item, **nada é bloqueado, nada empurra, nada pisca em vermelho**. O outro item só muda de largura no `pointerup`, sem transição. §1.7 R1.                                                                                                                                   |
| Anti-padrão evitado         | ❌ `top` animada (D15 — o que se vê durante o arraste tem de ser o que se grava). ❌ `transition` no item durante o arraste (a curva atrasaria o ponteiro). ❌ spring/bounce no settle. ❌ auto-ajuste de fuso (D6).                                                                                                                                |

### 3.6 Redimensionar uma ocorrência

Alças superior e inferior, spec 02 Fase 2.4.

| Campo                     | Especificação                                                                                                                                                                                                                                                                                                                                                                           |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| O que move                | A borda em reshape, 1:1 com o ponteiro, por `height` no WAAPI. **Exceção documentada a DM4**: durante o arraste o `height` **é** a interação — não há `transform` equivalente (`scaleY` deformaria o texto e a borda, que é a leitura do item). **A exceção vale só para a duração do gesto**; ao soltar, se houver erro, o `height` é reconciliado com `scaleY` em `--duration-small`. |
| Alça                      | Aparece com fade de `--duration-micro` (90ms) no `hover` do item. 1px de altura, `cursor: ns-resize`.                                                                                                                                                                                                                                                                                   |
| Piso                      | 15min (`MIN_VISIBLE_PX = 18`, `grid-scale.ts:109,143`); ao chegar no piso, **para** — sem repique, sem "vibração de limite".                                                                                                                                                                                                                                                            |
| Corte do texto secundário | O `{formatClock(item.startsAt)}` só aparece se `placement.height > 30` (`CalendarScreen.tsx:464`), e o corte é **instantâneo** ao cruzar 30px. Um fade ali seria mais lento do que o corte.                                                                                                                                                                                             |
| Reduced motion            | Redimensionar não é afetado (interação direta).                                                                                                                                                                                                                                                                                                                                         |
| INV respeitada            | D15: `yToTime` e `snapY` são a **mesma** matemática da leitura.                                                                                                                                                                                                                                                                                                                         |

### 3.7 Expandir detalhe (as três colunas de D20)

Ainda não existe. Spec 02 Fase 2.5.

| Campo                                                  | Especificação                                                                                                                                                  |
| ------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| O que move                                             | A coluna de conteúdo expande: fade-in de `--duration-small` (150ms) no conteúdo novo + `transform: translateY(-4px → 0)`. O colapso é o inverso, também 150ms. |
| Duração                                                | 150ms.                                                                                                                                                         |
| Curva                                                  | `--ease-entrance` / `--ease-exit`.                                                                                                                             |
| Escalonamento                                          | **Proibido** (D20). As três colunas aparecem juntas, na ordem, mesmo vazias.                                                                                   |
| Abas internas (origem, sincronização — specs 04/05/06) | Cross-fade de 150ms no conteúdo, **sem** mover o `TabsList`.                                                                                                   |
| Reduced motion                                         | Nível 2: fade de 90ms sem `translateY`.                                                                                                                        |
| INV respeitada                                         | **D20** — a ordem é a regra, não a animação; e `AGENTS.md:23` — um bloco de planejamento **nunca** cria obrigação. Nenhuma coluna entra com "chamada".         |

### 3.8 Carregar dados

A1. `CalendarScreen.tsx:113` (`loaded`), `:154-194` (o `useEffect` de carga), `:217-221` (`empty`).

| Campo               | Especificação                                                                                                                                                                                                                                                                                                                                                                                                                               |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| O que aparece       | **Esqueleto com a geometria exata**: 7 colunas, cabeçalho de dias (`:372`), eixo de horas (`:406`), `height: visibleHeight(scale)` (`grid-scale.ts:42-44` = 1008px), e um **retângulo fantasma por item** na posição que `placeInDay` já devolveu, ou — sem cache — uma coluna de três blocos de altura variada. Isso elimina o salto de layout e o **"vazio falso"**: hoje o app mostra a grade vazia, indistinguível de "não há eventos". |
| Cor                 | `bg-muted`, **não** `bg-primary/10` (o `Skeleton` atual, `skeleton.tsx:4`, é rosa a 10% e viola `DESIGN.md:17`).                                                                                                                                                                                                                                                                                                                            |
| Shimmer             | Varredura de gradiente em 1.4s, **dentro de** `@media (prefers-reduced-motion: no-preference)`. No `reduce`: preenchimento estático (DM6 nível 1). `animate-pulse` do Tailwind é rejeitado — R1.                                                                                                                                                                                                                                            |
| Acessibilidade      | `<div role="status" aria-busy="true">` no contêiner, com um texto `sr-only` **fora** do esqueleto ("Carregando eventos"). O esqueleto em si é `aria-hidden`. `aria-busy` sozinho não é anunciado por quase nenhum leitor de tela (Adrian Roselli, atualizado 2026-04-14).                                                                                                                                                                   |
| Quando o dado chega | O esqueleto faz fade-out de `--duration-small` e o conteúdo faz fade-in de `--duration-small`, **sobrepostos** — sem stagger, sem cascata (DM7 item 6). Se o conteúdo tiver a **mesma forma** do esqueleto, o cross-fade é invisível e não há CLS.                                                                                                                                                                                          |
| Quando falha        | Ver §3.9. O esqueleto **não** fica pulsando para sempre.                                                                                                                                                                                                                                                                                                                                                                                    |
| Reduced motion      | Nível 1: shimmer desligado, preenchimento estático; os fades de 150ms viram 90ms.                                                                                                                                                                                                                                                                                                                                                           |
| Anti-padrão evitado | ❌ spinner no centro (a grade tem 1008px de altura; um spinner no meio não diz onde está o dado). ❌ skeleton de página inteira. ❌ delay de 300ms antes de mostrar o esqueleto (atrasa quem tem dado rápido). ❌ `animate-pulse` infinito (R1).                                                                                                                                                                                            |

### 3.9 Erro inline

A9. Hoje o erro é engolido em `CalendarScreen.tsx:183-186` (`catch {}` sem efeito) e D17 já promete: _"se o comando falhar, o item volta e a grade mostra um aviso inline"_.

| Campo                             | Especificação                                                                                                                                                                                              |
| --------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Onde                              | Uma faixa **dentro da região afetada** (rodapé da grade, ou sob o item, se o erro for de um item) — **não** toast (DM7 item 8).                                                                            |
| O que move                        | Nada. Fade-in de `--duration-small` (150ms) na opacidade, com `--duration-small` de atraso, para não piscar em erro transitório de IPC.                                                                    |
| O que mostra                      | `text-destructive`, borda de 1px `--destructive/30`, fundo `destructive/6` (mesmo tom de `App.tsx:549`), e uma ação "Tentar de novo" que re-executa o `useEffect` de carga (`CalendarScreen.tsx:154-194`). |
| Ao sair                           | Fade-out de `--duration-small` + `--ease-exit`, ou **persiste** se o erro não for transatório — decisão de produto, não de motion: erro de validação persiste.                                             |
| Foco                              | **Não** rouba o foco. A faixa tem `role="alert"` e é anunciada, mas o foco permanece onde a usuária estava.                                                                                                |
| `origin === "google"` (D17/INV-6) | Falha ao gravar um item importado **não é erro de domínio** — a UI já mostra que não é arrastável. A faixa só aparece para falha de leitura.                                                               |
| Reduced motion                    | Nível 1: sem fade (aparece e some), porque o texto já é o sinal.                                                                                                                                           |
| INV respeitada                    | **D7** — o erro **nunca** é bloqueante nem impede a decisão: a faixa convida a tentar de novo, nunca a aceitar uma restrição.                                                                              |
| Anti-padrão evitado               | ❌ shake ❌ borda vermelha pulsante ❌ modal ❌ toast. ❌ esconder o erro atrás de um `console.error` (é o que `ErrorBoundary.tsx:14` faz, e é papel diferente).                                           |

### 3.10 Toggle de camada / filtro

`LayerFilter` (não existe; spec 02 §3, Fase 2.3). `bridge.ts:133` — `setLayerVisibility`.

| Campo                 | Especificação                                                                                                                                                                                                                                                        |
| --------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| O que move            | Os itens da camada que **sai** fazem fade-out de `--duration-small` e saem do fluxo; os que **entram** fazem fade-in de `--duration-small` e **ocupam a posição final desde o primeiro quadro** — sem FLIP, porque o filtro muda o _conjunto_, não move um conjunto. |
| Duração               | 150ms.                                                                                                                                                                                                                                                               |
| Curva                 | `--ease-exit` na saída, `--ease-entrance` na entrada.                                                                                                                                                                                                                |
| O que **não** muda    | A grade **não** refaz o layout durante a saída. O `columnize` recalcula no commit final, uma vez.                                                                                                                                                                    |
| `google` (D4, neutro) | A caixa de filtro tem o mesmo peso visual das outras cinco, **sem** a borda tracejada — a diferença é de cor, e é a cor que comunica.                                                                                                                                |
| Foco                  | Mantém o foco na caixa de filtro; a lista reordena abaixo.                                                                                                                                                                                                           |
| Reduced motion        | Nível 1: sem fade, corte seco. O corte é mais honesto: filtrar é uma ação, não uma transição.                                                                                                                                                                        |
| INV respeitada        | **D4** — cor fixa por camada, sem personalização. **§1.7 R3**: o mesmo componente em dois modos (oculto na Semana, listado na Lista) **anima igual** — senão a interface mistura "o que está marcado" com "o que eu escolhi ver".                                    |
| Anti-padrão evitado   | ❌ stagger por camada (5 × 150ms = 750ms de espera). ❌ recolher a coluna (a largura da grade é derivada de `hourHeight` × colunas, não da visibilidade).                                                                                                            |

### 3.11 Hover de ocorrência

`TimedItem` (`CalendarScreen.tsx:448-469`, hoje **sem** hover) e o pino do `MonthView` (`:654-663`, **sem** hover). O `AgendaView` já tem (`:541`).

| Campo                                              | Especificação                                                                                                                                                                                                                                                    |
| -------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| O que muda                                         | `border-color` de `border-chart-N/30` para `border-chart-N/60` (tom da camada, D4) e `background-color` de `/12` para `/18`. **`box-shadow` NÃO muda** — é o paint mais caro da lista (§1.3) e `DESIGN.md:17` reserva sombra a "elementos realmente flutuantes". |
| Duração                                            | `--duration-small` (150ms) para a cor; `--duration-micro` (90ms) no `active`.                                                                                                                                                                                    |
| Curva                                              | `--ease-standard`.                                                                                                                                                                                                                                               |
| Proibido                                           | ❌ `scale` ❌ `translateY` (é o que o Kanban faz hoje em `App.tsx:818` — e é subliminar demais para valer o custo) ❌ brilho ❌ mudança de camada visual ❌ `will-change` permanente.                                                                            |
| Atraso                                             | Nenhum. O hover responde no primeiro quadro; `DESIGN.md:21` exige "estados de seleção claros".                                                                                                                                                                   |
| `origin === "google"`                              | Hover **igual** aos demais. O que muda é o `cursor` e a ausência de alça (D17), não o realce. A distinção google × interno é de **estilo** (tracejado, neutro), nunca de movimento.                                                                              |
| Cancelado (`overrideKind === "cancelled"`, `:455`) | `opacity-50 line-through` estático. O hover **não** intensifica.                                                                                                                                                                                                 |
| `prefers-contrast: more`                           | A borda do item vai de 1px para 2px com transição de cor de 150ms. A espessura **não** anima (`border-width` é layout).                                                                                                                                          |
| Reduced motion                                     | Nível 3: é só cor, que não é movimento.                                                                                                                                                                                                                          |
| INV respeitada                                     | **DM7 item 9** — a cor de camada não brilha. §1.7 R3 — o mesmo componente nos dois modos.                                                                                                                                                                        |
| Anti-padrão evitado                                | ❌ hover que "promove" o item (escala, sombra, elevation).                                                                                                                                                                                                       |

### 3.12 Transições já existentes no app, reescritas por esta spec

| Gesto                          | Onde                           | Hoje                                               | Depois                                                                                                  |
| ------------------------------ | ------------------------------ | -------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| Recolher/expandir a sidebar    | `App.tsx:192`                  | `transition-[width]` 200ms `ease-out` — **layout** | `transform: translateX` 200ms `--ease-entrance`, **ou** sem animação (DM4 Exceção 1; medir na M2)       |
| Card do Kanban no hover        | `App.tsx:818`                  | `transform` 150ms, `-translate-y-px`               | `background-color` + `border-color` 150ms `--ease-standard`; sem `transform`                            |
| ⌘K — overlay                   | `dialog.tsx:24`                | fade 150ms `ease`                                  | fade **200ms** `--ease-entrance`, via `--tw-duration`/`--tw-ease` (DM3b) — I2 resolvido                 |
| ⌘K — conteúdo                  | `dialog.tsx:43`                | fade 200ms `ease` (`duration-200` literal)         | fade 200ms `--ease-entrance`; **remover o literal** (I10)                                               |
| Tooltip                        | `tooltip.tsx:23`               | zoom 0.95 + slide 8px, 150ms `ease`                | `surface-in` 200ms `--ease-entrance`; sem `zoom`, slide de 4px só em `no-preference`                    |
| Checkbox                       | `checkbox.tsx:14`              | sem transição (C0)                                 | `motion-state` 150ms `--ease-standard` — paridade com `Switch` (I3)                                     |
| `Switch` — polegar             | `switch.tsx:20`                | `transform` 150ms C1                               | **mantém**; é o único `transform` correto. Só a curva passa a ser o token.                              |
| `TabsTrigger`                  | `tabs.tsx:30`                  | `transition-all` (inclui `box-shadow`)             | `transition-colors` + `transition-shadow` explícitos (I4)                                               |
| `Progress` — indicador         | `progress.tsx:18`              | `transition-all` sobre `transform`                 | `transition: transform` 150ms `--ease-linear` — linear é a curva honesta de progresso (I4)              |
| `SelectContent`                | `select.tsx:71,73`             | `translate-y-1` estático → salto de 4px            | **corrigir o template** (gate em `data-[state=open]`) antes de importar; senão a Fase M5 não entra (I7) |
| Esqueleto                      | `skeleton.tsx:4` (nunca usado) | `animate-pulse` infinito, `bg-primary/10`          | **reescrever** com shimmer dentro de `no-preference`; `bg-muted`; `role="status"` (§3.8)                |
| Item de tarefa concluído       | `App.tsx:675-703`              | `line-through` instantâneo                         | `motion-state` 150ms na cor e na opacidade; o `line-through` é instantâneo (é texto) (§3.4)             |
| Aba de `SubjectView`           | `App.tsx:1015-1086`            | render condicional, sem transição                  | cross-fade de 150ms no painel; `TabsList` não move                                                      |
| Troca de modo do dashboard     | `App.tsx:350-362`              | cinco `&&` condicionais, sem transição             | idem §3.1 — é o **mesmo** gesto com o mesmo token                                                       |
| Busca ⌘K — lista de resultados | `App.tsx:400-417`              | render direto                                      | fade de 150ms no bloco, sem cascade por item                                                            |

---

## 4. Riscos e anti-padrões

| #   | Anti-padrão                                                   | Por que quebra                                                                                                                                           | Onde o app já cai nele                                                             | Regra                                                                                                                                 |
| --- | ------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| X1  | Animar `width`/`height`/`top`/`left`/`margin`/`padding`       | Reflow por quadro. Em flex/grid propaga para os irmãos e para tudo dentro.                                                                               | `App.tsx:192`                                                                      | DM4. `rg "transition-\[(width\|height\|top\|left\|margin\|padding)" src` → vazio                                                      |
| X2  | `transition-all`                                              | Inclui propriedades de layout e `box-shadow`. Uma classe nova que muda tamanho passa a animar sem ninguém pedir.                                         | `tabs.tsx:30`, `progress.tsx:18`                                                   | Listar as propriedades. `transition-all` só com justificativa escrita.                                                                |
| X3  | Animação que **atrasa o input**                               | Quebra a perceived latency e é WCAG 2.3.3 (AAA) na prática: movimento não essencial que não pode ser desligado.                                          | —                                                                                  | DM10. `prefers-reduced-motion: reduce` é o botão de escape da usuária e **tem de funcionar**.                                         |
| X4  | `backdrop-filter` em `sticky` dentro de `overflow-auto`       | Re-amostragem do backdrop por quadro de rolagem. No WebView2 é o custo de paint mais alto do arquivo.                                                    | `CalendarScreen.tsx:372`, `:406`                                                   | DM9. `rg "backdrop-blur\|backdrop-filter" src/features` → vazio                                                                       |
| X5  | Jank no WebView2 por falta de contenção                       | A `WeekView` renderiza ~130 nós; qualquer promoção de camada ou recálculo de estilo se paga em todos.                                                    | —                                                                                  | `content-visibility: auto` nas células offscreen do `MonthView` (`:631-670`) e `contain: layout paint` no `DayColumn`. Avaliar na M4. |
| X6  | **Conflito com Radix**                                        | O Radix controla `data-[state]` e `Presence`; animar `display`/`overlay` briga com ele. E `allow-discrete` + Radix é caminho para duplicação de duração. | `dialog.tsx:43` (`duration-200` literal), `tooltip.tsx:23` (`animate-in` sem gate) | DM8: **não editar o set curado**. Deixar o Radix no `animate-in`/`animate-out` e controlar por `--tw-duration`/`--tw-ease` (DM3b)     |
| X7  | Animar `box-shadow`                                           | Pintura de área grande. Em lista, 20 elementos × 150ms.                                                                                                  | `tabs.tsx:30` (via `transition-all`)                                               | Usar `border-color` + `background-color` para realce. Sombra só em elemento **realmente** flutuante (`DESIGN.md:17`).                 |
| X8  | `will-change: transform` permanente                           | Promove uma camada que fica viva mesmo sem animação — consome memória de composição.                                                                     | `App.tsx:818` (hover `-translate-y-px`, promoção por quadro em 20 cards)           | Nenhum `will-change` no projeto. `translate` com `hover` não precisa dele.                                                            |
| X9  | Animação repetida/infinita não desligada sob `reduce`         | `0.01ms` + `infinite` = strobe. É WCAG 2.3.1 (Level A).                                                                                                  | `skeleton.tsx:4` + `styles.css:168-176`                                            | DM6. `animation-iteration-count: 1 !important` obrigatório; nenhum `infinite` fora de `no-preference`.                                |
| X10 | Animação que **sugere bloqueio**                              | Viola D7 (`AGENTS.md:25`) e a §13. Vermelho pulsando é o oposto de "só sinalizado, nunca bloqueante".                                                    | —                                                                                  | DM12. Conflito = contorno tracejado + badge, ambos estáticos.                                                                         |
| X11 | Animação que **promove** `recomendado`/`opcional`             | Viola INV-7 e D8: a sugestão pulsa, a usuária clica, vira obrigação. A animação fabricou o consentimento.                                                | —                                                                                  | DM7 item 7. `Aceitar`/`Recusar` com peso visual idêntico.                                                                             |
| X12 | Animações de _entrada_ decorativas na montagem                | Viola `DESIGN.md:25` literalmente. Atenção ao `StrictMode` (`main.tsx:12`), que monta duas vezes em dev.                                                 | —                                                                                  | DM7 item 5                                                                                                                            |
| X13 | Cascade / stagger                                             | Multiplica a latência pelo número de itens. Com 7 colunas × N itens, o stagger vira segundo.                                                             | —                                                                                  | DM7 item 6; §3.3, §3.7, §3.10. Nada escalonado, em lugar nenhum.                                                                      |
| X14 | Confundir transição de **layout** com transição de **estado** | É a origem de I1: quatro curvas porque ninguém declarou o que cada gesto era.                                                                            | §1.4                                                                               | DM3: a curva é escolhida pelo **tipo de movimento**, não pelo componente.                                                             |
| X15 | Assumir que `startViewTransition` "resolve FLIP de graça"     | Captura o DOM inteiro como imagem; custo proporcional aos nós; scroll não controlável; all-or-nothing.                                                   | —                                                                                  | DM1. Reabrir só se a troca de modo virar "uma imagem no lugar de outra".                                                              |
| X16 | Skeleton de página inteira / spinner central                  | Não diz _onde_ está o dado, e a grade tem 1008px de altura.                                                                                              | —                                                                                  | §3.8. Esqueleto com a geometria exata.                                                                                                |
| X17 | Confiar no reset de CSS para código JS                        | `transition-duration: 0.01ms` não alcança WAAPI, `startViewTransition` nem timers.                                                                       | R5                                                                                 | DM6: hook `use-reduced-motion` com listener de `change`, usado por DM5 e pela fase de drag.                                           |
| X18 | Animar **frequência** (pulso) para comunicar urgência/estado  | Pulso é movimento espacial, e é o que DM7 item 1 proíbe. Além disso, WCAG 2.2.2 exige poder pausar.                                                      | `skeleton.tsx:4`                                                                   | Urgência é comunicada por rótulo, borda e cor estática.                                                                               |
| X19 | Curva com _overshoot_ em grade com snap                       | `ease-spatial` do M3 (`0.27, 1.06, 0.18, 1.00`) mostra `14:52` antes de `15:00` e contradiz D15.                                                         | —                                                                                  | DM3: `--ease-spatial` é monótona.                                                                                                     |
| X20 | Animar o valor que o domínio considera verdade                | `top`/`height` animados durante arraste (D15) gravariam um instante que não é o que a usuária vê.                                                        | —                                                                                  | §3.5. O arraste é 1:1 com o ponteiro e o snap vem **antes** do `translate`.                                                           |

---

## 5. Fases de implementação

Cada fase é CSS/TypeScript puro exceto onde anotado. Nenhuma altera `src-tauri/**`, `tauri.conf.json` ou o domínio. M0, M1 e M3 são verificáveis sem o app de pé; M2 e M4 dependem dele (o bloqueio de MSVC registrado em `00-fundacao.md:199` continua valendo).

### Fase M0 — Baseline verificável

| Ação   | Detalhe                                                                                                                                                                                                                                                          |
| ------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Rodar  | `rg -c "transition-\|animate-\|duration-" src` e registrar a contagem por arquivo                                                                                                                                                                                |
| Rodar  | `npm run dev`; em DevTools, _Rendering → Paint flashing_ ligado; rolar a grade da semana e observar se o `backdrop-blur` dos `sticky` (`CalendarScreen.tsx:372`, `:406`) pisca. É a prova de X4 antes de mexer                                                   |
| Rodar  | Em _Rendering → Emulate CSS media feature `prefers-reduced-motion: reduce`_, habilitar `Skeleton` e observar. É a prova de R1 antes de corrigir                                                                                                                  |
| Editar | `docs/calendario/02-grade-desktop.md:40` — corrigir a afirmação "já desliga animação" para "`styles.css:168-176` desliga transição e duração de animação, mas **não** zera `animation-iteration-count` (R1) e **não** cobre `prefers-reduced-transparency` (R2)" |

**Aceite e teste:** contagem registrada; as duas provas feitas, com print de DevTools anexado ao registro da fase; `npm run typecheck` → 0 e `npm run lint` → `0 problems` **antes** de qualquer alteração.

### Fase M1 — Tokens de motion (sem tocar em nenhum componente)

| Ação          | Detalhe                                                                                                                                                                                                 |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Editar        | `src/styles.css` — bloco `@theme inline` (após `:63`): `--duration-*` (5), `--ease-*` (5), `--animate-surface-in/out`, `--animate-item-in`, `--animate-cross-fade-in` + os `@keyframes` correspondentes |
| Editar        | `src/styles.css` — `:root` (após `:103`): `--tw-duration: var(--duration-medium)`, `--tw-ease: var(--ease-entrance)`, `--surface-raised` (DM3b, DM9)                                                    |
| Editar        | `src/styles.css` — `@utility` (após `:205`): `motion-surface`, `motion-surface-closed`, `motion-item`, `motion-cross-fade`, `motion-state`, `motion-gesture`, `motion-spatial`                          |
| Editar        | `src/styles.css:168-176` — reescrever o bloco `reduce` (DM6): `+ animation-iteration-count: 1 !important`; adicionar `prefers-reduced-transparency` e `prefers-contrast` (R2, R3)                       |
| Criar         | `src/hooks/use-reduced-motion.ts` — `matchMedia("(prefers-reduced-motion: reduce)")` + listener de `change` (R5, DM6)                                                                                   |
| **Não tocar** | `src/components/ui/**` (DM8). Nenhum componente deve mudar de aparência com esta fase                                                                                                                   |

**Aceite e teste:** `npm run typecheck` → 0; `npm run lint` → `0 problems`; `npm run build` ok; `rg -- "--duration-small" src/styles.css` → 1 ocorrência; `rg -n "duration-|ease-|animate-" src/components/ui` → contagem **inalterada** em relação ao M0; visualmente **idêntico ao baseline**, exceto o ⌘K, que passa de 150ms/`ease` para 200ms/curva de entrada — e **só** ele, porque `Dialog` e `Tooltip` são os únicos dois Radix que rodam hoje (verificado); com `prefers-reduced-motion: reduce` emulado, o ⌘K abre e fecha instantaneamente e **sem** piscar.

### Fase M2 — Correção do custo: sidebar e `backdrop-blur`

| Ação       | Detalhe                                                                                                                                                                                                                                          |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 2a — medir | DevTools → _Performance_ com a visão Calendário aberta; clicar em `PanelLeft`; ler _Event Timing → Layout_ e a contagem de `Layout` events. **Se ≤ 1ms por quadro e nenhum > 8ms:** ir para 2b-opção-1. **Se > 8ms em algum quadro:** 2b-opção-2 |
| 2b-opção-1 | `src/App.tsx:192` — remover `transition-[width] duration-200 ease-out`; animar `transform: translateX` com o conteúdo interno em largura fixa (`:196` já é) e o estado "rail" de 68px (`:193`) num elemento separado                             |
| 2b-opção-2 | `src/App.tsx:192` — remover a transição; a sidebar colapsa por corte seco                                                                                                                                                                        |
| 2c         | `src/features/calendar/ui/CalendarScreen.tsx:372` e `:406` — trocar `bg-popover/95 backdrop-blur` por `background: var(--surface-raised)`; remover `backdrop-blur`                                                                               |
| Criar      | `src/features/calendar/ui/hooks/use-flip.ts` — a implementação de DM5, exposta como `measure()` / `play()`, **ainda não usada** pela grade (a spec 02 Fase 2.4 liga)                                                                             |

**Aceite e teste:** `rg "transition-\[width" src` → vazio; `rg "backdrop-blur|backdrop-filter" src/features` → vazio; `npm run typecheck` → 0; `npm run lint` → `0 problems`; em _Performance_, rolar a grade da semana por 3s: **zero** `Layout` events acima de 4ms e nenhum quadro perdido; com _Paint flashing_, o cabeçalho de dias e o eixo de horas **não** piscam durante a rolagem.

### Fase M3 — Troca de modo, navegação e enter/exit de item

| Ação   | Detalhe                                                                                                                                                                                                            |
| ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Editar | `src/features/calendar/ui/CalendarScreen.tsx:305-330` — substituir os cinco `&&` por uma região única com `motion-cross-fade`; o `header` (`:260-301`) e o `ItemPanel` (`:335`) ficam **fora** da região que troca |
| Editar | `src/features/calendar/ui/CalendarScreen.tsx:266,270-272,277` — envolver `step`/`stepMonth`/`goToday` (`:225-242`) na medição FLIP (`use-flip.ts`)                                                                 |
| Editar | `src/features/calendar/ui/CalendarScreen.tsx:443-471` — `motion-item` + `@starting-style`/`allow-discrete` no `TimedItem`, para que entrada e saída de item não dependam de JS                                     |
| Editar | `src/features/calendar/ui/CalendarScreen.tsx:541` e o `MonthView` (`:654-663`) — `motion-state` no hover, conforme §3.11                                                                                           |
| Editar | `src/App.tsx:350-362` — mesma estratégia de cross-fade para os cinco modos do dashboard, com os mesmos tokens                                                                                                      |
| Editar | `src/components/ui/dialog.tsx:43` — remover o `duration-200` literal (I10), para que o valor passe a vir de `--tw-duration`                                                                                        |

**Aceite e teste:** `npm run typecheck` → 0; `npm run lint` → `0 problems`; observável: **trocar de modo faz cross-fade de 160ms sem remontar a grade** (o `TimedItem` selecionado e o `ItemPanel` continuam no DOM — verificável no inspetor do React, ou por um `console.log` no repositório de dados, que não deve disparar durante a transição); `←` move os itens com FLIP de 280ms; em `prefers-reduced-motion: reduce`, `←` reposiciona instantaneamente com fade de 90ms; `rg "duration-200" src` → vazio.

### Fase M4 — Carregamento, erro inline e gravação

| Ação   | Detalhe                                                                                                                                                                                  |
| ------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Editar | `src/components/ui/skeleton.tsx` — reescrever: `bg-muted`, shimmer dentro de `no-preference`, `role="status"`, sem `animate-pulse`                                                       |
| Criar  | `src/features/calendar/ui/WeekGridSkeleton.tsx` — esqueleto com a geometria de `CalendarScreen.tsx:371-486` e `visibleHeight(scale)`                                                     |
| Editar | `src/features/calendar/ui/CalendarScreen.tsx:113,187,217-221` — `loading` separado de `loaded`; o esqueleto cobre `loaded === false`; o `empty` continua sendo só para `loaded === true` |
| Editar | `src/features/calendar/ui/CalendarScreen.tsx:183-186` — o `catch` passa a registrar o erro num estado e a renderizar a faixa inline (§3.9), com "Tentar de novo"                         |
| Editar | `src/features/calendar/ui/CalendarScreen.tsx:169` — `saving`/`saved` no DM11, com o indicador **na posição final**                                                                       |
| Editar | `src/features/calendar/ui/CalendarScreen.tsx:448-469` — hover de `TimedItem` (§3.11), `will-change` **ausente** (X8)                                                                     |
| Editar | `src/components/ui/checkbox.tsx:14`, `progress.tsx:18`, `tabs.tsx:30` — paridade e remoção de `transition-all` (I3, I4)                                                                  |

**Aceite e teste:** `npm run typecheck` → 0; `npm run lint` → `0 problems`; `rg "animate-pulse" src` → vazio; `rg "transition-all" src` → vazio; observável: **abrir o app em `npm run dev` com IPC indisponível mostra o esqueleto com a geometria da grade, e depois o estado vazio — nunca o esqueleto piscando**; com `prefers-reduced-motion: reduce`, o esqueleto é um preenchimento estático; um `list_events_in_window` que falha mostra a faixa inline com "Tentar de novo" e **não** engole o erro.

### Fase M5 — Arraste e redimensionamento (requer o app de pé)

| Ação   | Detalhe                                                                                                                                                                 |
| ------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Criar  | `src/features/calendar/ui/hooks/use-grid-drag.ts` (A4) — `pointerdown/move/up` + `setPointerCapture`, `duration: 0` no WAAPI, snap **antes** do `translate` (DM5, §3.5) |
| Editar | `TimedItem` — alças superior e inferior; `origin === "google"` sem `cursor-grab` e sem alça (D17)                                                                       |
| Editar | `use-grid-drag.ts` — ler `use-reduced-motion` (R5) para o comportamento de `Escape` e do retorno em caso de erro                                                        |

**Aceite e teste:** `npm run typecheck` → 0; `npm run lint` → `0 problems`; observável: arrastar um item 40px para baixo move-o exatamente 15min (snap), e **não** 40px; arrastar para cima até o topo da grade não deixa `startsAt` antes do início do dia; com `Escape` no meio do arraste o item volta **por corte** e nenhum comando é chamado; um item `origin: "google"` não tem `cursor-grab` e não exibe alça; **arrastar um item sobre outro não desloca nenhum vizinho durante o gesto** (a redistribuição de `columnize` só acontece no `pointerup`, sem transição — DM7 item 2).

---

## 6. Critérios de aceite finais

1. `npm run typecheck` → 0 e `npm run lint` → `0 problems` em todas as fases.
2. `rg "transition-\[(width|height|top|left|margin|padding)" src` → **vazio** (DM4).
3. `rg "backdrop-blur|backdrop-filter" src/features` → **vazio** (DM9).
4. `rg "transition-all" src` → **vazio**; `rg "animate-pulse" src` → **vazio** (X2, R1).
5. `rg "will-change" src` → **vazio** (X8).
6. `rg -n "duration-[0-9]|ease-" src/components/ui` → **vazio** (DM8: o set curado não decide motion).
7. `rg -n "matchMedia" src` → **≥ 1**, em `src/hooks/use-reduced-motion.ts` (R5).
8. `rg -- "--duration-(micro|small|medium|large|xlarge)" src/styles.css` → **5** ocorrências, uma de cada, e nenhuma fora do `@theme`.
9. `styles.css:168-176` (ou seu sucessor) contém `animation-iteration-count: 1 !important`, `prefers-reduced-transparency` e `prefers-contrast` (R1, R2, R3).
10. **Observável:** trocar de modo de visualização faz cross-fade de 200ms **sem remontar a grade** — o `TimedItem` selecionado e o `ItemPanel` permanecem no DOM.
11. **Observável:** `←`/`→` reposiciona os itens com FLIP de 280ms; a largura dos itens **não** anima (DM7 item 2).
12. **Observável:** com `prefers-reduced-motion: reduce` emulado, **nada** se desloca no espaço em nenhum gesto; nenhum esquema pisca; nenhum indicador fica em laço.
13. **Observável:** o esqueleto de carregamento tem a geometria da grade e nunca pisca sob `reduce`.
14. **Observável:** falha de leitura mostra faixa inline com "Tentar de novo" e o esqueleto **não** fica pulsando.
15. **Observável:** concluir um item muda cor e rótulo em 150ms e **nada mais** — sem pop, sem stamp, sem pulso (DM7 item 1).
16. **Observável:** arrastar um item sobre outro não move nenhum vizinho durante o gesto (D7).
17. `package.json` **inalterado** em relação ao estado de 2026-09-29 — nenhuma biblioteca de animação adicionada (DM1).

---

## 7. Fora de escopo

- **Implementação.** Este documento não implementa nada; a Fase M1 é a primeira alteração de código.
- Arraste e redimensionamento de verdade — spec 02 Fase 2.4. Aqui só o gesto é especificado.
- `Sheet` de tela cheia (D18) — spec 02 Fase 2.2. Aqui só o degrau `--duration-xlarge` é reservado.
- `ViewSwitch` movido para `features/calendar/ui/` (D19) — spec 02 Fase 2.3.
- Sinalização visual de sobreposição no `TimedItem` — spec 02 Fase 2.3 entrega o atributo `overlapped`; aqui só a política de "estático, nunca animado" (DM12) está fechada.
- Timer e registro de execução (spec 03) — a animação de um timer parado seria um movimento contínuo e provavelmente violaria WCAG 2.2.2; fora daqui.
- Indicador de sincronização (spec 05) e motor de sugestões (spec 06) — DM7 item 7 e DM12 já restringem o que eles podem animar, mas o catálogo completo é das specs donas.
- Animações de Gráfico/insight de planejamento × realidade — `02-grade-desktop.md:266` já põe isso fora de escopo como produto novo.
- Cross-document View Transitions — irrelevante sem roteador (`AGENTS.md:9`).
- `motion` / `framer-motion` / `@formkit/auto-animate` como dependência — DM1 fecha em CSS + WAAPI. **Gatilho de revisão:** se um dia a grade passar de ~50 itens visíveis por dia e o FLIP manual virar gargalo mensurável, ou se surgir uma necessidade real de _shared element_ entre modos que o WAAPI não expresse, DM1 é reaberta com medição anexada.
- Modo escuro e `forced-colors` — `styles.css:105-139` tem o bloco `.dark` e nada alterna a classe (é `roadmap.md:11`); a estratégia de motion para `forced-colors` é a mesma de `prefers-contrast: more`.

---

## 8. Rastreabilidade

| Documento                                                   | O que esta spec consome                                                                                                                  | O que esta spec devolve                                                                                                                                                    |
| ----------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `DESIGN.md:23-25`                                           | "Transições rápidas de 150–200ms apenas para mudanças de estado. Sem animações de entrada decorativas. Movimento reduzido é respeitado." | DM2 fixa a escala de forma que **toda** mudança de estado cabe em 150/200; DM7 item 5 proíbe entrada decorativa; DM6 é o "movimento reduzido é respeitado", em três níveis |
| `DESIGN.md:17`                                              | "Sombras curtas e discretas apenas em elementos realmente flutuantes"                                                                    | X7 proíbe animar `box-shadow`; §3.11 usa cor em vez de sombra no hover; §3.8 usa `bg-muted` no esqueleto em vez de `bg-primary/10`                                         |
| `DESIGN.md:5,21`                                            | "precisa, silenciosa e orientada ao conteúdo"; controles 32–36px, cantos 6–10px                                                          | §3.3 rejeita _spring_ no painel; §3.4 e §3.9 rejeitam shake; DM2 justifica `--duration-micro` por área, não por componente                                                 |
| `AGENTS.md:12`                                              | shadcn via `npx shadcn@latest add`                                                                                                       | DM8: o set curado não é editado; DM3b resolve o que hoje se resolveria editando quatro arquivos                                                                            |
| `AGENTS.md:19`                                              | Todo IPC por `src/lib/ipc.ts`                                                                                                            | A Fase M4 toca `CalendarScreen.tsx:183-186` e `:169`, que já falam por `data/bridge.ts` (`:32`)                                                                            |
| `AGENTS.md:23`                                              | Um bloco de planejamento nunca cria obrigação                                                                                            | DM11: o item salvo não "pula"; §3.3: o painel entra sem escalar as três colunas                                                                                            |
| `AGENTS.md:24`                                              | `recomendado`/`opcional` nunca viram obrigação                                                                                           | §1.7 R2, DM7 item 7 (peso visual idêntico entre `Aceitar` e `Recusar`), X11                                                                                                |
| `AGENTS.md:25`                                              | Conflito e sobreposição são derivados e só sinalizados                                                                                   | §1.7 R1, DM7 item 2 (largura nunca interpola), DM12 (sinal estático), §3.5 (nada se move ao arrastar sobre outro)                                                          |
| `AGENTS.md:17`                                              | `domain/` é TypeScript puro, sem React                                                                                                   | O hook `use-reduced-motion` e `use-flip` vivem em `hooks/` e `ui/hooks/`, nunca em `domain/`                                                                               |
| `calendario.md` §3, §13, INV-7                              | Níveis de compromisso; regra de ouro                                                                                                     | §1.7; DM7 itens 1 e 7; X10, X11                                                                                                                                            |
| `calendario.md` §12.7                                       | Conflito derivado, nunca bloqueante                                                                                                      | DM12, DM7 item 2                                                                                                                                                           |
| `calendario.md` §7                                          | Grade, painel persistente, arrastar/redimensionar                                                                                        | §3.2, §3.3, §3.5, §3.6, §3.11                                                                                                                                              |
| `docs/calendario/00-fundacao.md` D4                         | Cor fixa por camada, sem personalização                                                                                                  | DM7 item 9, §3.10, §3.11                                                                                                                                                   |
| `docs/calendario/00-fundacao.md` D7 / A8 INV-1..7           | Sobreposição permitida; `recomendado`/`opcional`                                                                                         | §1.7, DM7, DM12                                                                                                                                                            |
| `docs/calendario/00-fundacao.md` A4                         | Arrasto por ponteiro próprio, sem biblioteca                                                                                             | DM1 (WAAPI) e §3.5                                                                                                                                                         |
| `docs/calendario/01-dominio-persistencia.md`                | `GridItem`, `listEventsInWindow`, comandos de escrita                                                                                    | A6 e §3.9–§3.11 nascem dos comandos que a spec 01 entregou                                                                                                                 |
| `docs/calendario/02-grade-desktop.md` D15                   | Uma escala de tempo só                                                                                                                   | §3.2, §3.5, §3.6, X19, X20                                                                                                                                                 |
| `docs/calendario/02-grade-desktop.md` D16                   | 7 dias, 56px/hora, densidades                                                                                                            | DM2 (`--duration-large` para mudança de `hourHeight`); §3.8 (geometria do esqueleto)                                                                                       |
| `docs/calendario/02-grade-desktop.md` D17                   | Arrasto só para `origin !== "google"`                                                                                                    | §3.5, §3.11                                                                                                                                                                |
| `docs/calendario/02-grade-desktop.md` D18                   | Painel é coluna, não modal; `Sheet` no `max-lg`                                                                                          | §3.3, DM2 (`--duration-xlarge` reservado ao `Sheet`)                                                                                                                       |
| `docs/calendario/02-grade-desktop.md` D19                   | `ViewSwitch` interno                                                                                                                     | §3.1 (o toggle de modo é o mesmo gesto com o mesmo token)                                                                                                                  |
| `docs/calendario/02-grade-desktop.md` D20                   | Painel em três colunas                                                                                                                   | §3.3, §3.7 — sem escalonamento                                                                                                                                             |
| `docs/calendario/02-grade-desktop.md:40`                    | "`styles.css:168-176` já desliga animação"                                                                                               | **Corrigido na Fase M0**: o bloco não zera `animation-iteration-count` (R1) nem cobre `prefers-reduced-transparency` (R2)                                                  |
| `docs/calendario/03-mobile-execucao.md` D3x (agenda mobile) | Edição por ações, sem ponteiro                                                                                                           | §3.10, §3.11 são gesture-equivalentes em todas as plataformas                                                                                                              |
| `docs/calendario/05-google-calendar.md`                     | Camada `google` somente leitura                                                                                                          | §3.11: a distinção é de estilo, nunca de movimento                                                                                                                         |
| `docs/calendario/06-sugestoes.md`                           | Três ações explícitas, nenhum caminho automático (D8)                                                                                    | DM7 item 7, X11 — o peso visual de `Aceitar` e `Recusar` é idêntico                                                                                                        |
| `roadmap.md:11`                                             | Modo escuro pendente                                                                                                                     | Fora de escopo (§7); a estratégia de `forced-colors` é a de `prefers-contrast: more`                                                                                       |
| `SPEC.md:337,403`                                           | Curated set; `overflow:hidden` + `h-dvh` e teste em DPI 150%                                                                             | §1.1 (4 de 19 componentes importados); X5 (contenção como alavanca de performance)                                                                                         |
