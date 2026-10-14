//! A árvore do documento e o texto com marcas (SPEC-D-013, `D85`).
//!
//! A árvore é uma **lista de adjacência ordenada** (`pai_id` + `ordem`, não
//! JSON aninhado). O inline é uma linha própria, ordenada, não um blob de HTML.
//! A geometria não mora aqui.
//!
//! Este arquivo é o par em Rust do contrato fixado em
//! `cecistudy-desktop/docs/espacos/tcc-modelo-do-documento.md`. Ele existe para
//! o motor de paginação e para a serialização canônica da versão (D87). O
//! espelho TypeScript, quando existir, deriva daqui e não ganha regra nova.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;

/// Marca de formatação de um trecho de texto.
///
/// A ordem de declaração é a ordem canônica da coluna `marcas` (`D85`): `Ord`
/// derivado por `Negrito < Italico < Sublinhado < Sobrescrito < Subscrito`.
/// A coluna guarda essa ordem, unida por vírgula; a árvore em memória pode
/// chegar fora de ordem e é normalizada por [`normalizar`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MarcaInline {
  Negrito,
  Italico,
  Sublinhado,
  Sobrescrito,
  Subscrito,
}

impl MarcaInline {
  /// O token estável gravado na coluna `marcas`.
  pub fn token(self) -> &'static str {
    match self {
      MarcaInline::Negrito => "negrito",
      MarcaInline::Italico => "italico",
      MarcaInline::Sublinhado => "sublinhado",
      MarcaInline::Sobrescrito => "sobrescrito",
      MarcaInline::Subscrito => "subscrito",
    }
  }

  /// Lê um token da coluna `marcas`. Token desconhecido é erro, não é ignorado.
  pub fn de_token(token: &str) -> Result<Self, DocError> {
    match token {
      "negrito" => Ok(MarcaInline::Negrito),
      "italico" => Ok(MarcaInline::Italico),
      "sublinhado" => Ok(MarcaInline::Sublinhado),
      "sobrescrito" => Ok(MarcaInline::Sobrescrito),
      "subscrito" => Ok(MarcaInline::Subscrito),
      outro => Err(DocError::MarcaDesconhecida(outro.to_string())),
    }
  }
}

/// Trecho de texto com as marcas que o cobrem.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Inline {
  pub texto: String,
  pub marcas: Vec<MarcaInline>,
}

impl Inline {
  /// Um trecho sem marcas.
  pub fn simples(texto: impl Into<String>) -> Self {
    Inline {
      texto: texto.into(),
      marcas: Vec::new(),
    }
  }
}

/// Como uma citação/fragmento aponta para a fonte (`D66`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModoVinculo {
  /// Aponta para a referência viva; a fonte pode mudar.
  Vivo,
  /// Congela o texto no momento da citação.
  Snapshot,
  /// Cópia editorial, sem vínculo.
  CopiaEditorial,
}

/// Um nó da árvore do documento.
///
/// O nó carrega só o conteúdo. A posição na árvore (`pai_id`, `ordem`) é da
/// linha de persistência, não do nó. `id` é `String` (uuid do aplicativo),
/// nunca inteiro de autoincremento (`D85`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "tipo", rename_all = "kebab-case")]
pub enum NoDocumento {
  /// Título de capítulo. Agrupa filhos. Não tem inline próprio.
  #[serde(rename_all = "camelCase")]
  Capitulo {
    id: String,
    titulo: String,
    filhos: Vec<NoDocumento>,
  },
  /// Parágrafo. Texto com marcas. Pode referenciar a Biblioteca na nota (a
  /// referência é da Biblioteca, não do TCC — ADR-010), mas o vínculo da
  /// citação é o nó `Citacao`, não o parágrafo.
  #[serde(rename_all = "camelCase")]
  Paragrafo { id: String, inline: Vec<Inline> },
  /// Citação que aponta para uma referência da Biblioteca. Sem FK no banco,
  /// porque a Biblioteca pode não existir ainda (`D66`/`D85`).
  #[serde(rename_all = "camelCase")]
  Citacao {
    id: String,
    referencia_id: String,
    modo: ModoVinculo,
  },
  /// Referência viva para um fragmento do Conhecimento. Idem: sem FK.
  #[serde(rename_all = "camelCase")]
  ReferenciaViva { id: String, fragmento_id: String },
}

