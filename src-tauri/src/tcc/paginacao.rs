//! O motor de paginação puro (SPEC-D-013, `D86`).
//!
//! Paginação é **cálculo puro**: entra a árvore, a [`Geometria`] e o
//! [`MedidorDeTexto`]; sai a composição — linhas, altura de cada nó e em que
//! página cada linha cai. Sem banco, sem `tauri`, sem fonte instalada — é por
//! isso que ela é testável e vira golden.
//!
//! A geometria é **parâmetro**, não coluna (`D85`): mudar margem, corpo ou
//! entrelinha reprocessa o documento sem tocar na árvore. É o que permite o
//! mesmo documento paginar em A4 ABNT e no template da instituição.
//!
//! **Fonte única do layout.** [`compor`] é a única função que decide onde cada
//! linha cai, e [`paginar`] deriva dela. Antes havia um segundo modelo — a altura
//! reservada por `ceil(largura / largura_util)` — e o render de PDF (`D95`) ia
//! precisar de um terceiro. Agora há um: quebrar o texto em linhas e distributing
//! blocos é a mesma operação para o editor, para o DOCX e para o PDF, e a altura
//! reservada de um nó é a soma das alturas das linhas que ele de fato ocupa.
//!
//! Política de viúvas e órfãs da v1 (consequência de `D86`): **um parágrafo não
//! é dividido entre páginas**. Ou cabe inteiro, ou vai inteiro para a página
//! seguinte. Um título nunca fica separado do primeiro parágrafo (`keep_with_next`).
//! Assim nenhuma página começa ou termina com uma linha sozinha de um parágrafo
//! — a garantia é estrutural, não um limiar a ajustar.
//!
//! Quebra de linha: **por palavra**, com a mesma métrica do [`MedidorDeTexto`].
//! Palavra mais larga que a coluna é fatiada em pedaços que caibam, senão o
//! parágrafo transbordaria em vez de quebrar — é o mesmo `max(1)` que `medir`
//! fazia por altura.

use crate::tcc::documento::{MarcaInline, NoDocumento};
use crate::tcc::medidor::MedidorDeTexto;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A altura das letras de título em relação ao corpo.
const FATOR_TITULO: f64 = 1.4;

/// A entrelinha padrão, quando a geometria não a fixa.
const ENTRELINHA_PADRAO: f64 = 1.5;

/// Um epsilon para comparação de milímetros. Ponto flutuante não tem igualdade.
const EPS: f64 = 1e-6;

/// A geometria da página. Tudo em milímetros; o corpo, em pontos.
///
/// A4 ABNT (margens 3/3/2/2 cm) é só o padrão. A instituição da usuária pode ter
/// outro template; por isso é dado, não constante.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Geometria {
  pub largura_mm: f64,
  pub altura_mm: f64,
  pub margem_sup_mm: f64,
  pub margem_inf_mm: f64,
  pub margem_esq_mm: f64,
  pub margem_dir_mm: f64,
  pub corpo_pt: f64,
  pub entrelinha: f64,
}

impl Geometria {
  /// A4 com as margens da ABNT e corpo 12 (o texto de trabalho acadêmico).
  pub fn a4_abnt() -> Self {
    Geometria {
      largura_mm: 210.0,
      altura_mm: 297.0,
      margem_sup_mm: 30.0,
      margem_inf_mm: 20.0,
      margem_esq_mm: 30.0,
      margem_dir_mm: 20.0,
      corpo_pt: 12.0,
      entrelinha: ENTRELINHA_PADRAO,
    }
  }

  /// Largura útil de texto, em milímetros.
  pub fn largura_util(&self) -> f64 {
    (self.largura_mm - self.margem_esq_mm - self.margem_dir_mm).max(EPS)
  }

  /// Altura útil de texto, em milímetros.
  pub fn altura_util(&self) -> f64 {
    (self.altura_mm - self.margem_sup_mm - self.margem_inf_mm).max(EPS)
  }

  /// A largura útil convertida em pontos, para o render de PDF (`D95`).
  ///
  /// A geometria é a **mesma** nos dois motores; o PDF só troca a unidade de
  /// medida, porque a página dele é medida em pontos pelo padrão do formato.
  pub fn largura_util_pt(&self) -> f64 {
    mm_para_pt(self.largura_util())
  }
}

