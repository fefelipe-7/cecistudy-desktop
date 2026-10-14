-- cecistudy - schema do módulo TCC & Projetos (SPEC-D-013, D85..D88)
-- Convenções iguais às de 0001/0003/0004: id TEXT PRIMARY KEY (UUID v4 em
-- Rust), instantes INTEGER epoch em milissegundos (UTC), data civil TEXT
-- YYYY-MM-DD, booleanos INTEGER 0/1.
--
-- O que NÃO existe aqui, por decisão:
--   - Nenhuma coluna de página, altura de bloco ou posição de cursor (D85/D86,
--     I1). A página é derivada do modelo pelo motor puro em `src-tauri/src/tcc/`;
--     a geometria (A4, margens, corpo) é PARÂMETRO do motor, não coluna.
--   - Nenhuma tabela de referência ou de fragmento (D85, I11). Referência é da
--     Biblioteca e Fragmento é do Conhecimento (`ADR-010`); o documento guarda o
--     vínculo (`referencia_id`, `fragmento_id`, `alvo_id`) SEM FK enquanto as
--     tabelas donas não existem — é referência viva (SPEC-D-007 D66).

-- 23. Projeto: dono de Projeto (SPEC-D-007 D65). Sete tipos fixos (§4.7 [D]).
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

CREATE INDEX idx_projeto_estado ON projeto (estado);

-- 24. Documento: um por projeto na v1. O conteúdo é a árvore (no_documento),
--     não uma coluna de texto; o inline é `inline_run`.
CREATE TABLE documento (
  id         TEXT PRIMARY KEY,
  projeto_id TEXT NOT NULL UNIQUE REFERENCES projeto (id) ON DELETE CASCADE,
  titulo     TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

-- 25. Nó do documento: lista de adjacência ordenada (D85). O CHECK amarra as
--     colunas de cada tipo — nó de tipo errado não entra, coluna de outro tipo
--     não sobra. `pai_id = NULL` é a raiz; a ordem é única entre irmãos pelo
--     índice abaixo. `referencia_id`/`fragmento_id` são vínculos sem FK (D66).
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

-- Ordem única entre irmãos. `COALESCE(pai_id, '')` porque NULL é distinto de
-- NULL no UNIQUE do SQLite: sem ele, a raiz aceitaria duas ordens iguais.
CREATE UNIQUE INDEX idx_no_ordem
  ON no_documento (documento_id, COALESCE(pai_id, ''), ordem);
CREATE INDEX idx_no_documento ON no_documento (documento_id);
CREATE INDEX idx_no_pai ON no_documento (pai_id);

-- 26. Inline do parágrafo: um trecho de texto com marcas (D85). `marcas` é
--     string canônica — lista fechada, ordenada, sem repetição — para que a
--     serialização seja determinística e a comparação de versões não dependa da
--     ordem de digitação.
CREATE TABLE inline_run (
  id     TEXT PRIMARY KEY,
  no_id  TEXT NOT NULL REFERENCES no_documento (id) ON DELETE CASCADE,
  ordem  INTEGER NOT NULL CHECK (ordem >= 0),
  texto  TEXT NOT NULL,
  marcas TEXT NOT NULL DEFAULT ''
);

CREATE UNIQUE INDEX idx_inline_ordem ON inline_run (no_id, ordem);

-- 27. Versão: snapshot completo e canônico da árvore por escrita confirmada
--      (D87). Sem diff na v1; `numero` é sequencial por documento. O autosave
--      coalesce digitação numa escrita, e é ela que versiona — não por tecla.
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

CREATE INDEX idx_versao_documento ON versao (documento_id);

-- 28. Item a resolver: saída da importação de DOCX (D88). O estado de encaixe
--      é dado, não exibição. `autor_resolvido = 0` é o autor desconhecido (D78).
--      A âncora é o TEXTO citado (`citado`), nunca um deslocamento.
CREATE TABLE item_a_resolver (
  id              TEXT PRIMARY KEY,
  documento_id    TEXT NOT NULL REFERENCES documento (id) ON DELETE CASCADE,
  tipo            TEXT NOT NULL CHECK (tipo IN ('comentario','sugestao')),
  autor           TEXT NOT NULL,
  autor_resolvido INTEGER NOT NULL DEFAULT 1 CHECK (autor_resolvido IN (0, 1)),
  data_autor_ms   INTEGER,
  citado          TEXT NOT NULL,
  substituto      TEXT,
  texto           TEXT,
  origem_arquivo  TEXT,
  importado_em    INTEGER NOT NULL,
  estado          TEXT NOT NULL CHECK (estado IN
                  ('pendente','resolvido','nao-encaixou','duplicado')),
  resolvido_em    INTEGER,
  created_at      INTEGER NOT NULL
);

CREATE INDEX idx_item_documento ON item_a_resolver (documento_id);
CREATE INDEX idx_item_estado ON item_a_resolver (estado);

-- 29. Base de pesquisa: vínculo do projeto ao acervo (Biblioteca) e ao
--     Conhecimento. `alvo_id` é referência viva sem FK (D66).
CREATE TABLE base_de_pesquisa (
  id         TEXT PRIMARY KEY,
  projeto_id TEXT NOT NULL REFERENCES projeto (id) ON DELETE CASCADE,
  alvo_tipo  TEXT NOT NULL CHECK (alvo_tipo IN ('referencia','fragmento')),
  alvo_id    TEXT NOT NULL,
  criado_em  INTEGER NOT NULL
);

CREATE INDEX idx_base_projeto ON base_de_pesquisa (projeto_id);

-- 30. Saída: registro das exportações (§3.1). O binário não é dado do projeto;
--     o registro é.
CREATE TABLE saida (
  id         TEXT PRIMARY KEY,
  projeto_id TEXT NOT NULL REFERENCES projeto (id) ON DELETE CASCADE,
  formato    TEXT NOT NULL CHECK (formato IN ('docx','pdf')),
  destino    TEXT NOT NULL,
  criado_em  INTEGER NOT NULL
);

CREATE INDEX idx_saida_projeto ON saida (projeto_id);
