# Estudos — FSRS e o histórico de revisão

> Fonte de verdade no app para a metade de **Estudos** da
> [`SPEC-D-010`](../../../docs/specs/desktop/SPEC-D-010-motor-de-notas-e-fsrs.md).
> A metade de **Faculdade** (motor de notas) mora em
> [`faculdade.md`](./faculdade.md).
> Decisões do app: `D76`.
> Origem: spec referencial §1.4 (**[D]**, cálculo no Rust), §3.1 (Estudos é dono do
> Conteúdo e do histórico FSRS), §4.4 (**[D]** FSRS; **[P]** retenção desejada,
> otimização de parâmetros, modo preparação), §6.1 (a prova aciona a preparação).
> Estado: **backend e tela v1 existem (2026-10-09)** — o motor FSRS está em
> `src-tauri/src/estudos/fsrs.rs`, o schema em `src-tauri/migrations/0004_estudos.sql`
> (3 tabelas), o golden em `golden/fsrs.jsonl` (8 casos), e os comandos IPC
> (`create_conteudo`, `create_cartao`, `fila_do_dia`, `revisar_cartao`) com a
> ponte `bridge.ts` e a `EstudosScreen` (fila + revisão). As **quatro telas de
> §4.4** seguem em spec própria — ver §Bloqueios.

Este documento é a decisão de **módulo** de Estudos, ao lado de
`00-shell-e-navegacao.md`, `01-fronteiras.md` e `faculdade.md`. Ele **não**
cobre as quatro telas de §4.4 (Hoje · Conteúdos · Prática · Sessões e Histórico):
a `SPEC-D-010` §4 é explícita em que este escopo é o **cálculo** — FSRS — e o
resto do módulo é assunto de spec própria.

## Contexto

Estudos responde *"o que eu estudo hoje e como estou indo?"* (§4.4). O **Conteúdo**
é a unidade estudável e pertence a Estudos, porque ela também estuda temas livres;
Disciplina e Trilha são **agrupamentos** de conteúdos (um conteúdo pode estar em
zero, um ou vários).

O cálculo que este módulo introduz é a **revisão espaçada com FSRS**, no Rust. O
FSRS modela cada cartão por estabilidade, dificuldade e recuperabilidade, e pode
ser ajustado ao histórico da própria usuária. A spec referencial diz "conferir
versão e licença na hora de implementar" — ou seja, a escolha da crate é item de
implementação, e o que se decide aqui é a **adaptação ao produto**.

## D76 · O FSRS é uma crate auditada em Rust, com estado por cartão no banco, e nenhum parâmetro é inventado

- **Decisão.** O motor de repetição espaçada é uma **crate auditada** de FSRS em
  Rust (mesma política de `SPEC-D-002` D50: o algoritmo é de terceiro, a adaptação
  ao produto é nossa). O que é nosso: (a) o **estado por cartão** — estabilidade,
  dificuldade, última revisão, estado de relearning — vive no banco, no módulo
  Estudos, que é o dono; (b) o **relógio é injetado** e a data de vencimento é
  calculada com a **mesma âncora de fuso** da recorrência (`SPEC-C-002`); (c) a
  **retenção desejada** é parâmetro do motor, não constante; (d) os casos do
  cálculo — nova, fácil, difícil, relearning, correta, incorreta — são o **golden**
  compartilhado, porque FSRS é numérico e "quase certo" não passa.
- **Justificativa.** §4.4 é **[D]** e põe o FSRS no Rust; §1.4 **[D]** põe o
  cálculo no Rust. O ponto (d) é o que separa "chamar a crate" de um motor
  correto: uma reimplementação parcial ou um parâmetro divergente produz uma fila
  que funciona e está **errada** — o app continua revisando, só revisa no dia
  errado. Golden é o que torna isso detectável.
- **Alternativa rejeitada.** (a) Reimplementar FSRS em Rust. Rejeitada pelo mesmo
  motivo de D50: reimplementar o que não é nosso é o caminho da divergência
  silenciosa. (b) Usar SM-2. Rejeitada porque §4.4 é **[D]** e o diferencial do
  produto (§7) é o FSRS em Rust. (c) Calcular o FSRS no TypeScript. Rejeitada
  porque §1.4 **[D]** põe o cálculo no Rust, e a fila do mobile tem que ser a
  mesma do desktop. (d) Otimizar os parâmetros agora. Rejeitada porque §4.4 marca
  a otimização como **[P]**: otimizar sem histórico é otimizar para um número
  inventado.
