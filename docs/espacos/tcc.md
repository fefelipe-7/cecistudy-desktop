# TCC & Projetos — editor paginado e o round-trip DOCX

> Fonte de verdade no app para a
> [`SPEC-D-011`](../../../docs/specs/desktop/SPEC-D-011-editor-do-tcc-e-round-trip-docx.md).
> A cópia de grupo vive em
> `docs/specs/desktop/SPEC-D-011-editor-do-tcc-e-round-trip-docx.md`.
> Decisões do app: `D77`, `D78`, `D79`.
> Origem: spec referencial §1.3 (**[D]**, cada dado tem um dono), §1.4 (**[D]**,
> o cálculo e o dado são Rust), §1.7 (**[D]**, a Ceci lê e sugere, nunca escreve),
> §3.1 (TCC & Projetos é dono de Projeto, Base de pesquisa, Documento, árvore
> acadêmica, Versões e Saídas), §3.2 (**[D]** leitura no mobile; escrita longa só
> no desktop), §4.5 (Referência é da Biblioteca), §4.6 (Fragmento é do
> Conhecimento e entra por vínculo), §4.7 (**[D]**: editor paginado, sete tipos de
> projeto, até cinco ativos, orientação fora do app, Ceci que não escreve),
> §6.4 (o relatório de estágio pode ser um Projeto).
> Estado: **contrato fechado (2026-10-10)** — nenhum código existe ainda. Este
> documento fixa as decisões de módulo (`D77..D79`) e o **contrato de tolerância
> do round-trip**; o **modelo de dados do documento** está em
> [`tcc-modelo-do-documento.md`](./tcc-modelo-do-documento.md) (`D85..D88`, cópia
> de grupo `SPEC-D-013`). O que deixou de existir foi a dependência não escrita do
> modelo (§Bloqueios, e `SPEC-D-011` §12).

Este documento é a decisão de **módulo** de TCC & Projetos, ao lado de
`00-shell-e-navegacao.md`, `01-fronteiras.md`, `faculdade.md` e `estudos.md`. Ele
não renumera `D1..D46` de `docs/calendario/` nem `D62..D67`/`D74..D76`. Onde o
índice de grupo errar sobre o app, o app está certo.

## Contexto

O TCC & Projetos responde *"o que eu preciso escrever, e em que pé está?"*
(§4.7). É o módulo da **escrita longa**, e é o único do desktop que produz um
**arquivo** que sai do app — o DOCX que vai para o orientador e volta.

Três fatos governam o desenho:

