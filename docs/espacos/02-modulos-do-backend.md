# Módulos do backend Rust

> Onde vive: `docs/espacos/02-modulos-do-backend.md` (fonte de verdade no app)
> Origem de grupo: [`SPEC-D-015`](../../../docs/specs/desktop/SPEC-D-015-modulos-do-backend-rust.md)
> Decisões do app: `D96`, `D97`, `D98`, `D99`, `D100`
> Data: 2026-10-14
> Verificado em: 2026-10-14 (`cargo test --lib --no-default-features` → 160
> passed, 1 ignored; `npm run check:boundaries` → 0 violações)

> Por que este arquivo existe. O [`01-fronteiras.md`](./01-fronteiras.md) decide
> **quem é dono de qual dado** e promete, em `D67`, "um módulo Rust por dono de
> dado". A segunda metade dessa frase foi cumprida e verificada: nenhum comando
> escreve em tabela de outro dono. A **primeira metade nunca foi implementada** —
> `src-tauri/src/commands.rs` é um módulo Rust só, e ele é o caminho de escrita
> de **quatro** donos. Este arquivo cumpre a primeira metade.

---

## Contexto

`src-tauri/src/` tem 12285 linhas de Rust. `commands.rs` tem **6537** — 53,2% do
crate inteiro, e **5,8 vezes** o maior arquivo depois dele
(`src-tauri/src/tcc/docx.rs`, 1124 linhas).

Esse arquivo não é uma unidade coerente. Ele é quatro:

| Bloco | Linhas de produção | Núcleos `*_core` | Tabelas |
|---|---|---|---|
| 1–19 · Calendário + Settings | 1854 | 19 | `calendar_event`, `event_occurrence`, `event_recurrence`, `responsibility`, `responsibility_step`, `plan_block`, `execution_record`, `layer`, `setting` |
| 20 · Faculdade | 909 | 11 | `semestre`, `disciplina`, `avaliacao`, `nota`, `regra_padrao`, `regra_disciplina`, `regra_congelada` |
| 21 · Estudos | 378 | 4 | `conteudo`, `cartao`, `revisao` |
| 22 · TCC & Projetos | 978 | 13 | `projeto`, `documento`, `no_documento`, `inline_run`, `versao`, `item_a_resolver`, `base_de_pesquisa` |

Mais 2418 linhas de teste (37% do arquivo) em dois `#[cfg(test)] mod` no fim.

O módulo Rust já existe — para o **motor**. `materia/`, `estudos/` e `tcc/` são
puros e não tocam SQL. A fronteira de módulo, porém, **para no motor**: ela não
alcança o caminho de escrita, que é onde `guard_origin` (INV-6),
`exigir_transicao` (C5), `exigir_semestre_aberto` (I7) e o `UNIQUE (event_id,
original_start)` que dá idempotência à materialização (D10) são realmente
impostos. Toda invariant do backend é imposta em um arquivo que não pertence a
nenhum módulo.

O mesmo app, do lado TypeScript, já faz o certo: `src/features/` tem uma pasta
por módulo, com `data/`, `domain/`, `ui/` e testes em `.test.ts` separados, e
**zero** import entre pastas de módulos. A disciplina existe na casa; falta no
Rust.

### O que a quebra já custou

O arquivo cresce por adição. As 2.265 linhas de Faculdade, Estudos e TCC
chegaram depois das 1.854 do Calendário, e as **2.265 foram anexadas ao final do
mesmo arquivo** (`commands.rs:1855`, `:2764`, `:3142`). Um módulo novo entra
depois do último `}` do anterior, sem marcador de onde começa. O cabeçalho do
arquivo, que documenta o módulo Calendário e cita `INV-1`, `INV-3`, `D12` e o
débito C5, agora descreve um arquivo que também contém a regra de congelamento
de semestre (I7) e a lista de projeto do TCC (D85).

E o gate não pega. `D67` prometeu gate dos dois lados; a regra 8 de
`check-boundaries.mjs` existe e roda. Ela **lê um arquivo só**
(`scripts/check-boundaries.mjs:353`) — `src-tauri/src/commands.rs` — porque é o
único que tinha escrita. Quando o arquivo é dividido sem mudar o gate, a regra 8
passa a ver **zero escrita** e continua verde.

Isso foi medido, não suposto (§"A armadilha do gate").

