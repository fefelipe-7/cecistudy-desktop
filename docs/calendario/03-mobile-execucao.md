# Spec 03 — Agenda mobile, edição rápida, timer e registro de execução

Data: 2026-09-28
Escopo: entregar a experiência mobile de `calendario.md` §8 (agenda compacta, sem grade) e fechar a metade de interface de §6 (planejamento × realidade) com o timer, o registro de execução e o painel de histórico.
Status: proposta fechada, aguardando implementação.

**Rastreabilidade — `calendario.md`:** §6 (integral do ponto de vista da interface: planejado, real, timer, desvios), §7 (a decisão de que o painel é o desktop; aqui é o `Sheet` do mobile), §8 (integral), §13 (a terceira pergunta — "o que realmente aconteceu" — é o objeto desta spec).
**Entregas de `calendario.md` §11 cobertas:** **3** (agenda diária mobile com criação e edição rápida) e **6** (blocos de estudo manuais e timer de execução).
**Depende de:** spec 00 (A7, A8, D2, D6, D7) e spec 01 (domínio, comandos, `plan_block.timer_state`).
**Deixa deliberadamente para:**

- spec 04 — as "ações rápidas" incluem "ver na origem" e "abrir no módulo"; aqui só "criar responsabilidade para este compromisso" e "vincular a um contexto" (que grava `owner_ref` já previsto na spec 01);
- spec 05 — as ações rápidas **não** incluem sincronizar/excluir no Google; aqui a aba de sincronização do painel aparece com "Não sincronizado" em todo item;
- spec 06 — as ações rápidas **não** incluem aceitar sugestão; aqui a seção de sugestão fica oculta quando não há sugestão.
- spec 02 — a grade semanal mobile resumida é declarada aqui, mas implementada como **agenda**, não como grade (ver §2.2 D22).

---

## 1. Diagnóstico do estado atual

| Requisito                                                                                    | Onde encosta hoje                                      | Situação                                                                                                                             |
| -------------------------------------------------------------------------------------------- | ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------ |
| §8 detecção de mobile                                                                        | `useIsMobile` (`src/hooks/use-mobile.tsx:1-19`), 768px | Existe — mas só controla a sidebar (`src/App.tsx:131-135`, `:170-176`). Não há agenda.                                               |
| §8 faixa de dias + agenda do dia                                                             | —                                                      | Ausente                                                                                                                              |
| §8 semana resumida                                                                           | —                                                      | Ausente                                                                                                                              |
| §8 criação e edição rápida por ações                                                         | —                                                      | Ausente — não há nenhum formulário em `src/`; `Input`/`Label`/`Textarea`/`Select`/`Switch`/`Checkbox` existem e nunca são importados |
| §8 concluir / adiar / reagendar / alterar duração / registrar execução / cancelar ocorrência | —                                                      | Ausente                                                                                                                              |
| §8 "sem arrastar livremente"                                                                 | `cursor-grab` decorativo em `src/App.tsx:918`          | Ausente de fato — e o item da spec 02 torna arrasto exclusivo do desktop                                                             |
| §8 campos avançados no desktop                                                               | —                                                      | Ausente (depende da spec 02)                                                                                                         |
| §6 timer                                                                                     | `"12h 40min"` estático (`src/App.tsx:689`)             | Ausente                                                                                                                              |
| §6 registro de execução                                                                      | —                                                      | Ausente                                                                                                                              |
| §6 desvios planejado × real                                                                  | —                                                      | Ausente                                                                                                                              |

Restrições:

- `minWidth: 1024` / `minHeight: 640` (`src-tauri/tauri.conf.json:22-23`) — o app **nunca** chega a 375px. "Mobile" aqui é a **mesma janela encolhida ao mínimo**, mais o `Sheet`. Isso muda o desenho: não há tela dedicada, há **layout** dedicado. Registrado em D21.
- O `.dark` existe e `roadmap.md:11` ainda não tem controle — esta spec não mexe.
- `useIsMobile` (768px) é o breakpoint já vigente em `src/App.tsx:131`; esta spec o reutiliza em vez de introduzir outro.

---

## 2. Decisões fechadas

### 2.1 Herdadas

