-- Campus — schema do Calendário (spec 01, §3.3)
-- Convenções:
--   id           TEXT PRIMARY KEY (UUID v4 gerado em Rust)
--   instantes    INTEGER epoch em milissegundos (UTC)
--   fuso         TEXT IANA
--   booleanos    INTEGER 0/1
-- `PRAGMA foreign_keys` é ligado em db.rs a cada conexão: sem isso o SQLite
-- ignora ON DELETE CASCADE e o D10 (materialização) fica inconsistente.

-- 1. Camadas visíveis na grade. `position` define a ordem de empilhamento.
CREATE TABLE layer (
  id        TEXT PRIMARY KEY,
  title     TEXT NOT NULL,
  -- D4: o tom é 1..5 (chart-1..chart-5) ou 'neutral' (leitura, Google).
  -- Guardado como TEXT porque `LayerTone` é a união `1|2|3|4|5|"neutral"`.
  tone      TEXT NOT NULL CHECK (tone IN ('1', '2', '3', '4', '5', 'neutral')),
  icon      TEXT NOT NULL,
  visible   INTEGER NOT NULL DEFAULT 1 CHECK (visible IN (0, 1)),
  position  INTEGER NOT NULL
);

-- 2. Evento do Calendário: a única entidade que ocupa espaço na grade.
--    INV-2: NÃO existe coluna de responsabilidade aqui — o prazo de uma
--    responsabilidade é `responsibility.due_at`, nunca o evento.
CREATE TABLE calendar_event (
  id           TEXT PRIMARY KEY,
  layer_id     TEXT NOT NULL REFERENCES layer (id),
  title        TEXT NOT NULL,
  notes        TEXT,
  starts_at    INTEGER NOT NULL,
  ends_at      INTEGER NOT NULL,
  all_day      INTEGER NOT NULL CHECK (all_day IN (0, 1)),
  plain_date   TEXT,
  time_zone    TEXT NOT NULL,
  location     TEXT,
  commitment   TEXT NOT NULL CHECK (commitment IN ('obrigatorio', 'importante', 'recomendado', 'opcional')),
  state        TEXT NOT NULL CHECK (state IN ('planejado', 'em_andamento', 'concluido', 'adiado', 'nao_realizado', 'cancelado', 'dispensado')),
  origin       TEXT NOT NULL CHECK (origin IN ('cecistudy', 'faculdade', 'estudos', 'tcc', 'estagio', 'conhecimento', 'google')),
  owner_type   TEXT,
  owner_id     TEXT,
  created_at   INTEGER NOT NULL,
  updated_at   INTEGER NOT NULL,
  -- Evento de dia inteiro carrega PlainDate; evento com hora não.
  CHECK ((all_day = 1 AND plain_date IS NOT NULL) OR (all_day = 0 AND plain_date IS NULL)),
  CHECK (ends_at >= starts_at),
  -- D14: o vínculo com o módulo de origem é par ou nenhum.
  CHECK ((owner_type IS NULL) = (owner_id IS NULL))
);

CREATE INDEX idx_calendar_event_window ON calendar_event (starts_at, ends_at);
CREATE INDEX idx_calendar_event_origin ON calendar_event (origin);
CREATE INDEX idx_calendar_event_layer ON calendar_event (layer_id);

-- 3. Regra de recorrência. Uma regra por evento (`event_id UNIQUE`).
--    D10: as instâncias ficam materializadas em `event_occurrence`.
CREATE TABLE event_recurrence (
  id            TEXT PRIMARY KEY,
  event_id      TEXT NOT NULL UNIQUE REFERENCES calendar_event (id) ON DELETE CASCADE,
  rrule         TEXT NOT NULL,
  dtstart_tz    TEXT NOT NULL,
  -- Colunas são a fonte da verdade de limite; o que estiver na string RRULE é
  -- descartado ao expandir (ver domain/recurrence.ts).
  until_at      INTEGER,
  count         INTEGER CHECK (count IS NULL OR count > 0),
  exdates_json  TEXT NOT NULL DEFAULT '[]'
);

