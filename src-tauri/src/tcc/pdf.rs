//! O render de PDF do documento (SPEC-D-011, decisão do app `D95`; `I3`).
//!
//! O PDF **não tem motor de layout próprio**. Ele consome a
//! [`Composicao`](super::paginacao::Composicao) que o editor já usa: mesma
//! geometria, mesmo medidor, mesma quebra de linha. É o que torna `I3`
//! verificável — se o PDF tivesse medição própria, o número de páginas dependeria
//! de duas medidas diferentes, e a paridade seria ficção.
//!
//! **O que é declarado, não escondido.** O PDF usa as fontes base do formato
//! (Helvetica), que existem em qualquer leitor. A fonte da usuária não é
//! embutida: sem medição da fonte real, `MedidorDeterministico` é o medidor
//! declarado, e a interface mostra isso (`D86`). Caractere fora de WinAnsi vira
//! `?` e a troca é **contada** no [`Relatorio`], nunca silenciosa.
//!
//! O Writer é mínimo de propósito: um PDF de texto, sem imagem, sem tabela e sem
//! fonte embutida. É o suficiente para o par de `I3` e é o que a v1 promete.

use std::fmt::Write as _;

use chrono::{DateTime, Utc};

use super::documento::{MarcaInline, NoDocumento};
use super::medidor::MedidorDeTexto;
use super::paginacao::{Geometria, mm_para_pt};

/// O que o render perdeu e o que ele mediu. Sai com o arquivo, para a interface
/// poder declarar em vez de fingir fidelidade.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Relatorio {
  /// Caracteres que WinAnsi não tem e que saíram como `?`.
  pub caracteres_substituidos: u32,
  /// A fonte real do modelo. `None` = o medidor determinístico, que é o que a
  /// paginação usou (`D86`).
  pub fonte_real: Option<String>,
  /// Medidor que produziu a composição.
  pub medidor: &'static str,
}

/// Erro do render. Só o formato é erro; conteúdo é dealtado pelo modelo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PdfErro {
  /// A composição veio sem nenhuma linha de texto em página alguma, e o
  /// documento não tem nem uma página. Não acontece com [`compor`](super::paginacao::compor),
  /// que sempre devolve uma página.
  SemPaginas,
}

impl std::fmt::Display for PdfErro {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      PdfErro::SemPaginas => write!(f, "a composição não tem página para escrever"),
    }
  }
}

impl std::error::Error for PdfErro {}

/// Renderiza o documento em PDF a partir da composição.
///
/// `medidor` entra para o relatório: o render não mede, quem mediu foi a
/// composição, e é isso que a interface precisa declarar.
pub fn escrever(
  arvore: &[NoDocumento],
  geometria: &Geometria,
  medidor: &dyn MedidorDeTexto,
  titulo: &str,
  agora: DateTime<Utc>,
) -> Result<(Vec<u8>, Relatorio), PdfErro> {
  let composicao = super::paginacao::compor(arvore, geometria, medidor);
  if composicao.paginas.is_empty() {
    return Err(PdfErro::SemPaginas);
  }

  let largura_pt = mm_para_pt(geometria.largura_mm);
  let altura_pt = mm_para_pt(geometria.altura_mm);
  let margem_esq_pt = mm_para_pt(geometria.margem_esq_mm);
  let margem_sup_pt = mm_para_pt(geometria.margem_sup_mm);

  let mut substituidos = 0u32;
  let mut corpos: Vec<String> = Vec::with_capacity(composicao.paginas.len());
  for pagina in &composicao.paginas {
    let mut fluxo = String::new();
    for linha in &pagina.linhas {
      if linha.texto.is_empty() {
        continue;
      }
      // A linha de base fica no terço inferior da caixa da linha: sem fonte
      // embutida não há métricas reais, e a aproximação é declarada.
      let base = altura_pt - margem_sup_pt - mm_para_pt(linha.y_mm + linha.altura_linha_mm * 0.8);
      let (texto, trocas) = para_winansi(&linha.texto);
      substituidos += trocas;
      let _ = writeln!(
        fluxo,
        "BT /{} {} Tf 1 0 0 1 {:.2} {:.2} Tm ({texto}) Tj ET",
        fonte_de(&linha.marcas, linha.titulo),
        f2(linha.fonte_pt),
        margem_esq_pt,
        base
      );
    }
    corpos.push(fluxo);
  }

  let bytes = montar(&corpos, titulo, agora, largura_pt, altura_pt);
  Ok((
    bytes,
    Relatorio {
      caracteres_substituidos: substituidos,
      fonte_real: None,
      medidor: "deterministico",
    },
  ))
}

