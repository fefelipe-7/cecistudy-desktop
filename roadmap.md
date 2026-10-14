# Roadmap do app desktop (Campus)

Plano de implementação do **desktop**, conciliando backend Rust e frontend. O
roadmap do **grupo** vive em [`../roadmap.md`](../roadmap.md); aqui só entra o que
é do desktop.

Regra de leitura: este plano é uma **fatia vertical**. Cada fase move backend e
frontend juntos, para que uma regra só exista quando houver chamador em produção.

> Origem: [`../spec-referencial-workspace-academico.md`](../spec-referencial-workspace-academico.md)
> §2 (line 31-54, os nove módulos) e o roadmap do grupo, fases F7 e F8.

## Estado verificado em 2026-10-13

Números gerados, não copiados. A fonte é o comando.

| Medida | Comando | Resultado |
|---|---|---|
| Testes TS | `npm run test` | **279 pass, 0 fail** |
| Golden | `npm run test:golden` | **6 pass, 0 fail** |
| Fronteiras | `npm run check:boundaries` | **69 arquivos, 30 tabelas, 8 regras** |
| Testes Rust | `cd src-tauri; cargo test --lib --no-default-features` | **134 pass, 0 fail** (+1 `#[ignore]`: gerador do golden de FSRS; exige MinGW com `libgcc_eh`; ver D0.1) |
| Comandos IPC | `Select-String -Path src-tauri\src\lib.rs -Pattern "commands::" -Encoding UTF8 \| Measure-Object` | **48** (20 do Calendário + 11 da Faculdade + 4 dos Estudos + **13** do TCC) |
| Gate completo | `npm run verify` | **EXIT 0** (2026-10-13, com o PATH reordenado — D0.1) |
| Repositório | `git log --oneline` | **4 commits** (2026-09-28), branch `master` |

**Backend hoje:** `domain.rs` (`TRANSITIONS`, `transition_allowed`, `is_overdue`,
`mark_overlaps`, `can_generate_delay`), `commands.rs` (48 comandos — 20 do
Calendário + 11 do Faculdade + 4 do Estudos + 13 do TCC, regra imposta na escrita
e na leitura, recorrência pelo crate `rrule` RFC 5545), `db.rs` (5 migrações, 30
tabelas: 12 do Calendário + 7 do Faculdade + 3 do Estudos + 8 do TCC). C5 e C6
fechados; INV-1..7 implementadas. Falta a camada `sync/` (ADR-009 item 3) e há
três tabelas sem comando: `external_link`, `suggestion`, `sync_cursor`.

**Frontend hoje:** o **shell** dos nove módulos do §2 existe desde D2 (2026-10-09):
[`src/shell/navigation.ts`](./src/shell/navigation.ts) é a tabela declarativa
(`MODULOS`, `WORKSPACES`, `UTILITARIOS`, `CANDIDATOS_RECUSADOS`) e
[`src/App.tsx`](./src/App.tsx) é o shell que a lê. Quatro módulos estão
**prontos**: **Calendário** (5 visões ligadas ao Rust), **Faculdade**
(`FaculdadeScreen`: semestres, disciplinas com média derivada, avaliações e nota,
encerramento), **Estudos** (`EstudosScreen`: fila do dia do FSRS com revisão e
seletor de retenção — padrão 0,9 provisório **[P]**) e **TCC & Projetos**
(`TccScreen`: projetos, painel do documento com árvore, versão, itens a resolver
e Base de pesquisa; o botão **Escrever** abre o modo de editor,
`EditorDoDocumento`, desde D6.2 em 2026-10-13). Os outros **5** aparecem
declarados, clicáveis, com estado honesto "em construção". A regra vive em
[`docs/espacos/00-shell-e-navegacao.md`](./docs/espacos/00-shell-e-navegacao.md).

**Divergências doc ↔ código a corrigir (Natureza I):**