- **Consequência aceita.** A retenção desejada é parâmetro e mudá-la **reescreve**
  o agendamento de todo o histórico; isso é o comportamento do algoritmo e é
  decisão de produto. O que a spec exige é que a mudança seja **explícita e
  registrada**, com data, e que a fila seja recalculada de forma determinística a
  partir do mesmo histórico — não por ajuste incremental. E o custo de manter a
  crate é o custo declarado em D50: dependência de terceiro com golden, que é
  terceirização de implementação e **não** de regra.

### Cláusula do vencimento (dentro de D76) · nenhuma coluna derivada, e a fila é por dia

- **Decisão.** O estado por cartão (estabilidade, dificuldade, última revisão,
  estado de relearning) é persistido; o **vencimento** da revisão é **calculado**
  com o relógio injetado e a âncora de fuso, e a fila é por **dia** — não por
  instante. Trocar o fuso da máquina **não** desloca a fila (I11).
- **Justificativa.** É a mesma disciplina do atraso e da média: derivado não se
  grava. O vencimento depende da retenção alvo, que é configurável e pode ser
  recalculada; gravá-lo o tornaria uma segunda fonte de verdade. E a fila por dia
  evita a classe de bug de DST: uma data local com hora fixa vira instante pela
  âncora, e horários inexistentes de mudança de fuso não podem mover a fila.
- **Alternativa rejeitada.** (a) Gravar `proxima_vencimento` como coluna.
  Rejeitada porque muda quando a retenção muda, e vira estado para
  dessincronizar. (b) Fila por instante. Rejeitada porque o produto pergunta "o
  que revisar **hoje**", e "hoje" é data local.
- **Consequência aceita.** Toda leitura da fila recalculа os vencimentos a partir
  do estado dos cartões — O(n) sobre os cartões devidos, e o custo só importa
  depois de existir histórico.

## Invariantes

As de Estudos; as de Faculdade estão em [`faculdade.md`](./faculdade.md).

| # | Invariante | Onde | Estado em 2026-10-09 |
|---|---|---|---|
| I10 | O estado por cartão de FSRS vive no banco, no módulo Estudos | `migrations/0004_estudos.sql` (`cartao`) | **feito** — `estabilidade`/`dificuldade`/`ultima_revisao`/`intervalo_dias`/`relearning` são colunas; `revisao` é o evento |
| I11 | Trocar o fuso da máquina não desloca a fila | `estudos/fsrs.rs::dia_de_vencimento` | **feito** — a zona é parâmetro; pino no caso `vencimento-na-ancora-SP` do golden |
| I12 | O cálculo de FSRS é determinístico a partir do mesmo estado | `estudos/fsrs.rs::revisar` | **feito** — relógio injetado, `FSRS::default()` por chamada; golden lê o mesmo arquivo e reproduz |
| I13 | Os casos de FSRS são o golden, com versão e licença da crate registradas | `golden/fsrs.jsonl`, `Cargo.toml` | **feito** — 8 casos; crate `fsrs 6.6.2` (BSD-3-Clause) registrada no `Cargo.toml` |
| I14 | Avaliação aponta Conteúdos e a fila prioriza por data de prova | cálculo de prioridade | **pendente** — modo preparação é **[P]** |

## Contrato

A forma da saída **refina** o esboço de §5 da `SPEC-D-010`, e as duas diferenças
são consequência direta da cláusula do vencimento (D76), decidida depois daquele
esboço:

1. o vencimento é **dia civil local** (`YYYY-MM-DD`), não instante UTC — a fila
   é por dia, e "hora fixa" seria inventar uma hora que o produto não precisava;
2. o `estado` não viaja como enum: viajam os **dados que o decidem**
   (`vencimento` null = novo, flag `relearning`), e o rótulo é apresentação da
   tela. "Revisada" do esboço não tem semântica definida e não foi inventada
   aqui.

```jsonc
{
  "cartaoId": "…",
  "frente": "…",
  "verso": "…",
  "vencimento": "2026-10-12",  // dia civil na zona da usuária; null = cartão novo
  "relearning": false,         // dado do cartão (D76), não derivado
  "retencaoAlvo": 0.9          // eco do parâmetro da revisão — o padrão é [P]
}
```