impl NoDocumento {
  /// O identificador do nó.
  pub fn id(&self) -> &str {
    match self {
      NoDocumento::Capitulo { id, .. }
      | NoDocumento::Paragrafo { id, .. }
      | NoDocumento::Citacao { id, .. }
      | NoDocumento::ReferenciaViva { id, .. } => id,
    }
  }

  /// O tipo, para casar com a CHECK de `no_documento.tipo`.
  pub fn tipo(&self) -> TipoNo {
    match self {
      NoDocumento::Capitulo { .. } => TipoNo::Capitulo,
      NoDocumento::Paragrafo { .. } => TipoNo::Paragrafo,
      NoDocumento::Citacao { .. } => TipoNo::Citacao,
      NoDocumento::ReferenciaViva { .. } => TipoNo::ReferenciaViva,
    }
  }

  /// Os filhos diretos, para percorrer a árvore sem escrever o `match` de novo.
  pub fn filhos(&self) -> &[NoDocumento] {
    match self {
      NoDocumento::Capitulo { filhos, .. } => filhos,
      _ => &[],
    }
  }

  /// O texto que o nó **renderiza** — o que o DOCX escreve e o que o PDF escreve.
  ///
  /// Parágrafo é o `inline` concatenado. Citação e referência viva **não têm
  /// texto próprio**: o conteúdo vem da fonte (Biblioteca/Conhecimento), que
  /// ainda não existe, então o que sai é o marcador de [`marcador_de_citacao`] /
  /// [`marcador_de_fragmento`].
  ///
  /// É a **fonte única** do texto renderizado: paginação, DOCX e PDF leem daqui,
  /// e é o que torna a paridade de `I3` comparável em vez de repetida.
  pub fn texto_renderizado(&self) -> String {
    match self {
      NoDocumento::Capitulo { titulo, .. } => titulo.clone(),
      NoDocumento::Paragrafo { inline, .. } => inline.iter().map(|r| r.texto.as_str()).collect(),
      NoDocumento::Citacao {
        referencia_id,
        modo,
        ..
      } => marcador_de_citacao(referencia_id, *modo),
      NoDocumento::ReferenciaViva { fragmento_id, .. } => marcador_de_fragmento(fragmento_id),
    }
  }
}

/// O marcador legível de uma citação enquanto a Referência não existe.
///
/// **Por que um marcador e não uma citação ABNT.** A citação é **gerada** do
/// vínculo (`D79`), e o vínculo aponta para a Referência da Biblioteca — que não
/// existe. Escrever "(Autor, 2026)" seria inventar metadado, que é a lacuna
/// silenciosa que `D79` proíbe. O marcador é feio e honesto: diz que ali há uma
/// citação e de qual referência, e nada mais.
pub fn marcador_de_citacao(referencia_id: &str, modo: ModoVinculo) -> String {
  let modo = match modo {
    ModoVinculo::Vivo => "vivo",
    ModoVinculo::Snapshot => "snapshot",
    ModoVinculo::CopiaEditorial => "copia-editorial",
  };
  format!("[citação: {referencia_id} · {modo}]")
}

/// O marcador da referência viva, pelo mesmo motivo.
pub fn marcador_de_fragmento(fragmento_id: &str) -> String {
  format!("[fragmento: {fragmento_id}]")
}

/// Reconhece o marcador de citação.
///
/// Fecha o ciclo do que a exportação produziu: o formato é deste arquivo, então
/// reconhecê-lo não é adivinhação.
pub fn ler_marcador_de_citacao(texto: &str) -> Option<(String, ModoVinculo)> {
  let interno = texto.strip_prefix("[citação: ")?.strip_suffix(']')?;
  let (id, modo) = interno.rsplit_once(" · ")?;
  let modo = match modo {
    "vivo" => ModoVinculo::Vivo,
    "snapshot" => ModoVinculo::Snapshot,
    "copia-editorial" => ModoVinculo::CopiaEditorial,
    _ => return None,
  };
  if id.is_empty() {
    return None;
  }
  Some((id.to_string(), modo))
}

