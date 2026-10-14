# TCC & Projetos — modelo de dados do documento

> Fonte de verdade no app para a
> [`SPEC-D-013`](../../../docs/specs/desktop/SPEC-D-013-modelo-de-dados-do-documento.md).
> A cópia de grupo vive em
> `docs/specs/desktop/SPEC-D-013-modelo-de-dados-do-documento.md`.
> Decisões do app: `D85`, `D86`, `D87`, `D88`.
> Origem: spec referencial §1.3 (**[D]**, cada dado tem um dono), §1.4 (**[D]**, o
> cálculo e o dado são Rust), §3.1 (TCC & Projetos é dono de Projeto, Base de
> pesquisa, Documento, Versões e Saídas), §4.6 (**[D]**, viva / snapshot / cópia
> editorial), §4.7 (**[D]**: editor paginado com modelo interno, sete tipos de
> projeto, até cinco ativos, round-trip DOCX, Ceci que não escreve).
> Estado: **modelo escrito (2026-10-10)**. Este documento **resolve** a
> dependência não escrita de [`tcc.md`](./tcc.md) (§Bloqueios) e da
> [`SPEC-D-011`](../../../docs/specs/desktop/SPEC-D-011-editor-do-tcc-e-round-trip-docx.md) §4/§12.
> Ele fixa `D85..D88` e o schema; o motor de DOCX real e o render de PDF entram
> em fase própria, com a dependência declarada.

Este documento é a decisão de **modelo de dados** do documento do TCC, ao lado de
`tcc.md` (que fixa `D77..D79`, o produto e o contrato do round-trip). Ele **não**
renumera `D77..D79` nem `D74..D76` nem `D62..D67`. O que `tcc.md` marcou como
"dependência não escrita" mora aqui.

## Contexto

A [`SPEC-D-011`](../../../docs/specs/desktop/SPEC-D-011-editor-do-tcc-e-round-trip-docx.md)
§12 e o [`tcc.md`](./tcc.md) §Bloqueios foram explícitos: o modelo de dados do
documento **não estava escrito em lugar nenhum**, e escrevê-lo por invenção dentro
da decisão de produto seria a dívida L-6 de [`docs/analise/desktop.md`](../../../docs/analise/desktop.md)
("documento que o projeto cita e não tem"). Este documento paga essa dívida de
forma declarada: aqui o modelo é **decidido**, com alternativa rejeitada e
consequência, e não improvisado.

Três forças moldam a decisão:

1. **A árvore precisa ser editável.** `D77` fixa que a página é derivada e que
   nenhuma posição de página é gravada. Isso empurra o problema para o modelo:
   o que se persiste é a **árvore de nós**, na ordem que a usuária vê, e não o
   layout.
2. **A citação e o fragmento são de outro dono.** `SPEC-D-007` D66 e `ADR-010`
   dizem que Referência é da Biblioteca e Fragmento é do Conhecimento. O
   documento guarda **o vínculo** (o id), nunca a cópia — mas as tabelas de lá
   ainda não existem, e por isso o vínculo é **referência viva sem FK**, com a FK
   chegando junto de cada módulo dono.
3. **Toda escrita é reversível.** `D79` e §1.7 **[D]** exigem que aceitar uma
   sugestão seja escrita comum e desfezível. O modelo precisa de **versão** desde
   o primeiro dia, não como recurso posterior.

## D85 · A árvore é lista de adjacência ordenada, o inline é tabela própria, e a geometria não é coluna

- **Decisão.** O Documento é persistido como **lista de adjacência ordenada**:
  cada nó guarda `pai_id` (nulo na raiz) e `ordem` entre irmãos, e o tipo do nó é
  uma coluna com `CHECK` que amarra as colunas específicas de cada tipo. O
  conteúdo inline do parágrafo mora em **tabela própria** (`inline_run`), uma
  linha por trecho, ordenada. A **geometria** (A4, margens, corpo, entrelinha) é
  **parâmetro** do motor de paginação, jamais coluna — mudar a geometria não
  toca em nenhuma linha da árvore.