/// Milímetros para pontos: 1 mm = 72/25,4 pt.
pub fn mm_para_pt(mm: f64) -> f64 {
  mm * 72.0 / 25.4
}

/// O resultado resumido da paginação: quanto o documento ocupa.
///
/// `paginas` é o total. `alturas_por_no` diz quanto cada nó ocupa, para o editor
/// desenhar a régua sem recomputar. A chave é o `id` do nó. As duas coisas saem de
/// [`compor`], que é a fonte única do layout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Paginacao {
  pub paginas: u32,
  pub alturas_por_no: BTreeMap<String, f64>,
}

/// Uma linha composta: o que o PDF escreve e o que a régua do editor mede.
///
/// `y_mm` é a distância da linha ao **topo da área útil** (isto é, já fora a
/// margem superior). O render converte para a coordenada de base do PDF.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinhaComposta {
  pub no_id: String,
  pub texto: String,
  pub marcas: Vec<MarcaInline>,
  pub fonte_pt: f64,
  /// A linha é um título de capítulo, não corpo de texto.
  pub titulo: bool,
  pub altura_linha_mm: f64,
  pub y_mm: f64,
}

/// Uma página composta.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginaComposta {
  pub linhas: Vec<LinhaComposta>,
}

/// A composição completa do documento.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Composicao {
  /// Sempre pelo menos uma página: um documento vazio ocupa uma página em branco.
  pub paginas: Vec<PaginaComposta>,
  pub alturas_por_no: BTreeMap<String, f64>,
}

impl Composicao {
  /// A página 1-based de um nó, se ele tem linha.
  pub fn pagina_do_no(&self, no_id: &str) -> Option<usize> {
    self
      .paginas
      .iter()
      .position(|p| p.linhas.iter().any(|l| l.no_id == no_id))
      .map(|i| i + 1)
  }
}

/// Um bloco de layout: o que a paginação enfileira. Não é nó do documento — é a
/// visão de página de um nó.
#[derive(Debug, Clone, PartialEq)]
struct Bloco {
  no_id: String,
  linhas: Vec<String>,
  marcas: Vec<MarcaInline>,
  fonte_pt: f64,
  altura_linha_mm: f64,
  altura_mm: f64,
  titulo: bool,
  keep_with_next: bool,
}

impl Bloco {
  fn altura_de(linhas: &[String], altura_linha_mm: f64) -> f64 {
    linhas.len() as f64 * altura_linha_mm
  }
}

/// Compõe o documento: linhas, páginas e altura de cada nó.
///
/// É a função pública do motor, e a única que decide layout. [`paginar`] é a
/// forma resumida dela, para quem só quer o total e a régua.
pub fn compor(
  arvore: &[NoDocumento],
  geometria: &Geometria,
  medidor: &dyn MedidorDeTexto,
) -> Composicao {
  let blocos = montar_blocos(arvore, geometria, medidor);
  let distribuicao = distribuir(&blocos, geometria);

  let mut paginas: Vec<PaginaComposta> = vec![PaginaComposta { linhas: Vec::new() }];
  let mut alturas: BTreeMap<String, f64> = BTreeMap::new();
  for (bloco, (indice_pagina, y_mm)) in blocos.iter().zip(distribuicao.iter()) {
    alturas.insert(bloco.no_id.clone(), bloco.altura_mm);
    // `distribuir` pode abrir página nova; a lista nasce com uma, para o
    // documento vazio ter página em branco.
    while paginas.len() <= *indice_pagina {
      paginas.push(PaginaComposta { linhas: Vec::new() });
    }
    let pagina = &mut paginas[*indice_pagina];
    for (i, texto) in bloco.linhas.iter().enumerate() {
      pagina.linhas.push(LinhaComposta {
        no_id: bloco.no_id.clone(),
        texto: texto.clone(),
        marcas: bloco.marcas.clone(),
        fonte_pt: bloco.fonte_pt,
        titulo: bloco.titulo,
        altura_linha_mm: bloco.altura_linha_mm,
        y_mm: y_mm + i as f64 * bloco.altura_linha_mm,
      });
    }
  }

  Composicao {
    paginas,
    alturas_por_no: alturas,
  }
}

/// A forma resumida: quantas páginas e quanto cada nó ocupa.
pub fn paginar(
  arvore: &[NoDocumento],
  geometria: &Geometria,
  medidor: &dyn MedidorDeTexto,
) -> Paginacao {
  let composicao = compor(arvore, geometria, medidor);
  Paginacao {
    paginas: composicao.paginas.len() as u32,
    alturas_por_no: composicao.alturas_por_no,
  }
}