/// Reconhece o marcador de fragmento.
pub fn ler_marcador_de_fragmento(texto: &str) -> Option<String> {
  let interno = texto.strip_prefix("[fragmento: ")?.strip_suffix(']')?;
  if interno.is_empty() {
    return None;
  }
  Some(interno.to_string())
}

/// O tipo de nó, estável para a coluna `tipo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoNo {
  Capitulo,
  Paragrafo,
  Citacao,
  ReferenciaViva,
}

impl TipoNo {
  /// O token gravado na coluna `tipo`.
  pub fn token(self) -> &'static str {
    match self {
      TipoNo::Capitulo => "capitulo",
      TipoNo::Paragrafo => "paragrafo",
      TipoNo::Citacao => "citacao",
      TipoNo::ReferenciaViva => "referencia-viva",
    }
  }
}

impl fmt::Display for TipoNo {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(self.token())
  }
}

/// A linha de `no_documento`, do jeito que o banco guarda: sem aninhamento.
///
/// `ordem` é a posição entre irmãos (mesmo `pai_id`). O índice único
/// `idx_no_ordem (documento_id, COALESCE(pai_id,''), ordem)` impõe isso; aqui a
/// validação é a mesma, antes de chegar ao banco.
#[derive(Debug, Clone, PartialEq)]
pub struct NoLinha {
  pub id: String,
  pub pai_id: Option<String>,
  pub ordem: u32,
  pub tipo: TipoNo,
  pub titulo: Option<String>,
  pub referencia_id: Option<String>,
  pub modo: Option<ModoVinculo>,
  pub fragmento_id: Option<String>,
  pub inlines: Vec<Inline>,
}

/// Erro do modelo do documento. Não é erro de banco: é o dado malformado
/// recusado antes de virar SQL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocError {
  IdVazio,
  IdDuplicado(String),
  PaiInexistente {
    filho: String,
    pai: String,
  },
  OrdemDuplicada {
    pai: String,
    ordem: u32,
  },
  Ciclo(String),
  /// O conjunto de colunas não casa com o tipo (a CHECK do banco).
  ColunasInvalidas {
    id: String,
    tipo: &'static str,
  },
  /// Citação ou referência viva com o alvo vazio.
  VinculoVazio {
    id: String,
    campo: &'static str,
  },
  MarcaDesconhecida(String),
  Serializacao(String),
}

impl fmt::Display for DocError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      DocError::IdVazio => write!(f, "nó sem id"),
      DocError::IdDuplicado(id) => write!(f, "id repetido: {id}"),
      DocError::PaiInexistente { filho, pai } => {
        write!(f, "o pai {pai} de {filho} não existe no documento")
      }
      DocError::OrdemDuplicada { pai, ordem } => {
        write!(f, "ordem {ordem} repetida entre irmãos de {pai}")
      }
      DocError::Ciclo(id) => write!(f, "ciclo na árvore no nó {id}"),
      DocError::ColunasInvalidas { id, tipo } => {
        write!(f, "colunas não casam com o tipo {tipo} no nó {id}")
      }
      DocError::VinculoVazio { id, campo } => {
        write!(f, "o nó {id} é um vínculo sem {campo}")
      }
      DocError::MarcaDesconhecida(token) => write!(f, "marca desconhecida: {token}"),
      DocError::Serializacao(msg) => write!(f, "falha ao serializar o documento: {msg}"),
    }
  }
}

impl std::error::Error for DocError {}