A7 (estrutura) · A8 (INV-3, INV-4, INV-7) · D2 (um prazo principal) · D6 (UTC + IANA) · D7 (sobreposição permitida, conflito derivado) · INV-1 (`plan_block.responsibility_id NOT NULL`).

### 2.2 Novas

**D21 — "Mobile" é layout, não plataforma. A agenda é ativada pelo mesmo `useIsMobile` de 768px.**
Decisão: a distinção é o `isMobile` que **já existe** (`src/hooks/use-mobile.tsx:3`, 768px). Abaixo dele, `CalendarScreen` renderiza `MobileAgenda` no lugar de `WeekGrid`. Não há código específico de plataforma, não há `capacitor`, não há rota mobile.
Justificativa: `AGENTS.md:5` define um dashboard único; `src-tauri/tauri.conf.json:22-23` impede a janela de ficar estreita o bastante para um telefone. Construir uma segunda aplicação mobile seria produto novo, não parte do §8 do documento. O §8 pede uma **experiência** diferente, não um artefato diferente — e ela é verificável encolhendo a janela até 767px.
Alternativa rejeitada: Tauri mobile (`src-tauri/gen/android`) — exige `tauri android init`, toolchain Android e muda o produto inteiro; `roadmap.md` não pede.
Consequência: a verificação é `npm run dev` + redimensionar a janela do Chrome para <768px, ou o DevTools em modo dispositivo.

**D22 — No mobile a agenda é a visão **padrão**, e a "semana resumida" é a agenda de 7 dias em lista compacta.**
Decisão: abaixo de 768px, `CalendarMode` é forçado a `"agenda"` e o `ViewSwitch` esconde "Mês". A agenda mostra (1) uma **faixa de dias** horizontal e rolável, com a data de hoje destacada, e (2) a **lista do dia selecionado**. A "visão semanal resumida" de §8 existe como a opção "Semana" na faixa: ao tocar "Semana", a agenda lista os 7 dias seguidos, sem grade, com os itens em duas linhas (hora + título).
Justificativa: §8 diz "agenda compacta, preferencialmente com faixa de dias e agenda do dia selecionado" e que a visão semanal "não tentará reproduzir a grade completa do desktop". Forçar agenda no mobile cumpre as duas frases sem criar uma terceira tela.
Alternativa rejeitada: manter a grade encolhida (quebra a instrução literal de §8); uma lista única sem faixa de dias (perde o "preferencialmente" de §8).
Consequência: `WeekGrid` nunca é montado abaixo de 768px — verificável com `rg "WeekGrid" src` e uma checagem de que o componente não é importado por `MobileAgenda`.

**D23 — Toda manipulação temporal no mobile é por ação explícita, nunca por arraste.**
Decisão: `RescheduleSheet` oferece escolher **dia**, **hora de início** e **duração**, cada um como lista de opções (passo de 15 min, até ±12 h) com um valor atual pré-selecionado. A grade e o `useGridDrag` (spec 02) não são usados.
Justificativa: §8 é explícito: "em vez de arrastar livremente na grade, o mobile oferecerá ações para escolher outro dia, horário ou duração".
Alternativa rejeitada: `<input type="datetime-local">` nativo (o WebView2 abre um picker do SO, quebrando a densidade e a paleta do produto — `PRODUCT.md:21` pune "interfaces vazias com baixa densidade").
Consequência: o mesmo `grid-scale.ts` da spec 02 é reusado para converter a escolha em instante, o que garante que mobile e desktop produzem **o mesmo** valor gravado (D15).

**D24 — O timer é local ao bloco, com acumulador persistido a cada pausa.**
Decisão: `PlanBlock.timerState ∈ {parado, rodando, pausado}` mais `accumulated: Minutes`, já no schema da spec 01. "Rodando" **não** é um `ExecutionRecord`: só o `finish` cria o registro (§6 — os dois métodos, manual e timer, coexistem, e o registro é único). Um timer em andamento é cancelado em 2 h de inatividade. O cronômetro aparece fixo no rodapé da agenda mobile e no painel contextual do desktop.
Justificativa: §6 diz que a duração real pode ser manual **ou** registrada por timer, e que os dois "devem coexistir". Se o timer criasse um `ExecutionRecord` por start, haveria dois registros para a mesma sessão. E persistir só `accumulated` (não o instante de início) sobrevive a fechar o app, sem depender de um registro incompleto.
Alternativa rejeitada: `ExecutionRecord` criado no `start` e atualizado no `finish` (quebra D12, append-only, e perde a sessão se o app fechar no meio).
Consequência: `start_timer` / `pause_timer` / `finish_timer` são três comandos; `finish_timer` grava `ExecutionRecord { source: "timer" }` e zera o acumulador.