- **Justificativa.** A lista de adjacência é o modelo que casa com o contrato já
  fixado em `SPEC-D-011` §5 (`capitulo.filhos`, `paragrafo.inline` recursivos):
  mover um capítulo é trocar `pai_id`/`ordem`, não reescrever um blob. O inline
  em tabela — e não em JSON dentro do parágrafo — existe porque o formato rico
  precisa de consulta por trecho (contar citação sem citação, marcar trecho
  sugerido) e porque um blob de JSON no schema é o mesmo tipo de "modelo
  implícito" que `SPEC-D-011` §12 recusou escrever. A geometria como parâmetro é
  a repetição de `D77`: é o mesmo padrão do atraso derivado e da `grid-scale.ts`,
  e é o que torna §4.7 "modelo da instituição **[P]**" um parâmetro novo, não um
  segundo motor.
- **Alternativa rejeitada.** (a) Documento inteiro em **um JSON** numa coluna
  (`documento.conteudo`). Rejeitada porque torna toda consulta por trecho uma
  varredura no cliente e devolve o modelo para o TypeScript — exatamente o que
  `SPEC-D-011` §12 proíbe. (b) Uma tabela **por tipo de nó** (`capitulo`,
  `paragrafo`, `citacao`, `referencia_viva`). Rejeitada porque a ordem entre
  irmãos de tipos diferentes (um parágrafo, depois uma citação, depois outro
  parágrafo) exige uma chave de ordenação comum, e uma tabela por tipo não a tem
  sem uma tabela de junção que reintroduz a lista de adjacência. (c) **Nested
  sets** (intervalos de `esq`/`dir`). Rejeitada porque inserir um nó no meio
  reescreve a chave de milhares de linhas, e o documento é editado o tempo todo.
  (d) Geometria em coluna por documento. Rejeitada porque muda com o modelo da
  instituição e, gravada, gera o backfill que `D77` proíbe.
- **Consequência aceita.** A montagem da árvore é uma leitura ordenada e uma
  reconstrução em memória no Rust; a edição é barata (reparentar e renumerar
  irmãos) e o custo é que a **ordem é dado** — uma reordenação é escrita, com
  versão. E a citação é **referência viva sem FK** enquanto Biblioteca e
  Conhecimento não existem: o vínculo órfão é possível, e é **sinalizado** na
  leitura (o item aparece como referência não resolvida), nunca apagado em
  silêncio.

### Schema da árvore (dentro de D85)