O mobile consome essa fila e **não** recalcula FSRS.

## Medição

Números gerados, não copiados.

| Métrica | Comando | Antes (2026-09-30) | Depois (2026-10-09) | Gate |
|---|---|---|---|---|
| Casos de golden de FSRS | `rg -c '\"caso\"' golden/fsrs.jsonl` | 0 | 8 (mínimo de SPEC-D-010 §9: 6) | `npm run verify` |
| Versão e licença da crate registradas | `rg -n "fsrs" src-tauri/Cargo.toml` | 0 | 1 — `fsrs 6.6.2`, BSD-3-Clause | revisão |
| Otimização declarada como feita | `rg -n "otimiza" docs/espacos/estudos.md` | 0 | 0 — é **[P]** | revisão |
| Tabelas de estado de cartão no módulo errado | `rg -n "CREATE TABLE.*cartao" src-tauri/migrations/` cruzado com a matriz de dono | 0 | 0 — a tabela é de Estudos | `SPEC-D-007` D67 |
| Testes Rust do módulo | `cargo test --lib --no-default-features` (de `src-tauri/`) | 0 de Estudos | 11 (7 de `estudos::fsrs` + 4 de caminho em `commands.rs`), +1 gerador `#[ignore]` | `npm run verify` |

## Bloqueios e o que falta

- **F7 — crate: FEITO (2026-10-09).** A crate é `fsrs 6.6.2`
  (open-spaced-repetition, BSD-3-Clause, `rust-version` 1.81 — conferidas na
  implementação, como §4.4 manda), registrada em `src-tauri/Cargo.toml` com o
  motivo de o otimizador não ser chamado. `rs-fsrs` foi recusada na hora da
  escolha por licença não declarada. O motor (`estudos/fsrs.rs`) e o golden
  (`fsrs.jsonl`, 8 casos) existem e rodam no gate.
- **Comandos IPC e tela v1 — FEITO (2026-10-09).** Quatro comandos:
  `create_conteudo`, `create_cartao`, `fila_do_dia` e `revisar_cartao` (seção
  "21. Estudos" de `commands.rs`), a ponte `src/features/estudos/data/bridge.ts`
  e a `EstudosScreen` — a fila do dia com a revisão (frente → verso → nota) e o
  seletor de retenção. O cartão nasce sem passado (I10/§7): a struct de criação
  **não tem** campo de estado.
- **As quatro telas de §4.4 (Hoje · Conteúdos · Prática · Sessões e Histórico) —
  spec própria.** A tela v1 é a fila e a revisão; o resto do módulo é
  `SPEC-D-010` §4: não entra sem spec.
- **Retenção desejada — `[P]`, com provisório.** §4.4 marca o **padrão** da
  retenção como **[P]**. Decisão de 2026-10-09 (relatada pela dona): a tela usa
  **0,9 provisório** com seletor visível, e o 0,9 segue registrado como [P]
  até confirmação no referencial. O motor continua recebendo a retenção como
  parâmetro validado em (0, 1).
- **Otimização de parâmetros — `[P]`.** Não entra (§4.4, §8). Otimizar sem
  histórico é ajustar para número inventado.
- **Modo preparação de prova — `[P]`.** O **cálculo** de prioridade depende de
  Avaliação apontar Conteúdos, e a interface é **[P]**. Não entra sem confirmação.

## Referências

| Documento | O que resolve |
|---|---|
| [`SPEC-D-010`](../../../docs/specs/desktop/SPEC-D-010-motor-de-notas-e-fsrs.md) | a cópia de grupo desta decisão e do motor de notas |
| [`SPEC-D-002`](../../../docs/specs/desktop/SPEC-D-002-uma-verdade-de-recorrencia.md) | a política de D50 (crate de terceiro + golden) |
| [`SPEC-D-007`](../../../docs/specs/desktop/SPEC-D-007-dono-do-dado-e-fronteira-entre-modulos.md) | dono do dado; Estudos é dono do histórico FSRS |
| [`spec-referencial-workspace-academico.md`](../../../spec-referencial-workspace-academico.md) §4.4 | a origem: Estudos **[D]** e as pendências **[P]** |
| [`faculdade.md`](./faculdade.md) | a metade de Faculdade (motor de notas) |
