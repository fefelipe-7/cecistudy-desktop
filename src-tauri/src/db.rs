//! Conexão SQLite e migrações versionadas do Calendário (spec 01, Fase 1.4).
//!
//! Decisões que moram aqui:
//!
//! - **A2** — SQLite embarcado, arquivo único em `app_data_dir`.
//! - **D10** — `event_occurrence` é tabela real, populada sob demanda.
//! - **Versionamento** — `PRAGMA user_version` é a única fonte de qual migration
//!   já rodou. Não existe tabela de histórico de migrations: o PRAGMA faz o
//!   papel e evita uma tabela extra que a spec §3.3 não pede.
//! - **Transação** — cada migration roda dentro de uma transação própria. Uma
//!   migration que falha no meio não deixa schema pela metade.
//!
//! `foreign_keys` fica `ON` de propósito: sem isso o SQLite ignora
//! `ON DELETE CASCADE` e o D10 (materialização) e o D12 (histórico) divergem.
//!
//! A conexão fica atrás de um `Mutex` porque `rusqlite` exige `&mut Connection`
//! para abrir transação, e o Tauri entrega `State<Db>`, que é `&Db`. Com o
//! `Mutex`, os comandos recebem `&Db` e mesmo assim abrem transação — e isso
//! também permite testar a lógica sem subir um app.

use std::fs;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;

/// Nome do arquivo de banco. Estável entre versões: é o mesmo arquivo que a
/// usuária já tem depois de uma atualização.
pub const DB_FILE_NAME: &str = "campus.sqlite";

/// Migrations embutidas no binário.
///
/// `include_str!` resolve em tempo de compilação: o `.sql` viaja dentro do
/// executável, então o app não depende de ler um diretório `migrations/` que
/// pode não existir no Windows depois de um empacotamento.
const MIGRATIONS: &[(&str, &str)] = &[
    ("0001_init.sql", include_str!("../migrations/0001_init.sql")),
    (
        "0002_seed_layers.sql",
        include_str!("../migrations/0002_seed_layers.sql"),
    ),
];

/// Estado gerenciado pelo Tauri e compartilhado por todos os comandos.
pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    /// Abre (ou cria) o banco em `dir` e sobe o schema até a última migration.
    /// Idempotente: chamar duas vezes não muda nada.
    pub fn open_in(dir: &Path) -> Result<Self, DbError> {
        fs::create_dir_all(dir)?;
        Self::from_connection(Connection::open(dir.join(DB_FILE_NAME))?)
    }

    /// Abre em memória. Usado nos testes, para não tocar no banco real.
    ///
    /// `#[allow(dead_code)]` porque é a costura de teste: nada na lib de
    /// produção a chama, mas `mod tests` depende dela — sem o atributo o build
    /// normal dispara `dead_code` e o clippy roda com `-D warnings`.
    #[allow(dead_code)]
    pub fn open_in_memory() -> Result<Self, DbError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(conn: Connection) -> Result<Self, DbError> {
        // WAL reduz bloqueio de leitura durante escrita. Em um app mono-usuário
        // não há paralelismo a ganhar, mas evita "database is locked" se uma
        // escrita longa coexistir com uma leitura da UI.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        // `foreign_keys` é resetado a cada conexão nova; não é persistido.
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        Ok(db)
    }

    /// Conexão sob lock. Sem `&mut`, porque quem chama é `&Db`.
    pub fn conn(&self) -> Result<MutexGuard<'_, Connection>, DbError> {
        self.conn.lock().map_err(|_| {
            DbError::Domain("conexão com o banco foi envenenada por um panic anterior".into())
        })
    }

    /// Aplica as migrations pendentes, em ordem, uma transação por arquivo.
    pub fn migrate(&self) -> Result<u32, DbError> {
        let mut conn = self.conn()?;
        let current: u32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
        for (index, (_name, sql)) in MIGRATIONS.iter().enumerate() {
            // `user_version` conta migrations aplicadas, então a primeira é 1.
            let version = index as u32 + 1;
            if version <= current {
                continue;
            }
            let tx = conn.transaction()?;
            tx.execute_batch(sql)?;
            // `PRAGMA user_version` não aceita placeholder, mas o valor é um
            // inteiro derivado do índice do array — nunca entrada do usuário.
            tx.pragma_update(None, "user_version", version)?;
            tx.commit()?;
        }
        // Mesmo formato da leitura no topo da função: a anotação `u32` é o que
        // fixa a inferência de `row.get(0)`. Sem ela o compilador tenta
        // `Result<u32, DbError>` como tipo da coluna e não encontra `FromSql`.
        let applied: u32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
        Ok(applied)
    }

    /// Roda `f` numa transação de escrita. Todo comando de escrita usa isto, para
    /// que materializar + ler (D10) e gravar estado + registro (INV-3) sejam
    /// atômicos.
    pub fn write<T>(
        &self,
        f: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T, DbError>,
    ) -> Result<T, DbError> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let out = f(&tx)?;
        tx.commit()?;
        Ok(out)
    }
}

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

    #[test]
    fn migrate_aplica_todas_e_e_idempotente() {
        let db = Db::open_in_memory().expect("abre em memória");
        assert_eq!(db.migrate().expect("versão"), MIGRATIONS.len() as u32);

        let tables: i64 = db
            .conn()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master \
                 WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
                [],
                |r| r.get(0),
            )
            .expect("conta tabelas");
        assert_eq!(tables, 12, "spec 01 §3.3: 12 tabelas");
    }

    #[test]
    fn reabrir_nao_reaplica_migration_nem_duplica_seed() {
        let dir = std::env::temp_dir().join("campus-db-idempotente");
        let _ = fs::remove_dir_all(&dir);
        {
            let first = Db::open_in(&dir).expect("primeira abertura");
            assert_eq!(first.migrate().expect("v"), MIGRATIONS.len() as u32);
        }
        let second = Db::open_in(&dir).expect("segunda abertura");
        assert_eq!(second.migrate().expect("v"), MIGRATIONS.len() as u32);
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
        let db = Db::open_in_memory().expect("abre");
        let on: i64 = db
            .conn()
            .unwrap()
            .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
            .expect("pragma");
        assert_eq!(on, 1, "ON DELETE CASCADE depende de foreign_keys = ON");
    }

    #[test]
    fn transaction_desfaz_tudo_em_caso_de_erro() {
        let db = Db::open_in_memory().expect("abre");
        let now = 1_757_000_000_000i64;
        let resultado: Result<(), DbError> = db.write(|tx| {
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
        let restantes: i64 = db
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