/// Ordena e deduplica as marcas de cada trecho, in-place.
///
/// A ordem da coluna `marcas` é canônica (`D85`); a árvore que chega do editor
/// pode estar fora de ordem. `Snapshot` (D87) normaliza antes de comparar, senão
/// a mesma árvore com marcas em ordem diferente viraria duas versões.
pub fn normalizar(arvore: &mut [NoDocumento]) {
  for no in arvore.iter_mut() {
    match no {
      NoDocumento::Capitulo { filhos, .. } => normalizar(filhos),
      NoDocumento::Paragrafo { inline, .. } => {
        for run in inline.iter_mut() {
          run.marcas.sort();
          run.marcas.dedup();
        }
      }
      _ => {}
    }
  }
}

/// Junta as marcas na ordem canônica da coluna.
pub fn marcas_para_coluna(marcas: &[MarcaInline]) -> String {
  let mut copia = marcas.to_vec();
  copia.sort();
  copia.dedup();
  copia
    .iter()
    .map(|m| m.token())
    .collect::<Vec<_>>()
    .join(",")
}

/// Lê a coluna `marcas`. String vazia é "sem marcas".
pub fn marcas_de_coluna(coluna: &str) -> Result<Vec<MarcaInline>, DocError> {
  if coluna.is_empty() {
    return Ok(Vec::new());
  }
  let mut marcas = Vec::new();
  for token in coluna.split(',') {
    marcas.push(MarcaInline::de_token(token)?);
  }
  marcas.sort();
  marcas.dedup();
  Ok(marcas)
}

/// Achata a árvore na lista de linhas de `no_documento`.
///
/// É o caminho de escrita: a árvore vem do editor, vira linha igual à coluna do
/// banco. Recusa id vazio, id repetido e vínculo sem alvo — as mesmas garantias
/// da CHECK e do índice único.
pub fn linhas_da_arvore(arvore: &[NoDocumento]) -> Result<Vec<NoLinha>, DocError> {
  let mut saida = Vec::new();
  let mut ids = HashSet::new();
  for (ordem, no) in arvore.iter().enumerate() {
    empilhar(no, None, ordem as u32, &mut saida, &mut ids)?;
  }
  Ok(saida)
}

fn empilhar(
  no: &NoDocumento,
  pai_id: Option<&str>,
  ordem: u32,
  saida: &mut Vec<NoLinha>,
  ids: &mut HashSet<String>,
) -> Result<(), DocError> {
  let id = no.id().to_string();
  if id.is_empty() {
    return Err(DocError::IdVazio);
  }
  if !ids.insert(id.clone()) {
    return Err(DocError::IdDuplicado(id));
  }

  let pai = pai_id.map(str::to_string);
  let mut linha = NoLinha {
    id: id.clone(),
    pai_id: pai,
    ordem,
    tipo: no.tipo(),
    titulo: None,
    referencia_id: None,
    modo: None,
    fragmento_id: None,
    inlines: Vec::new(),
  };

  match no {
    NoDocumento::Capitulo { titulo, filhos, .. } => {
      linha.titulo = Some(titulo.clone());
      saida.push(linha);
      for (i, filho) in filhos.iter().enumerate() {
        empilhar(filho, Some(&id), i as u32, saida, ids)?;
      }
      return Ok(());
    }
    NoDocumento::Paragrafo { inline, .. } => {
      linha.inlines = inline.clone();
    }
    NoDocumento::Citacao {
      referencia_id,
      modo,
      ..
    } => {
      if referencia_id.is_empty() {
        return Err(DocError::VinculoVazio {
          id,
          campo: "referencia_id",
        });
      }
      linha.referencia_id = Some(referencia_id.clone());
      linha.modo = Some(*modo);
    }
    NoDocumento::ReferenciaViva { fragmento_id, .. } => {
      if fragmento_id.is_empty() {
        return Err(DocError::VinculoVazio {
          id,
          campo: "fragmento_id",
        });
      }
      linha.fragmento_id = Some(fragmento_id.clone());
    }
  }

  saida.push(linha);
  Ok(())
}

