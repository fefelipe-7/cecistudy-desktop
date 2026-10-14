//! Calendário: o módulo dono de agendado, obrigação e execução (SPEC-D-015 D96).
//!
//! É o primeiro dos quatro diretórios de módulo, e o modelo que os outros três
//! seguem. A regra do produto é §1.3 da spec referencial: cada dado tem um dono.
//! A regra do código é a mesma, e ela tem um endereço:
//!
//! ```text
//! calendario/
//! ├── dominio.rs   as regras puras: máquina de estados, sobreposição, atraso
//! ├── comandos.rs  o caminho de escrita: o SQL e a guarda de D67
//! │   └── testes.rs  a prova, que é do mesmo módulo porque é da mesma regra
//! └── mod.rs       este arquivo
//! ```
//!
//! Os testes ficam dentro de `comandos`, e não ao lado: eles alcançam o `X_core`
//! pela resolução de escopo de um módulo aninhado, e `X_core` não é `pub` — é
//! essa a razão de eles não ocuparem o arquivo de escrita.
//!
//! A separação entre [`dominio`] e [`comandos`] é a mesma dos outros módulos: o
//! motor é puro e não toca SQL, o caminho de escrita é o único que sabe de
//! tabela. Por isso `media_da_disciplina` deriva média sem gravar nada, e por
//! isso `exigir_transicao` consegue ler o estado atual antes de escrever.
//!
//! **O que não mora aqui:** a camada de espelho em `domain/` do TypeScript
//! (que é teste, e só teste, desde D1), o motor de Faculdade e o de Estudos.
//! E o que é do Calendário mas vive na tela — `grid-scale`, `DayColumn`,
//! `hourHeight` — que é apresentação e não entra neste crate.

pub mod comandos;
pub mod dominio;

pub use dominio::{destinations_of, is_overdue, mark_overlaps, transition_allowed};