### A armadilha do gate

Verificação negativa em 2026-10-14, contra uma cópia do app em `/tmp`, para não
tocar no checkout:

1. Código original + uma violação real de `D67` injetada no corpo de produção
   (um `pub fn injetada_core` que faz `UPDATE responsibility`, invocado pela
   bridge de `faculdade`): o gate **acusa**.

   ```
   ✗ 1 violação(ões) de fronteira:
     src-tauri/src/commands.rs:1  [dono-de-dado]
       `injetada` escreve em tabela de `calendario`, mas é invocado pela bridge
       de `faculdade` — comando de um módulo não grava dado de outro (D67).
   ```

2. **O mesmo código**, a mesma violação, depois de dividir `commands.rs` em
   `calendario/`, `faculdade/`, `estudos/`, `tcc/`: o gate **aprova**.

   ```
   ✓ fronteiras ok: 69 arquivos, 30 tabelas do backend verificadas contra src/
   ```

A violação continua no código. O gate não a viu.

Isto generaliza o que F8b2 já mediu (o corte no primeiro `#[cfg(test)]` deixou a
regra 8 cega) e o torna **imediato**: não é um bug de recorte, é o desenho — a
regra lê um nome de arquivo em vez de percorrer a árvore. Por isso a ordem das
fatias em §"A migração" importa: o gate primeiro, o split depois.

---

## D96 · Um módulo Rust por dono de dado, e o caminho de escrita mora dentro dele

- **Decisão.** Cada dono de dado tem um diretório em `src-tauri/src/`, com o
  nome do dono, e o caminho de escrita dele mora nesse diretório. O
  `commands.rs` some. O diretório de cada módulo tem `dominio` (o motor puro,
  quando existe), `comandos` (o caminho de escrita) e `testes`.
- **Justificativa.** `D67` já manda; falta a metade. E a separação motor ×
  escrita é a que o app já pratica nos dois lados: os motores são puros e não
  tocam SQL (`materia/mod.rs:16-18`), e é por isso que `media_da_disciplina`
  deriva média na leitura sem gravar nada (I1). Com a escrita no módulo, essa
  propriedade fica visível: "onde está a escrita de Faculdade?" tem uma resposta
  que não depende de varrer 6537 linhas.
- **Alternativa rejeitada.** (a) **Só dividir os motores, mantendo o
  `commands.rs`.** É o estado de hoje e é o que produziu o arquivo: os motores já
  estão divididos desde 2026-10-03, e o arquivo continuou crescendo. (b) **Um
  módulo por tabela.** São 30 tabelas e 4 donos; módulo por tabela perde a
  unidade que `D65` declara e espalha a escrita de um dono por dez diretórios.
  (c) **Dividir por camada** — todas as leituras num arquivo, todas as escritas
  em outro. O corte atravessa o dono, então "este comando pertence a
  Faculdade?" deixa de ter resposta, e `D67` perde o objeto que ele cruza.
- **Consequência aceita.** `materia/` passa a se chamar `faculdade/`, e
  `src-tauri/src/domain.rs` passa a `calendario/dominio.rs`. Ambos são renomeação
  mecânica com consumidores pequenos (`crate::materia` em 4 pontos de
  `commands.rs`; `domain.rs` citado no `AGENTS.md` e em `D67`). O preço é uma
  onda de mudanças em documentação que cita caminho — e omitir o renomeamento
  significaria manter, para sempre, um mapa "diretório → dono" que envelhece em
  silêncio, que é exatamente o defeito que `D65` já registra como falha.

## D97 · O dono vem do caminho do arquivo, e o gate lê arquivo por arquivo

- **Decisão.** A regra 8 de `scripts/check-boundaries.mjs` passa a percorrer
  `src-tauri/src/**/*.rs`, **um arquivo por vez**, e o dono de cada escrita vem do
  **nome do diretório** em que o arquivo está. Não há mapa diretório → dono, e
  não há nome de arquivo de Rust escrito à mão no gate.
