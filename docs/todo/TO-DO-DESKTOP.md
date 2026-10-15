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
| **A3** | **Declarar `D95`.** Ela é citada em quatro arquivos do código (`tcc/mod.rs:12,16,17,24`, `tcc/docx.rs:1`, `tcc/paginacao.rs:14,86`, `tcc/pdf.rs:1`) e **não existe** em nenhum bloco de decisão. O código aponta para um número que ninguém escreveu | `grep -rn "^## D95" docs/espacos/` devolve uma linha, ou o ponto declara explicitamente que a decisão é de `SPEC-D-011` e quem fecha |
| **A4** | Decidir o `verify:fast` que apareceu no `package.json` sem commit (typecheck + lint + test + golden + boundaries, sem Rust). Não é meu e não commitei | `git status --porcelain package.json` devolve vazio |

**A3 é o item com prazo.** Decisão numerada que não existe é pior que
ausência: o código cita `D95` como se houvesse um argumento, e quem ler não
sabe se encontrou a decisão ou um número órfão.

---

## Bloco B · O drift que a migração criou

Cada item aqui é **Natureza I** — o documento mente e o código está certo. São
todos de escrita mecânica, e nenhum muda regra.

| # | O quê | Fecha quando |
|---|---|---|
| **B1** | [`roadmap.md`](../../roadmap.md) §"Estado verificado em 2026-10-13" está com seis fatos errados: **8 regras** (são 10), **134 testes Rust** (são 160), "**4 commits** (2026-09-28), branch `master`" (a main tem 7 e o trabalho está em branch), o caminho de `commands.rs`/`domain.rs`/`db.rs`, e o comando que conta comandos (`"commands::"`, que virou `"comandos::"` com caminho de dono) | cada linha do quadro re-gerada com o comando que a produz, no formato do quadro |
| **B2** | O mesmo roadmap em **D4.1** cita `src-tauri/src/materia/` (virou `faculdade/`) e em **D4.2** diz "**9 comandos IPC**" do TCC (são 13) | `grep -n "materia/\|9 comandos IPC" roadmap.md` devolve vazio |
| **B3** | **42 citações `commands.rs:NNN`** em 6 documentos, todas com linha que não existe mais | `grep -rn "commands\.rs:[0-9]" --include="*.md" .` devolve só as de propósito histórico |
| **B4** | As **3 specs de performance** (`docs/performance/`) têm 33 dessas citações e pelo menos um item **descreve problema já resolvido**: o `C-06` diz "`origin` ausente no wire", e `GridItem` tem `origin` e `allDay` desde antes (`types.ts:60,66`) | cada item do C-01..C-0n marcado como resolvido, aberto ou ainda verdadeiro, com o comando que prova |

**B3 e B4 juntos são a mesma tarefa**, e a ordem importa: primeiro o veredito
(o item ainda vale?), depois o caminho (onde a regra está agora?). Citar
`commands.rs:487` num documento que descreve um bug já corrigido produz um
documento que mente duas vezes — no número e no fato.

**O padrão de replacement** é o do próprio grupo: o dono do dado do
`commands.rs` virou diretório, então `commands.rs:NNN` vira
`<dono>/comandos.rs:NNN` — com o número **medido**, nunca herdado. E
`domain.rs:N` vira `calendario/dominio.rs:N`.

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
A3            D95 tem prazo
B1, B2        roadmap: seis fatos errados, escrita mecânica
B3 + B4       as 42 citações: veredito primeiro, caminho depois
C1            cobertura de contrato do editor — o próximo buraco de gate
C2            Conhecimento — destrava C7
C3            Ceci
C4..C7        bloqueados: só a dona do produto
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