**D25 — Desvio é informativo, nunca penalidade.**
Decisão: o painel mostra `planejado 90 min` / `real 40 min` / `desvio −50 min` como **texto neutro**, com `text-muted-foreground`. Não existe cor de erro, não existe badge, não existe meta de weekly goal vinda do desvio. A barra "68% da meta semanal de 18h" do shell (`src/App.tsx:684-700`) **não é populada** por esta spec.
Justificativa: §6 é literal: "A diferença entre planejado e realizado pode alimentar histórico e aprendizagem, mas **não deve ser usada para punir automaticamente a usuária**." `DESIGN.md:9` diz que a cor comunica estado — e desviar não é um estado ruim. §3 reforça: nunca transformar atividade em obrigação sem ação explícita.
Alternativa rejeitada: colorir o desvio de vermelho abaixo de 80% do planejado (é punição visual automática); mostrar uma "nota de produtividade" (contraria §6 e §3).
Consequência: `rg "destructive" src/features/calendar/ui` → **vazio**. A única exceção já existente é o `destructive` do prazo vencido, e ele vem de `isOverdue` (INV-4), que só dispara para `obrigatorio`/`importante` — nunca para `recomendado` ou `opcional`.

**D26 — Registro manual e por timer convergem para o mesmo formulário.**
Decisão: `ExecutionSheet` é **um** formulário, com `source` pré-preenchido (`manual` no atalho, `timer` quando vem do `finish`) e os mesmos campos: início, fim, duração real, resultado, observações. Corrigir um registro grava um **novo** registro com `corrected_by` (D12), nunca um `UPDATE` (INV-3 permanece verdadeiro: o item continua `concluido`, agora com um registro mais).
Justificativa: §6 exige que os dois métodos coexistam; dois formulários divergiriam em campos, e D12 já decidiu que o histórico é append-only.
Alternativa rejeitada: um `durationMin` solto fora do registro (perde quando, e quebra INV-3).
Consequência: `rg "correctedBy" src/features/calendar` presente em `ExecutionSheet` e no repositório de execução.

---

## 3. Componentes e comandos novos

```
src/features/calendar/ui/mobile/
  MobileAgenda.tsx            # D22 — faixa de dias + lista do dia
  DayStrip.tsx                # faixa horizontal rolável, hoje destacado
  QuickActions.tsx            # §8: concluir, adiar, reagendar, duração, registrar, cancelar
  RescheduleSheet.tsx         # D23 — dia / hora / duração
  TimerBar.tsx                # D24 — fixo no rodapé
src/features/calendar/ui/
  ExecutionSheet.tsx          # D26
  RealitySection.tsx          # §6 na interface (seção 3 do painel, D20 da spec 02)
  HistoryList.tsx             # cadeia de registros (D12)
```

Comandos Rust novos (se somam aos 19 da spec 01 → 25):

| Comando            | Assinatura                                                   | Observação                                               |
| ------------------ | ------------------------------------------------------------ | -------------------------------------------------------- |
| `start_timer`      | `({ blockId }): Promise<PlanBlock>`                          | `timer_state = "rodando"`                                |
| `pause_timer`      | `({ blockId }): Promise<PlanBlock>`                          | soma a diferença em `accumulated_min`                    |
| `finish_timer`     | `({ blockId; result?; notes? }): Promise<{ block; record }>` | grava `ExecutionRecord { source: "timer" }`              |
| `cancel_timer`     | `({ blockId }): Promise<PlanBlock>`                          | zera acumulador, sem registro                            |
| `list_executions`  | `({ target }): Promise<ExecutionRecord[]>`                   | mais recente primeiro                                    |
| `reschedule_block` | já existe na spec 01                                         | reutilizado — o mesmo caminho do desktop, garantindo D23 |