/// A fonte base que casa com as marcas da linha.
///
/// São as três do formato PDF (normal, negrito, itálico) porque elas existem sem
/// ser embutidas; itálico+negrito cai no negrito, e isso é o limite declarado da
/// v1 — o modelo tem cinco marcas e o PDF base tem três fontes.
fn fonte_de(marcas: &[MarcaInline], titulo: bool) -> &'static str {
  let negrito = titulo || marcas.contains(&MarcaInline::Negrito);
  let italico = marcas.contains(&MarcaInline::Italico);
  match (negrito, italico) {
    (true, _) => "F2",
    (false, true) => "F3",
    (false, false) => "F1",
  }
}

fn f2(valor: f64) -> String {
  format!("{valor:.2}")
}

/// Converte o texto para WinAnsi (latin-1 com os caracteres de_control do PDF).
///
/// O que WinAnsi não tem vira `?` e é contado: perder em silêncio é o que a
/// cláusula de tolerância proíbe.
fn para_winansi(texto: &str) -> (String, u32) {
  let mut saida = String::with_capacity(texto.len());
  let mut trocas = 0u32;
  for caractere in texto.chars() {
    let codigo = caractere as u32;
    if codigo < 0x20 || codigo == 0x7f {
      trocas += 1;
      saida.push('?');
      continue;
    }
    if codigo > 0xff {
      trocas += 1;
      saida.push('?');
      continue;
    }
    saida.push(char::from_u32(codigo).unwrap_or('?'));
  }
  (escapar_pdf(&saida), trocas)
}

/// Escapa o texto dentro de uma string literal PDF: `\`, `(` e `)`.
fn escapar_pdf(texto: &str) -> String {
  let mut saida = String::with_capacity(texto.len());
  for caractere in texto.chars() {
    match caractere {
      '\\' => saida.push_str("\\\\"),
      '(' => saida.push_str("\\("),
      ')' => saida.push_str("\\)"),
      c => saida.push(c),
    }
  }
  saida
}

