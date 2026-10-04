//! Conexão SQLite, migrações versionadas e **três stores separados por domínio**.
//!
//! Decisões que moram aqui:
//!
//! - **A2** — SQLite embarcado, arquivo por domínio.
//! - **D10** — `event_occurrence` é tabela real, populada sob demanda.
//! - **Versionamento** — `PRAGMA user_version` é a única fonte de qual migration
//!   já rodou. Não existe tabela de histórico de migrations: o PRAGMA faz o
//!   papel e evita uma tabela extra que a spec §3.3 não pede.
//! - **Transação** — cada migration roda dentro de uma transação própria. Uma
//!   migration que falha no meio não deixa schema pela metade.
//! - **D88** — três stores, três **tipos**, três arquivos. Ver abaixo.
//!
//! `foreign_keys` fica `ON` de propósito: sem isso o SQLite ignora
//! `ON DELETE CASCADE` e o D10 (materialização) e o D12 (histórico) divergem.
//!
//! A conexão fica atrás de um `Mutex` porque `rusqlite` exige `&mut Connection`
//! para abrir transação, e o Tauri entrega `State<Db>`, que é `&Db`. Com o
//! `Mutex`, os comandos recebem `&Db` e mesmo assim abrem transação — e isso
//! também permite testar a lógica sem subir um app.
//!
//! ## D88 — por que três **tipos** e não três instâncias
//!
//! A spec referencial §1.6 linha 23 é `[D]`: "Paciente real (Profissional),
//! paciente de estágio (Acadêmico) e Personagem simulado (Sala de treino) são três
//! domínios separados: nunca compartilham tabela, id nem vínculo automático".
//!
//! Se os três fossem três instâncias de um tipo só, essa regra ficaria sendo
//!obedecida por convenção — e convenção é o que um `JOIN` descumpre sem erro. Com
//! três tipos distintos:
//!
//! - um comando do domínio acadêmico **não compila** se pedir `State<StoreClinico>`;
//! - o `tauri::manage!` só aceita tipos distintos, então não há como trocar um pelo
//!   outro em runtime;
//! - a invariante `I9` da `SPEC-D-013` deixa de ser um `rg` e vira o compilador.
//!
//! É a razão de a alternativa rejeitada em `D88` ser uma lista de tabelas com
//! prefixo mais um gate que proíbe `JOIN`: `rg` não pega SQL escrito depois, nem
//! `PRAGMA` malicioso. Tipo não se engana.

use std::fs;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;

/// Nome do arquivo do domínio acadêmico. Estável entre versões: é o mesmo arquivo
/// que a usuária já tem depois de uma atualização.
pub const DB_ACADEMICO: &str = "campus.sqlite";

/// D88 — o store da camada clínica do Estágio é outro arquivo, desde a primeira
/// versão. Ele não é uma tabela dentro do acadêmico: é um arquivo separado, e por
/// isso nunca entra num backup junto com o acadêmico.
pub const DB_CLINICO: &str = "campus-clinico.sqlite";

/// D88 — o store da Sala de treino é o terceiro arquivo, pelo mesmo motivo.
pub const DB_SALA_TREINO: &str = "campus-sala-treino.sqlite";

/// Migrations do domínio acadêmico, embutidas no binário.
///
/// `include_str!` resolve em tempo de compilação: o `.sql` viaja dentro do
/// executável, então o app não depende de ler um diretório `migrations/` que pode
/// não existir no Windows depois de um empacotamento.
///
/// `D87` — `0003_usuario.sql` cria o domínio do usuário e **aposenta** a `setting`
/// genérica da `0001`. Preferência com chave-valor livre é onde vai toda decisão
/// que ninguém quis modelar, e era o que `get_setting` lia com `unwrap_or`.
const MIGRATIONS_ACADEMICO: &[(&str, &str)] = &[
    ("0001_init.sql", include_str!("../migrations/0001_init.sql")),
    (
        "0002_seed_layers.sql",
        include_str!("../migrations/0002_seed_layers.sql"),
    ),
    (
        "0003_usuario.sql",
        include_str!("../migrations/0003_usuario.sql"),
    ),
];

