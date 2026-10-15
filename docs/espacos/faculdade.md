# Faculdade — motor de notas e regra de média

> Fonte de verdade no app para a [`SPEC-D-010`](../../../docs/specs/desktop/SPEC-D-010-motor-de-notas-e-fsrs.md)
> (média e regra de aprovação). A cópia de grupo vive em
> `docs/specs/desktop/SPEC-D-010-motor-de-notas-e-fsrs.md`; a metade de **Estudos**
> (FSRS) mora em [`estudos.md`](./estudos.md).
> Decisões do app: `D74`, `D75`.
> Origem: spec referencial §1.4 (**[D]**, cálculo no Rust), §3.1 (Faculdade é dona
> da regra de média), §4.3 (**[D]**: motor de notas), §6.1 (avaliação → nota →
> recálculo).
> Estado: **motor, regra, schema, comandos IPC e tela v1 implementados** em
> 2026-10-09 (`src-tauri/src/faculdade/`, `migrations/0003_faculdade.sql`, o caminho
> de escrita `src-tauri/src/faculdade/comandos.rs` espelhado em
> `src/features/faculdade/data/bridge.ts`, e a
> [`FaculdadeScreen`](../../src/features/faculdade/ui/FaculdadeScreen.tsx) —
> semestre, disciplinas com a média derivada, avaliações e lançamento de nota).

Este documento é a decisão de **módulo** do Faculdade, ao lado de
`00-shell-e-navegacao.md` e `01-fronteiras.md`. Ele não renumera `D1..D46` de
`docs/calendario/` nem `D62..D67`. Onde o índice de grupo errar sobre o app, o
app está certo.

## Contexto

O Faculdade responde *"o que a instituição exige de mim, e como está meu
desempenho?"* (§4.3). A parte com **regra de cálculo** é o motor de notas: média
simples ou ponderada, nota mínima, exame, arredondamento e frequência mínima.

Dois fatos governam o desenho, e os dois são **[D]**:

1. **A média é derivada e não é gravada.** É a mesma disciplina do atraso
   (`INV-4`, `domain/overdue.ts`): não existe coluna de média, de situação nem de
   "quanto falta". O que muda a média — peso, exame, arredondamento — muda a
   regra, e regra que muda invalida valor guardado.
2. **A regra é viva no presente e congelada no passado.** Uma disciplina sem
   personalização herda o padrão global e **se move** com ele; ao encerrar um
   semestre, a regra efetiva é **gravada** e vira imutável. Encerrar semestre é
   um ato que grava, não um filtro de tela.

## D74 · O motor de notas é Rust, puro, e não guarda média