1. **A página é derivada, nunca gravada** (§4.7 **[D]**, "modelo interno de
   documento"). A página é resultado do cálculo de paginação sobre o modelo, como
   o atraso e a média são derivados — é a terceira vez que o projeto aplica a
   mesma disciplina.
2. **A orientação não entra no app** (§4.7 **[D]**). O ciclo é: exporta DOCX → o
   orientador comenta no Word → importa o DOCX devolvido → comentários e
   sugestões viram **itens a resolver**. O round-trip é o mecanismo, e ele **não
   é simétrico**: exportar é escrever um formato que conhecemos; importar é ler um
   arquivo que um terceiro reorganizou e converteu.
3. **Nada é duplicado** (§1.3, §3.1). A **Referência** é da Biblioteca; o
   **Fragmento** é do Conhecimento; o TCC **mostra por referência**, e a lista de
   referências do projeto é **gerada** dos vínculos — o documento não tem tabela
   de referências própria (`01-fronteiras.md:43-45`, D65).

O módulo tem **sete tipos de projeto, fechados** (§4.7 **[D]**): TCC · artigo ·
iniciação científica · relatório de estágio · projeto de pesquisa · revisão
bibliográfica · estudo de caso. E **até cinco projetos ativos** simultâneos.

As subtelas de um Projeto são as de §4.7 (Visão geral, Estrutura, Escrita,
Pesquisa, Referências, Orientação, Versões, Exportações); o módulo Módulo é a
lista de projetos. O que a `SPEC-D-011` chama de "modelo na spec do TCC" é o que
este documento **declara em aberto** — ver D77.

## D77 · O modelo interno de documento é do projeto, e a página é derivada

- **Decisão.** O **Documento** é uma entidade do módulo TCC & Projetos
  (`01-fronteiras.md:45`), com **modelo interno próprio** — árvore de nós, com nó
  de capítulo, de parágrafo, de citação e de referência viva — e a **página é
  derivada** do modelo por um motor de paginação no **Rust**: dada a geometria
  (A4, margens, corpo de texto) e o conteúdo, o Rust devolve as páginas. Nenhuma
  coluna guarda número de página, altura de bloco ou posição de cursor. A
  geometria é **parâmetro** do motor, não constante do componente; mudar a
  geometria reprocessa **todas** as páginas sem tocar no conteúdo.
- **Justificativa.** §4.7 **[D]** pede *"editor visual paginado próximo ao Word,
  com modelo interno de documento"*, e §1.4 **[D]** põe o cálculo no Rust. A parte
  fácil de ler e difícil de fazer é o **modelo interno**: um editor de texto rico
  que só guarda o texto não é paginável, e um editor que grava posições de página
  deixa de ser editável — o primeiro caractere digitado invalida tudo. O projeto
  já resolveu isso três vezes, com a mesma técnica: o schema **não tem** coluna de
  atraso (`commands.rs:22-23`), o `calendar_event` **não** tem `responsibility_id`
  (`commands.rs:19-20`) e a geometria da grade do Calendário é centralizada em um
  módulo puro com teste (`domain/grid-scale.ts`). A geometria **parâmetro** é a
  mesma disciplina, e ela é o que torna §4.7 "modelo da instituição **[P]**"
  possível sem um segundo motor.
- **Alternativa rejeitada.** (a) `contenteditable` com HTML salvo e paginação
  medida no DOM. Rejeitada porque medir DOM no webview dá paginação dependente da
  **fonte instalada da máquina** — o mesmo documento pagina diferente em dois
  computadores, e a ABNT mede margem em milímetros, não em pixels. Além disso,
  cálculo de layout no componente é regra de domínio na interface. (b) Gravar as
  páginas. Rejeitada pelo motivo de todo derivado gravado: o primeiro caractere
  digitado gera backfill. (c) Editor de terceiros (TipTap, ProseMirror, Lexical)
  para o modelo, com paginação nossa. Rejeitada **por enquanto**, com o critério
  declarado: o modelo interno é a parte que precisa ser nossa para a citação viva
  e a árvore acadêmica serem de primeira classe; um editor de terceiros resolve a
  digitação, que é a parte que menos dói. Se amarrar a decisão custar mais que
  implementar, a troca é registrada com o **custo medido**, não por preferência.
- **Consequência aceita.** O modelo interno é código novo e considerável — o maior
  item do produto depois do editor. Em troca, o documento é **reprocessável**
  (mudar margens, fonte ou modelo da instituição reprocessa tudo) e DOCX e PDF têm
  uma **fonte única** (o modelo), o que evita a classe de bug em que os dois
  divergem por virem de caminhos diferentes. E o custo mais chato: editar é lento
  onde o texto é rico, e isso precisa ser **medido**, não sentido.

### Modelo de dados do documento (escrito em 2026-10-10, dentro de D77)

- **Onde mora.** O modelo de dados do documento foi decidido em
  [`tcc-modelo-do-documento.md`](./tcc-modelo-do-documento.md) (cópia de grupo:
  [`SPEC-D-013`](../../../docs/specs/desktop/SPEC-D-013-modelo-de-dados-do-documento.md)),
  com quatro decisões numeradas: **`D85`** (a árvore é lista de adjacência
  ordenada, o inline é tabela própria e a geometria não é coluna), **`D86`** (a
  paginação é pura, com medidor de texto injetado), **`D87`** (toda escrita cria
  uma Versão por snapshot completo) e **`D88`** (o item a resolver é tabela
  própria, com estado de encaixe). Este documento **não** os reenumera; aponta
  para eles. A **forma** do contrato (§5 da `SPEC-D-011`) e a regra de que a
  página é derivada e a geometria é parâmetro continuam como estavam.
- **O que continua aberto, e por quê.** O **motor de DOCX real** e o **medidor de
  texto real** são **dependências declaradas** (`D88` e `D86`), com o critério
  duro já fixado: a biblioteca de DOCX precisa expor `w:ins`/`w:del` com autor e
  data (`D78`), e a medida de fonte é uma trait injetada (`D86`). Escolher a
  biblioteca é fase própria, com licença registrada — decisão de domínio não se
  delega à dependência (`D78`, alternativa (c)).
- **Por quê.** A `SPEC-D-011` §12 foi explícita ao recusar *"escrever o modelo
  aqui"*: o risco era inventar um schema sem revisão (débito L-6 de
  `docs/analise/desktop.md`). A correção escolhida foi escrevê-lo **com decisão,
  alternativa rejeitada e consequência**, em documento próprio — que é o que a
  `D85` faz, e o que fecha o débito sem trair o motivo de ele existir.