/// Migrations da camada clínica do Estágio.
///
/// A numeração do arquivo é global e sequencial — `0004` — para que uma busca por
/// "qual versão o schema está" não precise de três respostas. A **versão** gravada
/// em `user_version` é local: a `0004` é a versão **1** deste store.
const MIGRATIONS_CLINICO: &[(&str, &str)] = &[(
    "0004_clinico.sql",
    include_str!("../migrations/0004_clinico.sql"),
)];

/// Migrations da Sala de treino. A `0005` é a versão **1** deste store.
const MIGRATIONS_SALA_TREINO: &[(&str, &str)] = &[(
    "0005_sala_treino.sql",
    include_str!("../migrations/0005_sala_treino.sql"),
)];

/// A conexão de um store. Privada: quem usa o app só vê `StoreAcademico`,
/// `StoreClinico` e `StoreSalaTreino`, que são tipos distintos.
struct Conexao {
    conn: Mutex<Connection>,
    nome_do_arquivo: &'static str,
    nome_do_dominio: &'static str,
}

impl Conexao {
    fn abrir(
        caminho: &Path,
        nome_do_arquivo: &'static str,
        nome_do_dominio: &'static str,
    ) -> Result<Self, DbError> {
        Self::com_connection(Connection::open(caminho)?, nome_do_arquivo, nome_do_dominio)
    }

    /// Abre em memória. Usado nos testes, para não tocar no arquivo da usuária.
    fn abrir_in_memory(nome_do_dominio: &'static str) -> Result<Self, DbError> {
        Self::com_connection(Connection::open_in_memory()?, "", nome_do_dominio)
    }

    fn com_connection(
        conn: Connection,
        nome_do_arquivo: &'static str,
        nome_do_dominio: &'static str,
    ) -> Result<Self, DbError> {
        // WAL reduz bloqueio de leitura durante escrita. Em um app mono-usuário
        // não há paralelismo a ganhar, mas evita "database is locked" se uma
        // escrita longa coexistir com uma leitura da UI.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        // `foreign_keys` é resetado a cada conexão nova; não é persistido.
        conn.pragma_update(None, "foreign_keys", "ON")?;
        Ok(Self {
            conn: Mutex::new(conn),
            nome_do_arquivo,
            nome_do_dominio,
        })
    }

    /// Conexão sob lock. Sem `&mut`, porque quem chama recebe `&Store`.
    fn conn(&self) -> Result<MutexGuard<'_, Connection>, DbError> {
        self.conn.lock().map_err(|_| {
            DbError::Domain("conexão com o banco foi envenenada por um panic anterior".into())
        })
    }

    /// Aplica as migrations pendentes, em ordem, uma transação por arquivo.
    fn migrate(&self, migrations: &[(&str, &str)]) -> Result<u32, DbError> {
        let mut conn = self.conn()?;
        let current: u32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
        for (index, (nome, sql)) in migrations.iter().enumerate() {
            // `user_version` conta migrations aplicadas, então a primeira é 1.
            let version = index as u32 + 1;
            if version <= current {
                continue;
            }
            let tx = conn.transaction()?;
            tx.execute_batch(sql)?;
            // `PRAGMA user_version` não aceita placeholder, mas o valor é um inteiro
            // derivado do índice do array — nunca entrada do usuário.
            tx.pragma_update(None, "user_version", version)?;
            tx.commit()?;
            log::info!(
                "store {}: migration {nome} aplicada (v{version})",
                self.nome_do_dominio
            );
        }
        let applied: u32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
        Ok(applied)
    }

    /// Roda `f` numa transação de escrita. Todo comando de escrita usa isto, para
    /// que materializar + ler (D10) e gravar estado + registro (INV-3) sejam
    /// atômicos.
    fn write<T>(
        &self,
        f: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T, DbError>,
    ) -> Result<T, DbError> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let out = f(&tx)?;
        tx.commit()?;
        Ok(out)
    }

    /// Nome do arquivo, para log e para o diagnóstico de erro.
    fn arquivo(&self) -> &'static str {
        self.nome_do_arquivo
    }

    /// Domínio, para a mensagem de recusa.
    fn dominio(&self) -> &'static str {
        self.nome_do_dominio
    }
}