```sql
-- projeto: dono de Projeto (SPEC-D-007 D65). Sete tipos [D] (§4.7).
CREATE TABLE projeto (
  id         TEXT PRIMARY KEY,
  titulo     TEXT NOT NULL,
  tipo       TEXT NOT NULL CHECK (tipo IN
             ('tcc','artigo','iniciacao-cientifica','relatorio-de-estagio',
              'projeto-de-pesquisa','revisao-bibliografica','estudo-de-caso')),
  estado     TEXT NOT NULL CHECK (estado IN ('ativo','encerrado','arquivado')),
  descricao  TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

-- documento: um por projeto na v1 (UNIQUE). O conteúdo é a árvore, não o texto.
CREATE TABLE documento (
  id         TEXT PRIMARY KEY,
  projeto_id TEXT NOT NULL UNIQUE REFERENCES projeto (id) ON DELETE CASCADE,
  titulo     TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

-- no_documento: lista de adjacência ordenada. O CHECK amarra as colunas de cada
-- tipo — nó de tipo errado não entra, e coluna de outro tipo não sobra.
-- pai_id = NULL é a raiz. Referência e fragmento são vínculos sem FK (D66): as
-- tabelas donas ainda não existem.
CREATE TABLE no_documento (
  id            TEXT PRIMARY KEY,
  documento_id  TEXT NOT NULL REFERENCES documento (id) ON DELETE CASCADE,
  pai_id        TEXT REFERENCES no_documento (id) ON DELETE CASCADE,
  ordem         INTEGER NOT NULL CHECK (ordem >= 0),
  tipo          TEXT NOT NULL CHECK (tipo IN
                ('capitulo','paragrafo','citacao','referencia-viva')),
  titulo        TEXT,
  referencia_id TEXT,
  modo          TEXT,
  fragmento_id  TEXT,
  created_at    INTEGER NOT NULL,
  updated_at    INTEGER NOT NULL,
  CHECK (
    (tipo = 'capitulo'        AND titulo IS NOT NULL AND referencia_id IS NULL
                              AND modo IS NULL AND fragmento_id IS NULL) OR
    (tipo = 'paragrafo'       AND titulo IS NULL AND referencia_id IS NULL
                              AND modo IS NULL AND fragmento_id IS NULL) OR
    (tipo = 'citacao'         AND titulo IS NULL AND referencia_id IS NOT NULL
                              AND modo IN ('vivo','snapshot','copia-editorial')
                              AND fragmento_id IS NULL) OR
    (tipo = 'referencia-viva' AND titulo IS NULL AND referencia_id IS NULL
                              AND modo IS NULL AND fragmento_id IS NOT NULL)
  )
);

-- Ordem única entre irmãos: COALESCE(pai_id, '') porque NULL é distinto de NULL
-- no UNIQUE do SQLite, e sem o COALESCE a raiz aceitaria duas ordens iguais.
CREATE UNIQUE INDEX idx_no_ordem
  ON no_documento (documento_id, COALESCE(pai_id, ''), ordem);

-- inline_run: um trecho de texto com marcas. `marcas` é string canônica — lista
-- fechada, ordenada, sem repetição — para que a serialização seja determinística
-- e a comparação de versões não dependa de ordem de digitação.
CREATE TABLE inline_run (
  id     TEXT PRIMARY KEY,
  no_id  TEXT NOT NULL REFERENCES no_documento (id) ON DELETE CASCADE,
  ordem  INTEGER NOT NULL CHECK (ordem >= 0),
  texto  TEXT NOT NULL,
  marcas TEXT NOT NULL DEFAULT ''
);
CREATE UNIQUE INDEX idx_inline_ordem ON inline_run (no_id, ordem);
```

## D86 · A paginação é uma função pura com medidor de texto injetado, e a geometria é parâmetro

- **Decisão.** O motor de paginação (`src-tauri/src/tcc/paginacao.rs`) é **puro**:
  recebe a árvore já montada e uma `Geometria`, e devolve as páginas. A medida de
  texto é obtida por uma **trait `MedidorDeTexto` injetada** — `largura(texto,
  tamanho_pt, marcas)` e `altura_linha(tamanho_pt)`. O golden usa um medidor
  **determinístico** (largura proporcional ao número de caracteres), e a produção
  usa o medidor real de fontes, trocado **sem mudar o motor**. Nenhuma altura de
  bloco, número de página ou posição de cursor é gravada.
- **Justificativa.** `D77` exige "editor visual paginado próximo ao Word" **sem**
  medir DOM no webview (a alternativa (a) rejeitada de `D77`: fonte instalada
  muda o resultado). A única forma de ter paginação **determinística e testável**
  sem depender da máquina é isolar a medida atrás de uma trait: o motor vira
  aritmética de layout com relógio nenhum, o golden roda em qualquer lugar, e a
  fidelidade ao Word vira uma **propriedade do medidor**, medida, não presumida.
  É a mesma separação que o projeto já usa entre domínio puro e plataforma.
- **Alternativa rejeitada.** (a) Medir fontes dentro do motor, com o crate de
  métrica fixo. Rejeitada porque amarra o teste ao arquivo de fonte instalado — o
  mesmo defeito da alternativa (a) de `D77`, só que no Rust. (b) Aproximar a
  altura por contagem de caracteres **como produto** (não só como golden).
  Rejeitada porque a ABNT mede margem e entrelinha em milímetros, e uma
  aproximação grosseira erra a quebra real do Word que o round-trip confronta.
  (c) Calcular a paginação no TypeScript para reusar o layout do editor.
  Rejeitada por §1.4 **[D]**: cálculo é Rust, e cálculo de layout no componente é
  regra de domínio na interface.
- **Consequência.** O motor é mais um consumidor da disciplina de `D77`, e o
  medidor real é uma **dependência declarada** (crate de métrica de fonte), com
  licença registrada — o mesmo critério duro da biblioteca de DOCX. O contrato é
  a **saída** (`paginas`, `alturaPorNo`), e o teste de geometria (`D77` I2) muda o
  parâmetro e prova que o conteúdo não muda. Viúvas, órfãs e título de capítulo
  não separado do primeiro parágrafo entram no motor com regra declarada, não
  como ajuste visual.

