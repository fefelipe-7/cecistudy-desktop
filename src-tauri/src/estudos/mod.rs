//! Estudos: o módulo dono de conteúdo, cartão e fila de revisão
//! (SPEC-D-010, D76; SPEC-D-015 D96).
//!
//! O motor é a crate `fsrs` (ver `Cargo.toml` para versão e licença); o que é
//! **nosso** mora em [`fsrs`]: a forma do estado, a regra de relearning, o
//! vencimento por dia com âncora de fuso e o relógio injetado. Mesma fronteira
//! de D50 (SPEC-D-002): a crate decide o número, o app decide a forma.
//!
//! - [`fsrs`] — puro, sem SQL e sem Tauri.
//! - [`comandos`] — o caminho de escrita: conteúdo, cartão, fila e revisão.
//!
//! **O vencimento nunca é coluna.** Ele é derivado na leitura, a partir do
//! estado do cartão e da zona da usuária (cláusula do vencimento, D76). Toda
//! leitura da fila recalcula; nada é gravado.
//!
//! O que está **fora** daqui, e por quê (§4.4 do referencial):
//! - a retenção desejada é **parâmetro**, e o padrão dela é **[P]** — a dona do
//!   produto decide, e nenhum default é inventado aqui;
//! - o otimizador da crate não é chamado ([P]);
//! - Conteúdos, Trilhas, materiais e sessões são a tela (F8), não o motor.
//!
//! E o que é de outro dono: o conteúdo de estudo é **de Estudos**, e Faculdade
//! o mostra por visão (D65) — nunca o copia.

pub mod comandos;
pub mod fsrs;