`domain/timer.ts` é puro: `accumulate(prev, from, to): Minutes`, `formatDuration(min): "1h 30min"` (pt-BR, sem `Intl.DurationFormat`, que não é uniforme no WebView2).

---

## 4. Bloqueios

- [ ] **BLOQUEIO — MSVC ausente** (`SPEC.md:10`) para a verificação dentro do app. _Mitigação:_ as Fases 3.1 e 3.2 são TypeScript puro (`node --test`), e a parte visual se prova em `npm run dev` com a janela estreita.
- [ ] **BLOQUEIO — "mobile" não é testável como plataforma.** Como D21 estabelece, o alvo é a janela encolhida. Teste real em telefone exigiria `tauri android init` e um toolchain que não existe aqui.
- [ ] _(Sem bloqueio)_ `sonner` (toast) seria útil para "adiado com sucesso"; o set curado não tem (`SPEC.md:325` removeu). **Decisão: não adicionar.** O resultado aparece inline na própria lista, o que evita mais uma dependência e mantém `PRODUCT.md:21` (sem cartão decorativo).

---

## 5. Fases de implementação

### Fase 3.1 — Timer puro e formatação de duração

| Ação  | Detalhe                                                                          |
| ----- | -------------------------------------------------------------------------------- |
| Criar | `src/features/calendar/domain/timer.ts` com `accumulate`, `formatDuration` (D24) |
| Criar | `src/features/calendar/domain/timer.test.ts`                                     |

**Aceite e teste:** `node --test src/features/calendar/domain/timer.test.ts` verde com: `accumulate(30, 10:00, 10:20) = 50`; `formatDuration(90) = "1h 30min"`; `formatDuration(0) = "0min"`; `formatDuration(-10)` nunca ocorre (a função faz `Math.max(0, …)`).

### Fase 3.2 — Desvio e reality (painel, seção 3)

| Ação   | Detalhe                                                                                 |
| ------ | --------------------------------------------------------------------------------------- |
| Criar  | `ui/RealitySection.tsx`, `ui/HistoryList.tsx`, `domain/deviation.ts` + `.test.ts` (D25) |
| Editar | `ContextPanel.tsx` (spec 02) — a seção 3 passa a renderizar `RealitySection`            |

`deviation.ts`: `deviation(planned: Minutes, actual: Minutes): { delta: Minutes; label: string }`.

**Aceite e teste:** `npm run typecheck` e `npm run lint` em 0; `node --test` verde com `deviation(90, 40)` → `{ delta: -50, label: "50 min abaixo do planejado" }`; `deviation(90, 120)` → `+30 min acima`; **`rg "destructive" src/features/calendar/ui` → vazio**; comportamento observável: um bloco com planejado 90 e real 40 mostra o desvio em `text-muted-foreground`, sem ícone de alerta.

### Fase 3.3 — Comandos de timer e execução

| Ação   | Detalhe                                                                                                       |
| ------ | ------------------------------------------------------------------------------------------------------------- |
| Editar | `src-tauri/src/commands.rs` — `start_timer`, `pause_timer`, `finish_timer`, `cancel_timer`, `list_executions` |
| Editar | `src/lib/ipc.ts`, `src/features/calendar/data/execution-repository.ts`                                        |

**Aceite e teste:** `rg "tauri::command" src-tauri/src/commands.rs` → 25; teste TS com repositório em memória: `start_timer` seguido de `finish_timer` produz **um** `ExecutionRecord` com `source: "timer"` e `actualDuration` igual a `accumulated` (D24 — nenhum registro intermediário); `finish_timer` duas vezes no mesmo bloco é idempotente (a segunda não cria registro novo); `npm run typecheck` e `npm run lint` em 0.

### Fase 3.4 — `ExecutionSheet`

| Ação   | Detalhe                                                                                           |
| ------ | ------------------------------------------------------------------------------------------------- |
| Criar  | `ui/ExecutionSheet.tsx` (D26)                                                                     |
| Editar | `domain/commands.ts` (spec 01) — atalho "registrar execução" passa a exigir o formulário completo |

