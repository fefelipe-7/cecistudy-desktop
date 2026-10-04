-- Campus — D88 · camada clínica do Estágio (store próprio)
--
-- Este arquivo é a **versão 1** do store clínico, e ele vive em
-- `campus-clinico.sqlite`, não em `campus.sqlite`. A numeração do arquivo é global
-- e sequencial — `0004` — para que "qual versão o schema está" tenha uma resposta
-- só; a versão gravada em `user_version` é local, e aqui é 1.
--
-- §1.6 da spec referencial, linha 23, é `[D]`: "Paciente real (Profissional),
-- paciente de estágio (Acadêmico) e Personagem simulado (Sala de treino) são três
-- domínios separados: nunca compartilham tabela, id nem vínculo automático".
--
-- A separação aqui é **física**, e não por convenção de nome:
--
--   - este schema não tem chave estrangeira nenhuma, e por isso `PRAGMA
--     foreign_key_list` responde vazio em qualquer tabela;
--   - o schema do acadêmico não tem tabela de dado sensível, e o daqui não tem
--     `calendar_event`, `responsibility` nem `plan_block`;
--   - os dois arquivos não estão no mesmo backup, porque backup é por arquivo.
--
-- O que **não** está aqui, e por quê: retenção, exclusão e nível de sigilo. §4.8
-- linha 479 é `[A]` — as normas do CFP e da instituição ainda precisam ser lidas e
-- confirmadas com a supervisão. Agente não resolve `[A]`, e um schema que finge
-- saber o prazo de retenção seria pior do que um schema que não sabe.

-- 1. Registro de atendimento.
--
-- §4.8 linha 466 é `[D]`: paciente de estágio é identificado por **código ou
-- iniciais**, nunca por documento, idade ou dado sensível. Não há coluna para
-- qualquer um deles, e o teste `d88_store_clinico_nao_tem_coluna_de_documento_idade_ou_nome`
-- do `db.rs` prova pelo catálogo.
CREATE TABLE atendimento (
  id            TEXT PRIMARY KEY,
  -- Iniciais ou código. Texto livre porque a usuária escreve como quiser; o que
  -- não existe é o campo que identifica a pessoa.
  iniciais      TEXT NOT NULL,
  -- Epoch ms, UTC, o mesmo `Instant` do resto do app.
  data          INTEGER NOT NULL,
  duracao_min   INTEGER NOT NULL CHECK (duracao_min > 0),
  -- §4.8 linha 467: decisões tomadas, reflexões do atendimento e registro livre.
  -- Texto **cifrado em repouso** — a cifragem é `SPEC-D-008` `D68`, e o schema
  -- guarda o texto cifrado, não o texto.
  corpo_cifrado BLOB NOT NULL,
  -- §4.8 linha 472, único campo de texto que vai para o mobile, escolhido por ela.
  -- Vive **neste** store e é a razão de a lista de quatro campos existir.
  para_levar_cifrado BLOB,
  -- Estado do registro no ciclo do atendimento. `aberto` e `encerrado` são os
  -- estados, e nada mais: §4.8 linha 529 diz que sessão encerrada não é editada,
  -- e a regra é imposta pelo Rust.
  estado        TEXT NOT NULL CHECK (estado IN ('aberto', 'encerrado')),
  criado_em     INTEGER NOT NULL,
  atualizado_em INTEGER NOT NULL
);

CREATE INDEX idx_atendimento_data ON atendimento (data);
CREATE INDEX idx_atendimento_estado ON atendimento (estado);

-- 2. Chave de cifragem do domínio.
--
-- Uma chave por domínio, e não derivada do texto: §1.6 linha 23 pede separação de
-- domínio, e chave derivada do conteúdo é chave compartilhada por definição.
CREATE TABLE chave_clinica (
  id            INTEGER PRIMARY KEY CHECK (id = 1),
  chave         BLOB NOT NULL,
  criado_em     INTEGER NOT NULL
);