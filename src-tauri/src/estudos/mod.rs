//! Estudos (SPEC-D-010, D76): o estado de revisão espaçada por cartão.
//!
//! O motor é a crate `fsrs` (ver `Cargo.toml` para versão e licença); o que é
//! **nosso** mora em `fsrs.rs`: a forma do estado, a regra de relearning, o
//! vencimento por dia com âncora de fuso e o relógio injetado. Mesma fronteira
//! de D50 (SPEC-D-002): a crate decide o número, o app decide a forma.
//!
//! Este módulo não grava. Quem grava é o caminho de escrita, em `commands.rs`;
//! aqui a função é pura — recebe estado e instante, devolve estado.
//!
//! O que está **fora** daqui, e por quê (§4.4 do referencial):
//! - a retenção desejada é **parâmetro**, e o padrão dela é **[P]** — a dona
//!   do produto decide, e nenhum default é inventado aqui;
//! - o otimizador da crate não é chamado ([P]);
//! - Conteúdos, Trilhas, materiais e sessões são a tela (F8), não o motor.

pub mod fsrs;