-- 4. Instância materializada (D10). `original_start` é a identidade estável da
--    instância e nunca muda, mesmo quando ela é remarcada (§5).
CREATE TABLE event_occurrence (
  id              TEXT PRIMARY KEY,
  event_id        TEXT NOT NULL REFERENCES calendar_event (id) ON DELETE CASCADE,
  rule_id         TEXT REFERENCES event_recurrence (id) ON DELETE CASCADE,
  original_start  INTEGER NOT NULL,
  starts_at       INTEGER NOT NULL,
  ends_at         INTEGER NOT NULL,
  state           TEXT NOT NULL CHECK (state IN ('planejado', 'em_andamento', 'concluido', 'adiado', 'nao_realizado', 'cancelado', 'dispensado')),
  -- 'none'     instância materializada sem intervenção
  -- 'cancelled'.cancelada com motivo (INV-5)
  -- 'moved'    remarcada; `starts_at` != `original_start`
  override        TEXT NOT NULL CHECK (override IN ('none', 'cancelled', 'moved')),
  cancel_reason   TEXT,
  execution_id    TEXT REFERENCES execution_record (id),
  created_at      INTEGER NOT NULL,
  updated_at      INTEGER NOT NULL,
  UNIQUE (event_id, original_start),
  CHECK (ends_at >= starts_at),
  CHECK (override = 'cancelled' OR cancel_reason IS NULL)
);

CREATE INDEX idx_event_occurrence_window ON event_occurrence (starts_at);
CREATE INDEX idx_event_occurrence_event ON event_occurrence (event_id);

-- 5. Responsabilidade: o que precisa ser feito. Tem prazo próprio, que é a
--    única fonte de atraso (INV-4). D2: um prazo principal; prazos extras são
--    filhas em `parent_id`.
CREATE TABLE responsibility (
  id                   TEXT PRIMARY KEY,
  layer_id             TEXT NOT NULL REFERENCES layer (id),
  title                TEXT NOT NULL,
  notes                TEXT,
  kind                 TEXT NOT NULL CHECK (kind IN ('tarefa', 'leitura', 'pesquisa', 'revisao', 'escrita', 'preparacao')),
  commitment           TEXT NOT NULL CHECK (commitment IN ('obrigatorio', 'importante', 'recomendado', 'opcional')),
  state                TEXT NOT NULL CHECK (state IN ('planejado', 'em_andamento', 'concluido', 'adiado', 'nao_realizado', 'cancelado', 'dispensado')),
  due_at               INTEGER,
  planned_duration_min INTEGER CHECK (planned_duration_min IS NULL OR planned_duration_min >= 0),
  objective            TEXT,
  origin               TEXT NOT NULL CHECK (origin IN ('cecistudy', 'faculdade', 'estudos', 'tcc', 'estagio', 'conhecimento', 'google')),
  owner_type           TEXT,
  owner_id             TEXT,
  parent_id            TEXT REFERENCES responsibility (id) ON DELETE CASCADE,
  created_at           INTEGER NOT NULL,
  updated_at           INTEGER NOT NULL,
  CHECK ((owner_type IS NULL) = (owner_id IS NULL))
);

CREATE INDEX idx_responsibility_due_state ON responsibility (due_at, state);
CREATE INDEX idx_responsibility_parent ON responsibility (parent_id);
CREATE INDEX idx_responsibility_origin ON responsibility (origin);

-- 6. Etapa de uma responsabilidade.
CREATE TABLE responsibility_step (
  id                TEXT PRIMARY KEY,
  responsibility_id TEXT NOT NULL REFERENCES responsibility (id) ON DELETE CASCADE,
  title             TEXT NOT NULL,
  position          INTEGER NOT NULL CHECK (position >= 0),
  state             TEXT NOT NULL CHECK (state IN ('pendente', 'em_andamento', 'concluida')),
  due_at            INTEGER,
  completed_at      INTEGER,
  UNIQUE (responsibility_id, position)
);

-- 7. Bloco de tempo reservado. INV-1: um bloco sempre pertence a uma
--    responsabilidade; nunca cria obrigação nova. Por isso o NOT NULL.
CREATE TABLE plan_block (
  id                   TEXT PRIMARY KEY,
  responsibility_id    TEXT NOT NULL REFERENCES responsibility (id) ON DELETE CASCADE,
  layer_id             TEXT NOT NULL REFERENCES layer (id),
  starts_at            INTEGER NOT NULL,
  ends_at              INTEGER NOT NULL,
  time_zone            TEXT NOT NULL,
  planned_duration_min INTEGER NOT NULL CHECK (planned_duration_min >= 0),
  state                TEXT NOT NULL CHECK (state IN ('planejado', 'em_andamento', 'concluido', 'nao_realizado')),
  origin               TEXT NOT NULL CHECK (origin IN ('cecistudy', 'faculdade', 'estudos', 'tcc', 'estagio', 'conhecimento', 'google')),
  timer_state          TEXT NOT NULL CHECK (timer_state IN ('parado', 'rodando', 'pausado')),
  accumulated_min      INTEGER NOT NULL DEFAULT 0 CHECK (accumulated_min >= 0),
  created_at           INTEGER NOT NULL,
  updated_at           INTEGER NOT NULL,
  CHECK (ends_at >= starts_at)
);

