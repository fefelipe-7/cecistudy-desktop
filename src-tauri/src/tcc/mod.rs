//! TCC & Projetos (SPEC-D-013, `D85..D88`): o modelo de dados do documento.
//!
//! Este módulo é a **fonte única** do modelo do documento, ao lado da migração
//! `0005_tcc.sql`. A fronteira é a de sempre (D50/`SPEC-D-002`): o motor é puro,
//! não grava e não importa `tauri` nem `rusqlite`. Quem grava é o caminho de
//! escrita em `commands.rs`.
//!
//! - `documento` — a árvore (lista de adjacência ordenada) e o inline (D85).
//! - `medidor` — a medida de texto injetada; o motor nunca consulta a fonte
//!   instalada da máquina (D86, `I6`).
//! - `paginacao` — o motor de paginação puro com geometria parâmetro (D86),
//!   e a composição de linhas que o editor, o DOCX e o PDF compartilham (D95).
//! - `versao` — a serialização canônica do snapshot de versão (D87).
//! - `item` — o item a resolver da importação de DOCX (D88).
//! - `docx` — o round-trip DOCX: escrita e leitura, com o relatório do
//!   contrato de tolerância (D78, D95).
//! - `pdf` — o render do PDF a partir da mesma composição (D95, I3).
//!
//! O que **não** mora aqui, e por quê:
//! - A validação é do Rust, e a geometria é **parâmetro**, nunca coluna
//!   (`D85`/`D86`): mudá-la reprocessa o documento sem tocar na árvore.
//! - `w:ins`/`w:del` **não** é reimplementado por biblioteca de DOCX genérica:
//!   a cláusula de `D78` fixa que a revisão vira item `sugestao` com autor,
//!   data e âncora, e essa leitura é nossa (`D95`).

pub mod documento;
pub mod docx;
pub mod item;
pub mod medidor;
pub mod paginacao;
pub mod pdf;
pub mod versao;

pub use documento::{DocError, Inline, MarcaInline, ModoVinculo, NoDocumento, NoLinha, TipoNo};
pub use item::{EstadoItem, ItemAResolver, TipoItem};
pub use medidor::{MedidorDeTexto, MedidorDeterministico};
pub use paginacao::{
  Composicao, Geometria, LinhaComposta, PaginaComposta, Paginacao, compor, mm_para_pt, paginar,
};