- **Consequência.** A implementação de D4.2 (schema + `src-tauri/src/tcc/`)
  deixou de estar bloqueada pelo modelo e passou a depender só das duas
  dependências declaradas acima. O contrato de tolerância (`D78`) não mudou: é
  contrato de **produto**.

## D78 · O round-trip DOCX é um contrato de tolerância, com âncora e sem sobrescrita

- **Decisão.** Exportar e importar são operações **separadas e nomeadas**, e a
  importação é um **merge**, nunca uma substituição. Cada trecho importado entra
  como **item a resolver** com: autor, data, tipo (`comentario` ou `sugestao`),
  trecho citado e **âncora** — que é o **texto citado**, não um deslocamento de
  caractere. O conteúdo do documento **nunca** é sobrescrito: a importação
  **anexa** comentários e sugestões como itens, e a usuária resolve. O que a
  importação **reporta** é declarado: quantos itens entraram, quantos **não
  ancoraram** (o trecho citado não existe mais), quantos eram duplicata de uma
  importação anterior, e o que não foi reconhecido. Esse é o **contrato de
  tolerância**, listado abaixo.
- **Justificativa.** §4.7 **[D]** descreve um ciclo de quatro passos, e o passo 3
  é *"Ela importa o DOCX devolvido; comentários e sugestões aparecem como itens a
  resolver dentro do projeto"*. A palavra que carrega o peso é **itens**: o
  resultado da importação não é um documento novo, é uma **fila de pendência** —
  por isso ela não pode sobrescrever. E a âncora por **texto citado** em vez de
  posição é a única que sobrevive ao ciclo: o orientador edita, o Word renumera
  tudo, e uma âncora posicional aponta para o parágrafo errado — o que é pior do
  que não ancorar, porque parece funcionar.
- **Alternativa rejeitada.** (a) Importar o DOCX e **substituir** o documento
  local. Rejeitada porque é perda de trabalho sem aviso. (b) Âncora por
  deslocamento de caractere ou índice de parágrafo. Rejeitada porque não sobrevive
  à edição no Word, que é exatamente o passo 2 do ciclo. (c) Round-trip só de
  **texto**, ignorando comentários e sugestões. Rejeitada porque o motivo de o
  ciclo existir é o comentário e a sugestão. (d) **Fidelidade total de layout**
  no round-trip. Rejeitada porque a direção é invertida: o app não precisa
  reproduzir o layout do Word, precisa **ler o que o Word escreveu** — fidelidade
  de layout é problema de exportação (uma vez), e leitura é problema de
  **tolerância**.
- **Consequência aceita.** A importação produz uma lista que pode conter itens que
  "não encaixaram", e a interface precisa mostrar isso **explicitamente**: uma
  âncora perdida é informação, e escondê-la faz a usuária acreditar que todos os
  comentários foram vistos. O contrato de tolerância precisa ser **mantido** a
  cada mudança de formato que a biblioteca de DOCX suportar — é manutenção, e é o
  preço de passar por um editor que não é nosso.

### Cláusula da revisão de alterações (dentro de D78)

- **Decisão.** A **revisão de alterações** habilitada no DOCX (`w:ins` / `w:del`)
  é lida como **sugestão**, tipo distinto de **comentário**: cada inserção e cada
  exclusão entram como **item a resolver** com autor, data e âncora — o **trecho
  excluído** é o texto citado (`w:del`) e o **trecho inserido** é o substituto
  (`w:ins`). A importação **nunca** aplica a sugestão: o conteúdo do documento
  permanece o texto atual, e a usuária resolve item por item (§1.7 **[D]**).
- **Justificativa.** §4.7 **[D]** e §1.1 pedem que o ciclo devolva os itens a
  resolver; a sugestão do Word **é** um trecho com substituição, e tratá-la como
  "um comentário" — ou descartá-la — joga fora metade do que o orientador escreveu.
  Decidir isto **antes** de escolher a biblioteca de manipulação de DOCX é o que
  evita o defeito clássico: descobrir tarde que a biblioteca aplana a revisão e
  perder a informação, ou pior, aplicá-la e reescrever o texto por baixo.
