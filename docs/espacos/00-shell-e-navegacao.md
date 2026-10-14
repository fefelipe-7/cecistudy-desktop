# 00 · Shell, navegação e o Workspace Acadêmico

> Fonte de verdade no app para a [`SPEC-D-006`](../../../docs/specs/desktop/SPEC-D-006-shell-navegacao-e-workspace-academico.md).
> Cópia de grupo: `docs/specs/desktop/SPEC-D-006-shell-navegacao-e-workspace-academico.md`.
> Decisões do app: `D62`, `D63`, `D64`.
> Estado: implementado em 2026-10-09 (D62, D63 e D64).

Este documento é a decisão de **módulo** do shell, e não renumera `D1..D46` de
`docs/calendario/`. Onde o índice de grupo errar sobre o app, o app está certo.

## Contexto

O desktop era **um** módulo com cinco modos de leitura em `src/App.tsx`, seis
`useState` de navegação misturados com o dashboard (`App.tsx:137-142`) e uma
sidebar montada por dado de mock. O produto pede **nove entradas de sidebar**, um
seletor de workspace no topo, três utilitários no rodapé e telas de verdade.

A decisão "modos de um fluxo, não destinos" (`AGENTS.md`) estava **certa para o
que ela descrevia** — as cinco visões do Calendário são modos do mesmo conjunto de
dados. O problema era esticá-la para nove módulos.

## D62 · Os módulos são destinos de navegação com estado próprio, e a troca é instantânea

- **Decisão.** Existe uma **tabela de navegação declarativa**
  (`src/shell/navigation.ts`: `MODULOS`, `WORKSPACES`, `UTILITARIOS`,
  `CANDIDATOS_RECUSADOS`). A shell tem **três estados de navegação** num único
  lugar, em `src/App.tsx`: módulo ativo (`modulo`), painel lateral (`sidebar`) e
  camada de sobreposição (`searchOpen`); mais o workspace (`workspaceId`), que é
  decisão de D63. Trocar de módulo é troca de estado: sem `invoke` de carga de
  módulo, sem remount da shell. O padrão "modos de um fluxo" é preservado
  **dentro** de cada módulo e removido **entre** módulos.
- **Justificativa.** A Home precisa abrir "Sessão 04 da Ana" (Continuar), o painel
  do Calendário precisa de "Abrir no módulo de origem" e o relatório de estágio
  pode ser um Projeto em TCC & Projetos. Tudo isso é **navegação com destino**, e
  destino sem endereço não pode ser alvo de link, histórico nem volta. A tabela
  única é também o que elimina a classe de bug em que a sidebar e o roteamento
  interno discordam sobre quantos módulos existem.
- **Alternativa rejeitada.** (a) Continuar com `useState` e a sidebar escrevendo
  no mesmo estado; falha para nove módulos, e a falha é a falta de destino, não de
  estilo. (b) Adotar `react-router`/`wouter`; **por enquanto** rejeitada — a falta
  é de endereço, não de biblioteca. Gatilho de reabertura: existir deep-link por
  módulo (notificação, link externo, retrocesso), e aí é ADR, não ajuste. (c) Um
  armazenamento global de navegação; rejeitada porque seria a primeira dependência
  de estado de terceiros do app por um problema que uma tabela resolve.
- **Consequência aceita.** Passa a existir uma camada de navegação com arquivo e
  teste próprios (`src/shell/navigation.ts`, `src/shell/navigation.test.ts`), e o
  `App.tsx` deixa de ser "um arquivo que se lê inteiro" para ser "o shell, que tem
  regra aqui".

## D63 · A sidebar tem nove entradas, o topo tem o seletor de workspace e o rodapé tem três utilitários

- **Decisão.** A estrutura é a de §2 do referencial, com a ordem marcada como
  proposta:

  | Posição | Conteúdo | Marca |
  |---|---|---|
  | Topo | seletor de workspace (Acadêmico · Profissional) | **[D]** mapa |
  | Sidebar | Home · Calendário · Faculdade · Estudos · Biblioteca · Conhecimento · TCC & Projetos · Estágio · Sala de treino | **[P]** ordem |
  | Rodapé | Ceci · Busca · Notificações | **[D]** mapa |

  Os nomes são **fechados**: *Sala de treino* é o módulo de paciente simulado; o
  rótulo antigo da subtela de treino foi abandonado porque remetia a testes
  psicológicos. A lista é **declarada**, não derivada do que está implementado: um
  módulo em construção aparece com estado explícito e é clicável, mostrando um
  estado vazio honesto.
- **Justificativa.** A ordem é **[P]** no referencial e continua sendo decisão de
  produto; o que esta decisão fecha é o resto, de engenharia: onde a lista mora
  (D62), como uma entrada sabe se pode aparecer (D64), e o que acontece com um
  workspace que ainda não existe. Derivar a sidebar do código implementado
  tornaria a navegação um efeito colateral do build — a usuária perderia o lugar.
- **Alternativa rejeitada.** (a) Derivar a sidebar automaticamente dos módulos com
  tela implementada; rejeitada por transformar navegação em efeito do código.
  (b) Fechar a ordem agora; rejeitada porque o produto a marca **[P]**. (c)
  Colocar Ceci, Busca e Notificações na sidebar; rejeitada porque são utilitários
  globais (§5) e a distinção impede a sidebar de crescer sem limite.
