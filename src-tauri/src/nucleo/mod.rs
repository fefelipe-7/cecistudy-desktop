//! O núcleo do crate: o que **não** é regra de nenhum módulo.
//!
//! Duas entradas, e o critério de cada uma é o mesmo:
//!
//! - **não é regra de um módulo** — nada aqui decide estado, não calcula nota
//!   e não expande série;
//! - **é alcançado por mais de um** — os quatro módulos do produto usam.
//!
//! O que mora aqui hoje é [`db`] (a conexão, as migrations e o `DbError`) e as
//! duas funções sem dono, `new_id` e `now_ms`. Antes da SPEC-D-015 elas eram
//! helpers privados de `commands.rs` — e um arquivo só que, por acaso, as
//! deixava ao alcance dos quatro. Sem essa fronteira, elas não teriam dona.
//!
//! **O que não mora aqui:** a máquina de estados, a sobreposição e o atraso do
//! Calendário (`calendario/dominio.rs`), a média e a regra de Faculdade
//! (`faculdade/`), o FSRS (`estudos/`) e o modelo do documento (`tcc/`).
//! Nenhuma delas é infraestrutura, e todas são de um dono.
//!
//! `Db` não é dono de tabela nenhuma — a migration e a semente são bootstrap,
//! não comando de módulo. Por isso o gate trata este diretório como
//! `SEM_DONO_RUST`.

pub mod db;

pub use db::{Db, DbError, new_id, now_ms};
