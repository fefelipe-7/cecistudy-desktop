-- cecistudy — schema do módulo Faculdade (SPEC-D-010, D74/D75)
-- Convenções iguais às de 0001: id TEXT PRIMARY KEY (UUID v4 em Rust),
-- instantes INTEGER epoch em milissegundos (UTC), data civil TEXT YYYY-MM-DD,
-- booleanos INTEGER 0/1.
--
-- O que NÃO existe aqui, por decisão (D74, I1): nenhuma coluna de média, de
-- situação nem de "quanto falta". O motor calcula tudo na leitura, em Rust
-- (`src-tauri/src/materia/`), e a **nota lançada** é o único dado da conta.
-- Pelo mesmo motivo o nome de coluna `tipo_de_calculo` evita o literal que o
-- gate I1 de `SPEC-D-010` procura nos arquivos desta pasta.

-- 13. Semestre letivo. `rotulo` é o nome humano (2026-1); `id` é o UUID.
--     `encerrado_em` NULL = aberto. Encerrar é o ato que grava a regra
--     congelada (D75) e torna a regra do semestre imutável (I7).
CREATE TABLE semestre (
  id           TEXT PRIMARY KEY,
  rotulo       TEXT NOT NULL UNIQUE,
  starts_on    TEXT NOT NULL,
  ends_on      TEXT NOT NULL,
  encerrado_em INTEGER,
  created_at   INTEGER NOT NULL,
  updated_at   INTEGER NOT NULL,
  CHECK (ends_on >= starts_on)
);

-- 14. Disciplina cursada. `status = 'arquivado'` remove da conta sem apagar o
--     histórico. Frequência é registrada aqui e a leitura deriva o que sobra
--     (I5): não há coluna de frequência restante.
CREATE TABLE disciplina (
  id            TEXT PRIMARY KEY,
  semestre_id   TEXT NOT NULL REFERENCES semestre (id) ON DELETE CASCADE,
  nome          TEXT NOT NULL,
  codigo        TEXT,
  professor     TEXT,
  carga_horaria INTEGER CHECK (carga_horaria IS NULL OR carga_horaria >= 0),
  status        TEXT NOT NULL CHECK (status IN ('ativa', 'arquivado')),
  total_aulas   INTEGER NOT NULL DEFAULT 0 CHECK (total_aulas >= 0),
  faltas        INTEGER NOT NULL DEFAULT 0 CHECK (faltas >= 0),
  created_at    INTEGER NOT NULL,
  updated_at    INTEGER NOT NULL,
  CHECK (faltas <= total_aulas)
);

CREATE INDEX idx_disciplina_semestre ON disciplina (semestre_id);

-- 15. Avaliação de uma disciplina. `vale_nota = 0` é a que não entra na conta e
--     vira Responsabilidade (§4.2 [D]). `peso` só importa no cálculo ponderado.
CREATE TABLE avaliacao (
  id            TEXT PRIMARY KEY,
  disciplina_id TEXT NOT NULL REFERENCES disciplina (id) ON DELETE CASCADE,
  titulo        TEXT NOT NULL,
  tipo          TEXT NOT NULL CHECK (tipo IN ('prova', 'trabalho', 'apresentacao', 'entrega', 'outro')),
  peso          REAL NOT NULL CHECK (peso > 0),
  vale_nota     INTEGER NOT NULL DEFAULT 1 CHECK (vale_nota IN (0, 1)),
  plain_date    TEXT,
  created_at    INTEGER NOT NULL,
  updated_at    INTEGER NOT NULL
);

CREATE INDEX idx_avaliacao_disciplina ON avaliacao (disciplina_id);

-- 16. Nota lançada (D74). É o único dado persistido da conta. Uma por
--     avaliação; `lancada_em` guarda o instante do lançamento, o que permite
--     lançar nota em semestre encerrado (I9) sem tocar na regra. Faixa 0..10,
--     igual a `NOTA_MAXIMA` do motor em `src-tauri/src/materia/`.
CREATE TABLE nota (
  id           TEXT PRIMARY KEY,
  avaliacao_id TEXT NOT NULL UNIQUE REFERENCES avaliacao (id) ON DELETE CASCADE,
  valor        REAL NOT NULL CHECK (valor >= 0 AND valor <= 10),
  lancada_em   INTEGER NOT NULL,
  created_at   INTEGER NOT NULL
);