## D87 · Toda escrita cria uma Versão por snapshot completo, e a coalescência é do autosave

- **Decisão.** Cada **escrita confirmada** no documento grava uma linha em
  `versao` com o **snapshot completo** da árvore serializado de forma
  canônica, mais `autor`, `motivo` e `criado_em`. Não existe diff na v1. O
  **autosave** coalesce digitação em uma escrita (não grava por tecla), e é a
  escrita que gera a versão. O `numero` é sequencial por documento, e
  `UNIQUE(documento_id, numero)`.
- **Justificativa.** `D79` e §1.7 **[D]** exigem que aceitar a sugestão da Ceci
  seja escrita **desfezível**, e `SPEC-D-011` I15 exige que `Versões` registre
  toda escrita. Snapshot completo é o modelo que torna **qualquer** versão
  restaurável sem reconstruir uma cadeia de diffs — e a cadeia de diffs é
  precisamente o que quebra quando um passo do meio se perde. O custo é espaço,
  e espaço é mais barato que uma versão irrecuperável.
- **Alternativa rejeitada.** (a) Diff binário incremental por versão. Rejeitada
  na v1 porque exige reconstrução correta de toda a cadeia para ler uma versão
  antiga; um passo perdido inutiliza o histórico. (b) Gravar por tecla. Rejeitada
  porque produz milhares de versões inúteis e o histórico deixa de ser legível.
  (c) Sem versão; desfazer só na memória da sessão. Rejeitada porque §4.7 e
  §1.7 **[D]** pedem desfazer **depois** de fechar o app, e memória de sessão não
  sobrevive.
- **Consequência.** O banco cresce com o tamanho do documento vezes o número de
  escritas, e a política de retenção (compactar versões antigas, marcar
  "snapshot nomeado") é decisão de produto futura, declarada e fora desta fase.
  A serialização canônica é obrigatória para que "sem mudança" seja detectável:
  escrita que não altera a árvore canônica **não** cria versão.

## D88 · O item a resolver é tabela própria, com estado de encaixe, e a importação nunca sobrescreve

- **Decisão.** O resultado da importação de DOCX é persistido em
  `item_a_resolver`: `tipo` (`comentario` ou `sugestao`), `autor`,
  `autor_resolvido`, data do autor, `citado` (o texto-âncora), `substituto` (o
  texto proposto, em sugestão), `texto`, `origem_arquivo`, `importado_em` e
  `estado`. O `estado` é a coluna que carrega o contrato de tolerância de `D78`:
  `pendente`, `resolvido`, `nao-encaixou` (o texto citado não existe mais) ou
  `duplicado`. A importação **anexa** linhas; ela nunca toca em `no_documento`.
- **Justificativa.** `D78` fixa que a importação é um **merge** e que o que "não
  encaixou" precisa ser **reportado**. Um estado explícito por item é a forma de
  tornar o relatório de importação uma **consulta**, não uma mensagem efêmera: a
  usuária relê depois o que não ancorou. A âncora é o **texto citado** (`citado`),
  não uma posição — e a tabela guarda exatamente isso.
- **Alternativa rejeitada.** (a) Itens a resolver na **memória** da sessão, só
  exibidos no fim da importação. Rejeitada porque esconde o que não ancorou assim
  que a tela fecha, e `D78` exige a perda **declarada e persistente**. (b) Guardar
  o item como **comentário do DOCX** reexportado. Rejeitada porque acopla o estado
  do app ao arquivo de terceiro. (c) Âncora por **deslocamento** de caractere.
  Rejeitada por `D78` alternativa (b): não sobrevive à edição no Word.
- **Consequência.** Há dois lugares que descrevem pendência do documento (a árvore
  e os itens a resolver), e a interface precisa mostrar **os dois**: um comentário
  do orientador não é um nó do documento, é uma pendência sobre um trecho.
  Resolver o item (`estado='resolvido'`) é uma escrita versionada; o item
  permanece, com o estado, e não é apagado.