- **Justificativa.** O gate nomeia `src-tauri/src/commands.rs` em 7 pontos
  (`scripts/check-boundaries.mjs:353-354, 434, 446, 456, 463, 476, 493`). Com 4
  módulos ele teria 4 nomes escritos à mão, e um módulo novo nasce invisível
  até alguém lembrar de acrescentá-lo. Pior: ler o crate inteiro como uma
  string e cortar no **primeiro** `#[cfg(test)] mod` é o que produziu o corte
  cego de F8b2, e a medição acima mostra que a hipótese "sem escrita = tudo
  certo" é exatamente o que quebra a regra. Ler por arquivo elimina as duas
  fragilidade: nenhum nome escrito à mão, e nenhum corte heurístico.
- **Alternativa rejeitada.** (a) **Manter o nome do arquivo e apontá-lo para cada
  módulo novo.** Quatro nomes escritos à mão e nenhuma cobertura para o quinto
  módulo. (b) **Manter um mapa `diretório → dono`.** É o mesmo mapa que a regra
  já tem para `tabela → dono`, e `D65` já o classificou: mapa sem classificação é
  falha, e mapa velho é a matriz mentindo. (c) **Concatenar o crate e inferir o
  dono por Marcadores de comentário** (`// 20. Faculdade`). Comentário não é
  fronteira; alguém move um bloco e o comentário vai junto.
- **Consequência aceita.** O nome do diretório **é** o id do dono, sem
  tradução. Por isso `materia` → `faculdade`: o id já é `faculdade` em
  `DONO_DA_TABELA`, em `src/shell/navigation.ts` e na migration
  `0003_faculdade.sql`, e o diretório é a única peça fora do padrão. O gate
  ganha também a **verificação negativa** que faltava: um teste que injeta uma
  violação e exige que o gate acuse, senão o gate é de novo opinião.

## D98 · A dívida da migração é nomeada e encolhe; o gate nunca fica verde por acidente

- **Decisão.** A regra nova começa com uma lista `EXCECOES_NA_RAIZ` que declara,
  por nome, quais arquivos da raiz do crate ainda escrevem. A lista é impressa
  em **toda** execução, junto da contagem. Cada fatia da migração remove uma
  entrada. A última fatia **apaga o ramo de exceção**, e a partir daí arquivo de
  escrita na raiz é violação.
- **Justificativa.** As duas alternativasóbvias estão medidas. **Gate primeiro,
  split depois**, deixaria o gate vermelho no primeiro commit e transformaria as
  quatro fati numa mudança indivisível. **Split primeiro, gate depois** — a
  ordem natural de quem vai fazer o refactor — produz o verde falso da medição
  acima: um `✓` que significa "não vi nada" e não "está certo". A lista
  nomeada dá o terceiro caminho: cada fatia é verde porque o que falta está
  **escrito na saída**, e o número encolhe de 1 até 0 à vista.
- **Alternativa rejeitada.** (a) **Gate fail-now, split depois.** Correto no
  fundo, impossível de operar: um gate que falha desde o primeiro dia é
  desligado por quem o vê falhar, e um gate desligado não protege o refactor
  que ele deveria proteger. (b) **Split sem tocar no gate.** É o verde falso
  medido. (c) **Nenhum gate, confiar em revisão.** Quatro donos e 30 tabelas por
  olho, na exatamente-diff-em que o olho já está cansado.
- **Consequência aceita.** Existe um estado intermediário em que o gate aceita
  dívida declarada. Ele é aceitável **só** porque a dívida é nomeada, impressa e
  tem número que precisa chegar a zero; e o ramo de exceção é removido no fim,
  não fica. O gate passa a ter duas regras de escrita Rust onde tinha uma, e
  `EXCECOES_NA_RAIZ` é código que existe para morrer.

## D99 · Cada módulo Rust é `dominio` + `comandos` + `testes`

- **Decisão.** Todo diretório de módulo tem `mod.rs`, `comandos.rs` e
  `testes.rs`; `dominio.rs` quando o módulo tem motor. Os testes saem do corpo do
  arquivo e viram `#[cfg(test)] mod testes;`.
- **Justificativa.** Hoje 37% de `commands.rs` são dois `#[cfg(test)] mod` no
  fim — 2418 linhas que qualquer leitura do caminho de escrita atravessa. Pior, o
  `#[cfg(test)]` no meio do arquivo é **o ponto de corte que o gate usa**
  (`scripts/check-boundaries.mjs:456` corta no primeiro `mod`), e essa
  coincidência é o defeito de F8b2. Teste em arquivo próprio tira a dependência
  do corte e deixa o arquivo de escrita com uma responsabilidade só. O nome
  `testes` é o mesmo do Rust e o mesmo que o gate já usa para descrever o corte.