**Aceite e teste:** `npm run typecheck` e `npm run lint` em 0; `node --test` verde: "registrar execução" sem `finishedAt` é rejeitado; corrigir um registro anterior cria um **novo** com `correctedBy` apontando para o antigo e o antigo continua em `list_executions` (D12); `rg "UPDATE execution_record" src-tauri/src/commands.rs` → **vazio**.

### Fase 3.5 — Mobile: layout, faixa de dias e lista

| Ação   | Detalhe                                                                                                         |
| ------ | --------------------------------------------------------------------------------------------------------------- |
| Criar  | `ui/mobile/{MobileAgenda,DayStrip}.tsx`                                                                         |
| Editar | `CalendarScreen.tsx` (spec 02) — `mode` forçado a `"agenda"` quando `useIsMobile()`; `ViewSwitch` esconde "Mês" |
| Editar | `use-mobile.tsx` — sem mudança de contrato; o breakpoint de 768px é reutilizado tal como está                   |

**Aceite e teste:** `npm run typecheck` e `npm run lint` em 0; `rg "WeekGrid" src/features/calendar/ui/mobile` → **vazio** (D22); comportamento observável em `npm run dev` com a janela <768px: faixa de dias rolável com hoje destacado; a lista mostra os itens do dia selecionado em duas linhas (hora, título) e o **estado** com um ponto de cor de camada; nenhuma caixa é arrastável e não há alça de resize em lugar nenhum; a largura mínima de coluna é confortável para toque (≥44px de alvo, conforme `PRODUCT.md:33`).

### Fase 3.6 — Mobile: ações rápidas e reagendamento

| Ação  | Detalhe                                                                                                     |
| ----- | ----------------------------------------------------------------------------------------------------------- |
| Criar | `ui/mobile/{QuickActions,RescheduleSheet}.tsx` (D23)                                                        |
| Criar | `domain/quick-actions.ts` — tabela pura de ações por `origin` × `state`, reusando `item-menu.ts` da spec 02 |
| Criar | `.test.ts`                                                                                                  |

Ações de §8: **Concluir**, **Adiar** (→ `adiado`, e o bloco volta ao estado `planejado` no dia novo), **Reagendar** (`RescheduleSheet`), **Alterar duração** (mesma sheet, campo duração), **Registrar execução**, **Cancelar ocorrência**. Todas escrevem pelo mesmo `applyItemCommand` da spec 01 — o mobile não tem caminho de escrita próprio.

**Aceite e teste:** `npm run typecheck` e `npm run lint` em 0; `node --test src/features/calendar/domain/quick-actions.test.ts` verde: item `google` não oferece "Concluir" nem "Reagendar" (INV-6), oferece "Criar responsabilidade para este compromisso" (§10); item recorrente faz "Cancelar ocorrência" pedir escopo antes de gravar (INV-5); comportamento observável: em <768px, tocar "Reagendar" abre a sheet com dia, hora e duração; escolher 15:00 e 90 min grava exatamente `startsAt` das 15:00 e `endsAt` das 16:30 no fuso `America/Sao_Paulo`, **igual** ao que o arraste no desktop gravaria para a mesma posição (D15 + D23) — verificável pelos dois lados no mesmo banco.

### Fase 3.7 — Timer na interface

| Ação   | Detalhe                                                                     |
| ------ | --------------------------------------------------------------------------- |
| Criar  | `ui/mobile/TimerBar.tsx` (D24) e `ui/TimerBadge.tsx` para o desktop         |
| Editar | `RealitySection.tsx` — botão "Iniciar" / "Pausar" / "Finalizar e registrar" |

**Aceite e teste:** `npm run typecheck` e `npm run lint` em 0; comportamento observável: iniciar o timer num bloco, fechar e reabrir o app, o contador mostra o tempo acumulado e não zero (`accumulated` persistido); "Finalizar e registrar" abre a `ExecutionSheet` **pré-preenchida** com `source: "timer"` e a duração acumulada; "Cancelar timer" zera sem criar registro (`rg` no log do repositório: nenhuma linha de `execution_record` nova); um bloco com `timer_state = "rodando"` exibe um ponto pulsante, que é desligado sob `prefers-reduced-motion` (`src/styles.css:168-176` já desliga animação; a regra a obey é uma `motion-safe:` do Tailwind, não um `@keyframes` próprio).