- **Consequência aceita.** Nove entradas numa janela com `minWidth` original de
  1024 era apertado (D-15 do relatório); o `minWidth` foi corrigido para 1120
  (`src-tauri/tauri.conf.json:20`), compatível com a coluna de conteúdo de 836 px
  mais a sidebar aberta de 264 px. A lista de nomes fechados passa a ser
  **contrato verificado** por teste (`navigation.test.ts`): um rótulo que muda,
  falha.

### O seletor de workspace e o Profissional

O Acadêmico é o único workspace funcional. O Profissional é **selecionável** mas
declara o estado "em construção" e não oferece nenhum destino (§2 e §7 do
referencial): um clique que leva a lugar nenhum é pior que a ausência da entrada.
A escolha de "oferecer com estado" resolve a tensão entre §2 (`[D]` mapa) e §7
(não oferecer o que não existe) mantendo ambos: existe a entrada, e o estado
declarado é o que ela promete.

## D64 · A regra de §1.2 é aplicada por módulo, e a recusa é registrada, não implícita

- **Decisão.** A tabela `CANDIDATOS_RECUSADOS` (`src/shell/navigation.ts`) declara,
  para cada candidato a tela, o módulo que o **contém** (`contidoEm`), o módulo
  **dono** do dado (`dono`, o mesmo de `SPEC-D-007` D65) e o **motivo** da recusa.
  Os recusados pela regra de §1.2: Evento, Responsabilidade, regra de recorrência,
  Capítulo, Flashcard avulso, Avaliação isolada, Nota isolada e Conceito isolado.
  Não existe entrada cuja justificativa seja "já existia".
- **Justificativa.** §1.2 é **[D]** e impede o crescimento de abas. Uma regra assim
  só funciona se for **aplicável**: sem lista, "por que existe uma tela Notas?" não
  tem resposta, e a resposta que se arrisca é "porque já tinha". A lista liga a
  navegação ao dono do dado (`SPEC-D-007`), porque a mesma tabela decide "isto é
  dono" e "isto é tela".
- **Alternativa rejeitada.** (a) Não ter lista, tratando §1.2 como conselho;
  rejeitada porque conselho não é gate. (b) Lista dentro de `App.tsx` como array de
  strings; rejeitada porque código é fato e fato não se revisa — a lista precisa
  sobreviver à reescrita do componente. (c) Um único documento de telas de desktop
  e mobile juntos; rejeitada porque quem decide tela é o produto por app.
- **Consequência aceita.** A lista cresce com cada módulo e precisa ser revisada a
  cada revisão de produto. O custo é aceito porque a alternativa — abas que nascem
  por conveniência — é o defeito padrão do mercado.

## Invariantes e onde são garantidas

| # | Invariante | Onde | Teste |
|---|---|---|---|
| I1 | Existe **uma** lista de entradas, e sidebar, título e busca leem a mesma | `src/shell/navigation.ts` | `rg -n "export type ModuleId" src` → 1 |
| I3 | Todo rótulo é o nome fechado de §2 | `MODULOS` | `navigation.test.ts` (ordem e rótulos) |
| I4 | "Sala de treino" é o rótulo; o nome antigo com "testes" não volta | `MODULOS` | `navigation.test.ts` |
| I7 | Toda entrada declara o dono do seu dado | `MODULOS[].dono` | `navigation.test.ts` |
| I8 | Workspace sem destino aparece com estado explícito | `WORKSPACES`, seletor | `navigation.test.ts` |
| — | Nenhuma regra do Calendário é reimplementada fora do Rust | `check-boundaries.mjs` regra 6 | `npm run check:boundaries` |

## Medição

Números gerados, não copiados.

| Métrica | Comando | Antes (2026-09-30) | Depois (2026-10-09) |
|---|---|---|---|
| Entradas na sidebar | `MODULOS.length` | 5 modos de visão | 9 |
| Estados de navegação em `App.tsx` | os `useState` de navegação | 6 em `:137-142` | 4 (módulo, sidebar, sobreposição, workspace) |
| Testes do shell | `npm run test` | 12 | 24 (arquivos de `src/shell/`) |
| Rótulos fora do nome fechado | `navigation.test.ts` (nome abandonado não volta) | — | vazio |
| `minWidth` vs conteúdo | `rg -n minWidth src-tauri/tauri.conf.json` | 1024 (< 1100) | 1120 |

## Referências

| Documento | O que resolve |
|---|---|
| [`SPEC-D-006`](../../../docs/specs/desktop/SPEC-D-006-shell-navegacao-e-workspace-academico.md) | a cópia de grupo desta decisão |
| [`spec-referencial-workspace-academico.md`](../../../spec-referencial-workspace-academico.md) §2, §1.2, §1.3 | a origem: mapa, regra de tela, dono do dado |
| [`AGENTS.md`](../../AGENTS.md) | arquitetura do app, gates e invariantes |
| `src/shell/navigation.ts` | a tabela declarativa |