/// Remonta a árvore a partir das linhas de `no_documento`.
///
/// É o caminho de leitura. Valida o que o banco garante: pai existente, ordem
/// única entre irmãos, colunas casando com o tipo, sem ciclo. Um banco correto
/// nunca cai aqui; uma linha órfã ou um ciclo é dado corrompido, e o motor
/// recusa em vez de desenhar uma tela errada.
pub fn arvore_de_linhas(linhas: Vec<NoLinha>) -> Result<Vec<NoDocumento>, DocError> {
  let mut indice: HashMap<String, NoLinha> = HashMap::new();
  for linha in linhas {
    if linha.id.is_empty() {
      return Err(DocError::IdVazio);
    }
    if indice.contains_key(&linha.id) {
      return Err(DocError::IdDuplicado(linha.id));
    }
    validar_colunas(&linha)?;
    indice.insert(linha.id.clone(), linha);
  }

  let mut filhos: HashMap<Option<String>, Vec<String>> = HashMap::new();
  for linha in indice.values() {
    if let Some(pai) = &linha.pai_id
      && !indice.contains_key(pai)
    {
      return Err(DocError::PaiInexistente {
        filho: linha.id.clone(),
        pai: pai.clone(),
      });
    }
    filhos
      .entry(linha.pai_id.clone())
      .or_default()
      .push(linha.id.clone());
  }

  for (pai, lista) in filhos.iter_mut() {
    lista.sort_by_key(|id| indice[id].ordem);
    let mut anterior: Option<u32> = None;
    for id in lista.iter() {
      let ordem = indice[id].ordem;
      if Some(ordem) == anterior {
        return Err(DocError::OrdemDuplicada {
          pai: pai.clone().unwrap_or_default(),
          ordem,
        });
      }
      anterior = Some(ordem);
    }
  }

  let mut em_pilha = HashSet::new();
  let mut montados: HashSet<String> = HashSet::new();
  let raizes = filhos.get(&None).cloned().unwrap_or_default();
  let mut arvore = Vec::with_capacity(raizes.len());
  for id in &raizes {
    arvore.push(montar(id, &indice, &filhos, &mut em_pilha, &mut montados)?);
  }

  // Um nó que nenhuma raiz alcançou está preso num ciclo fechado: não há raiz
  // para ele, e a validação do banco não o pegaria. Recusar é melhor que
  // desenhar uma árvore incompleta.
  if montados.len() != indice.len() {
    let solto = indice
      .keys()
      .find(|id| !montados.contains(*id))
      .cloned()
      .unwrap_or_default();
    return Err(DocError::Ciclo(solto));
  }

  Ok(arvore)
}

fn montar(
  id: &str,
  indice: &HashMap<String, NoLinha>,
  filhos: &HashMap<Option<String>, Vec<String>>,
  em_pilha: &mut HashSet<String>,
  montados: &mut HashSet<String>,
) -> Result<NoDocumento, DocError> {
  if !em_pilha.insert(id.to_string()) {
    return Err(DocError::Ciclo(id.to_string()));
  }
  let linha = &indice[id];
  let no = match linha.tipo {
    TipoNo::Capitulo => {
      let mut nos = Vec::new();
      if let Some(lista) = filhos.get(&Some(id.to_string())) {
        for filho in lista {
          nos.push(montar(filho, indice, filhos, em_pilha, montados)?);
        }
      }
      NoDocumento::Capitulo {
        id: id.to_string(),
        titulo: linha.titulo.clone().unwrap_or_default(),
        filhos: nos,
      }
    }
    TipoNo::Paragrafo => NoDocumento::Paragrafo {
      id: id.to_string(),
      inline: linha.inlines.clone(),
    },
    TipoNo::Citacao => NoDocumento::Citacao {
      id: id.to_string(),
      referencia_id: linha.referencia_id.clone().unwrap_or_default(),
      modo: linha.modo.unwrap_or(ModoVinculo::Vivo),
    },
    TipoNo::ReferenciaViva => NoDocumento::ReferenciaViva {
      id: id.to_string(),
      fragmento_id: linha.fragmento_id.clone().unwrap_or_default(),
    },
  };
  em_pilha.remove(id);
  montados.insert(id.to_string());
  Ok(no)
}