| Onde | O que o texto diz | O que é verdade |
|---|---|---|
| [`AGENTS.md:5`](./AGENTS.md) | "keep the study workspace as a single stateful dashboard route" | **corrigido em D2** (2026-10-09): o item agora descreve o shell e a tabela de navegação |
| [`docs/calendario/00-fundacao.md`](./docs/calendario/00-fundacao.md) | `App.tsx` "com 1137 linhas com todos os componentes" | hoje vale `src/shell/SidebarNav` + `features/calendar/ui/CalendarScreen` |
| [`roadmap.md`](./roadmap.md) (este, antes) | contagens de teste antigas e "instalar o compilador da Microsoft" | toolchain é `windows-gnu` + MinGW-w64 (D83); contagens, no quadro "Estado verificado" acima |
| [`AGENTS.md`](../AGENTS.md) da raiz | "quatro arquivos" e "o Rust cumpre só a persistência" | arquivos do Calendário + módulo `materia/`; o domínio já é imposto |
| [`AGENTS.md`](./AGENTS.md) §Gate | contagens literais defasadas | **corrigido em 2026-10-13 (D0.5):** a tabela agora diz **como medir**, sem número |

## D0 — Destravar a máquina e o repositório

**Objetivo:** antes de mover qualquer regra, o gate tem de rodar e o trabalho tem
de estar no git.

| Item | Camada | Ação | Spec/Débito | Gate |
|---|---|---|---|---|
| D0.1 | ambos | **Fechado (2026-10-13).** Os dois MinGW já estavam no PATH de **usuário**, só em ordem errada: o `llvm-mingw` (sem `libgcc_eh`) vinha antes do `msys64\ucrt64` (com `libgcc_eh.a`). Reordenado para o `msys64\ucrt64\bin` vir primeiro — **nada instalado** | [`SPEC-D-005`](../docs/specs/desktop/SPEC-D-005-executar-o-backend-rust.md) D83/D84, C15 | `npm run verify` completo → **EXIT 0**; `cargo test --lib --no-default-features` = 132 pass, 0 fail (+1 `#[ignore]`) |
| D0.2 | ambos | **Cancelado (2026-10-09):** o app é trabalhado sem git neste checkout, por decisão da dona do produto. Sem commit. | C7, roadmap F0.3 | — |
| D0.3 | grupo | **Fechado (2026-10-10):** `package.json` e `package-lock.json` removidos da raiz, com confirmação da dona — o `ponytail` não tinha nenhum consumidor (`rg` sem uso em código, script ou doc) | C13, F0.9 | `Test-Path ..\package.json` = `False` |
| D0.4 | doc | **Fechado (2026-10-13).** A tabela "Divergências" foi reconciliada com o código: os números literais saíram (o comando é a fonte) e a menção ao compilador da Microsoft saiu | C11, C12, F1.6 | as linhas apontam para o quadro "Estado verificado"; sem contagem sem comando |
| D0.5 | doc | **Fechado (2026-10-13).** A tabela do gate no [`AGENTS.md`](./AGENTS.md) trocou a coluna de números pela coluna "Como medir"; o snapshot datado mora aqui | ADR-004 | nenhum número de teste sem o comando ao lado |

**Gate:** `npm run verify` = `EXIT 0` (provado em 2026-10-13 com o PATH reordenado);
raiz sem `package.json`. O "working tree commitado" **não** vale aqui: por D0.2 este
checkout trabalha sem git.

## D1 — Fechar o backend no caminho da UI (F7.2/F7.4 + INV-3)

**Objetivo:** nenhum arquivo de regra TS no caminho de produção, e concluir de
fato grava `execution_record` (INV-3).

**Fechado em 2026-10-09.** Os dois gaps eram:
- O botão "Concluir" chamava `set_occurrence_state`
  ([`src/features/calendar/ui/CalendarScreen.tsx`](./src/features/calendar/ui/CalendarScreen.tsx)),
  **não** `complete_item` — o caminho de `ExecutionRecord` existia na ponte
  ([`src/features/calendar/data/bridge.ts`](./src/features/calendar/data/bridge.ts))
  mas não estava ligado à tela.
- `state-machine.ts` e `overdue.ts` ainda eram importados pelo `CalendarScreen`.

O que fechou, por item:

