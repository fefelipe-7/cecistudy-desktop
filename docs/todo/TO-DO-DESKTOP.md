# To-do · desktop cecistudy ♡

> Onde vive: `docs/todo/TO-DO-DESKTOP.md`, dentro do app
> Por que aqui: um plano do desktop que mora fora do desktop é plano sem
> histórico — foi o que aconteceu com `cecigroup/docs/todo/`, que ninguém
> versionava. Este arquivo substitui aquele, e há uma fonte só.
> Origem: [`SPEC-D-015`](../../../docs/specs/desktop/SPEC-D-015-modulos-do-backend-rust.md)
> (`D96..D100`) · [`roadmap.md`](../../roadmap.md) §D4/§D6
> Atualizado em: 2026-10-15
> Estado do código ao abrir: `main` = `70a12d3`, PR #1 aberto e verde,
> `npm run verify` EXIT=0

> Um item só está fechado quando o **comando** da coluna "Fecha quando" passa.
> Sem isso, está escrito e não garantido — e item sem gate é dívida.

---

## Bloco A · Encerrar a SPEC-D-015

F0 a F5 estão aplicados e verificados no PR #1. Falta o fecho.

| # | O quê | Fecha quando |
|---|---|---|
| **A1** | **Decidir o merge do PR #1.** `MERGEABLE · CLEAN`, `verify=SUCCESS` em `ubuntu-latest` (1m42s). Squash preserva a ordem — que é o argumento do plano —; merge direto mantém os 6 commits legíveis | `gh pr view 1 --json state` devolve `MERGED` |
| **A2** | Apagar a branch de segurança `wip-2026-10-14` do remoto. É o único estado pré-refactor que existe fora deste disco, e só tem valor enquanto o PR pode falhar | `git ls-remote origin \| grep wip` devolve vazio |
| **A3** | ~~**Declarar `D95`.**~~ **Fechado em 2026-10-15.** Era citada em cinco lugares do código (`tcc/mod.rs:14,18,19,26`, `docx.rs:1`, `paginacao.rs:14,86`, `pdf.rs:1`) e não existia em nenhum bloco de decisão. Declarada em [`tcc.md`](../espacos/tcc.md) §`D95`, com a ressalva de que o texto foi remontado das citações | `grep -rn "^## D95" docs/espacos/` devolve uma linha |
| **A4** | Decidir o `verify:fast` que apareceu no `package.json` sem commit (typecheck + lint + test + golden + boundaries, sem Rust). Não é meu e não commitei | `git status --porcelain package.json` devolve vazio |

**A3 era o item com prazo e está fechado.** Decisão numerada sem registro é pior
que ausência: o código aponta para um argumento que ninguém pode ler. O mesmo
achado trouxe `I3` de marked "pendente" para "cumprida" — a exportação existe com
o teste de paridade da sequência de texto (`docx.rs:918`).

---

## Bloco B · O drift que a migração criou

Cada item aqui é **Natureza I** — o documento mente e o código está certo. São
todos de escrita mecânica, e nenhum muda regra.

| # | O quê | Fecha quando |
|---|---|---|
| **B1** | ~~[`roadmap.md`](../../roadmap.md) §"Estado verificado"~~ **Fechado em 2026-10-15.** Seis fatos errados: **8 regras** (10), **134 testes Rust** (160), "**4 commits**, branch `master`" (17 na `main`), os caminhos de `commands.rs`/`domain.rs`/`db.rs`, o comando que conta comandos, e o parágrafo "Backend hoje" duplicado | cada linha do quadro re-gerada com o comando que a produz |
| **B2** | ~~O roadmap em D4.1 e D4.2~~ **Fechado.** `materia/` → `faculdade/` e "9 comandos IPC" → 13. A varredura achou mais: [`faculdade.md`](../espacos/faculdade.md) tinha **15** referências a `materia/`, [`01-fronteiras.md`](../espacos/01-fronteiras.md) e [`tcc.md`](../espacos/tcc.md) citavam linhas de `commands.rs` que não existem | `grep -rn "materia/" docs/espacos/` devolve vazio |
| **B3** | ~~**42 citações `commands.rs:NNN`**~~ **Fechado em 2026-10-15, por decisão de escopo.** As 33 das specs de performance foram **arquivadas** com banner, não reapontadas; as 9 dos docs vivos foram corrigidas | `grep -rn "commands\.rs:[0-9]" --include="*.md" docs/espacos/ docs/todo/` devolve só as de propósito histórico |
| **B4** | ~~As 3 specs de performance~~ **Fechado por arquivamento.** 461 KB, **nenhuma medição rodada** — a própria spec admite que "a Fase P0 não rodou". O `C-06` delas afirma que `origin` está ausente do wire, e `GridItem` tem `origin` desde antes (`types.ts:60,66`) | os 4 arquivos de `docs/performance/` abrem com o banner de arquivado |
| **B5** | **Reapontar as citações de [`docs/calendario/`](../calendario/)** — 31 ocorrências em 5 specs | `00-onde-estas-citacoes-apontam.md` entra na lista, e cada spec da pasta aponta para ele |
| **B6** | **Rodar a Fase P0 das specs de performance**, se elas voltarem a valer. 461 KB de hipótese não medida só têm valor depois de medir | decisão de fase — ver `TO-DO` §C |

### Por que arquivar em vez de reapontar

**B3 e B4 mudaram de natureza no meio da execução.** A tarefa era "reapontar 33
citações", e a pergunta certa é *para que serve uma citação dentro de um
documento que ninguém deve consultar*. A resposta é nenhuma: a spec de 461 KB
declara que nenhuma das medições rodou, descreve um estado de 2026-09-29 que já
foi, e um item que descreve bug corrigido não é pendência — é distração.

