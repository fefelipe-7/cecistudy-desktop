-- Campus — D88 · Sala de treino (store próprio)
--
-- Este arquivo é a **versão 1** do store da Sala de treino, e ele vive em
-- `campus-sala-treino.sqlite`, nunca em `campus.sqlite`. A numeração do arquivo é
-- global e sequencial — `0005` — para que "qual versão o schema está" tenha uma
-- resposta só; a versão gravada em `user_version` é local, e aqui é 1.
--
-- §1.6 da spec referencial, linha 23, é `[D]`: Personagem simulado é um dos três
-- domínios separados, e nunca compartilha tabela, `id` nem vínculo com paciente
-- real nem com paciente de estágio. A phrase exata está em §4.9 linha 498:
-- "Personagem simulado nunca compartilha tabela ou id com paciente real ou de
-- estágio".
--
-- O schema não tem chave estrangeira nenhuma — `PRAGMA foreign_key_list` responde
-- vazio em qualquer tabela — e o `db.rs` tem teste que prova isso.
--
-- O que **não** está aqui, e por quê: modelo de IA, voz, e o conteúdo do
-- personagem. §4.9 linha 490 diz que as premissas técnicas são "mais ou menos"
-- confirmadas, e §8 linha 609 é `[A]`: revalidar com protótipo. Este schema guarda
-- o que é **log e estado**, que é estável mesmo com o modelo trocado — `D72` diz
-- que a premissa de CPU é hipótese, e um schema que depende da hipótese morre com
-- ela.

-- 1. Personagem simulado.
CREATE TABLE personagem (
  id            TEXT PRIMARY KEY,
  nome          TEXT NOT NULL,
  tema          TEXT NOT NULL DEFAULT '',
  -- §4.9 linha 497: schema genérico de personagem. `descricao_json` é um documento
  -- livre porque o formato é do modelo, e `D72` diz que o modelo é trocável —
  -- coluna com o formato do modelo seria refatoração a cada troca.
  descricao_json TEXT NOT NULL DEFAULT '{}',
  criado_em     INTEGER NOT NULL,
  atualizado_em INTEGER NOT NULL
);

-- 2. Sessão simulada.
--
-- `D71`: a sessão vem de uma **série de recorrência materializada**, e não de um
-- contador. Por isso `numero` tem `UNIQUE` e `personagem_id` é texto puro, sem
-- FK — e a série vive em `serie_sessao`.
--
-- §4.9 linha 495 é premissa ("continuidade semanal real"), não fato: a série é
-- criada pelo motor único de recorrência, e isto aqui guarda o que já foi
-- materializado.
CREATE TABLE serie_sessao (
  id            TEXT PRIMARY KEY,
  personagem_id TEXT NOT NULL,
  -- RRULE ancorada no fuso do personagem, no mesmo formato que
  -- `event_recurrence.rrule` no store acadêmico. O glossário do grupo é quem diz
  -- que o fuso é o da regra, não o da máquina (`D6`).
  rrule         TEXT NOT NULL,
  dtstart       INTEGER NOT NULL,
  dtstart_tz    TEXT NOT NULL,
  until_at      INTEGER,
  criado_em     INTEGER NOT NULL
);

CREATE INDEX idx_serie_sessao_personagem ON serie_sessao (personagem_id);

CREATE TABLE sessao_simulada (
  id            TEXT PRIMARY KEY,
  serie_id      TEXT NOT NULL,
  personagem_id TEXT NOT NULL,
  numero        INTEGER NOT NULL CHECK (numero > 0),
  -- §4.9 linha 506: modo voz ou texto, e os dois têm **o mesmo** estado e o mesmo
  -- log (`D73`). O modo é campo do log, não duas tabelas.
  modo          TEXT NOT NULL CHECK (modo IN ('voz', 'texto')),
  iniciada_em   INTEGER,
  encerrada_em  INTEGER,
  -- `nao_realizada` continua devendo: §4.9 linha 513 e o invariante `I3` da
  -- `SPEC-D-009` — a sessão não avança por clique, ela avança pelo calendário.
  estado        TEXT NOT NULL CHECK (estado IN ('agendada', 'em_andamento', 'realizada', 'nao_realizada')),
  UNIQUE (personagem_id, numero)
);

CREATE INDEX idx_sessao_simulada_serie ON sessao_simulada (serie_id);
CREATE INDEX idx_sessao_simulada_estado ON sessao_simulada (estado);

-- 3. Log de eventos da sessão.
--
-- `D73`: métrica e evento **nunca** são somados. Métricas objetivas (tempo,
-- interrupções, silêncios) e eventos interpretativos (§4.9 linha 523-524) vivem
-- em tabelas separadas, e a pós-sessão junta. Uma tabela só permitiria somar.
CREATE TABLE evento_sessao (
  id            TEXT PRIMARY KEY,
  sessao_id     TEXT NOT NULL,
  -- Epoch ms com fração: `D73` diz que nuance é evento com marca de tempo, e
  -- milissegulo inteiro perde o que distingue uma pausa de uma hesitação.
  em_ms         INTEGER NOT NULL,
  tipo          TEXT NOT NULL,
  -- `interpretativo` é o que separa evento de métrica. Um evento rotulado
  -- interpretativo nunca entra em soma de métrica, e é isso que a pós-sessão
  -- respeita.
  natureza      TEXT NOT NULL CHECK (natureza IN ('objetiva', 'interpretativa')),
  detalhe_json  TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX idx_evento_sessao_sessao ON evento_sessao (sessao_id);

-- 4. Feedback e desempenho.
--
-- §4.9 linha 529: sessão encerrada não é editada, e o feedback fica como registro.
-- Não há coluna de "nota": `D73` e §4.9 linha 522 rejeitam nota única, e um
-- `REAL` aqui seria a convite a voltar atrás.
CREATE TABLE feedback_sessao (
  id            TEXT PRIMARY KEY,
  sessao_id     TEXT NOT NULL,
  metricas_json TEXT NOT NULL,
  -- Reflexão da usuária, em campo separado das métricas objetivas.
  autorreflexao TEXT NOT NULL DEFAULT '',
  criado_em     INTEGER NOT NULL
);

CREATE INDEX idx_feedback_sessao_sessao ON feedback_sessao (sessao_id);

CREATE TABLE desempenho (
  id            TEXT PRIMARY KEY,
  personagem_id TEXT NOT NULL,
  -- Snapshot do desempenho em uma data. Comparável, porque o formato é o mesmo em
  -- toda linha e §4.9 linha 508 promete "métricas comparadas".
  em            INTEGER NOT NULL,
  metricas_json TEXT NOT NULL,
  UNIQUE (personagem_id, em)
);

CREATE INDEX idx_desempenho_personagem ON desempenho (personagem_id);