| Item | Camada | O que foi feito |
|---|---|---|
| D1.1 | front | Novo [`src/features/calendar/data/complete.ts`](./src/features/calendar/data/complete.ts) monta o `execution_record` e chama `complete_item({kind:"occurrence"})`; a tela usa isso no "Concluir". Teste em `complete.test.ts` prova que a escrita carrega o registro. |
| D1.2 | front | `CalendarScreen` consome `item.allowedTransitions` e `Responsibility.isOverdue`; os imports de `domain/state-machine.ts` e `domain/overdue.ts` saíram do caminho de produção. |
| D1.3 | back | `GridItem.allowed_transitions` derivado de `destinations_of` em `list_events_in_window_core`; teste Rust `leitura_da_grade_traz_os_destinos_validos`. |
| D1.4 | gate | 6ª regra em `check-boundaries.mjs`: `ui/`, `data/` e `sync/` não importam os quatro espelhos. |
| D1.5 | doc | Os quatro espelhos ficam **só** para o golden e o teste sem toolchain; registrado no [`AGENTS.md`](../AGENTS.md) da raiz e no [`AGENTS.md`](./AGENTS.md) do app. |

| Item | Camada | Ação | Spec/Débito | Gate |
|---|---|---|---|---|
| D1.1 | front | Ligar `complete_item`/`record_execution` na tela; concluir grava o registro | INV-3, [`SPEC-D-001`](../docs/specs/desktop/SPEC-D-001-estado-e-sobreposicao-no-caminho-de-escrita.md) | teste prova que concluir cria `execution_record` |
| D1.2 | front | Remover `state-machine.ts` e `overdue.ts` do caminho de produção; consumir `GridItem`/`Responsibility` derivados do backend | F7.4 | `rg "state-machine\|overdue" src/features/calendar/ui` retorna vazio |
| D1.3 | back | Expor no read-model os destinos válidos (`destinations_of` já existe em `domain.rs`) para a UI decidir botões sem reimplementar regra | F7.4, SPEC-D-001 | golden de transições cobre o payload |
| D1.4 | gate | Nova regra no `check-boundaries`: nenhum arquivo de `domain/` importado por `ui/` ou `data/` | F7.6 | gate falha se `domain/` for importado pela tela |
| D1.5 | doc | Decidir, arquivo a arquivo, o destino dos quatro espelhos | F7.2 | decisão registrada como `D<n>` com alternativa rejeitada |

## D2 — Shell e Workspace Acadêmico (F8.1 · SPEC-D-006)

**Fechado em 2026-10-09** (D2.3 adiada — ver abaixo). O que fechou, por item:

| Item | Camada | O que foi feito |
|---|---|---|
| D2.1 | front | [`src/shell/navigation.ts`](./src/shell/navigation.ts) tem `MODULOS` (9 entradas na ordem do §2, com `dono`, `contexto`, `estado`, `descricao`, `destinos`), `WORKSPACES`, `UTILITARIOS` e `CANDIDATOS_RECUSADOS`. Contrato verificado por [`navigation.test.ts`](./src/shell/navigation.test.ts). |
| D2.2 | front | [`src/App.tsx`](./src/App.tsx) deixou de ser o dashboard de 1.090 linhas com mock de disciplinas/tarefas/kanban; virou o shell: seletor de workspace no topo, sidebar de nove módulos, rodapé Ceci·Busca·Notificações, busca por módulo. O Calendário é o único módulo `pronto`; os demais renderizam estado honesto "em construção". |
| D2.3 | front | **Adiada.** `SPEC-D-006` §4 põe "a Home como aggregation com dados de todos os módulos" **fora de escopo**: ela depende de todos os módulos existirem. A Home existe declarada e honesta; o dado real entra em D4, quando houver módulos que a alimentem. |
| doc | doc | [`docs/espacos/00-shell-e-navegacao.md`](./docs/espacos/00-shell-e-navegacao.md) criado com `D62..D64` (justificativa, alternativa rejeitada, consequência). |
| gate | UI | `minWidth` de `src-tauri/tauri.conf.json` corrigido de 1024 para **1120** (coluna de 836 px + sidebar de 264 px), achado D-15. |
| doc | doc | [`AGENTS.md:5`](./AGENTS.md) e `:9` reescritos: "modo dentro do módulo" vs "destino entre módulos". |