- **Alternativa rejeitada.** (a) **Testes inline, como hoje.** O arquivo que a
  regra quer pequeno carrega 37% do próprio peso em teste, e o `#[cfg(test)]`
  continua sendo fragilidade do gate. (b) **Uma pasta `src-tauri/tests/` com um
  arquivo por módulo.** O teste de Faculdade deixa de estar ao lado de
  Faculdade, e a fronteira que se quer reforçar fica mais fraca, não mais forte.
  (c) **Um `testes/` por módulo dentro do diretório do módulo** (`comandos/`
  vira pasta). Custa um nível de diretório para um arquivo por módulo.
- **Consequência aceita.** Os testes de um módulo passam a precisar de
  `use super::*` sobre `comandos.rs` para alcançar o `*_core`, e nenhum `*_core`
  fica `pub` **apenas** por causa de teste. Hoje 46 dos 47 núcleos são `pub` e um
  é privado (`upsert_recurrence_core`, `commands.rs:509`) — auniformidade passa a
  ser explícita: núcleo é `pub` porque é a API do módulo, e o teste usa o
  caminho do módulo.

## D100 · O teto de `<módulo>/comandos.rs` é verificado por comando, e subirá-lo é ato deliberado

- **Decisão.** Nenhum `<módulo>/comandos.rs` passa de **2000 linhas**. O gate
  falha, nomeando o arquivo e a linha onde passa. Subir o teto é editar o gate,
  e o número vive no gate — não em documento.
- **Justificativa.** Sem teto, a divisão é um evento, não um estado: o arquivo
  volta a 6537 linhas em quatro módulos novos. O teto é a mesma forma que o
  codebase já aceitou para a expansão de recorrência (`const MAX: u16 = 65535`,
  `commands.rs:425`) — teto explícito, verificado, com o motivo escrito. E 2000
  não é arbitrário: o maior módulo depois do split é o Calendário, com 1854
  linhas de produção, então 2000 é folga para crescimento real e não licença
  para recomeçar.
- **Alternativa rejeitada.** (a) **Sem teto, com medição só em relatório.** A
  medição vira número de documento, e número de documento envelhece. (b) **Teto
  por módulo, afinado ao tamanho de cada um.** Quatro números para quatro
  arquivos vira quatro valores para discutir, e o menor (Estudos, 378 linhas) dá
  uma falsa urgência. (c) **Teto em contagem de comandos** em vez de linhas.
  Comando conta unidade, e a unidade que dói aqui é a linha: 37% do arquivo hoje
  é teste.
- **Consequência aceita.** Quando o teto estourar, a resposta **não** é subir o
  número. É dividir o `comandos.rs` do módulo por sub-recurso
  (`calendario/comandos/{evento,ocorrencia,responsabilidade,bloco,execucao}.rs`),
  e o dono do dado continua sendo o diretório. Esse desenho de segundo nível não
  é implementado agora: nenhum módulo está perto de 2000 linhas, e implementá-lo
  por antecipação é a forma mais cara de estar errado.

---

## Invariantes

| # | Invariante | Onde é garantida | Verificação |
|---|---|---|---|
| I1 | Todo arquivo Rust que escreve em tabela do schema mora num diretório de módulo, e o nome do diretório é o id do dono | regra 8 reescrita de `D97` | `npm run check:boundaries` |
| I2 | Nenhum `<módulo>/comandos.rs` escreve em tabela de outro dono, **seja o `fn` público ou privado** | regra 8 (herdada de `D67`) + verificação negativa nos dois casos | `npm run check:boundaries` + a verificação negativa |
| I3 | `EXCECOES_NA_RAIZ` está vazia; arquivo de escrita na raiz é violação | ramo de exceção removido na última fatia | `npm run check:boundaries` |
| I4 | Cada `pub fn X_core` tem casca `X` registrada em `lib.rs`, e nenhum nome é registrado duas vezes | regra nova em `D97` | `npm run check:boundaries` |
| I5 | Nenhum `<módulo>/comandos.rs` passa de 2000 linhas | `D100` | `npm run check:boundaries` |
| I6 | O gate acusa uma violação de dono injetada, **tanto em `pub fn` quanto em `fn` privado** | verificação negativa | `npm run test:gate` (script novo em F0) |
| I7 | O split não altera comportamento: mesmo número de testes Rust, mesmo golden, e **nenhum SQL alterado** | revisão por diff | `npm run verify` + o diff de literais SQL de §"Critérios de aceite" |