/// Declara um store público: tipo próprio, mesma máquina.
///
/// A distinção de tipo é o ponto de `D88`, e ela se perde se alguém escrever
/// `Store = Conexao`. Por isso os três são tipos nominais e não apelidos.
macro_rules! declarar_store {
    ($tipo:ident, $dominio:literal, $arquivo:path, $migrations:ident, $doc:literal) => {
        #[doc = $doc]
        ///
        /// `#[allow(dead_code)]` nos métodos que só um store em uso chama: os
        /// stores clínico e de Sala de treino existem **antes** do primeiro comando
        /// que os usa, porque a camada clínica é `SPEC-D-008` e a Sala de treino é
        /// `SPEC-D-009`, e as duas estão condicionadas a decisões `[A]`. Schema
        /// pronto antes da decisão é o que permite **verificar** a decisão: quando
        /// a dona decidir o que entra, a forma já está imposta e falta a regra.
        ///
        /// Sem este `allow`, o `clippy` com `-D warnings` reprova a compilação por
        /// código que é futuro declarado, e o efeito é o pior possível: alguém
        /// "conserta" o aviso apagando o store.
        #[allow(dead_code)]
        pub struct $tipo {
            inner: Conexao,
        }

        #[allow(dead_code)]
        impl $tipo {
            /// Abre (ou cria) o store em `dir` e sobe o schema até a última
            /// migration. Idempotente: chamar duas vezes não muda nada.
            pub fn open_in(dir: &Path) -> Result<Self, DbError> {
                fs::create_dir_all(dir)?;
                Self::open_file(&dir.join($arquivo))
            }

            /// Abre o arquivo direto. Usado nos testes, para não tocar no disco.
            #[allow(dead_code)]
            pub fn open_in_memory() -> Result<Self, DbError> {
                Self::from_connection(Conexao::abrir_in_memory($dominio)?)
            }

            fn open_file(caminho: &Path) -> Result<Self, DbError> {
                Self::from_connection(Conexao::abrir(caminho, $arquivo, $dominio)?)
            }

            fn from_connection(inner: Conexao) -> Result<Self, DbError> {
                inner.migrate($migrations)?;
                Ok(Self { inner })
            }

            /// Versão do schema deste store, contada por `user_version`.
            pub fn schema_version(&self) -> Result<u32, DbError> {
                self.inner.migrate($migrations)
            }

            /// Conexão sob lock. `pub` porque `commands.rs` lê e escreve por aqui.
            pub fn conn(&self) -> Result<MutexGuard<'_, Connection>, DbError> {
                self.inner.conn()
            }

            /// Transação de escrita. Todo comando de escrita passa por aqui.
            pub fn write<T>(
                &self,
                f: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T, DbError>,
            ) -> Result<T, DbError> {
                self.inner.write(f)
            }

            /// Nome do arquivo deste store.
            pub fn arquivo(&self) -> &'static str {
                self.inner.arquivo()
            }

            /// Nome do domínio, para mensagens de recusa.
            pub fn dominio(&self) -> &'static str {
                self.inner.dominio()
            }
        }
    };
}

declarar_store!(
    StoreAcademico,
    "acadêmico",
    DB_ACADEMICO,
    MIGRATIONS_ACADEMICO,
    "Store do Workspace Acadêmico. Único store que os módulos de produto openingem."
);
declarar_store!(
    StoreClinico,
    "clínico do Estágio",
    DB_CLINICO,
    MIGRATIONS_CLINICO,
    "D88 — camada clínica do Estágio. Arquivo separado, e §1.6 linha 23 proíbe \
     compartilhar tabela, `id` ou vínculo com qualquer outro domínio."
);
declarar_store!(
    StoreSalaTreino,
    "Sala de treino",
    DB_SALA_TREINO,
    MIGRATIONS_SALA_TREINO,
    "D88 — Sala de treino. Personagem simulado nunca compartilha `id` com paciente \
     real nem de estágio (§4.9 linha 498)."
);

/// Erro do backend. `Serialize` porque a ponte IPC devolve a mensagem para o
/// TypeScript; o `Display` já é legível para a usuária final.
#[derive(Debug)]
pub enum DbError {
    Sqlite(rusqlite::Error),
    Io(std::io::Error),
    Domain(String),
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sqlite(e) => write!(f, "erro de banco: {e}"),
            Self::Io(e) => write!(f, "erro de disco: {e}"),
            Self::Domain(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for DbError {}

impl serde::Serialize for DbError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl From<rusqlite::Error> for DbError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sqlite(e)
    }
}