fn validar_colunas(linha: &NoLinha) -> Result<(), DocError> {
  let id = linha.id.clone();
  let tem_titulo = linha.titulo.is_some();
  let tem_ref = linha.referencia_id.is_some();
  let tem_modo = linha.modo.is_some();
  let tem_frag = linha.fragmento_id.is_some();
  let tem_inline = !linha.inlines.is_empty();

  let ok = match linha.tipo {
    TipoNo::Capitulo => tem_titulo && !tem_ref && !tem_modo && !tem_frag && !tem_inline,
    TipoNo::Paragrafo => !tem_titulo && !tem_ref && !tem_modo && !tem_frag,
    TipoNo::Citacao => !tem_titulo && tem_ref && tem_modo && !tem_frag && !tem_inline,
    TipoNo::ReferenciaViva => !tem_titulo && !tem_ref && !tem_modo && tem_frag && !tem_inline,
  };

  if !ok {
    return Err(DocError::ColunasInvalidas {
      id,
      tipo: linha.tipo.token(),
    });
  }
  Ok(())
}

#[cfg(test)]
mod test {
  use super::*;

  fn paragrafo(id: &str, texto: &str) -> NoDocumento {
    NoDocumento::Paragrafo {
      id: id.to_string(),
      inline: vec![Inline::simples(texto)],
    }
  }

  fn capitulo(id: &str, titulo: &str, filhos: Vec<NoDocumento>) -> NoDocumento {
    NoDocumento::Capitulo {
      id: id.to_string(),
      titulo: titulo.to_string(),
      filhos,
    }
  }

  #[test]
  fn ida_e_volta_preserva_a_arvore() {
    let arvore = vec![
      capitulo(
        "c1",
        "Introdução",
        vec![
          paragrafo("p1", "A pesquisa parte de"),
          NoDocumento::Citacao {
            id: "cit1".to_string(),
            referencia_id: "ref-42".to_string(),
            modo: ModoVinculo::Vivo,
          },
        ],
      ),
      paragrafo("p2", "Fim"),
    ];

    let linhas = linhas_da_arvore(&arvore).expect("achata");
    let voltou = arvore_de_linhas(linhas).expect("remonta");
    assert_eq!(voltou, arvore);
  }

  #[test]
  fn ordem_e_a_posicao_entre_irmaos() {
    let arvore = vec![
      paragrafo("a", "primeiro"),
      paragrafo("b", "segundo"),
      paragrafo("c", "terceiro"),
    ];
    let linhas = linhas_da_arvore(&arvore).unwrap();
    let ordens: Vec<u32> = linhas.iter().map(|l| l.ordem).collect();
    assert_eq!(ordens, vec![0, 1, 2]);
    assert!(linhas.iter().all(|l| l.pai_id.is_none()));
  }

  #[test]
  fn id_repetido_e_recusado() {
    let arvore = vec![paragrafo("x", "a"), paragrafo("x", "b")];
    assert_eq!(
      linhas_da_arvore(&arvore),
      Err(DocError::IdDuplicado("x".into()))
    );
  }

  #[test]
  fn citacao_sem_alvo_e_recusada() {
    let arvore = vec![NoDocumento::Citacao {
      id: "c".into(),
      referencia_id: String::new(),
      modo: ModoVinculo::Vivo,
    }];
    assert_eq!(
      linhas_da_arvore(&arvore),
      Err(DocError::VinculoVazio {
        id: "c".into(),
        campo: "referencia_id"
      })
    );
  }

