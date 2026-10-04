-- Campus — D87 · domínio do usuário (SPEC-D-013, Fase 8.0.5)
--
-- Por que esta migration existe: `0001_init.sql` criou `setting` como chave-valor
-- genérica, e a alternativa rejeitada em `D87` é exatamente essa — "deixar cada
-- módulo ter sua própria tabela de preferência e um `setting` genérico". Preference
-- em chave livre é onde vai toda decisão que ninguém quis modelar, e era o que
-- `get_setting` lia com `unwrap_or` em falha silenciosa
-- (`commands.rs:1818`): preferência corrompida virava preferência ausente, e a
-- usuária nunca descobria.
--
-- O que muda:
--   - `profile`            — identidade da usuária, linha única
--   - `workspace_preferencia` — o que cada workspace prefere, por workspace
--   - `configuracao`       — configuração nomeada e **tipada**, com CHECK
--   - `setting`            — **aposentada**, com os dados levados para `configuracao`
--
-- §5.4 da spec referencial (linhas 559-561) é quem lista o que mora aqui: regra de
-- média padrão, retenção desejada do FSRS, integração do Google Calendar,
-- aparência e sincronização.

-- 1. Identidade. Linha única: o app é mono-usuário, e uma tabela com `id` deixaria
--    a pergunta "qual é a usuária?" sem resposta no schema.
CREATE TABLE profile (
  id                INTEGER PRIMARY KEY CHECK (id = 1),
  nome              TEXT NOT NULL DEFAULT '',
  curso             TEXT NOT NULL DEFAULT '',
  instituicao       TEXT NOT NULL DEFAULT '',
  -- §5.4 linha 561: "semestre atual". Guardado como texto porque o período letivo
  -- tem dono do mobile (`SPEC-C-006`) e o desktop o espelha por referência viva —
  -- ele não é dono, então não tem chave estrangeira para cá.
  semestre_atual    TEXT NOT NULL DEFAULT '',
  criado_em         INTEGER NOT NULL,
  atualizado_em     INTEGER NOT NULL
);

INSERT INTO profile (id, criado_em, atualizado_em) VALUES (1, 0, 0);

-- 2. Preferência por workspace. §2 linha 18: Workspace Acadêmico é ativo por
--    padrão até a formatura; o Profissional é outro contexto da mesma base.
--    O Workspace Profissional está fora de escopo (§2 linha 47), então só o
--    Acadêmico tem linha — e a chave é `workspace` e não `modulo`, porque
--    preferência é do contexto, não da tela.
CREATE TABLE workspace_preferencia (
  workspace         TEXT NOT NULL CHECK (workspace IN ('academico')),
  chave             TEXT NOT NULL,
  valor_json        TEXT NOT NULL,
  atualizado_em     INTEGER NOT NULL,
  PRIMARY KEY (workspace, chave)
);

-- 3. Configuração nomeada e tipada.
--
-- A lista de `chave` é **fechada** por CHECK, e é a mesma lista que
-- `contracts/usuario.json` declara. Uma chave fora daqui não entra: a alternativa
-- rejeitada em `D87` era chave livre, e chave livre é o que transforma
-- configuração em depositório de decisão adiada.
--
-- Cada `valor_json` tem o formato do valor, e isso é verificável por golden:
CREATE TABLE configuracao (
  -- PRIMARY KEY, e não só `NOT NULL CHECK`: sem chave primária o
  -- `ON CONFLICT (chave) DO UPDATE` do `set_setting` não casa com restrição
  -- nenhuma, e o `CHECK` sozinho garante validade mas não unicidade. A lista
  -- fechada é o que define **quais**; a chave primária é o que garante **uma vez
  -- cada**.
  chave             TEXT PRIMARY KEY CHECK (chave IN (
                     -- §5.4 linha 560, Configurações
                     'regra_media.padrao',        -- json: RegraMediaPadrao
                     'fsrs.retencao_desejada',     -- json: number em [0.5, 0.995]
                     'google.integracao',          -- json: {ativa: bool, calendario: string}
                     'aparencia',                  -- json: 'light' | 'dark' | 'sistema'
                     'sincronizacao'               -- json: Sincronizacao
                   )),
  valor_json        TEXT NOT NULL,
  atualizado_em     INTEGER NOT NULL
);

-- 4. Migra o que existia em `setting`.
--
-- A chave antiga era um nome solto; o mapeamento abaixo é a **única** ponte entre
-- o formato antigo e o novo, e ele é explícito de propósito. Um mapeamento
-- genérico (`chave = chave`) seria mais curto e levaria preference para dentro da
-- `configuracao` sem CHECK, que é exatamente a violação que esta migration fecha.
--
-- `retention.fsrs` é a única chave que o `get_setting` legado chegou a ler, e ela
-- vira `fsrs.retencao_desejada`. As demais são descartadas: preference sem dono
-- declarado é ruído, e levá-la adiante seria dar sobrevida a um formato que esta
-- migration aposenta.
INSERT OR IGNORE INTO configuracao (chave, valor_json, atualizado_em)
SELECT 'fsrs.retencao_desejada', value_json, 0
FROM setting
WHERE key = 'retention.fsrs';

-- 5. Aposenta a chave-valor genérica.
--
-- O `DROP` é o que torna a regra verificável: não existe mais lugar onde uma
-- preferência pode entrar sem passar pela lista fechada. Sem ele, a `0003` documenta
-- uma intenção e não impõe nada.
DROP TABLE setting;