| Item | Camada | Ação | Spec | Gate |
|---|---|---|---|---|
| D2.1 | front | Transformar [`src/shell/navigation.ts`](./src/shell/navigation.ts) na tabela real dos nove módulos, com os nomes fechados | [`SPEC-D-006`](../docs/specs/desktop/SPEC-D-006-shell-navegacao-e-workspace-academico.md) D62/D63 | tabela exportada e importada pela shell |
| D2.2 | front | Reescrever `App.tsx`: sair do enum mock de 5 views para o shell + workspace; aposentar disciplinas/tarefas hardcoded | SPEC-D-006 | shell tem as nove entradas do §2 |
| D2.3 | front | Home como módulo próprio, com dado real | SPEC-D-006 | Home não usa mock |

**Gate:** o shell tem as nove entradas do §2 com os nomes fechados, sem mock
hardcoded em `App.tsx`. Medido em 2026-10-10: `npm run verify` verde (252 TS / 6
golden / 60 fronteiras / 129 Rust).

## D3 — Dono do dado, fronteiras e sync (F8.2 · SPEC-D-007 + ADR-009)

| Item | Camada | Ação | Spec | Gate | Estado |
|---|---|---|---|---|---|
| D3.1 | back | Criar `src-tauri/src/sync/` com a tradução de formatos | ADR-009 item 3 | módulo existe com teste | **bloqueado** — ADR-009 pendências 1 (mapeamento) e 2 (envelope) abertas |
| D3.2 | back | Dar comando às tabelas órfãs: `external_link`, `suggestion`, `sync_cursor` | [`SPEC-D-012`](../docs/specs/desktop/SPEC-D-012-ceci-no-desktop.md) | nenhuma tabela do schema sem chamador | **bloqueado** — são spec 05 (sync Google) e spec 06 / `SPEC-D-012` D80-D82 |
| D3.3 | gate | Fronteira de dono verificável por comando | [`SPEC-D-007`](../docs/specs/desktop/SPEC-D-007-dono-do-dado-e-fronteira-entre-modulos.md) | §3.1 verificável por teste | **fechado em 2026-10-09 (metade TS)** |

**Fechado em 2026-10-09 (D3.3, metade TS — fatia determinística):**

- [`docs/espacos/01-fronteiras.md`](./docs/espacos/01-fronteiras.md) criado:
  matriz de dono do dado de §3.1 sem reordenação, `D65` (a matriz é a lista),
  `D66` (referência viva / snapshot / cópia editorial) e `D67` (fronteira
  verificável por comando). A decisão irmã é
  [`00-shell-e-navegacao.md`](./docs/espacos/00-shell-e-navegacao.md).
- **Regra 7 (`fronteira-de-modulo`)** em
  [`scripts/check-boundaries.mjs`](./scripts/check-boundaries.mjs): `ui/` de um
  módulo não importa `data/` de outro. Caso negativo verificado (import cruzado
  temporário reprovou o gate com exit 1; removido em seguida).

**Bloqueado, por decisão de produto ainda aberta:** D3.1 e D3.2. A tradução de
formatos exige o mapeamento entidade-a-entidade e o formato do envelope, ambos
declarados **não decididos** na ADR-009; os comandos para as três tabelas órfãs
antecipam a spec 05 (sync Google, sem credenciais) e a spec 06 / `SPEC-D-012`. A
metade **Rust** do gate de D67 (um módulo Rust por dono de dado) não tem o que
cruzar enquanto só existir um módulo Rust — entra com o segundo.

## D4 — Módulos verticais

Cada módulo: schema + `commands` em Rust → bridge + tipos em TS → UI. Ordem por
dependência e por desbloqueio.

| Item | Camada | Módulo | Spec | Bloqueio |
|---|---|---|---|---|
| D4.1 | ambos | Faculdade + Estudos (motor de notas + FSRS **em Rust**) | [`SPEC-D-010`](../docs/specs/desktop/SPEC-D-010-motor-de-notas-e-fsrs.md), SPEC-C-010/C-011 | — |
| D4.2 | ambos | TCC & Projetos + Biblioteca (editor paginado, round-trip DOCX) | [`SPEC-D-011`](../docs/specs/desktop/SPEC-D-011-editor-do-tcc-e-round-trip-docx.md), [`SPEC-D-013`](../docs/specs/desktop/SPEC-D-013-modelo-de-dados-do-documento.md), ADR-010 | **em andamento** — modelo resolvido por `SPEC-D-013` e editor paginado entregue (D6.1/D6.2); falta o round-trip DOCX (D6.3/D6.4) |
| D4.3 | ambos | Conhecimento | [`SPEC-C-006`](../docs/specs/comum/SPEC-C-006-matriz-de-dono-de-dado-e-reuso-sem-duplicacao.md) | — |
| D4.4 | ambos | Ceci: lê, sugere, **nunca escreve** | [`SPEC-D-012`](../docs/specs/desktop/SPEC-D-012-ceci-no-desktop.md), SPEC-C-009 | — |
| D4.5 | ambos | Estágio — camada clínica (sigilo, criptografia, lista fechada, fora da IA) | [`SPEC-D-008`](../docs/specs/desktop/SPEC-D-008-camada-clinica-do-estagio.md) | **`[A]`** normas do CFP/instituição |
| D4.6 | ambos | Sala de treino (CPU local, voz, continuidade semanal) | [`SPEC-D-009`](../docs/specs/desktop/SPEC-D-009-sala-de-treino.md) | **`[A]`** premissas técnicas; **`[P]`** enxugamento de subtelas |