I7 é o que separa "reorganizar" de "mexer". O critério não é "os testes passam" —
testes passam com SQL errado se o teste não cobre. É que **a string de SQL seja
byte-idêntica**: um `git diff` que só move linhas não produz diferença na lista
de literais.

## O gate

`scripts/check-boundaries.mjs` passa de **oito** para **dez** regras. As sete
primeiras (TypeScript) não mudam; a oitava é reescrita e ganha um ramo de
dívida; a nona e a décima são novas.

**Regra 8, reescrita.** Deixa de ler `src-tauri/src/commands.rs`
(`:353-354`) e passa a percorrer `src-tauri/src/**/*.rs`. Para cada arquivo:

1. o dono vem do diretório, pela primeira parte do caminho relativo
   (`calendario/comandos.rs` → `calendario`; `nucleo/db.rs` → sem dono);
2. o corpo é analisado **só naquele arquivo**, sem cortar em `#[cfg(test)]`;
3. **todo `fn` é analisado, público ou privado.** O privado continua sendo
   caminho de escrita: a casca `pub` do mesmo módulo o chama, então ele é
   alcançável de fora do crate. Ignorar `fn` privado devolveria o verde falso
   pela porta dos fundos — e já há um assim hoje
   (`upsert_recurrence_core`, `commands.rs:509`, privado entre 47 públicos);
4. escrito em tabela de dono ≠ dono do arquivo é violação.

A verificação negativa cobre os dois casos: um `pub fn` **e** um `fn` privado,
cada um gravando tabela de outro dono. Se só um dos dois for testado, a regra
volta a ter um lado cego.

**Regra 8 (parte de dívida).** Arquivo de escrita na raiz do crate é violação,
salvo se o nome estiver em `EXCECOES_NA_RAIZ`. A lista é impressa sempre:

```
✓ fronteiras ok: 69 arquivos, 30 tabelas
  escrita na raiz do crate: commands.rs — dívida D98 (1 de 4 módulos)
```

**Regra 9 (nova, `D97` · I4).** `lib.rs` registra por caminho completo do
módulo (`calendario::comandos::list_events_in_window`). O gate cruza a lista
registrada com os `pub fn` de cada `<módulo>/comandos.rs` e recusa: nome
registrado que não existe, `*_core` sem casca, casca sem registro, nome
registrado duas vezes, e registro cujo caminho não começa por um diretório de
módulo. Hoje os três primeiros são **zero** — medido: 47 núcleos públicos + 1
privado, 48 cascas, 48 registros — então a regra começa verde, e é verde por
medição, não por omissão.

**Regra 10 (nova, `D100` · I5).** Nenhum `<módulo>/comandos.rs` passa de 2000
linhas. O gate falha nomeando o arquivo.

## A migração

Seis fatias. Cada uma é um commit, cada uma é verde, e a ordem não é
preferência: a fatia 0 é a que impede o verde falso.

| # | O quê | O que fecha | Como se sabe que fechou |
|---|---|---|---|
| **F0** | Gate: regra 8 por arquivo, `EXCECOES_NA_RAIZ` nomeada e impressa, regra 9, verificação negativa | prepara I1, I2, I4, I6 | `npm run verify` verde **e** a saída nomeia `commands.rs` como dívida de 1 de 4 |
| **F1** | `nucleo/` (`Db`, `DbError`, `new_id`, `now_ms`) e `calendario/` (`dominio.rs`, `comandos.rs`, `testes.rs`) | I5 para o maior módulo | dívida cai para **3 de 4** |
| **F2** | `faculdade/`: `materia/` → `faculdade/`, com `comandos.rs` e `testes.rs` | I1 para Faculdade | dívida cai para **2 de 4** |
| **F3** | `estudos/comandos.rs` e `estudos/testes.rs` | idem | dívida cai para **1 de 4** |
| **F4** | `tcc/comandos.rs` e `tcc/testes.rs` | idem | dívida cai para **0 de 4** |
| **F5** | Apaga o ramo de exceção e a lista; `lib.rs` passa a registrar por caminho completo | I3 | `npm run verify` verde com a lista **ausente** da saída |