### Schema das versões, dos itens e dos vínculos (dentro de D87 e D88)

```sql
-- versao: snapshot completo e canônico por escrita confirmada (D87).
CREATE TABLE versao (
  id           TEXT PRIMARY KEY,
  documento_id TEXT NOT NULL REFERENCES documento (id) ON DELETE CASCADE,
  numero       INTEGER NOT NULL CHECK (numero > 0),
  snapshot     TEXT NOT NULL,
  autor        TEXT NOT NULL,
  motivo       TEXT,
  criado_em    INTEGER NOT NULL,
  UNIQUE (documento_id, numero)
);

-- item_a_resolver: saída da importação de DOCX (D88). Estado de encaixe é dado,
-- não exibição. `autor_resolvido = 0` é o autor desconhecido (D78).
CREATE TABLE item_a_resolver (
  id               TEXT PRIMARY KEY,
  documento_id     TEXT NOT NULL REFERENCES documento (id) ON DELETE CASCADE,
  tipo             TEXT NOT NULL CHECK (tipo IN ('comentario','sugestao')),
  autor            TEXT NOT NULL,
  autor_resolvido  INTEGER NOT NULL DEFAULT 1 CHECK (autor_resolvido IN (0,1)),
  data_autor_ms    INTEGER,
  citado           TEXT NOT NULL,
  substituto       TEXT,
  texto            TEXT,
  origem_arquivo   TEXT,
  importado_em     INTEGER NOT NULL,
  estado           TEXT NOT NULL CHECK (estado IN
                   ('pendente','resolvido','nao-encaixou','duplicado')),
  resolvido_em     INTEGER,
  created_at       INTEGER NOT NULL
);

-- base_de_pesquisa: vínculo do projeto ao acervo (Biblioteca) e ao Conhecimento.
-- `alvo_id` é referência viva sem FK (D66): donos ainda não existem.
CREATE TABLE base_de_pesquisa (
  id         TEXT PRIMARY KEY,
  projeto_id TEXT NOT NULL REFERENCES projeto (id) ON DELETE CASCADE,
  alvo_tipo  TEXT NOT NULL CHECK (alvo_tipo IN ('referencia','fragmento')),
  alvo_id    TEXT NOT NULL,
  criado_em  INTEGER NOT NULL
);

-- saida: registro das exportações (Saídas de §3.1). O conteúdo binário não é
-- dado do projeto; o registro é.
CREATE TABLE saida (
  id         TEXT PRIMARY KEY,
  projeto_id TEXT NOT NULL REFERENCES projeto (id) ON DELETE CASCADE,
  formato    TEXT NOT NULL CHECK (formato IN ('docx','pdf')),
  destino    TEXT NOT NULL,
  criado_em  INTEGER NOT NULL
);
```

## Invariantes

| # | Invariante | Onde é garantida | Teste |
|---|---|---|---|
| I1 | Nenhuma coluna de página, altura de bloco ou posição de cursor existe | `migrations/0005_tcc.sql` | `rg -n "pagina\|altura_bloco\|posicao_cursor" src-tauri/migrations/` → vazio |
| I2 | A ordem entre irmãos é única, e a raiz não aceita duas ordens iguais | índice `idx_no_ordem` | teste Rust: inserir dois nós com mesma `(pai, ordem)` falha |
| I3 | O tipo do nó amarra as colunas: nó de tipo errado não entra | `CHECK` de `no_documento` | teste Rust: `citacao` sem `referencia_id` falha |
| I4 | O inline é tabela própria, ordenado e determinístico | `inline_run` + `idx_inline_ordem` | teste Rust: duas linhas com mesma `(no_id, ordem)` falha |
| I5 | A geometria é parâmetro; mudá-la não altera o conteúdo | `tcc/paginacao.rs` | teste: paginar com duas geometrias e comparar a árvore |
| I6 | A paginação é determinística para a mesma entrada e o mesmo medidor | `tcc/paginacao.rs` | teste: duas passadas, mesma saída |
| I7 | Viúvas e órfãs não deixam uma linha sozinha no fim ou no início da página | `tcc/paginacao.rs` | teste: parágrafo de N linhas em página justa |
| I8 | Escrita que não muda a árvore canônica não cria versão | `commands.rs` (escrita) | teste Rust: salvar duas vezes sem editar gera uma versão |
| I9 | Toda escrita confirmada gera uma versão com `numero` sequencial | `versao` + escrita | teste: duas escritas → números 1 e 2 |
| I9b | A importação nunca altera `no_documento` | caminho de importação | teste: árvore antes e depois idêntica |
| I10 | O item a resolver guarda o estado de encaixe, e nada é apagado | `item_a_resolver` | teste: item `nao-encaixou` persiste e é relido |
| I11 | A citação e o vínculo são referência viva, sem cópia | `referencia_id`/`fragmento_id` sem FK | `rg -n "CREATE TABLE.*(referencia\|fragmento)" src-tauri/migrations/` → vazio |