- **Alternativa rejeitada.** (a) Aplicar a revisão no conteúdo e importar só o
  texto final. Rejeitada porque é substituição por outro caminho (D78 (a)), e
  porque apaga o **ato** de sugestão, que é o que a usuária precisa revisar. (b)
  Ignorar `w:ins`/`w:del` e ler só comentários. Rejeitada porque perde a sugestão,
  que é objeto de primeira classe (§4.7). (c) Deixar a decisão para a biblioteca
  escolhida. Rejeitada porque decisão de domínio não se delega a dependência — o
  golden do round-trip tem de cobrir os dois sentidos.
- **Consequência.** A escolha da biblioteca de DOCX passa a ter um critério
  **duro**: ela precisa expor `w:ins`/`w:del` com autor e data. Um pacote que só
  aplane o texto fica recusado. E o caso entra no golden do round-trip e no
  contrato de tolerância abaixo, porque é o que costuma ser descoberto tarde
  (`SPEC-D-011` §10, "O round-trip DOCX ser mais difícil do que §4.7 sugere").

### Contrato de tolerância do round-trip

O que a importação **preserva**, o que ela **reporta** e o que ela **recusa**. É
**lista fechada**, revisada a cada mudança de formato; item novo entra aqui antes
de entrar em código. Cada linha vale como caso do golden do round-trip.

- **Comentário** (`w:comment`) → item `comentario`, com autor, data e âncora por
  texto citado; o conteúdo do documento não muda.
- **Revisão de alterações** (`w:ins`/`w:del`) → item `sugestao`, com autor, data,
  trecho citado (o excluído) e substituto (o inserido); **nunca** aplicada
  automaticamente.
- **Âncora por texto citado** — trecho que existe no texto atual ancora; trecho
  que não existe entra como **não encaixou** e é **reportado com contagem**, nunca
  silenciado.
- **Autor desconhecido** → entra com autor **não resolvido**, visível; o arquivo
  inteiro **não** é recusado por causa de um item.
- **Reimportação do mesmo DOCX** → itens comparados por autor, data e texto
  citado; duplicatas são **sinalizadas**, não adicionadas em silêncio.
- **Formatação inline da citação** (itálico, aspas, negrito) → normalizada pelo
  modelo; como a citação é **gerada** do vínculo, a reimportação da formatação
  **não** sobrescreve a citação do modelo.
- **Numeração de listas e estilos de parágrafo** → o **texto** é preservado; a
  estrutura de numeração não vira estrutura do modelo (o sumário e a numeração de
  capítulos são gerados do modelo).
- **Sumário e campos automáticos do Word** → **não** importados como estrutura; o
  sumário do projeto é gerado conforme NBR 14724 **[P]**.
- **Imagens embutidas** → reportadas como **não suportadas** na v1 (o modelo é
  texto-estrutural); o relatório de importação diz quantas e onde estavam.
- **Tabelas do Word** → a **sequência de texto** é preservada como parágrafos; a
  estrutura de tabela é reportada como perdida.
- **Notas de rodapé e de fim** → reportadas como não importadas (quantas e em que
  trecho), nunca descartadas em silêncio.
- **Metadados do arquivo** (autor do documento, número de revisão, aplicativo que
  gravou) → lidos e **reportados**; não viram dado do projeto.
- **Qualquer constructo não reconhecido** → entra na lista **"não reconhecido"**
  do relatório, com contagem. O contrato é de **tolerância**: perder é aceitável
  desde que a perda seja **declarada**; o que é proibido é perder em silêncio.

## D79 · A orientação fica fora do app, e a Ceci nunca escreve no documento

- **Decisão.** Duas regras de isolamento, ambas **[D]** de §4.7:

  1. **O orientador não tem conta, acesso, papel nem convite.** Não existe tela de
     leitura, colaboração em tempo real ou "aceitar todas" de terceiro. O canal é
     o **arquivo**: exporta DOCX, importa DOCX.
  2. **A Ceci lê, sugere e nunca escreve no documento.** As sugestões são as de
     §4.7 (lacunas, coesão, referências citadas mas ausentes, itens sem citação),
     e **aceitar** uma sugestão é uma **escrita comum** — passa pelo caminho de
     escrita do Rust e por `Versões`, o que a torna reversível (§1.7 **[D]**).
     Não existe comando de escrita no documento fora desse caminho.

  E a consequência de dono (§3.1, §4.5): a **Referência** é da Biblioteca, o
  **Fragmento** é do Conhecimento, e a **lista de referências** do projeto é
  **gerada** dos vínculos da Base de pesquisa — o documento **não** tem tabela de
  referências própria.