### Por que `nucleo/`, e não `db.rs` na raiz

Só quatro itens são compartilhados por mais de um dono: `Db`, `DbError` (que é o
`DbError` de todo mundo), `new_id()` e `now_ms()`. Dos 42 helpers privados
de `commands.rs`, os outros **40** são de um dono só — `guard_origin` e
`exigir_transicao` são do
Calendário, `resolver_regra` e `semestre_esta_aberto` são de Faculdade,
`cartao_com_vencimento` é de Estudos, `montar_documento` e `tipo_no_de_str` são
do TCC. Um diretório com quatro itens é o **núcleo**, e ele não é dono de dado
nenhum: por isso o gate o trata como sem dono, e `setting` — que é escrita de
preferência local, sem dono de módulo — continua separada dele
(`DONO_DA_TABELA["setting"] = "local"`).

Deixar `db.rs` na raiz seria mais um arquivo sem dono, e a regra 8 passa a ter de
saber que `db.rs` escreve `layer` no seed. `nucleo/` deixa o caso explícito.

### Por que F1 vai primeiro

O Calendário é o maior bloco (1854 linhas de produção, 19 núcleos) e é o único
que traz **dois** renomeamentos junto (`db.rs` → `nucleo/`, `domain.rs` →
`calendario/dominio.rs`). Fazer o módulo maior primeiro, com o gate já medindo
a dívida, significa que cada uma das três fatias seguintes é uma repetição da
mesma mecânica num bloco três vezes menor. O contrário — começar por Estudos, com
378 linhas — treina o movimento num bloco pequeno e deixa o maior para o fim, com
o gate já maduro e o histórico de três fatias para consultar.

## Medição

Números gerados, não copiados. Todos de **2026-10-14**.

| Métrica | Como medir | Antes | Depois |
|---|---|---|---|
| Linhas em `commands.rs` | `wc -l src-tauri/src/commands.rs` | 6537 | o arquivo não existe |
| Maior arquivo Rust do crate | `find src-tauri/src -name '*.rs' \| xargs wc -l \| sort -rn \| head -1` | 6537 | ver §"Depois" abaixo |
| Escrita em arquivo sem dono | `npm run check:boundaries` (dívida impressa) | 1 de 4 | **0 de 4** |
| Diretórios de dono em `src-tauri/src/` | `find src-tauri/src -mindepth 1 -maxdepth 1 -type d` | 3 (`estudos`, `materia`, `tcc`) | 4 (`calendario`, `faculdade`, `estudos`, `tcc`) + `nucleo` |
| `#[test]` dentro do arquivo de comandos | `grep -c '#\[test\]' src-tauri/src/*/comandos.rs \| paste -sd+ \| bc` | 62 em um arquivo | 62 em quatro `testes.rs` |
| Comandos registrados | `grep -c 'commands::' src-tauri/src/lib.rs` | 48 | 48 — **não muda** |
| Testes Rust | `npm run verify:rust:domain` | 160 passed, 1 ignored | 160 passed, 1 ignored |

O "Depois" do maior arquivo é o que interessa: com o split, o maior passa a ser
`calendario/comandos.rs`, e o teto de `D100` o mantém abaixo de 2000. A métrica
que decide o fim da migração não é "o arquivo encolheu" — é **`escrita em
arquivo sem dono: 0`**, porque essa é a propriedade que o split compra.

## Critérios de aceite

- [ ] `npm run verify` sai com 0 (o gate único; `SPEC-D-005`)
- [ ] `npm run verify:rust:domain` sai com `160 passed` — o mesmo número de antes.
      **Queda de contagem é falha de aceite**, não notícia: um teste que
      desaparece no movimento de arquivo é o sintoma de um `mod testes;` que não
      declarava o que o `#[cfg(test)] mod` declarava
- [ ] `npm run test:golden` sai com 0 (os 6 goldens)
- [ ] `npm run check:boundaries` sai com 0 **e** a saída não nomeia dívida
- [ ] `npm run check:boundaries` sai com 0 **e** falha quando o teste de
      verificação negativa injeta uma violação