CREATE INDEX idx_plan_block_window ON plan_block (starts_at, ends_at);
CREATE INDEX idx_plan_block_responsibility ON plan_block (responsibility_id);

-- 8. O que realmente aconteceu (D12). Append-only: só INSERT e correção via
--    `corrected_by`. Sem FK para a entidade alvo, porque o alvo é polimórfico e
--    porque apagar um evento NÃO pode apagar o histórico (D12).
CREATE TABLE execution_record (
  id                  TEXT PRIMARY KEY,
  target_kind         TEXT NOT NULL CHECK (target_kind IN ('event', 'occurrence', 'responsibility', 'block')),
  target_id           TEXT NOT NULL,
  started_at          INTEGER NOT NULL,
  finished_at         INTEGER NOT NULL,
  actual_duration_min INTEGER NOT NULL CHECK (actual_duration_min >= 0),
  result              TEXT CHECK (result IS NULL OR result IN ('concluido', 'parcial')),
  notes               TEXT,
  source              TEXT NOT NULL CHECK (source IN ('manual', 'timer')),
  corrected_by        TEXT REFERENCES execution_record (id),
  created_at          INTEGER NOT NULL,
  CHECK (finished_at >= started_at)
);

CREATE INDEX idx_execution_record_target ON execution_record (target_kind, target_id);
CREATE INDEX idx_execution_record_corrected ON execution_record (corrected_by);

-- 9. Vínculo com um item externo. Criado vazio aqui; a spec 05 preenche.
CREATE TABLE external_link (
  id                        TEXT PRIMARY KEY,
  entity_type               TEXT NOT NULL,
  entity_id                 TEXT NOT NULL,
  system                    TEXT NOT NULL CHECK (system IN ('google')),
  remote_calendar_id        TEXT NOT NULL,
  remote_event_id           TEXT NOT NULL,
  remote_etag               TEXT,
  remote_ical_uid           TEXT,
  last_synced_at            INTEGER,
  last_seen_remote_updated  INTEGER,
  last_change_origin        TEXT NOT NULL CHECK (last_change_origin IN ('cecistudy', 'google')),
  sync_state                TEXT NOT NULL CHECK (sync_state IN ('sincronizado', 'pendente', 'conflito', 'removido_remoto')),
  sync_fields_json          TEXT NOT NULL DEFAULT '[]',
  conflict_local_json       TEXT,
  conflict_remote_json      TEXT,
  UNIQUE (system, remote_calendar_id, remote_event_id)
);

CREATE INDEX idx_external_link_entity ON external_link (entity_type, entity_id);
CREATE INDEX idx_external_link_sync_state ON external_link (sync_state);

-- 10. Sugestão de bloco. Schema criado aqui; a regra que produz é a spec 06.
CREATE TABLE suggestion (
  id                TEXT PRIMARY KEY,
  responsibility_id TEXT NOT NULL REFERENCES responsibility (id) ON DELETE CASCADE,
  layer_id          TEXT NOT NULL REFERENCES layer (id),
  starts_at         INTEGER NOT NULL,
  ends_at           INTEGER NOT NULL,
  rationale         TEXT NOT NULL,
  score             REAL NOT NULL CHECK (score >= 0),
  state             TEXT NOT NULL CHECK (state IN ('pendente', 'aceita', 'recusada', 'expirada')),
  produced_by       TEXT NOT NULL,
  created_at        INTEGER NOT NULL,
  resolved_at       INTEGER,
  CHECK (ends_at >= starts_at)
);

CREATE INDEX idx_suggestion_state ON suggestion (state);

-- 11. Preferências locais serializadas.
CREATE TABLE setting (
  key        TEXT PRIMARY KEY,
  value_json TEXT NOT NULL
);

-- 12. Cursor de sincronização por calendário (spec 05).
CREATE TABLE sync_cursor (
  system          TEXT NOT NULL CHECK (system IN ('google')),
  calendar_id     TEXT NOT NULL,
  sync_token      TEXT,
  last_full_sync_at INTEGER,
  PRIMARY KEY (system, calendar_id)
);