- **Justificativa.** §4.7 **[D]** diz que o orientador **fica fora do app**, e a
  razão é o ciclo: ele precisa do **Word**, porque é onde estão as convenções da
  instituição e as revisões de outras pessoas. E §1.7 **[D]** é ainda mais forte
  sobre a Ceci: um diff de IA em documento acadêmico é o recurso que passa a ser
  usado sem ler — o que transforma a revisão em conferência de estilo, que é
  exatamente o que a estudante não precisa.
- **Alternativa rejeitada.** (a) Convite de leitura para o orientador. Rejeitada
  porque quebra a premissa do ciclo (ele precisa do Word). (b) A Ceci escrever com
  diff e revisão. Rejeitada porque §4.7 **[D]** diz que ela **não** escreve, e
  porque revisão de texto acadêmico lida sem ler vicia. (c) A Ceci formatar
  citação e referência automaticamente. Rejeitada porque formatar é escrever; a
  conferência de campos de §4.5 **[P]** — o item que **avisa** — é a forma
  correta. (d) Tabela de referências no documento. Rejeitada por `SPEC-D-007`
  D65: Referência é da Biblioteca, e duplicar é a fonte concorrente que §4.6
  proíbe.
- **Consequência aceita.** Não há colaboração em tempo real, e isso é uma
  **limitação declarada**, não uma falha a corrigir em breve. Aceitar sugestão da
  Ceci é escrita no documento e por isso é **versionada** (`Versões`, §4.7), o que
  é o que permite desfazer. E a lista de referências **gerada** significa que um
  item de biblioteca com metadado incompleto aparece com aviso ou não aparece — a
  cláusula abaixo escolhe.

### Cláusula da referência incompleta (dentro de D79)

- **Decisão.** O módulo escolhe **"aparece com aviso"**: um item de biblioteca com
  metadado incompleto (falta autor, ano, editora ou título exigido pela ABNT)
  **entra** na lista de referências com um **aviso visível** de campo faltante, e
  **nunca** gera citação com lacuna silenciosa.
- **Justificativa.** §4.5 **[P]** pede o aviso, e a alternativa silenciosa é a que
  a usuária não percebe. Esconder o item também é informação perdida: a referência
  existe no projeto e a norma exige os campos.
- **Alternativa rejeitada.** "Não aparece" até o metadado ser completado.
  Rejeitada porque uma referência **citada no texto** e **ausente da lista** é um
  erro que passa na entrega, enquanto o aviso obriga a correção no lugar certo.
- **Consequência.** A ausência de campo é dado de apresentação (o módulo conta e
  marca), não uma coluna nova; a citação é sempre gerada a partir do vínculo, e a
  lacuna é sinalizada na lista e no item (§4.5 **[P]**).

## Invariantes

As quinze de `SPEC-D-011` §6, com o estado no app. Estado medido em 2026-10-13
(fase D6 do [`roadmap.md`](../../roadmap.md)); a coluna "Onde" diz onde cada uma é
garantida e cada linha diz **como medir**.