/// Monta o arquivo: objetos numerados, `xref` com os deslocamentos reais e
/// `trailer` apontando para o catálogo.
///
/// Um PDF sem `xref` correto abre em leitor tolerante e falha em leitor estrito,
/// que é a forma mais comum de "o PDF abre mas dá erro" — então o deslocamento é
/// medido, não estimado.
fn montar(
  corpos: &[String],
  titulo: &str,
  agora: DateTime<Utc>,
  largura: f64,
  altura: f64,
) -> Vec<u8> {
  // 1 catálogo, 2 páginas, 3..5 fontes, 6 info; depois, por página, o objeto de
  // página e o de conteúdo.
  const FONTES: usize = 3;
  let mut objetos: Vec<String> = Vec::new();
  objetos.push(String::new()); // 1: catálogo, preenchido no fim
  objetos.push(String::new()); // 2: páginas, preenchido no fim
  objetos.push(
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_string(),
  );
  objetos.push(
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>"
      .to_string(),
  );
  objetos.push(
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Oblique /Encoding /WinAnsiEncoding >>"
      .to_string(),
  );
  objetos.push(format!(
    "<< /Title ({}) /Producer (cecistudy) /Creator (cecistudy) /CreationDate (D:{}) >>",
    escapar_pdf(titulo),
    data_pdf(agora),
  ));

  let primeira_pagina = objetos.len() + 1;
  let mut filhos = String::new();
  for (i, corpo) in corpos.iter().enumerate() {
    let id_pagina = primeira_pagina + i * 2;
    let id_conteudo = id_pagina + 1;
    if !filhos.is_empty() {
      filhos.push(' ');
    }
    let _ = write!(filhos, "{id_pagina} 0 R");
    objetos.push(format!(
      "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.2} {:.2}] /Resources << /Font << /F1 3 0 R /F2 4 0 R /F3 5 0 R >> >> /Contents {id_conteudo} 0 R >>",
      largura, altura
    ));
    objetos.push(format!(
      "<< /Length {} >>\nstream\n{corpo}\nendstream",
      corpo.len()
    ));
  }
  let _ = FONTES;

  objetos[0] = "<< /Type /Catalog /Pages 2 0 R >>".to_string();
  objetos[1] = format!(
    "<< /Type /Pages /Kids [{filhos}] /Count {} >>",
    corpos.len()
  );

  let mut saida: Vec<u8> = Vec::new();
  saida.extend_from_slice(b"%PDF-1.4\n% cecistudy\n");
  let mut deslocamentos: Vec<usize> = Vec::with_capacity(objetos.len());
  for (i, corpo) in objetos.iter().enumerate() {
    deslocamentos.push(saida.len());
    saida.extend_from_slice(format!("{} 0 obj\n{corpo}\nendobj\n", i + 1).as_bytes());
  }

  let inicio_xref = saida.len();
  saida.extend_from_slice(format!("xref\n0 {}\n", objetos.len() + 1).as_bytes());
  saida.extend_from_slice(b"0000000000 65535 f \n");
  for deslocamento in &deslocamentos {
    saida.extend_from_slice(format!("{deslocamento:010} 00000 n \n").as_bytes());
  }
  saida.extend_from_slice(
    format!(
      "trailer\n<< /Size {} /Root 1 0 R /Info {} 0 R >>\nstartxref\n{}\n%%EOF\n",
      objetos.len() + 1,
      objetos.len() - corpos.len() * 2,
      inicio_xref
    )
    .as_bytes(),
  );
  saida
}