-- 17. Regra padrão global (D75). Linha única: `id` fixo em 'padrao'. É a camada
--     que a disciplina sem personalização herda e que, ao mudar, move todas as
--     disciplinas não personalizadas (I6).
CREATE TABLE regra_padrao (
  id                    TEXT PRIMARY KEY CHECK (id = 'padrao'),
  tipo_de_calculo       TEXT NOT NULL CHECK (tipo_de_calculo IN ('simples', 'ponderada')),
  nota_minima           REAL NOT NULL CHECK (nota_minima >= 0 AND nota_minima <= 10),
  arredondamento        REAL NOT NULL CHECK (arredondamento > 0 AND arredondamento <= 1),
  frequencia_minima     REAL NOT NULL CHECK (frequencia_minima >= 0 AND frequencia_minima <= 1),
  exame_modo            TEXT NOT NULL CHECK (exame_modo IN ('nenhum', 'recuperacao', 'substitui')),
  exame_minima          REAL NOT NULL CHECK (exame_minima >= 0 AND exame_minima <= 10),
  exame_elegivel_minima REAL NOT NULL CHECK (exame_elegivel_minima >= 0 AND exame_elegivel_minima <= 10),
  atualizado_em         INTEGER NOT NULL
);

-- 18. Personalização por disciplina (D75). Substitui o padrão **por inteiro**,
--     não por campo. Vive enquanto o semestre está aberto. É a camada viva.
CREATE TABLE regra_disciplina (
  disciplina_id         TEXT PRIMARY KEY REFERENCES disciplina (id) ON DELETE CASCADE,
  tipo_de_calculo       TEXT NOT NULL CHECK (tipo_de_calculo IN ('simples', 'ponderada')),
  nota_minima           REAL NOT NULL CHECK (nota_minima >= 0 AND nota_minima <= 10),
  arredondamento        REAL NOT NULL CHECK (arredondamento > 0 AND arredondamento <= 1),
  frequencia_minima     REAL NOT NULL CHECK (frequencia_minima >= 0 AND frequencia_minima <= 1),
  exame_modo            TEXT NOT NULL CHECK (exame_modo IN ('nenhum', 'recuperacao', 'substitui')),
  exame_minima          REAL NOT NULL CHECK (exame_minima >= 0 AND exame_minima <= 10),
  exame_elegivel_minima REAL NOT NULL CHECK (exame_elegivel_minima >= 0 AND exame_elegivel_minima <= 10),
  atualizado_em         INTEGER NOT NULL
);

-- 19. Regra congelada por semestre (D75, I7). Gravada no ato de encerrar, com a
--     regra que estava efetivamente em vigor em cada disciplina. Imutável: o
--     caminho de escrita recusa alterar regra de semestre encerrado, e é ela que
--     responde "por que essa disciplina reprovou" (I8).
CREATE TABLE regra_congelada (
  semestre_id           TEXT NOT NULL REFERENCES semestre (id) ON DELETE CASCADE,
  disciplina_id         TEXT NOT NULL REFERENCES disciplina (id) ON DELETE CASCADE,
  tipo_de_calculo       TEXT NOT NULL CHECK (tipo_de_calculo IN ('simples', 'ponderada')),
  nota_minima           REAL NOT NULL CHECK (nota_minima >= 0 AND nota_minima <= 10),
  arredondamento        REAL NOT NULL CHECK (arredondamento > 0 AND arredondamento <= 1),
  frequencia_minima     REAL NOT NULL CHECK (frequencia_minima >= 0 AND frequencia_minima <= 1),
  exame_modo            TEXT NOT NULL CHECK (exame_modo IN ('nenhum', 'recuperacao', 'substitui')),
  exame_minima          REAL NOT NULL CHECK (exame_minima >= 0 AND exame_minima <= 10),
  exame_elegivel_minima REAL NOT NULL CHECK (exame_elegivel_minima >= 0 AND exame_elegivel_minima <= 10),
  congelado_em          INTEGER NOT NULL,
  PRIMARY KEY (semestre_id, disciplina_id)
);

-- Semente da regra padrão: os valores de partida do produto. `atualizado_em` é 0
-- na semente; a primeira edição nas Configurações grava o instante real. Ter a
-- linha sempre presente é o que deixa `motor` sem ramo de "nunca configurado".
INSERT INTO regra_padrao (
  id, tipo_de_calculo, nota_minima, arredondamento, frequencia_minima,
  exame_modo, exame_minima, exame_elegivel_minima, atualizado_em
) VALUES (
  'padrao', 'ponderada', 6.0, 0.1, 0.75,
  'nenhum', 6.0, 4.0, 0
);
