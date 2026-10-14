//! A versão por snapshot canônico (SPEC-D-013, `D87`).
//!
//! Toda escrita confirmada gera uma versão. A versão é o **snapshot completo e
//! canônico** do documento — não um diff. Não há "v1 com diff": diff fica para
//! quando houver necessidade medida, e a decisão registra que ele **não** existe
//! agora (`D87`).
//!
//! "Canônica" quer dizer que a mesma árvore sempre serializa para a mesma
//! string: marcas em ordem fixa (`documento::normalizar`), nós na ordem da
//! árvore. Sem isso, a mesma árvore com marcas em ordem diferente viraria duas
//! versões e o autosave criaria versão a cada tecla.
//!
//! Este módulo é puro: serializa e compara. Quem grava é `commands.rs`.

use crate::tcc::documento::{DocError, NoDocumento, normalizar};

/// A serialização canônica da árvore. É o conteúdo gravado em `versao`.
///
/// Normaliza uma cópia antes de serializar — a árvore recebida não é alterada.
pub fn snapshot_canonico(arvore: &[NoDocumento]) -> Result<String, DocError> {
  let mut copia = arvore.to_vec();
  normalizar(&mut copia);
  serde_json::to_string(&copia).map_err(|e| DocError::Serializacao(e.to_string()))
}

/// Diz se duas árvores diferem, pelo critério do snapshot.
///
/// É o que o autosave consulta: escrita sem mudança **não** versiona (`D87`).
/// Como o snapshot normaliza, reordenar marcas não conta como mudança.
pub fn mudou(antes: &[NoDocumento], depois: &[NoDocumento]) -> Result<bool, DocError> {
  Ok(snapshot_canonico(antes)? != snapshot_canonico(depois)?)
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::tcc::documento::{Inline, MarcaInline};

  #[test]
  fn a_mesma_arvore_gera_o_mesmo_snapshot() {
    let a = vec![NoDocumento::Paragrafo {
      id: "p".into(),
      inline: vec![],
    }];
    let b = vec![NoDocumento::Paragrafo {
      id: "p".into(),
      inline: vec![],
    }];
    assert_eq!(
      snapshot_canonico(&a).unwrap(),
      snapshot_canonico(&b).unwrap()
    );
  }

  #[test]
  fn ordem_das_marcas_nao_cria_versao_nova() {
    let com = |marcas: Vec<MarcaInline>| {
      vec![NoDocumento::Paragrafo {
        id: "p".into(),
        inline: vec![Inline {
          texto: "x".into(),
          marcas,
        }],
      }]
    };
    let a = com(vec![MarcaInline::Italico, MarcaInline::Negrito]);
    let b = com(vec![MarcaInline::Negrito, MarcaInline::Italico]);
    assert_eq!(
      snapshot_canonico(&a).unwrap(),
      snapshot_canonico(&b).unwrap()
    );
    assert!(!mudou(&a, &b).unwrap());
  }

  #[test]
  fn conteudo_diferente_cria_versao_nova() {
    let a = vec![NoDocumento::Paragrafo {
      id: "p".into(),
      inline: vec![],
    }];
    let b = vec![
      NoDocumento::Paragrafo {
        id: "p".into(),
        inline: vec![],
      },
      NoDocumento::Paragrafo {
        id: "q".into(),
        inline: vec![],
      },
    ];
    assert_ne!(
      snapshot_canonico(&a).unwrap(),
      snapshot_canonico(&b).unwrap()
    );
    assert!(mudou(&a, &b).unwrap());
  }
}
