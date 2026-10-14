//! A medida de texto injetada (SPEC-D-013, `D86`).
//!
//! A paginação precisa saber **quanto cada texto ocupa**. Essa resposta depende
//! da fonte — e a fonte depende da máquina. O motor nunca consulta a fonte
//! instalada: ele recebe um [`MedidorDeTexto`]. É o mesmo padrão do relógio
//! injetado (`Clock`, glossário): a regra fica pura e testável, e o ambiente
//! entra por parâmetro.
//!
//! Em produção, o medidor real depende de um crate de métrica de fonte — uma
//! **dependência declarada** (`D86`), não implementada aqui. Nos testes e no
//! golden, vale o [`MedidorDeterministico`], que dá o mesmo resultado em
//! qualquer máquina. É o que torna a paginação um golden estável.

use crate::tcc::documento::MarcaInline;

/// Pontos por polegada divididos por 72: converte pt (tipografia) para mm.
pub const PT_PARA_MM: f64 = 25.4 / 72.0;

/// Quem sabe medir texto. A paginação depende desta interface, nunca de um
/// medidor concreto.
pub trait MedidorDeTexto {
  /// Largura, em milímetros, de `texto` no tamanho `tamanho_pt`, com as marcas
  /// dadas. Marcos que não mudam a largura (cor, por exemplo) não entram.
  fn largura(&self, texto: &str, tamanho_pt: f64, marcas: &[MarcaInline]) -> f64;

  /// Altura de uma linha no tamanho `tamanho_pt`, em milímetros, antes da
  /// entrelinha.
  fn altura_linha(&self, tamanho_pt: f64) -> f64;
}

/// Medidor determinístico: mesma entrada, mesma saída, em qualquer máquina.
///
/// Não é o medidor de produção. É o do golden e dos testes, e existe para que o
/// número de páginas não dependa da fonte instalada — a consequência registrada
/// em `D86`. A largura aproxima cada caractere por metade do corpo.
#[derive(Debug, Clone, Copy, Default)]
pub struct MedidorDeterministico;

impl MedidorDeterministico {
  pub fn novo() -> Self {
    MedidorDeterministico
  }
}

impl MedidorDeTexto for MedidorDeterministico {
  fn largura(&self, texto: &str, tamanho_pt: f64, _marcas: &[MarcaInline]) -> f64 {
    let caracteres = texto.chars().count() as f64;
    caracteres * (tamanho_pt * 0.5) * PT_PARA_MM
  }

  fn altura_linha(&self, tamanho_pt: f64) -> f64 {
    tamanho_pt * 1.2 * PT_PARA_MM
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn medidor_deterministico_e_estavel() {
    let medidor = MedidorDeterministico::novo();
    let a = medidor.largura("Trabalho de conclusão", 12.0, &[]);
    let b = medidor.largura("Trabalho de conclusão", 12.0, &[]);
    assert_eq!(a, b);
    assert!(a > 0.0);
  }

  #[test]
  fn texto_vazio_tem_largura_zero() {
    let medidor = MedidorDeterministico::novo();
    assert_eq!(medidor.largura("", 12.0, &[]), 0.0);
  }
}