Corrigir as 33 seria o mesmo trabalho que a `SPEC-D-015` acabou de pagar no
backend, aplicado a um documento que a própria spec declara não medido. O que
falta ali não é remendo: é **rodar a Fase P0** e re-derivar o que for medido.
Isso virou `B6`, que é decisão de fase, não de escrita.

### Por que `docs/calendario/` não foi arquivada

Porque ela é a **origem** — `00-fundacao.md` com `D1..D46` é a fonte da
governança do desktop e o melhor formato de decisão do grupo, e as invariantes
`INV-1..INV-7` continuam verdadeiras. Regra não mudou de arquivo, mudou de
lugar. Então ela fica, com um índice de correspondência
([`00-onde-estas-citacoes-apontam.md`](../calendario/00-onde-estas-citacoes-apontam.md))
e um aviso em cada spec que aponta para ele.

---

## Bloco C · O que continua do plano do produto

O roadmap tem D4 (módulos verticais) e D6 (`SPEC-D-011`). O que está fechado
não volta; o que está aberto ou bloqueado está abaixo, com o bloqueio honesto.

### C.1 Aberto, sem bloqueio — dá para começar

| # | O quê | Spec | Por que está aberto |
|---|---|---|---|
| **C1** | **Cobertura de contrato do editor** — D6.6, a única de §D6 que é `gate` | `SPEC-D-011` §9 | o editor existe (`editar.ts` + `EditorDoDocumento.tsx`, D6.2) e **não tem** golden nem contrato verificado por comando. É a mesma lacuna que a SPEC-D-015 acabou de fechar no backend, agora no front |
| **C2** | **Conhecimento** — D4.3 | `SPEC-C-006`, `SPEC-D-007` D65 | está na matriz de dono e não existe. Sem ele, a citação e a referência viva do TCC ficam sem alvo (`I13`), que é o bloqueio de D6.5 |
| **C3** | **Ceci** — D4.4, lê e sugere, **nunca escreve** | `SPEC-D-012`, `SPEC-C-009` | o schema `block_suggestion` existe e não tem comando nem UI; a camada `sync/` do backend está vazia |

### C.2 Bloqueado por decisão de produto — o agente não destrava

| # | O quê | O bloqueio | Quem resolve |
|---|---|---|---|
| **C4** | **Round-trip DOCX** — D6.3/D6.4 | `D78` exige preservar `w:ins`/`w:del` **com autor e data**. Uma biblioteca que só lê texto entrega o documento e perde a revisão. Escolher por critério é decisão de fase | a dona do produto, com o critério escrito |
| **C5** | **Estágio — camada clínica** — D4.5 | **`[A]`** normas do CFP/instituição, dado sensível. §4.8 é a única `[A]` de dado sensível do conjunto | a dona do produto. **Agente não fecha `[A]`** |
| **C6** | **Sala de treino** — D4.6 | **`[A]`** premissas técnicas (CPU local, voz) e **`[P]`** enxugamento de subtelas | a dona do produto |
| **C7** | **Lista de referências** — D6.5 | depende de C2: Biblioteca e Conhecimento não existem | depois de C2 |

### C.3 Dívidas de governança que sobraram (desktop)

| # | O quê | Onde | Nota |
|---|---|---|---|
| **C8** | Três tabelas sem comando: `external_link`, `suggestion`, `sync_cursor` | `roadmap.md` §"Backend hoje" | `sync/` é a tradução de formatos da ADR-009 item 3, que tem pendência de mapeamento entidade-a-entidade e de formato de envelope |
| **C9** | Camada `sync/` vazia no TypeScript | `src/features/calendar/sync/` | idem |

---

## Ordem sugerida

```
A1, A2        merge e limpeza — destrava o resto
A3            ~~D95~~  FECHADO
B1, B2        ~~roadmap e os caminhos mortos~~  FECHADO
B3, B4        ~~as citações e as specs de performance~~  FECHADO por arquivamento
B5            reapontar docs/calendario — 31 citações, com o índice de correspondência pronto
C1            cobertura de contrato do editor — o próximo buraco de gate
C2            Conhecimento — destrava C7
C3            Ceci
B6, C4..C7    bloqueados: só a dona do produto
```

**Por que C1 vem antes de C2 e C3.** C2 e C3 são produto: telas e fluxos que
não existem. C1 é buraco de **verificação** num código que já existe e já
funciona — e é exatamente a classe de dívida que a SPEC-D-015 acabou de pagar.
Enquanto C1 estiver aberto, o editor pode regredir sem ninguém ver, que é o que
`D5.4` (golden como contrato, não cópia) existe para impedir.

---

## Regra que vale para este arquivo

**Nada entra aqui sem o comando que fecha.** Um item sem comando é uma
intenção, e intenção escrita em lista vira dívida no prazo em que ninguém
reabre a lista. Se um item ficar bloqueado, o motivo é uma `[A]` do produto ou
uma decisão de fase nomeada — nunca "falta tempo".

## Referências

| Documento | O que resolve |
|---|---|
| [`02-modulos-do-backend.md`](../espacos/02-modulos-do-backend.md) | `D96..D100` — o que a SPEC-D-015 decidiu e por quê |
| [`roadmap.md`](../../roadmap.md) | D0..D6, com o estado verificado e os bloqueios |
| [`01-fronteiras.md`](../espacos/01-fronteiras.md) | `D65..D67` — dono do dado e a fronteira verificável |
| [`AGENTS.md`](../../AGENTS.md) §Gate | o gate único e a tabela de como medir |