impl From<std::io::Error> for DbError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tabelas que o domínio acadêmico tem de ter. Declaradas **por nome** de
    /// propósito: um `assert_eq!(tabelas, 12)` diz que a contagem mudou, e não
    /// diz o quê. Com nome, o erro da migration é legível.
    const TABELAS_ACADEMICO: &[&str] = &[
        "bloco_planejamento", /* grafado plan_block no schema */
        "calendar_event",
        "configuracao",
        "event_occurrence",
        "event_recurrence",
        "execution_record",
        "external_link",
        "layer",
        "plan_block",
        "profile",
        "responsibility",
        "responsibility_step",
        "suggestion",
        "sync_cursor",
        "workspace_preferencia",
    ];

    fn tabelas(store: &StoreAcademico) -> Vec<String> {
        let conn = store.conn().expect("conexão");
        let mut stmt = conn
            .prepare(
                "SELECT name FROM sqlite_master \
                 WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
            )
            .expect("prepare");
        let nomes = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .expect("query")
            .filter_map(Result::ok)
            .collect::<Vec<_>>();
        // `bloco_planejamento` existe só para a lista acima ficar legível; não é
        // uma tabela real e por isso é filtrado aqui.
        nomes
    }

    #[test]
    fn dominio_academico_tem_as_tabelas_declaradas() {
        let store = StoreAcademico::open_in_memory().expect("abre em memória");
        let obtidas = tabelas(&store);
        for esperada in TABELAS_ACADEMICO {
            // `plan_block` e `bloco_planejamento` são o mesmo nome; o alias acima
            // existe só para o olho achar na lista.
            let nome = if *esperada == "bloco_planejamento" {
                "plan_block"
            } else {
                esperada
            };
            assert!(
                obtidas.iter().any(|t| t == nome),
                "falta a tabela {nome}; obtidas: {obtidas:?}"
            );
        }
    }

    #[test]
    fn d87_a_setting_generica_saiu_e_a_configuracao_ficou() {
        let store = StoreAcademico::open_in_memory().expect("abre");
        let obtidas = tabelas(&store);
        assert!(
            !obtidas.iter().any(|t| t == "setting"),
            "a setting genérica de 0001 tinha de ser aposentada por D87: {obtidas:?}"
        );
        assert!(obtidas.iter().any(|t| t == "configuracao"));
        assert!(obtidas.iter().any(|t| t == "profile"));
        assert!(obtidas.iter().any(|t| t == "workspace_preferencia"));
    }

    #[test]
    fn d87_dados_da_setting_migraram_para_a_configuracao() {
        // Reabre um store já na versão antiga, com uma linha em `setting`, e prova
        // que a `0003` levou o valor junto. Sem este teste, a `DROP TABLE setting`
        // perderia a preferência de quem já usava o app.
        let dir = std::env::temp_dir().join("campus-db-migracao-usuario");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("cria diretório");

        {
            // Versão 1 do schema: só a `0001`, com a `setting` no formato antigo.
            let store = StoreAcademico {
                inner: Conexao::abrir(&dir.join(DB_ACADEMICO), DB_ACADEMICO, "acadêmico")
                    .expect("abre"),
            };
            store.inner.migrate(&MIGRATIONS_ACADEMICO[..1]).expect("v1");
            store
                .conn()
                .expect("conn")
                .execute(
                    "INSERT INTO setting (key, value_json) VALUES ('retention.fsrs', '0.9')",
                    [],
                )
                .expect("insere preferência antiga");
        }

        let store = StoreAcademico::open_in(&dir).expect("reabre e migra");
        // A chave **mudou de nome**: `retention.fsrs` virou
        // `fsrs.retencao_desejada`. O mapeamento é explícito na `0003` porque chave
        // livre é o que a migration está aposentando — e é por isso que a coluna
        // `chave` tem CHECK com a lista fechada.
        let valor: Option<String> = store
            .conn()
            .expect("conn")
            .query_row(
                "SELECT valor_json FROM configuracao WHERE chave = 'fsrs.retencao_desejada'",
                [],
                |r| r.get(0),
            )
            .ok();
        assert_eq!(
            valor.as_deref(),
            Some("0.9"),
            "a preferência de quem já usava o app não pode sumir na migração"
        );

        let chaves: i64 = store
            .conn()
            .expect("conn")
            .query_row("SELECT COUNT(*) FROM configuracao", [], |r| r.get(0))
            .expect("conta");
        assert_eq!(chaves, 1, "a 0003 leva o que existe e não inventa chave");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn d88_sao_tres_arquivos_e_tres_tipos() {
        let dir = std::env::temp_dir().join("campus-db-tres-stores");
        let _ = fs::remove_dir_all(&dir);
        {
            let a = StoreAcademico::open_in(&dir).expect("acadêmico");
            let c = StoreClinico::open_in(&dir).expect("clínico");
            let s = StoreSalaTreino::open_in(&dir).expect("sala de treino");
            assert_eq!(a.arquivo(), DB_ACADEMICO);
            assert_eq!(c.arquivo(), DB_CLINICO);
            assert_eq!(s.arquivo(), DB_SALA_TREINO);
            assert_ne!(a.arquivo(), c.arquivo());
            assert_ne!(a.arquivo(), s.arquivo());
            assert_ne!(c.arquivo(), s.arquivo());
        }
        for nome in [DB_ACADEMICO, DB_CLINICO, DB_SALA_TREINO] {
            assert!(
                dir.join(nome).exists(),
                "{nome} tem de existir em disco depois de abrir o store"
            );
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn d88_a_camada_clinica_nao_vive_no_store_academico() {
        // A prova de que a separação é **física**: o nome da tabela clínica não
        // existe no arquivo do acadêmico, e vice-versa.
        let academico = StoreAcademico::open_in_memory().expect("acadêmico");
        let clinico = StoreClinico::open_in_memory().expect("clínico");

        let no_academico: i64 = academico
            .conn()
            .expect("conn")
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'atendimento'",
                [],
                |r| r.get(0),
            )
            .expect("conta");
        assert_eq!(no_academico, 0, "atendimento não pode estar no acadêmico");

        let no_clinico: i64 = clinico
            .conn()
            .expect("conn")
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'atendimento'",
                [],
                |r| r.get(0),
            )
            .expect("conta");
        assert_eq!(no_clinico, 1, "atendimento tem de estar no store clínico");

        let eventos_no_clinico: i64 = clinico
            .conn()
            .expect("conn")
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'calendar_event'",
                [],
                |r| r.get(0),
            )
            .expect("conta");
        assert_eq!(
            eventos_no_clinico, 0,
            "o store clínico não tem dado do acadêmico"
        );
    }

    #[test]
    fn d88_store_clinico_nao_tem_coluna_de_documento_idade_ou_nome() {
        // §4.8 linha 466 é `[D]`: paciente de estágio é identificado por código ou
        // iniciais, nunca por documento, idade ou dado sensível. Isso é schema, e
        // schema é verificável por catálogo.
        let clinico = StoreClinico::open_in_memory().expect("clínico");
        let conn = clinico.conn().expect("conn");
        let mut stmt = conn
            .prepare("SELECT name FROM pragma_table_info('atendimento')")
            .expect("prepare");
        let colunas = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .expect("query")
            .filter_map(Result::ok)
            .collect::<Vec<_>>();

        for proibida in [
            "documento",
            "cpf",
            "idade",
            "nascimento",
            "nome_completo",
            "telefone",
        ] {
            assert!(
                !colunas.iter().any(|c| c == proibida),
                "coluna sensível {proibida} não pode existir na camada clínica: {colunas:?}"
            );
        }
        assert!(
            colunas.iter().any(|c| c == "iniciais"),
            "§4.8 linha 466 exige iniciais"
        );
    }

    #[test]
    fn d88_personagem_simulado_nao_compartilha_id_com_paciente() {
        let sala = StoreSalaTreino::open_in_memory().expect("sala");
        let clinico = StoreClinico::open_in_memory().expect("clínico");
        let conexao_sala = sala.conn().expect("conn");
        let conexao_clinico = clinico.conn().expect("conn");

        conexao_sala
            .execute(
                "INSERT INTO personagem (id, nome, tema, criado_em, atualizado_em)
                 VALUES ('shared-1', 'Simulado', 'luto', 0, 0)",
                [],
            )
            .expect("insere personagem");
        conexao_clinico
            .execute(
                "INSERT INTO atendimento (id, iniciais, data, duracao_min, corpo_cifrado, estado, criado_em, atualizado_em)
                 VALUES ('shared-1', 'M.', 1700000000000, 50, x'00', 'encerrado', 0, 0)",
                [],
            )
            .expect("insere atendimento");

        // O mesmo id nos dois stores não é colisão: são arquivos diferentes. O que
        // não pode existir é FK, e não existe porque as tabelas nem se conhecem.
        let fks_sala: i64 = conexao_sala
            .query_row(
                "SELECT COUNT(*) FROM pragma_foreign_key_list('sessao_simulada')",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        assert_eq!(fks_sala, 0, "§4.9 linha 498: sem vínculo com paciente");
        let fks_clinico: i64 = conexao_clinico
            .query_row(
                "SELECT COUNT(*) FROM pragma_foreign_key_list('atendimento')",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        assert_eq!(fks_clinico, 0);
    }

    #[test]
    fn cada_store_conta_versao_do_seu_schema() {
        assert_eq!(
            StoreAcademico::open_in_memory()
                .expect("a")
                .schema_version()
                .expect("v"),
            MIGRATIONS_ACADEMICO.len() as u32
        );
        assert_eq!(
            StoreClinico::open_in_memory()
                .expect("c")
                .schema_version()
                .expect("v"),
            MIGRATIONS_CLINICO.len() as u32
        );
        assert_eq!(
            StoreSalaTreino::open_in_memory()
                .expect("s")
                .schema_version()
                .expect("v"),
            MIGRATIONS_SALA_TREINO.len() as u32
        );
    }

    #[test]
    fn reabrir_nao_reaplica_migration_nem_duplica_seed() {
        let dir = std::env::temp_dir().join("campus-db-idempotente");
        let _ = fs::remove_dir_all(&dir);
        {
            let first = StoreAcademico::open_in(&dir).expect("primeira abertura");
            assert_eq!(
                first.schema_version().expect("v"),
                MIGRATIONS_ACADEMICO.len() as u32
            );
        }
        let second = StoreAcademico::open_in(&dir).expect("segunda abertura");
        assert_eq!(
            second.schema_version().expect("v"),
            MIGRATIONS_ACADEMICO.len() as u32
        );
        let layers: i64 = second
            .conn()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM layer", [], |r| r.get(0))
            .expect("camadas");
        assert_eq!(layers, 6, "D4: 6 camadas, sem duplicar");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn foreign_keys_esta_ativo() {
        let store = StoreAcademico::open_in_memory().expect("abre");
        let on: i64 = store
            .conn()
            .unwrap()
            .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
            .expect("pragma");
        assert_eq!(on, 1, "ON DELETE CASCADE depende de foreign_keys = ON");
    }

    #[test]
    fn transaction_desfaz_tudo_em_caso_de_erro() {
        let store = StoreAcademico::open_in_memory().expect("abre");
        let now = 1_757_000_000_000i64;
        let resultado: Result<(), DbError> = store.write(|tx| {
            tx.execute(
                "INSERT INTO calendar_event
                   (id, layer_id, title, starts_at, ends_at, all_day, plain_date,
                    time_zone, commitment, state, origin, created_at, updated_at)
                 VALUES ('evt-rollback', 'faculdade', 'Teste', ?1, ?2, 0, NULL,
                         'UTC', 'opcional', 'planejado', 'cecistudy', ?3, ?3)",
                rusqlite::params![now, now + 3_600_000, now],
            )?;
            // Viola o CHECK ends_at >= starts_at (migration 0001, linha 45) depois do
            // primeiro insert. Precisa ser `ends_at` **menor** que `starts_at`:
            // com os dois iguais o `>=` é satisfeito e nada falha — a versão
            // anterior deste teste passava `?1, ?1` e por isso nunca exercitou
            // o rollback que ele existe para provar.
            tx.execute(
                "INSERT INTO calendar_event
                   (id, layer_id, title, starts_at, ends_at, all_day, plain_date,
                    time_zone, commitment, state, origin, created_at, updated_at)
                 VALUES ('evt-ruim', 'faculdade', 'Ruim', ?1, ?2, 0, NULL,
                         'UTC', 'opcional', 'planejado', 'cecistudy', ?1, ?1)",
                rusqlite::params![now, now - 3_600_000],
            )?;
            Ok(())
        });
        assert!(resultado.is_err(), "a segunda escrita viola o CHECK");
        let restantes: i64 = store
            .conn()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM calendar_event WHERE id = 'evt-rollback'",
                [],
                |r| r.get(0),
            )
            .expect("conta");
        assert_eq!(restantes, 0, "a transação inteira foi desfeita");
    }
}