| # | Invariante | Onde é garantida | Estado |
|---|---|---|---|
| I1 | Nenhuma coluna de página, altura de bloco ou posição de cursor | `src-tauri/migrations/0005_tcc.sql` | **cumprida** — `rg -n "pagina\|altura_bloco\|cursor" src-tauri/migrations/` só acha um comentário e a tabela `sync_cursor`, que é da sincronização, não do documento |
| I2 | A página é derivada, e o modelo não depende da geometria | `src-tauri/src/tcc/paginacao.rs` + `paginar_documento` | **cumprida** — testes `mudar_a_geometria_nao_altera_o_conteudo` e `tcc_paginar_muda_a_geometria_sem_mudar_o_conteudo`; o comando grava nada |
| I3 | DOCX e PDF saem do mesmo modelo interno | exportação (a criar) | **pendente** — teste de paridade da sequência de texto |
| I4 | A importação não altera o conteúdo do documento | `src-tauri/src/tcc/docx.rs` (a criar) | **pendente** — teste de merge: texto idêntico antes e depois |
| I5 | A âncora é o texto citado e sobrevive à edição no Word | `docx.rs` | **pendente** — teste: editar antes do comentário e reimportar |
| I6 | A importação reporta o que não encaixou, com contagem | relatório de importação | **pendente** — teste: trecho inexistente é reportado |
| I7 | Comentário e sugestão são tipos distintos e ambos preservados | `docx.rs` | **pendente** — teste com os dois no mesmo arquivo |
| I8 | Reimportar o mesmo DOCX não duplica item sem sinalizar | `docx.rs` | **pendente** — teste de reimportação |
| I9 | Não existe comando, tela, convite ou papel de orientador | `src/` · `src-tauri/src` | **cumprida** — `rg -n "orientador\|convidar" src/ src-tauri/src/` volta vazio |
| I10 | A Ceci não escreve; sugestão aceita passa pelo caminho de escrita do Rust | painel da Ceci | **pendente** — a Ceci não existe (`SPEC-D-012`) |
| I11 | A lista de referências é gerada; o documento não tem tabela própria | `migrations/0005_tcc.sql` | **cumprida** — `rg -n "CREATE TABLE.*referencia" src-tauri/migrations/` volta vazio; a Base de pesquisa guarda só o id do alvo (`vinculo`) |
| I12 | Referência é da Biblioteca e Fragmento é do Conhecimento | `01-fronteiras.md` | **cumprida** — `vinculo.alvo_tipo` é lista fechada (`referencia`/`fragmento`) e o gate de fronteira classifica a tabela por dono |
| I13 | Inserir citação puxa da Biblioteca; não há citação à mão | `src/features/tcc/ui/EditorDoDocumento.tsx` | **cumprida por ausência** — o editor não tem caminho que crie nó `citacao`; o vínculo que a árvore já traz é exibido como leitura. Teste automático é D6.6 |
| I14 | Item de biblioteca incompleto **avisa**; nunca há lacuna silenciosa | lista de referências | **pendente** — depende da lista de referências (D6.5), que espera a Biblioteca |
| I15 | `Versões` registra toda escrita, inclusive a sugestão aceita | `src-tauri/src/tcc/versao.rs` + `salvar_documento` | **parcial** — toda escrita confirmada passa pelo snapshot (`conteudo_diferente_cria_versao_nova`, `a_mesma_arvore_gera_o_mesmo_snapshot`); a "sugestão aceita" não existe, porque a Ceci não existe |

I1, I4 e I5 são as três que protegem contra as duas classes de bug mais caras:
valor derivado gravado e round-trip que sobrescreve trabalho.

## Contrato

A forma abaixo refina o esboço de §5 da `SPEC-D-011`; ela é a **forma** do
contrato, não o modelo de dados (que está em
[`tcc-modelo-do-documento.md`](./tcc-modelo-do-documento.md), `D85..D88`).

- **Forma do documento** (árvore de nós; a citação aponta para a Biblioteca e a
  referência viva, para o Conhecimento):

  ```ts
  export type NoDocumento =
    | { readonly tipo: "capitulo"; readonly id: string; readonly titulo: string;
        readonly filhos: readonly NoDocumento[] }
    | { readonly tipo: "paragrafo"; readonly id: string; readonly inline: readonly NoInline[] }
    | { readonly tipo: "citacao"; readonly id: string;
        readonly referenciaId: string;        // Referência, dona: Biblioteca
        readonly modo: ModoVinculo; }          // "vivo" | "snapshot" | "copia-editorial"
    | { readonly tipo: "referencia-viva"; readonly id: string;
        readonly fragmentoId: string; };      // Fragmento, dona: Conhecimento
  ```

- **Saída da paginação** (derivada, **nunca** gravada):

  ```jsonc
  { "paginas": 14, "alturasPorNo": { "no-42": 138.5 }, "geometria": "A4-ABNT-2.5" }
  ```

