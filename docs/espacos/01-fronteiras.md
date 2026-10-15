# 01 · Dono do dado e fronteira entre módulos

> Fonte de verdade no app para a [`SPEC-D-007`](../../../docs/specs/desktop/SPEC-D-007-dono-do-dado-e-fronteira-entre-modulos.md).
> Cópia de grupo: `docs/specs/desktop/SPEC-D-007-dono-do-dado-e-fronteira-entre-modulos.md`.
> Decisões do app: `D65`, `D66`, `D67`.
> Estado: matriz e **os dois gates** implementados em 2026-10-09 — o
> TypeScript (regra 7) e o de dono no backend (regra 8). Os itens de sync
> (`src-tauri/src/sync/`) seguem bloqueados — ver §Bloqueios.

Este documento é a decisão de **módulo** das fronteiras, ao lado de
`00-shell-e-navegacao.md` (`SPEC-D-006`). Ele não renumera `D1..D46` de
`docs/calendario/`. Onde o índice de grupo errar sobre o app, o app está certo.

## Contexto

O produto tem nove módulos, e §1.3 do referencial é a regra que impede a
"coleção de ferramentas": **cada dado tem um dono**. O módulo dono edita e
guarda; os outros mostram por referência viva; nenhum módulo duplica conteúdo de
outro.

O desktop hoje tem **um** módulo (Calendário), e ele já é dono de quase tudo que
§3.1 lhe atribui. A metade mecânica do padrão já existe: nenhum nome de tabela
atravessa a fronteira (`src/features/calendar/data/bridge.ts:3-5` e o cabeçalho
de [`calendario/comandos.rs`](../../src-tauri/src/calendario/comandos.rs)) e
`data/` conhece o schema, não o SQL. O que
faltava era a metade de **produto**: dizer quem é dono de quê, no app, de forma
que um gate leia.

## A matriz de dono do dado (§3.1, sem reordenação)

Transcrição da matriz de §3.1 do referencial, sem reordenar. Esta lista é a
**única** fonte de propriedade de dado do desktop: cada módulo novo entra nela
**antes** de ter tela, e toda tabela nova tem uma linha que diz de quem ela é.

| Dado | Módulo dono | Quem só mostra (por referência) |
|---|---|---|
| Semestre, Disciplina, Aula, Frequência | Faculdade | Calendário, Home, Estudos |
| Avaliação (prova, trabalho, apresentação, entrega) | Faculdade | Calendário, Home, Estudos |
| Regra de média / aprovação | Faculdade (padrão em Configurações) | Notas (visões) |
| Responsabilidade (item sem nota) | Calendário | Faculdade, Home |
| Evento, Bloco de planejamento, Ocorrência, Registro de execução | Calendário | Home, Estudos, Estágio |
| **Conteúdo** (tópico de estudo) | **Estudos** | Faculdade (visão por disciplina), TCC, Biblioteca |
| Sessão de estudo, Flashcard, Questão, Trilha, histórico FSRS | Estudos | Home, Calendário |
| Item de biblioteca, **Referência**, anexo | **Biblioteca** | Conhecimento, TCC, Estudos |
| Fragmento, Conceito, Relação | Conhecimento | Estudos, TCC, Biblioteca |
| Projeto, Base de pesquisa, Documento, Versão, Saída | TCC & Projetos | Calendário (prazos), Biblioteca (onde é citado) |
| Horas, Supervisão, Entrega de estágio | Estágio (camada acadêmica) | Calendário, Home |
| Registro clínico de estágio | Estágio (camada clínica, só desktop) | **ninguém** |
| Personagem simulado, Sessão simulada, Feedback, Desempenho | Sala de treino | Home ("Continuar") |
| Documentos emitidos (Profissional) | módulo de origem; registro em Documentos | Documentos |

As tabelas que existem hoje em `src-tauri/migrations/` pertencem todas a linhas
desta matriz: as do Calendário (`layer`, `calendar_event`, `event_recurrence`,
`event_occurrence`, `responsibility`, `responsibility_step`, `plan_block`,
`execution_record`), a de sugestão de bloco (`suggestion`) e as de integração
(`external_link`, `sync_cursor`, spec 05); `setting` é preferência local, sem
dono de módulo. Desde 2026-10-09 o Faculdade tem `semestre`, `disciplina` (com a
frequência), `avaliacao`, `nota` e as tabelas de regra (`regra_padrao`,
`regra_disciplina`, `regra_congelada`), que caem nas linhas "Semestre,
Disciplina, Frequência", "Avaliação" e "Regra de média / aprovação"; e o
Estudos tem `conteudo`, `cartao` (o estado FSRS por cartão, D76) e `revisao`
(o histórico), que caem nas linhas "Conteúdo" e "Sessão de estudo, Flashcard,
Questão, Trilha, histórico FSRS".

