//! Módulo Faculdade — o motor de cálculo do §4.3.
//!
//! Aqui vivem as duas regras que a spec referencial põe no Rust e que o módulo
//! Faculdade introduz (D4.1 do [`roadmap`](../../roadmap.md), `SPEC-D-010`):
//!
//! - [`regra`] — D75: a regra de média é **viva** no semestre corrente (herança
//!   do padrão global) e **congelada** no semestre passado. "Encerrar semestre"
//!   é um ato que **grava**, não um filtro de exibição.
//! - [`media`] — D74: o motor de notas é uma **função pura com relógio no
//!   chamador**, e ele **não guarda** média, situação nem "quanto falta".
//!
//! Os dois módulos são puros: não tocam SQL, não abrem relógio, não conhecem
//! `tauri`. É o que permite testá-los sem banco e sem subir o app — a mesma
//! separação que `domain.rs` usa no Calendário.
//!
//! A média é **derivada na leitura** e nunca persistida (I1). A regra congelada
//! é a única coisa que se grava, e é dado de auditoria: responde "por que essa
//! disciplina reprovou?" anos depois.

pub mod media;
pub mod regra;