/// A data do PDF: `D:AAAAMMDDHHmmSS+00'00'`.
fn data_pdf(agora: DateTime<Utc>) -> String {
  let f = |v: i64| format!("{v:02}");
  format!(
    "D:{}{}{}{}{}{}+00'00'",
    agora.format("%Y"),
    agora.format("%m"),
    agora.format("%d"),
    agora.format("%H"),
    agora.format("%M"),
    agora.format("%S")
  )
  .replace("  ", &f(0))
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::tcc::documento::{Inline, ModoVinculo};
  use crate::tcc::medidor::MedidorDeterministico;

  fn geometria() -> Geometria {
    Geometria::a4_abnt()
  }

  fn arvore_de_teste() -> Vec<NoDocumento> {
    vec![
      NoDocumento::Capitulo {
        id: "c1".to_string(),
        titulo: "Introdução".to_string(),
        filhos: vec![NoDocumento::Paragrafo {
          id: "p1".to_string(),
          inline: vec![Inline::simples("A pesquisa parte da leitura.")],
        }],
      },
      NoDocumento::Citacao {
        id: "cit".to_string(),
        referencia_id: "ref-42".to_string(),
        modo: ModoVinculo::Vivo,
      },
    ]
  }

  fn render(arvore: &[NoDocumento]) -> Vec<u8> {
    escrever(
      arvore,
      &geometria(),
      &MedidorDeterministico::novo(),
      "TCC",
      DateTime::from_timestamp(1_800_000_000, 0).unwrap(),
    )
    .expect("renderiza")
    .0
  }

  /// O texto que o PDF carrega, lido do arquivo gerado.
  ///
  /// Lê o arquivo de verdade, como um leitor leria: é o que torna `I3` um teste
  /// entre dois escritores independentes e não uma comparação de função com ela
  /// mesma.
  fn texto_do_pdf(bytes: &[u8]) -> Vec<String> {
    let bruto = String::from_utf8_lossy(bytes);
    bruto
      .split(") Tj")
      .filter_map(|fatia| {
        let inicio = fatia.rfind('(')?;
        Some(fatia[inicio + 1..].to_string())
      })
      .collect()
  }

  #[test]
  fn o_pdf_tem_cabecalho_e_estrutura() {
    let bytes = render(&arvore_de_teste());
    assert!(bytes.starts_with(b"%PDF-1.4"));
    assert!(String::from_utf8_lossy(&bytes).contains("%%EOF"));
    assert!(String::from_utf8_lossy(&bytes).contains("/Type /Catalog"));
  }

  #[test]
  fn o_xref_aponta_para_o_inicio_de_cada_objeto() {
    let bytes = render(&arvore_de_teste());
    let texto = String::from_utf8_lossy(&bytes).to_string();
    let inicio_xref: usize = texto
      .find("startxref")
      .and_then(|i| texto[i + "startxref".len()..].trim().lines().next())
      .and_then(|n| n.trim().parse().ok())
      .expect("startxref numérico");
    assert!(texto[inicio_xref..].starts_with("xref"));

    // xref, "0 N" e a entrada livre: as entradas de objeto começam na quarta.
    // Cada entrada tem espaço antes do `\n` — é o formato de 20 bytes do xref —
    // então o `lines()` deixa o espaço no fim.
    let mut entradas = texto[inicio_xref..].lines().skip(3);
    for numero in 1..=6usize {
      let linha = entradas.next().expect("entrada do xref");
      assert!(linha.ends_with(" n "), "entrada {linha} não é de objeto");
      let deslocamento: usize = linha[..10].trim().parse().expect("deslocamento numérico");
      assert!(
        texto[deslocamento..].starts_with(&format!("{numero} 0 obj")),
        "o xref do objeto {numero} aponta para o lugar errado"
      );
    }
  }

  #[test]
  fn o_texto_do_pdf_e_a_sequencia_do_modelo() {
    let bytes = render(&arvore_de_teste());
    let linhas = texto_do_pdf(&bytes);
    assert_eq!(
      linhas,
      vec![
        "Introdução",
        "A pesquisa parte da leitura.",
        "[citação: ref-42 · vivo]"
      ]
    );
  }

  #[test]
  fn parentese_e_barra_sao_escapados() {
    let arvore = vec![NoDocumento::Paragrafo {
      id: "p".to_string(),
      inline: vec![Inline::simples("a (b) c \\ d")],
    }];
    let bytes = render(&arvore);
    let texto = String::from_utf8_lossy(&bytes).to_string();
    assert!(
      texto.contains("a \\(b\\) c \\\\ d"),
      "o PDF não escapa: {texto}"
    );
  }

  #[test]
  fn caractere_fora_de_winansi_e_contado() {
    let arvore = vec![NoDocumento::Paragrafo {
      id: "p".to_string(),
      inline: vec![Inline::simples("ok 中文")],
    }];
    let (_, relatorio) = escrever(
      &arvore,
      &geometria(),
      &MedidorDeterministico::novo(),
      "TCC",
      DateTime::from_timestamp(1_800_000_000, 0).unwrap(),
    )
    .expect("renderiza");
    assert_eq!(relatorio.caracteres_substituidos, 2);
    assert_eq!(relatorio.medidor, "deterministico");
  }

  #[test]
  fn acento_e_acentuado_preservado() {
    let arvore = vec![NoDocumento::Paragrafo {
      id: "p".to_string(),
      inline: vec![Inline::simples("Introdução à leitura")],
    }];
    let bytes = render(&arvore);
    assert_eq!(texto_do_pdf(&bytes), vec!["Introdução à leitura"]);
  }

  #[test]
  fn o_numero_de_paginas_do_pdf_e_o_da_composicao() {
    let arvore: Vec<NoDocumento> = (0..60)
      .map(|i| NoDocumento::Paragrafo {
        id: format!("p{i}"),
        inline: vec![Inline::simples("palavra ".repeat(20))],
      })
      .collect();
    let composicao =
      crate::tcc::paginacao::compor(&arvore, &geometria(), &MedidorDeterministico::novo());
    let bytes = render(&arvore);
    let texto = String::from_utf8_lossy(&bytes);
    assert_eq!(
      texto.matches("/Type /Page\n").count() + texto.matches("/Type /Page ").count(),
      composicao.paginas.len()
    );
  }
}