fn montar_blocos(
  arvore: &[NoDocumento],
  geometria: &Geometria,
  medidor: &dyn MedidorDeTexto,
) -> Vec<Bloco> {
  let mut blocos = Vec::new();
  for no in arvore {
    match no {
      NoDocumento::Capitulo { titulo, filhos, .. } => {
        let fonte = geometria.corpo_pt * FATOR_TITULO;
        blocos.push(medir(no.id(), titulo, &[], fonte, geometria, medidor, true));
        blocos.extend(montar_blocos(filhos, geometria, medidor));
      }
      NoDocumento::Paragrafo { inline, .. } => {
        let mut texto = String::new();
        let mut marcas = Vec::new();
        for run in inline {
          texto.push_str(&run.texto);
          marcas.extend(run.marcas.iter().copied());
        }
        marcas.sort();
        marcas.dedup();
        blocos.push(medir(
          no.id(),
          &texto,
          &marcas,
          geometria.corpo_pt,
          geometria,
          medidor,
          false,
        ));
      }
      // Citação e referência viva ocupam, na v1, a linha do marcador de vínculo.
      // O texto vem do modelo ([`NoDocumento::texto_renderizado`]), que é a fonte
      // única: o DOCX e o PDF escrevem a mesma linha, e é por isso que a
      // paridade de `I3` é comparável.
      NoDocumento::Citacao { .. } | NoDocumento::ReferenciaViva { .. } => {
        let fonte = geometria.corpo_pt;
        blocos.push(medir(
          no.id(),
          &no.texto_renderizado(),
          &[],
          fonte,
          geometria,
          medidor,
          false,
        ));
      }
    }
  }
  blocos
}

#[allow(clippy::too_many_arguments)]
fn medir(
  no_id: &str,
  texto: &str,
  marcas: &[MarcaInline],
  fonte_pt: f64,
  geometria: &Geometria,
  medidor: &dyn MedidorDeTexto,
  keep_with_next: bool,
) -> Bloco {
  let altura_linha_mm = medidor.altura_linha(fonte_pt) * geometria.entrelinha;
  let linhas = quebrar_linhas(texto, marcas, fonte_pt, geometria, medidor);
  Bloco {
    no_id: no_id.to_string(),
    altura_mm: Bloco::altura_de(&linhas, altura_linha_mm),
    linhas,
    marcas: marcas.to_vec(),
    fonte_pt,
    altura_linha_mm,
    titulo: keep_with_next,
    keep_with_next,
  }
}

/// Quebra o texto em linhas que cabem na largura útil.
///
/// Greedy por palavra: a linha fecha quando a **próxima** palavra não cabe. Uma
/// palavra que sozinha é mais larga que a coluna é fatiada, porque a alternativa
/// — deixá-la transbordar — é o defeito que a quebra existe para evitar.
///
/// Texto vazio produz **uma** linha vazia: um nó sem texto continua ocupando a
/// altura mínima, que é o que a citação reserva.
fn quebrar_linhas(
  texto: &str,
  marcas: &[MarcaInline],
  fonte_pt: f64,
  geometria: &Geometria,
  medidor: &dyn MedidorDeTexto,
) -> Vec<String> {
  let limite = geometria.largura_util();
  let espaco = medidor.largura(" ", fonte_pt, marcas);
  let mut linhas: Vec<String> = Vec::new();
  let mut atual = String::new();
  let mut largura_atual = 0.0;

  for palavra in texto.split_whitespace() {
    let partes = fatiar_palavra(palavra, marcas, fonte_pt, limite, medidor);
    let fatiada = partes.len() > 1;
    for parte in partes.iter() {
      let largura_parte = medidor.largura(parte, fonte_pt, marcas);
      if fatiada {
        // Palavra mais larga que a coluna: cada pedaço ocupa a própria linha, e
        // nenhum deles entra colado ao que já estava na linha.
        if !atual.is_empty() {
          linhas.push(atual);
        }
        atual = String::new();
        atual.push_str(parte);
        largura_atual = largura_parte;
        continue;
      }
      if !atual.is_empty() && largura_atual + espaco + largura_parte > limite + EPS {
        linhas.push(atual);
        atual = String::new();
        largura_atual = 0.0;
      }
      if !atual.is_empty() {
        atual.push(' ');
        largura_atual += espaco;
      }
      atual.push_str(parte);
      largura_atual += largura_parte;
    }
  }

  if !atual.is_empty() || linhas.is_empty() {
    linhas.push(atual);
  }
  linhas
}