## Contrato

A forma do contrato é a de `SPEC-D-011` §5, agora **com tipo concreto para o
inline** (que estava referenciado e não definido) e com os tipos que o motor
consome e produz.

- **Nó do documento** (o que a interface manipula e o que o Rust pagina):

  ```ts
  export type ModoVinculo = "vivo" | "snapshot" | "copia-editorial";
  export type MarcaInline = "negrito" | "italico" | "sublinhado" | "sobrescrito" | "subscrito";

  export interface Inline { readonly texto: string; readonly marcas: readonly MarcaInline[]; }

  export type NoDocumento =
    | { readonly tipo: "capitulo"; readonly id: string; readonly titulo: string;
        readonly filhos: readonly NoDocumento[] }
    | { readonly tipo: "paragrafo"; readonly id: string; readonly inline: readonly Inline[] }
    | { readonly tipo: "citacao"; readonly id: string; readonly referenciaId: string;
        readonly modo: ModoVinculo }
    | { readonly tipo: "referencia-viva"; readonly id: string; readonly fragmentoId: string };
  ```

- **Entrada e saída do motor** (derivada, **nunca** gravada):

  ```rust
  pub struct Geometria { /* mm: largura, altura, margens; pt: corpo, entrelinha */ }
  pub trait MedidorDeTexto { /* largura(texto, pt, marcas) -> mm; altura_linha(pt) -> mm */ }
  pub fn paginar(nos: &[NoDocumento], geom: &Geometria, medidor: &dyn MedidorDeTexto)
      -> Paginacao; // { paginas: u32, altura_por_no: BTreeMap<String, f64> }
  ```

- **Item a resolver** (contrato de `SPEC-D-011` §5, com o estado de encaixe):

  ```jsonc
  {
    "tipo": "comentario" | "sugestao",
    "autor": "…", "autorResolvido": true, "dataAutorMs": 1759612800000,
    "citado": "trecho exato do documento",
    "substituto": "…",                         // só em "sugestao"
    "texto": "…",
    "origem": { "arquivo": "…", "importadoEm": 1759708800000 },
    "estado": "pendente" | "resolvido" | "nao-encaixou" | "duplicado"
  }
  ```

- **Dono do dado.** Projeto, Documento, Versão, Item a resolver, Base de pesquisa
  e Saída são de **TCC & Projetos**. Referência é da **Biblioteca**; Fragmento é
  do **Conhecimento** (`01-fronteiras.md:43-45`). O documento guarda o **id** do
  vínculo, nunca a cópia.

## Casos de borda

| Entrada | Comportamento |
|---|---|
| Capítulo com subcapítulos | a lista de adjacência endereça por `pai_id`; a profundidade é livre, e a numeração ABNT é **gerada** do modelo |
| Mover um nó | é reescrita de `pai_id`/`ordem` entre irmãos, com versão; nenhuma página é tocada |
| Excluir um capítulo | `ON DELETE CASCADE` remove a subárvore; a versão anterior permanece e restaura |
| Citação para referência que ainda não existe | é referência viva sem FK (D66): a leitura sinaliza "referência não resolvida", nunca apaga o nó |
| Parágrafo com trecho em negrito no meio | vira mais de uma linha em `inline_run`, com marcas canônicas ordenadas |
| Escrever sem mudar nada | não cria versão: a serialização canônica detecta a ausência de diferença (D87) |
| Autosave durante digitação | coalesce em uma escrita confirmada, e é ela que versiona; não versiona por tecla |
| Importar DOCX com comentário repetido | o item entra com `estado='duplicado'`, sinalizado, não somado em silêncio (D78) |
| Importar DOCX com trecho citado reescrito | `estado='nao-encaixou'`, visível e persistido (D88) |
| Geometria da instituição diferente | é um `Geometria` novo passado ao motor; nenhuma linha da árvore muda (D86) |
| Fonte do medidor ausente na produção | o medidor real é declarado ausente e há fallback determinístico **dito**; a paginação nunca é silenciosamente aproximada |