**Gate:** cada módulo entregue tem comando Rust com chamador, UI ligada por IPC, e
as `[A]`/`[P]` acima continuam abertas (agente não fecha `[A]`).

**D4.1 — estado (2026-10-09, F8 fechado).** Faculdade: motor e regra em
`src-tauri/src/materia/` (18 testes), schema em `migrations/0003_faculdade.sql`
(7 tabelas), onze comandos IPC com a guarda I7 no caminho de escrita e a ponte
`src/features/faculdade/data/bridge.ts` — 6 testes de caminho em `commands.rs`,
o golden de média fechado (`golden/media.jsonl`, 10 casos, lido por teste em
`materia/media.rs`) e a tela v1 (`FaculdadeScreen`: semestres, disciplinas com
média derivada, avaliações, nota, encerramento). Estudos: backend e tela v1 —
crate `fsrs 6.6.2` (BSD-3-Clause) registrada no `Cargo.toml`, motor e cláusula
do vencimento em `src-tauri/src/estudos/fsrs.rs`, schema em
`migrations/0004_estudos.sql` (`conteudo`, `cartao`, `revisao`), golden
`golden/fsrs.jsonl` (8 casos), quatro comandos IPC (4 testes de caminho), a
ponte `src/features/estudos/data/bridge.ts` e a `EstudosScreen` (fila do dia +
revisão, com seletor de retenção — **0,9 provisório, padrão [P]**, decisão
relatada pela dona em 2026-10-09). Fora da v1: as quatro telas de §4.4, trilhas
e o modo preparação de prova (**[P]**).

**D4.2 — estado (2026-10-10, v1 do módulo entregue).** Dois artefatos, em dois
tempos.

Primeiro, o **contrato**: `docs/espacos/tcc.md` existe com `D77` (modelo interno e
página derivada), `D78` (round-trip DOCX) e `D79` (orientação fora do app; a Ceci
não escreve), a **origem em §4.7**, o **contrato de tolerância do round-trip**
(13 itens, incluindo a revisão de alterações do Word) e as 15 invariantes de
`SPEC-D-011` §6.

Depois, o **modelo de dado** — antes a dependência que bloqueava D4.2 — foi
escrito e decidido: `docs/espacos/tcc-modelo-do-documento.md` (`D85` árvore de
nós em lista de adjacência ordenada, `D86` paginação pura com medidor injetado,
`D87` versão por snapshot em toda escrita confirmada, `D88` tabela própria de
`item_a_resolver`), espelhado em
[`SPEC-D-013`](../docs/specs/desktop/SPEC-D-013-modelo-de-dados-do-documento.md).

A **v1 vertical** (Rust → bridge → UI) saiu: `migrations/0005_tcc.sql` (8 tabelas
+ índices), motor puro em `src-tauri/src/tcc/` (documento, medidor, paginação,
versão, item — 28 testes), **9 comandos IPC** com a guarda de escrita, ponte
`src/features/tcc/data/bridge.ts`, `view.ts`/`view.test.ts` e a `TccScreen`
(projetos, painel do documento com árvore, versão, itens a resolver). O
`get_documento_do_projeto` foi adicionado para a tela abrir o documento do projeto
sem copiar o id. **Fora da v1:** o editor paginado de fato e o round-trip DOCX,
que continuam em `SPEC-D-011`; a tela apenas lê e sinaliza. **Atualização:** o
editor paginado chegou em D6.1/D6.2 (2026-10-13); o round-trip continua em
D6.3/D6.4.

