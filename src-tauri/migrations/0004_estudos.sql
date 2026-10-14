-- cecistudy - schema do módulo Estudos (SPEC-D-010, D76)
-- Convenções iguais às de 0001 e 0003: id TEXT PRIMARY KEY (UUID v4 em Rust),
-- instantes INTEGER epoch em milissegundos (UTC), data civil TEXT YYYY-MM-DD,
-- booleanos INTEGER 0/1.
--
-- O que NÃO existe aqui, por decisão (D76, cláusula do vencimento): nenhuma
-- coluna de vencimento, de "devido hoje" nem de fila. O vencimento é calculado
-- na leitura, em Rust (`src-tauri/src/estudos/`), a partir de
-- `ultima_revisao + intervalo_dias` na zona da usuária. O estado FSRS por
-- cartão é que é dado — estabilidade, dificuldade, última revisão, intervalo do
-- último agendamento e o marcador de relearning.
--
-- A retenção desejada não é coluna nem seed: o padrão dela é [P] (§4.4) e a
-- dona do produto decide. Ela entra como parâmetro no comando, vinda da tela.

-- 20. Conteúdo (tópico de estudo). Estudos é o dono (matriz D65); Faculdade,
--     TCC e Biblioteca mostram por referência, nunca por cópia (I8).
CREATE TABLE conteudo (
  id         TEXT PRIMARY KEY,
  titulo     TEXT NOT NULL,
  texto      TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

-- 21. Cartão com o estado FSRS persistido. `estabilidade`/`dificuldade` NULL
--     é o cartão novo, que nunca foi revisado. `intervalo_dias` é o intervalo
--     do ÚLTIMO agendamento — o histórico de agendamentos é a tabela
--     `revisao`. O vencimento não é coluna (D76).
CREATE TABLE cartao (
  id              TEXT PRIMARY KEY,
  conteudo_id     TEXT NOT NULL REFERENCES conteudo (id) ON DELETE CASCADE,
  frente          TEXT NOT NULL,
  verso           TEXT NOT NULL,
  estabilidade    REAL,
  dificuldade     REAL,
  ultima_revisao  INTEGER,
  intervalo_dias  INTEGER CHECK (intervalo_dias IS NULL OR intervalo_dias >= 1),
  relearning      INTEGER NOT NULL DEFAULT 0 CHECK (relearning IN (0, 1)),
  created_at      INTEGER NOT NULL,
  updated_at      INTEGER NOT NULL
);

CREATE INDEX idx_cartao_conteudo ON cartao (conteudo_id);

-- 22. Histórico de revisão (matriz D65: histórico FSRS é de Estudos). O
--     evento, não o estado: o que a usuária avaliou (1..4, a ordem de
--     `Avaliacao`) e quando. O estado resultante vive em `cartao`.
CREATE TABLE revisao (
  id          TEXT PRIMARY KEY,
  cartao_id   TEXT NOT NULL REFERENCES cartao (id) ON DELETE CASCADE,
  rating      INTEGER NOT NULL CHECK (rating IN (1, 2, 3, 4)),
  revisado_em INTEGER NOT NULL,
  created_at  INTEGER NOT NULL
);

CREATE INDEX idx_revisao_cartao ON revisao (cartao_id);