/// Devolve a palavra inteira, ou seus pedaços quando ela não cabe na coluna.
fn fatiar_palavra(
  palavra: &str,
  marcas: &[MarcaInline],
  fonte_pt: f64,
  limite: f64,
  medidor: &dyn MedidorDeTexto,
) -> Vec<String> {
  if medidor.largura(palavra, fonte_pt, marcas) <= limite + EPS {
    return vec![palavra.to_string()];
  }
  let mut partes = Vec::new();
  let mut atual = String::new();
  for caractere in palavra.chars() {
    let tentativa = format!("{atual}{caractere}");
    if !atual.is_empty() && medidor.largura(&tentativa, fonte_pt, marcas) > limite + EPS {
      partes.push(atual);
      atual = String::new();
    }
    atual.push(caractere);
  }
  if !atual.is_empty() {
    partes.push(atual);
  }
  partes
}

/// Distribui os blocos em páginas. Devolve, para cada bloco, o índice da página
/// (0-based) e a distância do topo da área útil.
///
/// A página é 1-based para quem lê; aqui é índice, porque vira posição no `Vec`.
/// Um bloco só empurra para a próxima quando não cabe e a página já tem conteúdo.
fn distribuir(blocos: &[Bloco], geometria: &Geometria) -> Vec<(usize, f64)> {
  let util = geometria.altura_util();
  let mut destino = Vec::with_capacity(blocos.len());
  let mut pagina = 0usize;
  let mut restante = util;
  let mut tem_conteudo = false;

  for (i, bloco) in blocos.iter().enumerate() {
    if bloco.keep_with_next
      && let Some(proximo) = blocos.get(i + 1)
      && bloco.altura_mm + proximo.altura_mm > restante + EPS
      && tem_conteudo
    {
      pagina += 1;
      restante = util;
      tem_conteudo = false;
    }

    if bloco.altura_mm > restante + EPS && tem_conteudo {
      pagina += 1;
      restante = util;
    }

    destino.push((pagina, util - restante));
    restante -= bloco.altura_mm;
    tem_conteudo = true;
  }

  destino
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::tcc::documento::{Inline, ModoVinculo};
  use crate::tcc::medidor::MedidorDeterministico;

  /// Geometria de teste: altura útil redonda e largura útil conhecida.
  fn geometria_de_teste() -> Geometria {
    Geometria {
      largura_mm: 110.0,
      altura_mm: 50.0,
      margem_sup_mm: 0.0,
      margem_inf_mm: 0.0,
      margem_esq_mm: 5.0,
      margem_dir_mm: 5.0,
      corpo_pt: 10.0,
      entrelinha: 1.0,
    }
  }

  /// Medidor de teste: cada caractere mede 100 mm, cada linha tem 10 mm.
  struct MedidorFixo;

  impl MedidorDeTexto for MedidorFixo {
    fn largura(&self, texto: &str, _tamanho_pt: f64, _marcas: &[MarcaInline]) -> f64 {
      texto.chars().count() as f64 * 100.0
    }
    fn altura_linha(&self, _tamanho_pt: f64) -> f64 {
      10.0
    }
  }

  fn texto(no_id: &str, caracteres: usize) -> NoDocumento {
    NoDocumento::Paragrafo {
      id: no_id.to_string(),
      inline: vec![Inline::simples("x".repeat(caracteres))],
    }
  }

  fn paragrafo(no_id: &str, palavras: &str) -> NoDocumento {
    NoDocumento::Paragrafo {
      id: no_id.to_string(),
      inline: vec![Inline::simples(palavras)],
    }
  }

  #[test]
  fn documento_vazio_tem_uma_pagina() {
    let comp = compor(&[], &geometria_de_teste(), &MedidorFixo);
    assert_eq!(comp.paginas.len(), 1);
    assert!(comp.alturas_por_no.is_empty());
  }

  #[test]
  fn documento_que_cabe_tem_uma_pagina() {
    // Largura útil 100 mm, medidor 100 mm por caractere: 1 caractere = 1 linha.
    // Altura útil 50 mm, cada linha 10 mm: cabem 5 linhas.
    let arvore = vec![texto("a", 1), texto("b", 1), texto("c", 1)];
    let pag = paginar(&arvore, &geometria_de_teste(), &MedidorFixo);
    assert_eq!(pag.paginas, 1);
    assert_eq!(pag.alturas_por_no.len(), 3);
  }

  #[test]
  fn documento_que_nao_cabe_vira_duas_paginas() {
    // 6 parágrafos de 1 linha cada: 6 linhas, 5 cabem, a sexta passa.
    let arvore: Vec<NoDocumento> = (0..6).map(|i| texto(&format!("n{i}"), 1)).collect();
    let pag = paginar(&arvore, &geometria_de_teste(), &MedidorFixo);
    assert_eq!(pag.paginas, 2);
  }

  #[test]
  fn paragrafo_com_mais_de_uma_linha_vai_inteiro_para_a_pagina_seguinte() {
    // Cada caractere = 1 linha de 10 mm (largura útil 100 mm, medidor 100 mm por
    // caractere). Altura útil 50 mm = 5 linhas.
    // O filler ocupa 4 linhas (40 mm) e deixa 10 mm. O parágrafo de 3 linhas
    // (30 mm) não cabe nos 10 mm e vai inteiro para a página 2 — sem viúva.
    let arvore = vec![texto("filler", 4), texto("a", 3), texto("c", 1)];
    let comp = compor(&arvore, &geometria_de_teste(), &MedidorFixo);
    assert_eq!(comp.pagina_do_no("filler"), Some(1));
    assert_eq!(comp.pagina_do_no("a"), Some(2));
    assert_eq!(comp.pagina_do_no("c"), Some(2));
    assert_eq!(comp.paginas.len(), 2);
  }

  #[test]
  fn titulo_nao_fica_separado_do_primeiro_paragrafo() {
    // Filler de 4 linhas (40 mm) deixa 10 mm na página 1. Título 1 linha
    // (10 mm) + parágrafo 3 linhas (30 mm) = 40 mm: cabem juntos, mas não nos
    // 10 mm restantes. O título tem que descer com o parágrafo.
    let titulo = NoDocumento::Capitulo {
      id: "cap".to_string(),
      titulo: "1".to_string(),
      filhos: vec![texto("p", 3)],
    };
    let arvore = vec![texto("filler", 4), titulo];
    let comp = compor(&arvore, &geometria_de_teste(), &MedidorFixo);
    assert_eq!(comp.pagina_do_no("filler"), Some(1));
    assert_eq!(
      comp.pagina_do_no("cap"),
      comp.pagina_do_no("p"),
      "o título desceu junto com o parágrafo"
    );
    assert_eq!(comp.pagina_do_no("cap"), Some(2));
  }

  #[test]
  fn a_mesma_entrada_da_a_mesma_paginacao() {
    let arvore = vec![texto("a", 250), texto("b", 120), texto("c", 700)];
    let geo = Geometria::a4_abnt();
    let primeira = paginar(&arvore, &geo, &MedidorDeterministico::novo());
    let segunda = paginar(&arvore, &geo, &MedidorDeterministico::novo());
    assert_eq!(primeira, segunda);
  }

  #[test]
  fn mudar_a_geometria_nao_altera_o_conteudo() {
    let arvore = vec![texto("a", 250), texto("b", 120), texto("c", 700)];
    let estreita = Geometria {
      largura_mm: 120.0,
      ..Geometria::a4_abnt()
    };
    let larga = Geometria {
      largura_mm: 210.0,
      ..Geometria::a4_abnt()
    };

    let pag_estreita = paginar(&arvore, &estreita, &MedidorDeterministico::novo());
    let pag_larga = paginar(&arvore, &larga, &MedidorDeterministico::novo());

    // O conjunto de nós é o mesmo: a geometria move texto de página, não some
    // nem cria conteúdo.
    let chaves_estreita: Vec<&String> = pag_estreita.alturas_por_no.keys().collect();
    let chaves_larga: Vec<&String> = pag_larga.alturas_por_no.keys().collect();
    assert_eq!(chaves_estreita, chaves_larga);
  }

  #[test]
  fn citacao_ocupa_a_linha_do_marcador() {
    let arvore = vec![NoDocumento::Citacao {
      id: "cit".to_string(),
      referencia_id: "ref-1".to_string(),
      modo: ModoVinculo::Vivo,
    }];
    let comp = compor(
      &arvore,
      &Geometria::a4_abnt(),
      &MedidorDeterministico::novo(),
    );
    let linhas = &comp.paginas[0].linhas;
    assert_eq!(linhas.len(), 1, "o marcador cabe em uma linha no A4");
    assert_eq!(linhas[0].texto, "[citação: ref-1 · vivo]");
    // A altura reservada é a soma das linhas que o nó de fato ocupa: fonte
    // única entre a régua do editor e o que o render escreve.
    assert_eq!(
      comp.alturas_por_no.get("cit"),
      Some(&(linhas.len() as f64 * linhas[0].altura_linha_mm))
    );
  }

  /// Geometria larga: a coluna cabe três caracteres do medidor fixo.
  fn geometria_larga_de_teste() -> Geometria {
    Geometria {
      largura_mm: 310.0,
      ..geometria_de_teste()
    }
  }

  #[test]
  fn quebra_por_palavra_e_nao_por_caractere() {
    // Coluna de 300 mm, medidor de 100 mm por caractere: "aa" (200) + espaço
    // (100) + "bb" (200) estoura, então cada palavra fecha a sua linha.
    let comp = compor(
      &[paragrafo("p", "aa bb")],
      &geometria_larga_de_teste(),
      &MedidorFixo,
    );
    let textos: Vec<&str> = comp.paginas[0]
      .linhas
      .iter()
      .map(|l| l.texto.as_str())
      .collect();
    assert_eq!(textos, vec!["aa", "bb"]);
    assert_eq!(comp.alturas_por_no.get("p"), Some(&20.0));
  }

  #[test]
  fn palavra_mais_larga_que_a_coluna_e_fatiada() {
    // "aaaa" mede 400 mm numa coluna de 100 mm: sem fatiamento transbordaria.
    let comp = compor(
      &[paragrafo("p", "aaaa")],
      &geometria_de_teste(),
      &MedidorFixo,
    );
    let textos: Vec<&str> = comp.paginas[0]
      .linhas
      .iter()
      .map(|l| l.texto.as_str())
      .collect();
    assert_eq!(textos, vec!["a", "a", "a", "a"]);
  }

  #[test]
  fn palavra_fatiada_nao_cola_no_que_ja_estava_na_linha() {
    // "aaaa bb" com coluna de 300 mm: "aaaa" (400) é fatiada em "aa"/"aa" e a
    // palavra seguinte não entra colada.
    let comp = compor(
      &[paragrafo("p", "aaaa bb")],
      &geometria_larga_de_teste(),
      &MedidorFixo,
    );
    let textos: Vec<&str> = comp.paginas[0]
      .linhas
      .iter()
      .map(|l| l.texto.as_str())
      .collect();
    assert_eq!(textos, vec!["aaa", "a", "bb"]);
  }

  #[test]
  fn texto_vazio_reserva_uma_linha() {
    let comp = compor(&[texto("p", 0)], &geometria_de_teste(), &MedidorFixo);
    assert_eq!(comp.paginas[0].linhas.len(), 1);
    assert_eq!(comp.alturas_por_no.get("p"), Some(&10.0));
  }

  #[test]
  fn a_composicao_sabe_a_pagina_de_cada_no() {
    let titulo = NoDocumento::Capitulo {
      id: "cap".to_string(),
      titulo: "T".to_string(),
      filhos: vec![paragrafo("p", "texto")],
    };
    let comp = compor(
      &[titulo],
      &Geometria::a4_abnt(),
      &MedidorDeterministico::novo(),
    );
    assert_eq!(comp.pagina_do_no("cap"), Some(1));
    assert_eq!(comp.pagina_do_no("p"), Some(1));
    assert_eq!(comp.pagina_do_no("fantasma"), None);
  }

  #[test]
  fn y_mm_acumula_a_altura_da_linha() {
    let comp = compor(
      &[paragrafo("p", "aaa aaa aaa")],
      &geometria_larga_de_teste(),
      &MedidorFixo,
    );
    let ys: Vec<f64> = comp.paginas[0].linhas.iter().map(|l| l.y_mm).collect();
    assert_eq!(ys, vec![0.0, 10.0, 20.0]);
  }
}