## D65 · A matriz de §3.1 é a lista de dono do dado do app, e ela é a única

- **Decisão.** A matriz desta seção é a **única** fonte de verdade de propriedade
  de dado no desktop, com as colunas `dado`, `dono` e `quem só mostra`. Nenhum
  módulo cria tabela, campo ou arquivo de conteúdo sem uma linha que diga quem é
  o dono. A ausência da linha é falha de aceite, e o destino do dado é o dono.
- **Justificativa.** §1.1 **[D]** nega explicitamente a "coleção de ferramentas":
  *"os módulos se conhecem e trocam dados por vínculos, nunca por cópias"*. Matriz
  sem lista **no app** é matriz que vive num documento de produto e morre quando o
  primeiro módulo novo a contradiz. "Lista antes da tela" responde "isso é meu?"
  antes de escrever a migration.
- **Alternativa rejeitada.** (a) Não declarar e deixar cada módulo assumir o que
  parece seu — todos assumiriam o **Conteúdo**, que §4.4 dá a Estudos. (b) Declarar
  só as linhas onde há dúvida hoje — lista parcial não é lista: a função dela é a
  checagem. (c) Declarar só no documento de produto — é o que acontece hoje com
  `docs/calendario/04-modulos-origem.md`, e nenhum código obedece.
- **Consequência aceita.** A lista é manutenção: alguém tem de lembrar de
  atualizá-la. O custo cai porque a lista é a **fonte** que o gate lê — o
  esquecimento aparece no gate, não em produção.

## D66 · Quem só mostra usa referência viva, e o modo de vínculo é escolhido pela usuária

- **Decisão.** Três modos de vínculo, com semântica explícita, e **nenhum** quarto:

  | Modo | Semântica | Quando |
  |---|---|---|
  | **Referência viva** | o destino lê do dono a cada leitura; mudar no dono muda aqui | padrão; é o modo de §3.1 |
  | **Snapshot** | o destino guarda o valor no momento da vinculação, com data; não muda mais | consistência histórica (avaliação já lançada) |
  | **Cópia editorial** | o dono grava, com crédito explícito ao original, e os dois passam a ter dono | o texto precisa mudar de forma para o destino |

  Nenhum módulo **grava** em dado de outro. A referência viva é o padrão e a
  exceção exige justificativa escrita na spec do módulo. Todo vínculo declara o
  modo **no dado**, não no código que o criou.
- **Justificativa.** §4.6 é **[D]**: *"quando TCC ou Estudos usam um fragmento, o
  vínculo é referência viva, snapshot ou cópia editorial, nunca uma fonte
  concorrente."* Três modos com semânticas diferentes, e a escolha é da usuária;
  por isso o sistema precisa saber qual modo está em uso, e o modo mora no dado.
- **Alternativa rejeitada.** (a) Só referência viva — reduzir a lista é decidir
  contra §4.6 **[D]**, e um TCC que citou texto mudado não pode ter a citação
  reescrita por baixo. (b) Só cópia — é duplicação, que §1.3 proíbe. (c) O modo
  escolhido pelo implementador conforme o componente — é decisão de domínio tomada
  onde não há domínio.
- **Consequência aceita.** Todo dado vinculável carrega um campo de modo, e toda
  leitura atravessa o dono: "ver Conteúdos dentro de Disciplina" é **consulta ao
  módulo Estudos** com filtro, não join copiado. Isso dá dependência de leitura
  entre módulos e um caso novo no cache por janela; o plano B é materialização com
  invalidação explícita (risco registrado na spec §10).

O contrato do vínculo, no formato que o dado carrega:

```ts
export type ModoVinculo = "vivo" | "snapshot" | "copia-editorial";
export interface Vinculo<TId> {
  readonly alvo: TId;          // id no módulo dono
  readonly modo: ModoVinculo;
  readonly emIsoUtc: Instant;  // data da vinculação
  readonly credito?: string;   // exigido em "copia-editorial"
}
```

"Cópia editorial" sem `credito` falha na gravação: sem crédito, a cópia é fonte
concorrente, que é o que §4.6 proíbe.

## D67 · A fronteira é verificável por comando

- **Decisão.** A fronteira tem gate, e ele é dos **dois** lados. Do lado
  TypeScript: nenhuma `ui/` importa a `data/` de outro módulo, e nenhuma camada de
  `ui/` escreve em dado de outro módulo. Do lado Rust: **um** módulo Rust por dono
  de dado, e nenhum `#[tauri::command]` de um módulo escreve em tabela de outro —
  a verificação cruza a contagem de comandos com a lista de D65.