  #[test]
  fn ordem_duplicada_entre_irmaos_e_recusada() {
    let linhas = vec![
      NoLinha {
        id: "a".into(),
        pai_id: Some("cap".into()),
        ordem: 0,
        tipo: TipoNo::Paragrafo,
        titulo: None,
        referencia_id: None,
        modo: None,
        fragmento_id: None,
        inlines: vec![],
      },
      NoLinha {
        id: "cap".into(),
        pai_id: None,
        ordem: 0,
        tipo: TipoNo::Capitulo,
        titulo: Some("t".into()),
        referencia_id: None,
        modo: None,
        fragmento_id: None,
        inlines: vec![],
      },
      NoLinha {
        id: "b".into(),
        pai_id: Some("cap".into()),
        ordem: 0,
        tipo: TipoNo::Paragrafo,
        titulo: None,
        referencia_id: None,
        modo: None,
        fragmento_id: None,
        inlines: vec![],
      },
    ];
    assert_eq!(
      arvore_de_linhas(linhas),
      Err(DocError::OrdemDuplicada {
        pai: "cap".into(),
        ordem: 0
      })
    );
  }

  #[test]
  fn pai_inexistente_e_recusado() {
    let linhas = vec![NoLinha {
      id: "a".into(),
      pai_id: Some("fantasma".into()),
      ordem: 0,
      tipo: TipoNo::Paragrafo,
      titulo: None,
      referencia_id: None,
      modo: None,
      fragmento_id: None,
      inlines: vec![],
    }];
    assert_eq!(
      arvore_de_linhas(linhas),
      Err(DocError::PaiInexistente {
        filho: "a".into(),
        pai: "fantasma".into()
      })
    );
  }

  #[test]
  fn ciclo_sem_raiz_e_recusado() {
    let linhas = vec![
      NoLinha {
        id: "a".into(),
        pai_id: Some("b".into()),
        ordem: 0,
        tipo: TipoNo::Paragrafo,
        titulo: None,
        referencia_id: None,
        modo: None,
        fragmento_id: None,
        inlines: vec![],
      },
      NoLinha {
        id: "b".into(),
        pai_id: Some("a".into()),
        ordem: 0,
        tipo: TipoNo::Paragrafo,
        titulo: None,
        referencia_id: None,
        modo: None,
        fragmento_id: None,
        inlines: vec![],
      },
    ];
    assert!(matches!(arvore_de_linhas(linhas), Err(DocError::Ciclo(_))));
  }

  #[test]
  fn colunas_incoerentes_com_o_tipo_sao_recusadas() {
    let linhas = vec![NoLinha {
      id: "p".into(),
      pai_id: None,
      ordem: 0,
      tipo: TipoNo::Paragrafo,
      titulo: Some("não deveria ter título".into()),
      referencia_id: None,
      modo: None,
      fragmento_id: None,
      inlines: vec![],
    }];
    assert_eq!(
      arvore_de_linhas(linhas),
      Err(DocError::ColunasInvalidas {
        id: "p".into(),
        tipo: "paragrafo"
      })
    );
  }

  #[test]
  fn normalizar_ordena_as_marcas() {
    let mut arvore = vec![NoDocumento::Paragrafo {
      id: "p".into(),
      inline: vec![Inline {
        texto: "x".into(),
        marcas: vec![
          MarcaInline::Subscrito,
          MarcaInline::Negrito,
          MarcaInline::Negrito,
        ],
      }],
    }];
    normalizar(&mut arvore);
    match &arvore[0] {
      NoDocumento::Paragrafo { inline, .. } => {
        assert_eq!(
          inline[0].marcas,
          vec![MarcaInline::Negrito, MarcaInline::Subscrito]
        );
      }
      _ => unreachable!(),
    }
  }

  #[test]
  fn coluna_de_marcas_ida_e_volta() {
    let marcas = vec![
      MarcaInline::Subscrito,
      MarcaInline::Negrito,
      MarcaInline::Negrito,
    ];
    let coluna = marcas_para_coluna(&marcas);
    assert_eq!(coluna, "negrito,subscrito");
    assert_eq!(
      marcas_de_coluna(&coluna),
      Ok(vec![MarcaInline::Negrito, MarcaInline::Subscrito])
    );
  }

  #[test]
  fn marca_desconhecida_e_erro() {
    assert_eq!(
      marcas_de_coluna("negrito,tachado"),
      Err(DocError::MarcaDesconhecida("tachado".into()))
    );
  }
}