### Fase 3.8 — Indicadores e resumo do dia

| Ação   | Detalhe                                                                                                        |
| ------ | -------------------------------------------------------------------------------------------------------------- |
| Criar  | `ui/mobile/DaySummary.tsx` — uma linha: itens marcados, a fazer, concluídos, minutos planejados, minutos reais |
| Editar | `TodayView` (`src/App.tsx:476-706`) — **não** muda nesta fase                                                  |

A linha de resumo é a **única** forma de "indicadores" que §8 autoriza no mobile ("O mobile poderá mostrar resumos e indicadores para que a usuária saiba que existem informações mais completas sincronizadas"), e ela é declarativa: sem cor de alerta, sem meta, sem punição (D25).

**Aceite e teste:** `npm run typecheck` e `npm run lint` em 0; `rg "meta|goal|nota|score" src/features/calendar/ui/mobile` → **vazio**; `rg "TodayView" src/App.tsx` → inalterado (a Home é da spec 00/01, não desta); comportamento observável: a linha de resumo bate com a contagem da lista no mesmo dia.

---

## 6. Resumo de arquivos

**Criar**
`src/features/calendar/ui/mobile/{MobileAgenda,DayStrip,QuickActions,RescheduleSheet,TimerBar,DaySummary}.tsx` · `src/features/calendar/ui/{ExecutionSheet,RealitySection,HistoryList,TimerBadge}.tsx` · `src/features/calendar/domain/{timer,deviation,quick-actions}.ts` + `.test.ts`

**Modificar**
`src-tauri/src/commands.rs` (+5 comandos) · `src/lib/ipc.ts` · `src/features/calendar/data/execution-repository.ts` · `src/features/calendar/domain/commands.ts` · `src/features/calendar/ui/{CalendarScreen,ContextPanel,ViewSwitch}.tsx` (spec 02)

**Não tocar**
`src/styles.css` · `src-tauri/tauri.conf.json` · `src/App.tsx` — **exceto** que `TodayView` fica explicitamente **fora** desta spec · `AGENTS.md`

---

## 7. Critérios de aceite finais

1. `npm run typecheck` → 0 e `npm run lint` → `0 problems` em todas as fases.
2. `rg "tauri::command" src-tauri/src/commands.rs` → **25**.
3. `rg "destructive" src/features/calendar/ui` → **vazio** (D25).
4. `rg "UPDATE execution_record" src-tauri/src/commands.rs` → vazio; `rg "correctedBy" src/features/calendar` → presente (D12, D26).
5. `rg "WeekGrid" src/features/calendar/ui/mobile` → vazio (D22).
6. `node --test src/features/calendar/domain/` verde, incluindo `deviation(90, 40) = −50` e `accumulate(30, …, +20) = 50`.
7. Em <768px: nenhum item é arrastável, nenhuma alça aparece, e a semana resumida é uma lista de 7 dias.
8. O mesmo par (dia, hora, duração) produz o **mesmo** `startsAt`/`endsAt` no mobile e no desktop.
9. `rg "type View = " src/App.tsx` → inalterado (`AGENTS.md:5,9`).

---

## 8. Fora de escopo

- Arraste, redimensionamento, colunas de sobreposição, painel de 360px, visões dia/mês/agenda do desktop — spec 02.
- Comandos de retorno aos módulos de origem, "abrir no módulo", vínculo profundo — spec 04.
- OAuth, sincronização, "sincronizar agora", exclusão cruzada, resolução de conflito, indicador de estado de sync — spec 05.
- Sugestão de bloco, aceitar/ajustar/recusar — spec 06.
- Notificações push, lembretes de prazo, alarme no fim do timer.
- Timer com categorias, metas, Pomodoro, Pomodoros de revisão do módulo Estudos — `roadmap.md` não lista; produto novo.
- Insights, médias, tendências de desvio — §6 diz que o desvio **pode** alimentar aprendizagem, e isso é produto posterior, não parte de §11.
- Tauri mobile / Android / iOS — ver D21.
- `sonner`/toast — ver §4.
- Repopular a `TodayView` (`src/App.tsx:476-706`) com dados reais — segue como backlog; esta spec só **não** a impede.