- **Decisão.** `src-tauri/src/faculdade/media.rs` calcula, para uma disciplina, a
  média atual, a situação em linguagem e a frequência restante. As entradas são
  as avaliações (peso, `vale_nota`, nota lançada) e a frequência; a regra vem de
  [`regra.rs`](#d75--a-regra-de-média-é-viva-no-semestre-corrente-e-congelada-no-passado).
  A função é **pura**, sem relógio, e **nenhuma** saída é persistida: não há coluna
  de média, de situação nem de "quanto falta" em nenhuma tabela (I1). A nota
  lançada é o único dado gravado.
- **Justificativa.** §1.4 **[D]** põe o cálculo no Rust. O padrão de derivado do
  projeto é explícito (`docs/analise/desktop.md` §Conformidade): `INV-4` está
  garantida **por ausência** — não existe coluna de atraso, logo não existe estado
  para dessincronizar. O mesmo vale, e vale melhor, para média: ela é função de
  notas, pesos e regra, e qualquer um dos três que mude depois invalida o valor
  guardado.
- **Alternativa rejeitada.** (a) Persistir a média no lançamento da nota.
  Rejeitada porque mudar o arredondamento de 6,95 reescreve todas as médias do
  semestre: com coluna, a correção é um backfill; sem coluna, é zero trabalho.
  (b) Recalcular no TypeScript, no render. Rejeitada porque é regra de domínio na
  interface (§1.4 proíbe), e a média da tela e a da Ceci seriam cálculos
  distintos — a mesma classe de bug de `FREQ=MONTHLY` (`SPEC-D-002`).
- **Consequência aceita.** A média é recalculada na leitura, O(n) sobre as
  avaliações — irrelevante para seis disciplinas. A situação em linguagem
  ("precisa tirar 6,8") é **string derivada**: não é ordenável nem comparável em
  banco. Se algum lugar precisar filtrar por situação, o filtro é o enum
  [`Situacao`](../../src-tauri/src/faculdade/media.rs) em Rust
  (I15); a string é apresentação.
- **Cláusula do exame — a fórmula é configurável por disciplina.** §4.3 diz
  *"a regra do exame é da disciplina"* e **não** fixa a aritmética. A decisão é
  que o **modo** do exame é configuração da disciplina: `nenhum` (só a média
  decide), `media_com_exame` (recuperação: `(média + exame) / 2 ≥ exame_minima`) ou
  `exame_substitui` (aprovar se `exame ≥ exame_minima`). A média para **ter
  direito** ao exame também é configurável (`exame_elegivel_minima`); abaixo dela,
  a situação é "não alcança".
  - *Alternativa rejeitada.* Fixar **uma** fórmula de produto (só recuperação por
    média, ou só substituição). Rejeitada porque a spec não fixou a fórmula e a
    atribuiu à disciplina; fixá-la seria inventar regra de produto. A escolha
    ficou onde a spec a pôs.
  - *Consequência.* O schema precisa guardar o **modo** e as duas notas mínimas
    por disciplina, e o golden precisa cobrir os três modos. O motor não escolhe
    modo: ele aplica o que a regra efetiva diz.

### O cálculo, em uma tela

- **Média crua.** `soma(nota × peso_efetivo) / soma(peso_efetivo)`, onde
  `peso_efetivo` é o peso em média ponderada e `1` em média simples (I3: a média
  simples **ignora** o peso).
- **Média apresentada.** Arredondada no passo da regra (0,1 ou 0,01). A fronteira
  é o passo: crua 6,95 com `nota_minima` 7,0 aprova no passo 0,1 e não aprova no
  passo 0,01 (I4).
- **Frequência.** `limite = floor(total_aulas × (1 - frequencia_minima))`;
  `faltas > limite` reprova. A frequência restante é `max(0, limite - faltas)` —
  derivada, nunca gravada (I5).
- **Situação** (enum `Situacao`, string no contrato):
  - `sem-notas` — nenhuma nota lançada. Não é "não alcança": sem nota não se
    reprova.
  - `aprovada` — média arredondada alcançou a nota mínima.
  - `precisa-tirar` — sem exame, e há avaliação pendente; traz a nota necessária
    **na próxima** (arredondada **para cima**, para não prometer nota que reprova).
  - `so-com-exame` — média abaixo do mínimo, com direito a exame; traz a nota
    necessária no exame.
  - `nao-alcanca` — reprovado por frequência, sem pendente, abaixo da
    elegibilidade do exame, ou exame que exigiria nota maior que a máxima (10).

## D75 · A regra de média é viva no semestre corrente e congelada no passado

- **Decisão.** A configuração da regra tem camadas: um **padrão global** nas
  Configurações e uma **personalização por disciplina** que o substitui **por
  inteiro** (não por campo). A resolução de `(semestre, disciplina)` segue esta
  ordem e **nenhum outro caminho**: regra **congelada** do semestre; senão,
  personalização da disciplina; senão, padrão global. Ao **encerrar** o semestre,
  a regra efetiva de cada disciplina é **gravada** e passa a ser imutável.
  - *Nota de implementação.* D75 enuncia quatro candidatos ("personalização do
    semestre · congelada do semestre · personalização corrente · padrão"), mas
    "personalização do semestre" e "personalização corrente" são a **mesma**
    personalização por disciplina — uma disciplina pertence a um semestre só, e a
    personalização vive nele enquanto ele está aberto. Persistir as duas seria
    duplicar a linha; o `Camadas` de `faculdade/regra.rs` tem **três**, e o
    referencial §4.3 (herança viva × congelada) fica fiel.
- **Justificativa.** §4.3 é **[D]** em duas frases que precisam valer juntas:
  herança viva no presente e congelamento no passado. Sem o ato de gravar, o
  histórico mente: um semestre avaliado com média simples aparece com média
  ponderada depois que a usuária mudou o padrão, e o número do histórico muda com
  o tempo. O histórico responde *"por que essa disciplina reprovou?"* — e essa
  pergunta exige a regra congelada, não só a nota final.
- **Alternativa rejeitada.** (a) Congelar um **snapshot da nota final** em vez da
  regra. Rejeitada porque responde com o número e não com o porquê. (b) Herança
  viva sem congelamento, calculando o histórico com a regra atual. Rejeitada
  porque §4.3 **[D]** diz que mudar o padrão não altera o histórico. (c)
  Personalização **por campo**. Rejeitada porque "mudar o padrão" muda de sentido
  conforme o campo; a substituição por inteiro é explícita e provável.
- **Consequência aceita.** Encerrar semestre **grava** e a regra de um semestre
  encerrado **não pode ser corrigida**. A correção real é lançar a nota que
  faltou — e isso é permitido, porque **nota é dado, regra é regra** (I9). A
  distinção precisa aparecer na interface.
- **Onde é garantida hoje.** `faculdade/regra.rs::resolver` implementa a ordem e
  `exigir_semestre_aberto` é a guarda. Desde F5 (2026-10-09) a guarda está no
  caminho de escrita: `faculdade/comandos.rs::exigir_escrita_do_semestre` protege
  `set_regra_disciplina`, `create_disciplina` e `create_avaliacao`, e
  `encerrar_semestre` resolve a regra efetiva de cada disciplina e **grava** em
  `regra_congelada` antes de fechar o semestre. Disciplina e avaliação novas
  também são recusadas em semestre encerrado: a regra efetiva de estrutura nova
  seria viva, e o histórico se moveria — é a mesma I7. A única escrita liberada
  depois do encerramento é `lancar_nota` (I9).

## Invariantes

As relevantes para o Faculdade, com o estado real. As demais (I10..I14) são de
Estudos ([`estudos.md`](./estudos.md)).

| # | Invariante | Onde | Estado em 2026-10-09 |
|---|---|---|---|
| I1 | Nenhuma coluna de média, situação ou "quanto falta" no schema | `migrations/0003_faculdade.sql` | **feito** (F4) — `rg -n "media\|situacao\|nota_necessaria" src-tauri/migrations/` → vazio |
| I2 | O motor é determinístico e não abre relógio | `faculdade/media.rs` | **feito** — `cargo test` |
| I3 | Média ponderada usa peso; simples ignora | `faculdade/media.rs` | **feito** — `simples_ignora_o_peso_e_ponderada_usa` |
| I4 | Arredondamento afeta a fronteira (6,95) | `faculdade/media.rs` | **feito** — `arredondamento_afeta_a_fronteira_de_6_95` |
| I5 | Frequência mínima reprova e a restante é derivada | `faculdade/media.rs` | **feito** — `frequencia_reprova_e_a_restante_e_derivada` |
| I6 | Disciplina sem personalização herda o padrão, e mudar o padrão a move | `faculdade/regra.rs` | **feito** — `sem_personalizacao_herda_o_padrao_e_mudar_o_padrao_move` |
| I7 | Regra de semestre encerrado é imutável | `faculdade/regra.rs` + `faculdade/comandos.rs::exigir_escrita_do_semestre` | **feito** (F5) — `faculdade_regra_e_estrutura_de_semestre_encerrado_sao_recusadas` |
| I8 | A regra congelada responde "por que reprovou" | `faculdade/regra.rs` + `regra_congelada` | **feito** — `congelada_vence_a_personalizacao_da_disciplina` (F2/F4) |
| I9 | Nota continua lançável em semestre encerrado; regra não | `faculdade/comandos.rs::lancar_nota` | **feito** no caminho de escrita (F5) — `faculdade_nota_continua_lancavel_em_semestre_encerrado`; interface **feita** (F8, 2026-10-09): o campo de nota segue ativo com o semestre encerrado |
| I15 | A situação como string nunca filtra; o filtro é o enum | `Situacao` | **feito** — o enum é a forma do filtro, `as_str` é apresentação |

## Contrato

A saída do motor é a de §5 da `SPEC-D-010` — `media`, `mediaSemArredondamento`,
`situacao`, `notaNecessariaProxima`, `frequenciaRestante`, `regraAplicada` — e o
desktop é quem calcula. O mobile, quando consumir, **não** recalcula média.

Desde F5 (2026-10-09) o contrato tem chamador em Rust e par em TypeScript: os
onze comandos de `faculdade/comandos.rs` (`list_semestres`, `list_disciplinas`,
`media_da_disciplina`, `list_avaliacoes`, `create_semestre`,
`create_disciplina`, `create_avaliacao`, `lancar_nota`, `set_regra_padrao`,
`set_regra_disciplina`, `encerrar_semestre`) são o bloco Faculdade da lista em
`src-tauri/src/lib.rs`, e o espelho é
[`src/features/faculdade/data/bridge.ts`](../../src/features/faculdade/data/bridge.ts)
— sem SQL e sem segundo cálculo: a ponte tipa, o Rust responde. O
`list_avaliacoes` fechou em F8 (2026-10-09): a tela não lança nota às cegas —
ela lista avaliação **com** a nota atual (`LEFT JOIN` no Rust, nota `0,0`
conta como nota).

## Medição

Números gerados, não copiados.

| Métrica | Comando | Antes (2026-09-30) | Depois (2026-10-09) |
|---|---|---|---|
| Casos de teste do motor e da regra | `cargo test --lib --no-default-features` | 0 (módulo não existia) | 18 (5 de `regra`, 13 de `media`) |
| Casos de teste do caminho IPC (guarda, congelamento, I9) | idem, testes `faculdade_*` de `faculdade/testes.rs` | 0 | 6 (5 em F5 + `faculdade_list_avaliacoes_traz_a_nota_atual` em F8) |
| Casos de golden de média | `rg -c '\"caso\"' golden/media.jsonl` | 0 | 10 (F6, 2026-10-09) |
| Colunas de derivado no schema | `rg -n "media\|situacao\|nota_necessaria" src-tauri/migrations/` | 0 | 0 (F4, 2026-10-09) |
| Tabelas do Faculdade no schema | `rg -c "^CREATE TABLE" src-tauri/migrations/0003_faculdade.sql` | 0 | 7 (F4, 2026-10-09) |
| Cascas de comando registradas (Calendário + Faculdade) | `rg -c "commands::" src-tauri/src/lib.rs` | 20 | 30 (F5, 2026-10-09) |

## Bloqueios e o que falta

- **F4 — schema (feito em 2026-10-09).** `0003_faculdade.sql` cria `semestre`,
  `disciplina`, `avaliacao`, `nota`, `regra_padrao`, `regra_disciplina` e
  `regra_congelada`, e semeia a regra padrão. A matriz de dono de
  `01-fronteiras.md` já atribui essas tabelas ao Faculdade.
- **F5 — comandos e IPC (feito em 2026-10-09).** O bloco Faculdade de
  `faculdade/comandos.rs` resolve a regra na leitura (`media_da_disciplina`), aplica
  `exigir_semestre_aberto` no caminho de escrita (`set_regra_disciplina`,
  `create_disciplina`, `create_avaliacao`), **grava a regra congelada** no
  encerramento e deixa `lancar_nota` sem guarda (I9).
  `src/features/faculdade/data/bridge.ts` espelha os onze comandos.
- **F6 — golden (feito em 2026-10-09).** [`golden/media.jsonl`](../../golden/media.jsonl)
  com 10 casos: o limite 6,95 nos dois passos (I4), os três modos de exame,
  frequência, `sem-notas` e a regra congelada (D75/I8). Os valores são
  aritmética de mão documentada no arquivo — aqui há **uma** implementação só,
  e o papel do golden é pinar o valor: mudança de fronteira falha alto. O
  teste que lê mora em `faculdade/media.rs`, junto da regra que ele prova, e foi
  verificado quebrando de propósito.
- **F8 — tela (v1 feita em 2026-10-09).**
  [`FaculdadeScreen.tsx`](../../src/features/faculdade/ui/FaculdadeScreen.tsx):
  semestres (com badge aberto/encerrado), disciplinas com a **média derivada**
  do Rust, avaliações com nota atual, lançamento/correção de nota e encerramento
  com confirmação. A UI desabilita estrutura em semestre encerrado e deixa a
  nota lançável (I7/I9) — e se algo escapar, o Rust recusa. Notas é **visão**
  dentro da disciplina, nunca tela solta (§4.3 **[D]**). Fora da v1:
  aulas/frequência por disciplina e a edição de regra (é Configurações).
- **[P] fora de escopo.** O simulador "e se" usa `mediaSemArredondamento` e não
  entra agora (§4.3 **[P]**). O modo preparação de prova é de Estudos.

## Referências

| Documento | O que resolve |
|---|---|
| [`SPEC-D-010`](../../../docs/specs/desktop/SPEC-D-010-motor-de-notas-e-fsrs.md) | a cópia de grupo desta decisão e do FSRS |
| [`SPEC-D-007`](../../../docs/specs/desktop/SPEC-D-007-dono-do-dado-e-fronteira-entre-modulos.md) | dono do dado; Faculdade é dona da regra de média |
| [`spec-referencial-workspace-academico.md`](../../../spec-referencial-workspace-academico.md) §4.3 | a origem: telas, entidades e o motor **[D]** |
| [`estudos.md`](./estudos.md) | a metade de Estudos (FSRS) |
| `src-tauri/src/faculdade/media.rs` | o motor implementado |
| `src-tauri/src/faculdade/regra.rs` | a resolução da regra implementada |