- [ ] A lista de literais SQL de `src-tauri/src` é **byte-idêntica** antes e
      depois:

      ```bash
      # antes, no commit-base
      rg -o '"[^"]*(SELECT|INSERT INTO|UPDATE|DELETE FROM)[^"]*"' src-tauri/src \
        | sed 's/^[^:]*://' | sort > /tmp/sql-antes.txt
      # depois, no fim da migração
      rg -o '"[^"]*(SELECT|INSERT INTO|UPDATE|DELETE FROM)[^"]*"' src-tauri/src \
        | sed 's/^[^:]*://' | sort > /tmp/sql-depois.txt
      diff /tmp/sql-antes.txt /tmp/sql-depois.txt && echo "nenhum SQL mudou"
      ```

      Diferença aqui é violação de `I7`: a fatia moveu código **e** mexeu em
      SQL, e as duas coisas caberiam no mesmo commit sem ninguém perceber
- [ ] `git grep -c 'pub fn .*_core' src-tauri/src` devolve o mesmo total de 48
- [ ] `wc -l src-tauri/src/*/comandos.rs` — nenhum acima de 2000

## Riscos

| Risco | Probabilidade | Impacto | Plano B |
|---|---|---|---|
| **A árvore de trabalho não está no git.** `git status --porcelain` devolve 59 arquivos modificados e o último commit é de 2026-09-28 — 16 dias de trabalho sem ponto de retorno | alta | **irrecuperável** | **Pré-condição, não plano B.** F0 só começa depois de a árvore estar commitada. Um refactor que move 6537 linhas sem rede não tem como reverter |
| `cargo fmt` reformatar o repositório inteiro no meio da onda, misturando 2 espaços com 4 e tornando o diff ilegível | média | diff inútil para revisão | `src-tauri/rustfmt.toml` já fixa 2 espaços; rodar `cargo fmt` **antes** de F1, em commit próprio, e confirmar que ele produz diff vazio |
| `verify:rust:shell` (`cargo check --all-targets`, com a feature padrão) é o único passo que pega a quebra de `tauri build` | média | empacotamento quebrado | ele já está em `npm run verify`. F0 tem de rodar o gate **completo**, não `cargo test` |
| A regra 8 reescrita acquire falso-positivo e o gate for desligado | baixa | fronteira sem verificação | a verificação negativa (`I6`) é a defesa: ela prova que o gate acusa, e roda em `npm run verify` |
| O teto de 2000 linhas vira o novo alvo (código escrito para caber) | baixa | código partido em pedaços arbitrários | o teto é de `<módulo>/comandos.rs`, e a divisão de segundo nível é por sub-recurso do domínio, não por contagem |

## Bloqueios

| Item | Bloqueio | Quem resolve |
|---|---|---|
| Os 59 arquivos modificados sem commit | C7, aberto desde 2026-10-10; esta spec não fecha C7 | a dona do projeto, commitando antes de F0 |
| `D95` é citado em `tcc/mod.rs:12,16,17,24`, `tcc/docx.rs:1` e `tcc/pdf.rs:1` e **não está declarado** em nenhum bloco de decisão | o número está tomado na prática; esta spec começa em `D96` por causa disso | a spec que implementou `D95` (`SPEC-D-011`) declara-o |
| `src-tauri/src/sync/` (tradução de formatos, ADR-009) | quando chegar, `sync` é um diretório de módulo sem dono de tabela — a regra 8 precisa saber que ele é de integração, e não de módulo | junto com a spec de sync |

## Referências

| Documento | O que resolve |
|---|---|
| [`SPEC-D-015`](../../../docs/specs/desktop/SPEC-D-015-modulos-do-backend-rust.md) | a cópia de grupo desta spec |
| [`01-fronteiras.md`](./01-fronteiras.md) | `D65..D67` — quem é dono de qual dado; esta spec cumpre a primeira metade de `D67` |
| [`faculdade.md`](./faculdade.md) | `D74..D75` — o motor que vira `faculdade/` |
| [`estudos.md`](./estudos.md) | `D76` — o motor que já é `estudos/` |
| [`tcc-modelo-do-documento.md`](./tcc-modelo-do-documento.md) | `D85..D88` — o motor que já é `tcc/` |
| [`SPEC-D-005`](../../../docs/specs/desktop/SPEC-D-005-executar-o-backend-rust.md) | `D83..D84` — por que o gate roda `cargo test --lib --no-default-features` |