- **Justificativa.** O desktop já tem metade do padrão e ele é executável:
  `rg -n "invoke" src` devolve só `ipc.ts` e `bridge.ts`, e nenhum comando aceita
  SQL, nome de tabela ou string de consulta. Com oito módulos novos essa
  propriedade cai sozinha, e a quebra é silenciosa. A lista de D65 é o que torna o
  gate possível.
- **Alternativa rejeitada.** (a) Só o gate TypeScript — com o backend em Rust
  (§1.4), a escrita acontece no Rust, e um gate só de TS deixa o caminho real sem
  cobertura. (b) Só o gate Rust — a UI é onde a duplicata **aparece**. (c) Revisão
  de código — não é reproduzível, e nove módulos por olho são nove chances de não
  olhar.
- **Consequência aceita.** O gate é escrito do zero; a metade TypeScript entrou
  em 2026-10-09 (regra 7 de `scripts/check-boundaries.mjs`) e a metade Rust em
  2026-10-09 também (regra 8, F8b).
  **A primeira metade da decisão — "um módulo Rust por dono de dado" — ficou sem
  cumprir até 2026-10-15**: `src-tauri/src/commands.rs` era um módulo Rust só e
  o caminho de escrita de quatro donos. Foi cumprida pela
  [`SPEC-D-015`](../../../docs/specs/desktop/SPEC-D-015-modulos-do-backend-rust.md)
  (`D96..D100`, PR #1), e o que a regra 8 ganhou com isso está em
  [`02-modulos-do-backend.md`](./02-modulos-do-backend.md): ela passou a tirar o
  dono do **caminho** do arquivo, e a primeira verificação negativa provou que a
  versão anterior ficava verde sobre uma violação real de `D67`.

## Invariantes e onde são garantidas

| # | Invariante | Onde | Teste |
|---|---|---|---|
| I1 | Todo dado tem exatamente um dono declarado | a matriz desta seção + regra 8 de `check-boundaries.mjs` (tabela sem classificação é falha do gate) | `npm run check:boundaries` + revisão da lista |
| I4 | Nenhuma `ui/` importa `data/` de outro módulo | regra 7 de `check-boundaries.mjs` | `npm run check:boundaries` |
| I5 | Nenhum nome de tabela atravessa a fronteira IPC | `bridge.ts:3-5` e o cabeçalho de `<dono>/comandos.rs` de cada módulo | `rg -n "invoke" src` → só `ipc.ts` e `bridge.ts` |
| I6 | Todo vínculo declara modo, e `copia-editorial` exige crédito | `ModoVinculo`/`Vinculo` (D66) | teste do módulo dono, quando existir |
| I8 | Conteúdo é de Estudos; Faculdade o mostra por visão | a matriz (D65) | nenhum campo de conteúdo na tabela do módulo Faculdade |
| I9 | Referência é da Biblioteca; o TCC a mostra por lista gerada | a matriz (D65) | a lista |
| I12 | A camada clínica não tem "quem só mostra" preenchido | a matriz (D65) | a linha é "ninguém" |

I5 é a única que já é verdade e é o que a spec generaliza; as outras dependem de
os módulos existirem. I4 passa a ser verificada por comando a partir de
2026-10-09.

## O gate de fronteira de módulo

O gate mora em `scripts/check-boundaries.mjs` e roda em `npm run verify`.
São **dez** regras: sete de arquitetura TypeScript, a oitava de dono de dado no
Rust, e a nona (registro × função) e a décima (teto de linhas), que vieram com a
`SPEC-D-015`. As duas da fronteira de módulo:

- **Regra 7 (`fronteira-de-modulo`)** transforma I4 em comando: para um arquivo
  em `src/features/<módulo>/ui/`, um import que resolve para
  `src/features/<outro>/data/` com `outro ≠ módulo` é violação.
- **Regra 8 (`dono-de-dado`)** transforma D67 em comando, e o cruzamento é o que
  a decisão descreve: o **módulo dono** de cada comando vem da bridge que o
  invoca (`src/features/<módulo>/data/`), as **tabelas escritas** vêm do corpo
  das funções de `src-tauri/src/**`, e o gate recusa: comando
  invocado por duas bridges de módulos, função que escreve em tabela de dois
  módulos, comando cujo módulo difere do dono da tabela escrita, tabela do
  schema sem dono classificado, e classificação de tabela que não existe mais.
  **Desde 2026-10-15 (F0 da `SPEC-D-015`) o dono vem de dois lados**: o
  nome do diretório onde o arquivo está — que é o dono do **dado** — e a bridge
  que invoca — que é o dono do **uso**. A versão anterior lia um arquivo só por
  nome, e dividir aquele arquivo a deixava verde sem ver escrita nenhuma.
  O corte do módulo de teste é no `#[cfg(test)] mod` — **não** no primeiro
  `#[cfg(test)]`, porque helpers marcados assim existem no meio do arquivo, e o
  corte no primeiro deixou o gate cego (medido em F8b2: verde sem ver escrita
  nenhuma; a verificação negativa revelou).

O que ainda **não** tem gate, e por quê:

- **Coluna de dado de outro módulo no schema**: as tabelas do Faculdade e as de
  Estudos (`conteudo`, `cartao`, `revisao`) não carregam conteúdo, referência
  nem nota de outro módulo. A checagem entra quando houver dado cruzado entre
  módulos.

## Medição

Números gerados, não copiados.

| Métrica | Comando | Antes (2026-09-30) | Depois (2026-10-09) |
|---|---|---|---|
| Linhas na lista de dono | `rg -c "^\|" docs/espacos/01-fronteiras.md` | 0 (não existia) | 14 na matriz de §3.1 |
| Dados sem dono | `rg -n "^\| [^|]+ \|  \|" docs/espacos/01-fronteiras.md` | — | 0 |
| Arquivos que chamam `invoke` | `rg -l "invoke<" src` | 2 (`ipc.ts`, `bridge.ts` do Calendário) | 4 (`ipc.ts` + as bridges de `calendar/`, `faculdade/` e `estudos/`) |
| Diretórios de módulo Rust | `Get-ChildItem src-tauri/src -Directory` | 0 | 2 (`materia/`, Faculdade · `estudos/`, Estudos) |
| Violações de fronteira | `npm run check:boundaries` | 6 regras | 8 regras, 0 violações |

## Bloqueios

Itens que a fatia determinística **não** fecha, e o motivo:

| Item | Bloqueio | Quem resolve |
|---|---|---|
| `src-tauri/src/sync/` com a tradução de formatos (ADR-009 item 3) | a ADR-009 deixa abertas a pendência 1 (mapeamento entidade-a-entidade) e a pendência 2 (formato do envelope); decidir por conta própria seria inventar regra de integração | a dona do produto, via spec de sync |
| Comandos para `external_link` e `sync_cursor` | são spec 05 (sync Google), bloqueada por credenciais | spec 05 |
| Comandos para `suggestion` | é o motor da spec 06 / `SPEC-D-012` D80-D82 | `SPEC-D-012` |
| ~~Gate Rust de dono de dado~~ | **feito em 2026-10-09 (F8b)**: regra 8 de `check-boundaries.mjs`, com verificação negativa — mas **só metade**: a regra lê `src-tauri/src/commands.rs` por nome (`:353`), então ela deixa de ver escrita no dia em que esse arquivo for dividido. Verificado em 2026-10-14: o gate dá `✓` sobre uma violação real de `D67` depois do split | — |
| **"Um módulo Rust por dono de dado"** — a outra metade de `D67` | `commands.rs` tem 6537 linhas e é o caminho de escrita de **quatro** donos; a fronteira de módulo para no motor e não alcança a escrita | [`SPEC-D-015`](../../../docs/specs/desktop/SPEC-D-015-modulos-do-backend-rust.md) (`D96..D100`), fonte em [`02-modulos-do-backend.md`](./02-modulos-do-backend.md) |

## Referências

| Documento | O que resolve |
|---|---|
| [`SPEC-D-007`](../../../docs/specs/desktop/SPEC-D-007-dono-do-dado-e-fronteira-entre-modulos.md) | a cópia de grupo desta decisão |
| [`SPEC-D-012`](../../../docs/specs/desktop/SPEC-D-012-ceci-no-desktop.md) | a Ceci como consumidor que respeita a fronteira |
| [`ADR-009`](../../../docs/decisoes/ADR-009-um-contrato-de-dados-por-app.md) | um contrato de dados por app; a tradução do sync é Rust, no desktop |
| [`spec-referencial-workspace-academico.md`](../../../spec-referencial-workspace-academico.md) §1.3, §3.1, §4.4, §4.6 | a origem: dono de dado, matriz, Conteúdo em Estudos, modos de vínculo |
| `scripts/check-boundaries.mjs` | a regra 7 que verifica I4 |
| [`00-shell-e-navegacao.md`](./00-shell-e-navegacao.md) | a decisão irmã: a mesma lista decide entrada de navegação |