- **Item a resolver**, vindo da importação de DOCX:

  ```jsonc
  {
    "tipo": "comentario" | "sugestao",     // são objetos diferentes no DOCX
    "autor": "…", "data": "2026-10-05T00:00:00Z",
    "citado": "trecho exato do documento",  // âncora é o texto, não a posição
    "texto": "…",                           // em "sugestao": o trecho substituto
    "origem": { "arquivo": "…", "importadoEm": "2026-10-06T00:00:00Z" }
  }
  ```

- **Tipos de projeto** (fechados, sete — §4.7 **[D]**): TCC · artigo · iniciação
  científica · relatório de estágio · projeto de pesquisa · revisão bibliográfica ·
  estudo de caso. Ao chegar a **cinco** ativos, a interface pede qual encerrar e
  **não** permite criar.
- **Dono do dado:** Projeto, Base de pesquisa, Documento, Versão e Saída são de
  TCC & Projetos; Referência é da Biblioteca; Fragmento é do Conhecimento
  (`01-fronteiras.md:43-45`). O mobile **lê** o conteúdo e os comentários (§3.2
  **[D]**) e **não** reimplementa o modelo.

## Casos de borda

| Entrada | Comportamento |
|---|---|
| Primeiro caractere digitado | **não** invalida a paginação: a página é derivada e o motor recomputa. O que não pode é gravar a posição (I1) |
| Mudar fonte ou geometria | reprocessa todas as páginas a partir do modelo, sem tocar no conteúdo; fonte ausente é **dita**, e o fallback é declarado, não silencioso |
| Modelo da instituição, margens diferentes | §4.7 **[P]**: geometria é **parâmetro**; um segundo perfil é um parâmetro novo, **não** um segundo motor |
| Orientador edita antes do comentário | a âncora por texto citado sobrevive se o trecho existe; se foi reescrito, a importação **reporta** que não encaixou (I6) |
| Orientador substitui o trecho | a importação reporta o trecho inexistente; substituição não é comentário, e o app não a adivinha |
| Reimportar o mesmo DOCX | itens comparados por autor, data e texto citado; duplicatas **sinalizadas**, não adicionadas em silêncio (I8) |
| DOCX com autor desconhecido | entra com autor **não resolvido**, visível; o arquivo inteiro não é recusado |
| Mais de cinco projetos ativos | limite de cinco (§4.7 **[D]**): a interface pede qual encerrar e não permite criar |
| Relatório de estágio como Projeto | tipo de §4.7 **[D]**; criado no TCC com vínculo ao Estágio, e a camada clínica **nunca** entra nele (`SPEC-D-008`) |
| Ceci sugere "referência citada mas ausente" | sugestão de **verificação**; a Ceci **não** insere a referência sozinha (D79) |
| Usuária aceita sugestão e quer desfazer | `Versões` desfaz: a sugestão aceita é escrita comum e versionada (I15) |
| Celular abrindo o projeto | §3.2 **[D]**: leitura do conteúdo real e dos comentários; o mobile **não** precisa paginar para ler |
| PDF divergir do DOCX em nº de páginas | I3 falha: os dois saem do mesmo modelo com a mesma geometria; divergir é defeito do motor, não escolha de layout |

## Medição

Números são **gerados**, não copiados. Esta é a etapa "contrato primeiro": o
código ainda não existe, então o valor de partida é zero e o gate é o comando.

| Métrica | Comando | Antes (2026-10-10) | Gate |
|---|---|---|---|
| Itens do contrato de tolerância do round-trip | `rg -c "^- " docs/espacos/tcc.md` na seção do contrato | 13 | §8 da `SPEC-D-011` (≥ 1, incluindo a revisão de alterações) |
| Colunas de página ou cursor no schema | `rg -n "pagina\|altura_bloco\|posicao_cursor" src-tauri/migrations/ \| Measure-Object -Line` | 0 | o mesmo `rg` (I1) |
| Tabelas de referência próprias do TCC | `rg -n "CREATE TABLE.*referencia" src-tauri/migrations/ \| Measure-Object -Line` | 0 | o mesmo `rg` (I11) |
| Comandos de convite ou de orientador | `rg -n "convidar\|orientador" src-tauri/src \| Measure-Object -Line` | 0 | o mesmo `rg` (I9) |
| Tipos de projeto na lista | teste em `src/features/tcc/domain/projeto.ts` (a criar) cruzado com §4.7 | 0 (não existe) | teste = 7 |
| Casos de teste de âncora e reimportação | `rg -c "\bit\(" src/features/tcc/domain/*.test.ts` | 0 (não existe) | `npm run test` (≥ 4) |
| Testes Rust do módulo | `cargo test --lib --no-default-features` (de `src-tauri/`) | 0 (não existe) | `npm run verify` |