## Medição

Números são **gerados**, não copiados.

| Métrica | Comando | Antes (2026-10-10) | Gate |
|---|---|---|---|
| Colunas de página ou cursor no schema | `rg -n "pagina\|altura_bloco\|posicao_cursor" src-tauri/migrations/ \| Measure-Object -Line` | 0 | o mesmo `rg` (I1) |
| Tabelas do TCC | `rg -c "CREATE TABLE" src-tauri/migrations/0005_tcc.sql` | 0 (não existe) | 8 |
| Tabelas próprias de referência/fragmento | `rg -n "CREATE TABLE.*referencia\|CREATE TABLE.*fragmento" src-tauri/migrations/` | 0 | o mesmo `rg` (I11) |
| Casos de teste do motor de paginação | `rg -c "\bit\(" src-tauri/src/tcc/*.rs` | 0 (não existe) | `npm run verify` |

## Bloqueios e o que falta

- **Biblioteca e Conhecimento não existem.** `referencia_id`, `fragmento_id` e
  `alvo_id` são vínculos **sem FK** por isso, e a FK chega junto de cada módulo
  dono. Isto é `D66` (referência viva) aplicado, não uma dívida nova.
- **Motor de DOCX real e render de PDF — dependência declarada.** A cláusula de
  `D78` exige um pacote que exponha `w:ins`/`w:del` com autor e data. A escolha
  concreta e o render de PDF entram em fase própria, com a licença registrada
  (reduz o C8). O que este documento fixa é onde o resultado da importação
  **cai**: `item_a_resolver`, sem sobrescrita.
- **Medidor de texto real — dependência declarada.** O `MedidorDeTexto` de
  produção depende de um crate de métrica de fonte; o golden roda com o medidor
  determinístico. A fidelidade ao Word é propriedade do medidor, medida.
- **Política de retenção de versões** (compactar, snapshot nomeado): decisão de
  produto futura, fora desta fase.
- **`[P]` fora de escopo.** Capa, folha de rosto e sumário NBR 14724; modelo da
  instituição; importação por DOI/ISBN; estilos CSL além de ABNT
  (`SPEC-D-011` §4).
- **`[A]` do grupo.** Nenhuma pendência `[A]` do referencial §8 é do TCC.

## Referências

| Documento | O que resolve |
|---|---|
| [`tcc.md`](./tcc.md) | o produto e o contrato do round-trip (`D77..D79`) |
| [`SPEC-D-013`](../../../docs/specs/desktop/SPEC-D-013-modelo-de-dados-do-documento.md) | a cópia de grupo desta decisão |
| [`01-fronteiras.md`](./01-fronteiras.md) | a matriz de dono; TCC, Biblioteca e Conhecimento |
| [`SPEC-D-007`](../../../docs/specs/desktop/SPEC-D-007-dono-do-dado-e-fronteira-entre-modulos.md) | dono do dado e os modos de vínculo (`D65`, `D66`, `D67`) |
| [`SPEC-D-011`](../../../docs/specs/desktop/SPEC-D-011-editor-do-tcc-e-round-trip-docx.md) §4, §5, §12 | a dependência que este documento resolve |
| [`ADR-010`](../../../docs/decisoes/ADR-010-referencia-e-dono-do-dado-bibliografico.md) | Referência é da Biblioteca; o TCC cita |
| [`docs/analise/desktop.md`](../../../docs/analise/desktop.md) §L-6 | o débito que esta decisão fecha |
