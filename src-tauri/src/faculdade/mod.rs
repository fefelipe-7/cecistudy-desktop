//! Faculdade: o módulo dono de semestre, disciplina, avaliação e nota
//! (SPEC-D-010, D74/D75).
//!
//! Este é o módulo de dominio mais puro do crate, e ele é a prova de que a
//! separação de D96 funciona: o motor não sabe que banco existe.
//!
//! - [`media`] — D74: a média é **função pura com o relógio no chamador**, e ela
//!   **não guarda** média, situação nem "quanto falta". Nada disso vira coluna.
//! - [`regra`] — D75: a regra é **viva** no semestre corrente (herança do padrão
//!   global) e **congelada** no semestre passado. Encerrar semestre é um ato que
//!   **grava**, não um filtro de exibição.
//! - [`comandos`] — o caminho de escrita: o SQL, a guarda de I7 e o
//!   congelamento.
//!
//! O que não mora aqui: a camada clínica do Estágio (que o produto decidiu
//! manter no mobile, `C14`) e o conteúdo de estudo, que é de [`crate::estudos`]
//! e que Faculdade só mostra por referência viva (D65).
//!
//! A média é **derivada na leitura** e nunca persistida (I1). A regra congelada
//! é a única coisa que se grava, e é dado de auditoria: responde "por que essa
//! disciplina reprovou?" anos depois.

pub mod comandos;
pub mod media;
pub mod regra;