## D5 — Paridade contínua (F6)

| Item | Camada | Ação |
|---|---|---|
| D5.1 | ambos | Todo termo novo de domínio entra no [glossário](../contratos/glossario-do-grupo.md) antes de virar nome de código |
| D5.2 | ambos | Todo `D<n>` novo passa pelo procedimento dos 6 passos do contrato de portabilidade |
| D5.3 | ambos | Todo valor espelhado do Rust carrega `Provenance`; regra nova nasce em Rust |
| D5.4 | ambos | Golden como contrato entre Rust e espelho, não cópia |

## D6 — SPEC-D-011: editor paginado e round-trip DOCX

**Objetivo:** fechar a `SPEC-D-011` em fatias, cada uma com chamador em produção.
A fase inteira não fecha numa vez: o editor paginado é determinístico; o round-trip
DOCX depende de uma biblioteca que ainda não foi escolhida por critério.

| Item | Camada | Ação | Spec/Débito | Estado |
|---|---|---|---|---|
| D6.1 | back+front | `paginar_documento` expõe a paginação derivada; `paginarDocumento` na ponte | D77/D86, I2 | **fechado em 2026-10-13** |
| D6.2 | front | Editor paginado: `editar.ts` (funções puras) + `EditorDoDocumento.tsx`, autosave com debounce, botão **Escrever** | D77, I1, I15 | **fechado em 2026-10-13** |
| D6.3 | ambos | Exportar DOCX/PDF | I3 | **bloqueado** — falta a biblioteca DOCX |
| D6.4 | ambos | Importar DOCX preservando a revisão de alterações (`w:ins`/`w:del`) | D78, I4..I8 | **bloqueado** — a biblioteca precisa expor autor e data |
| D6.5 | ambos | Lista de referências gerada do documento | I11, I13, I14 | **bloqueado** — Biblioteca e Conhecimento não existem |
| D6.6 | gate | Cobertura de contrato do editor | SPEC-D-011 §9 | **aberto** |

**D6.1 — fechado em 2026-10-13.** O motor `paginar` e o `MedidorDeterministico`
já existiam em `src-tauri/src/tcc/` desde a v1 do módulo, **sem chamador**: o que
faltava era a exposição. `paginar_documento_core` reusa `get_documento_core`,
aceita `geometria` opcional (o padrão é o A4-ABNT de `Geometria::a4_abnt()`) e
**não grava nada**. O retorno `PaginacaoDaTela` achata `Paginacao` com
`#[serde(flatten)]` e **declara** o medidor e a geometria aplicada — `D86` proíbe
paginação silenciosamente aproximada, então o número de páginas exibido vem
sempre acompanhado de qual medidor o produziu.

**D6.2 — fechado em 2026-10-13.** Duas peças, deliberadamente separadas.

- [`editar.ts`](./src/features/tcc/ui/editar.ts) são **funções puras** da árvore
  (`atualizarTitulo`, `atualizarTexto`, `remover`, `mover`, `inserirIrmao`,
  `adicionarFilho`), com 11 testes. É **operação de UI**: a validação de verdade
  continua no Rust (`linhas_da_arvore`/`salvar_documento`, `D85`) e a geometria
  A4-ABNT é aplicada lá, no motor `paginar`. Nenhuma regra do modelo é re-derivada
  no TS. Na v1 o parágrafo é **texto simples**: `atualizarTexto` troca o `inline`
  por um único trecho sem marcas, e isso está declarado no arquivo e coberto por
  teste — o editor não finge que preserva a marcação inline.

- [`EditorDoDocumento.tsx`](./src/features/tcc/ui/EditorDoDocumento.tsx) é a tela
  do modo Escrita (modo interno ao módulo, `SPEC-D-006` D62): edita título e
  parágrafo em qualquer profundidade, insere, remove e move nós, e mostra citação
  e referência viva como **vínculo de leitura** (`I13`: a citação vem da Biblioteca
  e a referência viva, do Conhecimento — nenhum dos dois existe, então o botão de
  inserir não existe). O autosave coalesce a digitação numa escrita (`D87`,
  debounce de 800 ms) e o número de páginas é recalculado depois da escrita,
  sempre rotulado com o medidor (`D86`).