## Bloqueios e o que falta

- **Modelo de dados do documento — resolvido (2026-10-10).** O bloqueio central de
  D4.2 foi fechado: o schema, as tabelas, o catálogo de nós e o motor de paginação
  estão decididos em [`tcc-modelo-do-documento.md`](./tcc-modelo-do-documento.md)
  (`D85..D88`; cópia de grupo `SPEC-D-013`). Isso **fecha** o débito L-6 de
  `docs/analise/desktop.md` e o item que a `SPEC-D-011` §11 registrava. O contrato
  de **produto** segue como estava: D77 (a forma e a página derivada), D78 (o
  round-trip e a tolerância) e D79 (a orientação fora e a Ceci que não escreve).
- **Dependências declaradas (não são bloqueios de decisão).** O **motor de DOCX
  real** (`D88`) e o **medidor de texto real** (`D86`) entram em fase própria, com
  licença registrada; o critério de cada um já está fixado (`D78` e `D86`). A
  implementação de D4.2 (Rust → bridge → UI) pode começar pelo schema e pelo motor
  de paginação determinístico.
- **Escolha da biblioteca de DOCX — decidida por critério, não por preferência.**
  A cláusula de D78 exige um pacote que exponha `w:ins`/`w:del` com autor e data;
  um pacote que só aplane o texto fica recusado. A escolha concreta entra na
  implementação, com a licença registrada (isto **reduz** o C8: um DOCX externo é
  onde a licença de terceiro precisa estar declarada).
- **`[P]` fora de escopo agora.** Capa, folha de rosto e sumário conforme NBR
  14724 e o **modelo da instituição** (§4.7 **[P]**); importação por DOI/ISBN,
  estilos CSL além de ABNT e leitor de PDF com destaques (§4.5 **[P]**);
  **registro de solicitações e prazos do orientador** (§4.7 **[P]**). Nenhum entra
  sem confirmação no referencial.
- **`[P]` de recorte.** O enxugamento de subtelas — inclusive as do TCC
  (referencial §8) — é da dona do produto; o número e o nome das subtelas ficam
  como o referencial §4.7 os fixa até que a §8 seja fechada.
- **`[A]` do grupo.** Nenhuma pendência `[A]` do referencial §8 é do TCC &
  Projetos; nada aqui depende de decisão em aberto da dona do produto.

## Referências

| Documento | O que resolve |
|---|---|
| [`tcc-modelo-do-documento.md`](./tcc-modelo-do-documento.md) | o **modelo de dados** do documento (`D85..D88`): a árvore, a paginação pura, a versão por snapshot e o item a resolver |
| [`SPEC-D-013`](../../../docs/specs/desktop/SPEC-D-013-modelo-de-dados-do-documento.md) | a cópia de grupo do modelo de dados |
| [`SPEC-D-011`](../../../docs/specs/desktop/SPEC-D-011-editor-do-tcc-e-round-trip-docx.md) | a cópia de grupo desta decisão e do contrato de tolerância |
| [`01-fronteiras.md`](./01-fronteiras.md) | a matriz de dono; TCC, Biblioteca e Conhecimento |
| [`00-shell-e-navegacao.md`](./00-shell-e-navegacao.md) | a entrada de navegação do módulo |
| [`SPEC-D-007`](../../../docs/specs/desktop/SPEC-D-007-dono-do-dado-e-fronteira-entre-modulos.md) | dono do dado e os modos de vínculo (`D65`, `D66`, `D67`) |
| [`ADR-010`](../../../docs/decisoes/ADR-010-referencia-e-dono-do-dado-bibliografico.md) | Referência é da Biblioteca; o TCC cita |
| [`estudos.md`](./estudos.md) · [`faculdade.md`](./faculdade.md) | os módulos irmãos de D4 |
| [`spec-referencial-workspace-academico.md`](../../../spec-referencial-workspace-academico.md) §3.1, §3.2, §4.5, §4.6, §4.7, §6.4 | a origem: dono do dado, tipos de projeto, editor, round-trip e a Ceci |
| [`docs/analise/desktop.md`](../../../docs/analise/desktop.md) §L-6 | o débito que esta spec declara e reduz |