**Por que D6.3/D6.4 esperam.** O critério duro de `D78` é preservar a revisão de
alterações do Word. Isso exige uma biblioteca DOCX que exponha `w:ins` e `w:del`
**com autor e data**; uma biblioteca que só lê texto entrega o documento e perde a
revisão, que é exatamente o que `D78` proíbe. Escolher por critério é item de
fase, não detalhe de implementação.

## Calendário — acabamento (app local)

| Item | Camada | Ação |
|---|---|---|
| CA.1 | front | Arraste, redimensionamento, criação e edição na grade — Fase 2.4 da spec 02 |
| CA.2 | front | Corrigir o overflow horizontal abaixo de ~480px de largura |

## Infra e aparência (app local)

| Item | Camada | Ação |
|---|---|---|
| IA.1 | front | Habilitar o modo escuro — o bloco `.dark` existe em `src/styles.css`, falta o controle |
| IA.2 | front | Ícone dedicado do produto — hoje usa o favicon de 20 KB |
| IA.3 | front | Auto-hospedar a Inter (hoje é requisição ao Google Fonts a cada launch) |

## Bloqueios que o agente não fecha

| Bloqueio | Origem | Quem resolve |
|---|---|---|
| Normas do CFP e da instituição sobre registro de atendimento de estágio | spec referencial §4.8 linha 454-461, **`[A]`** | a supervisão, via a dona do produto |
| Premissas técnicas da Sala de treino | §4.9 e §8, **`[A]`** | protótipo |
| Enxugamento de subtelas (4 telas de Estudos, 5 visões do Calendário, subtelas de Conhecimento/TCC) | §8 linha 585, **`[P]`** | a dona do produto |
| Licença do repositório (`LICENSE`) | roadmap do grupo F3.2 | decisão da dona do produto |
| Dependência `ponytail` na raiz | C13 | **FECHADO (2026-10-10):** `package.json` e `package-lock.json` removidos da raiz com confirmação da dona; o `ponytail` não tinha nenhum consumidor. `Test-Path ..\package.json` = `False` |

## Feito (histórico)

- [x] Migrar a stack para React + Vite + npm e remover o framework SSR com Nitro.
- [x] Empacotar como app desktop com Tauri v2.
- [x] Remover código morto: server entry, roteador, telemetria e 28 componentes shadcn não usados.
- [x] Zerar os 2 warnings de `react-refresh/only-export-components` do baseline.
- [x] Corrigir os defeitos do shell: busca rápida com focus trap, controles inertes, checkbox sem estado acessível, tarefa duplicada no Kanban, contadores divergentes, ARIA inválido e `<a href="/">` no `ErrorBoundary`.
- [x] Fundação do módulo Calendário (`docs/calendario/00-fundacao.md`): decisões §12 fechadas, `src/features/calendar/{domain,data,ui,sync}/` criada, `src/lib/ipc.ts` e o vocabulário de camadas e níveis.
- [x] Domínio do Calendário no Rust: tipos, máquina de estados, recorrência por RRULE (crate `rrule`), 12 tabelas e 20 comandos Tauri.
- [x] Estrutura das cinco visões do Calendário (D15bis) e a geometria da grade (`domain/grid-scale.ts`).
- [x] C5 e C6 fechados em 2026-10-03: transição ilegal recusada na escrita; motor de recorrência único com golden.
- [x] Gate único `npm run verify` com `typecheck`, `lint`, `test`, `test:golden`, `check:boundaries` e quatro passos Rust.
- [x] D0 fechado em 2026-10-09 (D0.2 cancelado, sem git): textos de divergência corrigidos, gate Rust reprodutível (60 pass). **Ressalva (reverificado em 2026-10-10):** o item D0.3 não se sustentou — `package.json`/`package-lock.json` voltaram à raiz; C13 segue **ABERTO** (ver "Bloqueios que o agente não fecha").
- [x] D1 fechado em 2026-10-09: concluir passa por `complete_item` e grava `execution_record` (INV-3); `GridItem.allowedTransitions` e `Responsibility.isOverdue` vêm derivados do backend; 6ª regra de fronteira impede os quatro espelhos no caminho de produção; os espelhos ficam só para golden e teste.
