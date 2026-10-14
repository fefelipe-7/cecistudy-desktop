//! O round-trip DOCX (SPEC-D-011, `D78`, decisão do app `D95`).
//!
//! Duas metades no mesmo arquivo, porque elas são o **mesmo contrato** lido nas
//! duas direções:
//!
//! - [`escrever`] — a árvore vira um pacote OOXML.
//! - [`ler`] — um pacote OOXML vira **itens a resolver** e o relatório do
//!   contrato de tolerância. Nunca vira árvore: `D78` proíbe sobrescrever.
//!
//! **Por que o leitor é nosso e não de uma biblioteca de DOCX.** A cláusula de
//! `D78` fixa o significado de `w:ins`/`w:del`: revisão vira item `sugestao`, com
//! autor, data e âncora por **texto citado**, e nunca é aplicada. `D78`
//! alternativa (c) — "deixar a decisão para a biblioteca escolhida" — foi
//! rejeitada: decisão de domínio não se delega à dependência. Então o que vem de
//! biblioteca é só o **contêiner** (`zip`) e a **sintaxe** (`quick-xml`); o que
//! `w:ins` significa mora aqui.
//!
//! **O que o exportador entrega e o que ele declara.** A citação e a referência
//! viva são geradas do vínculo (ADR-010), e a Referência **não existe ainda** —
//! não há metadado para formatar. O que sai é um **marcador legível**
//! (`[citação: ref-42 · vivo]`), declarado no contrato de tolerância, e não um
//! ABNT inventado: citação com lacuna silenciosa é proibido por `D79`. O PDF
//! escreve o mesmo marcador, e é o que torna a paridade de `I3` verificável.

use std::io::{Cursor, Read, Write};

use chrono::{DateTime, Utc};
use quick_xml::Reader;
use quick_xml::events::Event;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use super::documento::{MarcaInline, NoDocumento, marcador_de_citacao};
use super::item::{ItemAResolver, TipoItem};
use super::paginacao::Geometria;

/// O namespace do WordprocessingML. Só este é usado; o resto do pacote OOXML é
/// plumbing declarado no fim do arquivo.
const NS_W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const NS_R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const NS_CT: &str = "http://schemas.openxmlformats.org/package/2006/content-types";
const NS_REL: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const NS_CP: &str = "http://schemas.openxmlformats.org/package/2006/metadata/core-properties";
const NS_DC: &str = "http://purl.org/dc/elements/1.1/";
const NS_DCTERMS: &str = "http://purl.org/dc/terms/";
const NS_XSI: &str = "http://www.w3.org/2001/XMLSchema-instance";

/// Milímetros para twips: 1 mm = 1440/25,4 twips (o Word mede em twentieths de
/// polegada). É a **mesma** geometria do modelo, em outra unidade — `I3`.
const MM_PARA_TWIPS: f64 = 1440.0 / 25.4;

/// Erro do round-trip. É erro de **arquivo**, não de banco.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocxErro {
  /// O pacote não é um ZIP legível, ou uma parte não pôde ser lida.
  PacoteInvalido(String),
  /// O pacote é um ZIP, mas não é um DOCX: falta a parte principal.
  ParteAusente(&'static str),
  /// XML de parte que não pôde ser lido.
  Xml(String),
}

impl std::fmt::Display for DocxErro {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      DocxErro::PacoteInvalido(msg) => write!(f, "arquivo DOCX inválido: {msg}"),
      DocxErro::ParteAusente(parte) => write!(f, "o DOCX não tem a parte {parte}"),
      DocxErro::Xml(msg) => write!(f, "XML de DOCX inválido: {msg}"),
    }
  }
}

impl std::error::Error for DocxErro {}

// ---------------------------------------------------------------------------
// Texto do modelo
// ---------------------------------------------------------------------------

/// O texto corrido do documento, um parágrafo por linha.
///
/// É o que a âncora da importação casa (`D78`): o trecho citado precisa existir
/// **aqui**, no texto atual, e não numa posição. Por isso a âncora é o texto e
/// esta função existe — para o motor de âncora ter contra o que casar.
pub fn texto_plano(arvore: &[NoDocumento]) -> String {
  let mut partes: Vec<String> = Vec::new();
  for no in arvore {
    if let NoDocumento::Capitulo { filhos, .. } = no {
      partes.push(no.texto_renderizado());
      partes.push(texto_plano(filhos));
      continue;
    }
    partes.push(no.texto_renderizado());
  }
  partes.retain(|p| !p.is_empty());
  partes.join("\n")
}

/// O texto de um nó em particular, que é o que a linha da composição carrega.
pub fn texto_do_no(no: &NoDocumento) -> String {
  no.texto_renderizado()
}

/// Reconhece o marcador de citação que o exportador escreve.
///
/// Fica aqui para o chamador ter os dois lados no mesmo arquivo; a forma do
/// marcador é do modelo ([`ler_marcador_de_citacao`]).
pub use super::documento::ler_marcador_de_citacao;
/// O mesmo para o marcador de fragmento.
pub use super::documento::ler_marcador_de_fragmento;

// ---------------------------------------------------------------------------
// Escrita
// ---------------------------------------------------------------------------

/// As opções de saída: o que o exportador assume sobre o documento.
///
/// Título e autor vão para os metadados do arquivo (`D78`: metadados são lidos e
/// **reportados** na importação, não viram dado do projeto).
pub struct OpcoesDeExportacao<'a> {
  pub titulo: &'a str,
  pub autor: &'a str,
  pub geometria: Geometria,
  pub agora: DateTime<Utc>,
}

/// Escreve o documento como pacote OOXML.
///
/// O que sai é o **modelo interno**, não uma cópia do que o app mostra: o sumário,
/// a numeração de capítulos e a lista de referências são gerados, e por isso não
/// são gravados no arquivo (`D78`, contrato de tolerância).
pub fn escrever(
  arvore: &[NoDocumento],
  opcoes: OpcoesDeExportacao<'_>,
) -> Result<Vec<u8>, DocxErro> {
  let partes = vec![
    ("[Content_Types].xml", content_types().to_string()),
    ("_rels/.rels", rels_raiz().to_string()),
    ("word/document.xml", document_xml(arvore, &opcoes.geometria)),
    ("word/styles.xml", styles_xml()),
    ("word/_rels/document.xml.rels", rels_documento().to_string()),
    (
      "docProps/core.xml",
      core_xml(opcoes.titulo, opcoes.autor, opcoes.agora),
    ),
    ("docProps/app.xml", app_xml()),
  ];

  let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
  let compressao = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
  for (nome, conteudo) in partes {
    writer
      .start_file(nome, compressao)
      .map_err(|e| DocxErro::PacoteInvalido(e.to_string()))?;
    writer
      .write_all(conteudo.as_bytes())
      .map_err(|e| DocxErro::PacoteInvalido(e.to_string()))?;
  }
  let cursor = writer
    .finish()
    .map_err(|e| DocxErro::PacoteInvalido(e.to_string()))?;
  Ok(cursor.into_inner())
}

/// O `word/document.xml`: o corpo, na ordem da árvore.
///
/// O título de capítulo vira `Heading1..9` pela profundidade, com `w:b` no run
/// para o arquivo abrir legível mesmo em visualizador que não aplica o estilo.
fn document_xml(arvore: &[NoDocumento], geometria: &Geometria) -> String {
  let mut corpo = String::new();
  escrever_corpo(arvore, 1, &mut corpo);
  format!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<w:document xmlns:w=\"{NS_W}\" xmlns:r=\"{NS_R}\"><w:body>{corpo}{}</w:body></w:document>",
    sect_pr(geometria)
  )
}

fn escrever_corpo(arvore: &[NoDocumento], nivel: usize, saida: &mut String) {
  for no in arvore {
    match no {
      NoDocumento::Capitulo { titulo, filhos, .. } => {
        let estilo = format!("Heading{}", nivel.min(9));
        saida.push_str(&format!(
          "<w:p><w:pPr><w:pStyle w:val=\"{estilo}\"/></w:pPr>{}</w:p>",
          run("<w:rPr><w:b/></w:rPr>", titulo)
        ));
        escrever_corpo(filhos, nivel + 1, saida);
      }
      NoDocumento::Paragrafo { inline, .. } => {
        let runs: String = inline
          .iter()
          .map(|r| run(&r_pr(&r.marcas), &r.texto))
          .collect();
        saida.push_str(&format!("<w:p>{}{runs}</w:p>", p_pr_corpo()));
      }
      NoDocumento::Citacao {
        referencia_id,
        modo,
        ..
      } => {
        saida.push_str(&format!(
          "<w:p><w:pPr><w:pStyle w:val=\"CecistudyVinculo\"/></w:pPr>{}</w:p>",
          run("", &marcador_de_citacao(referencia_id, *modo))
        ));
      }
      NoDocumento::ReferenciaViva { fragmento_id, .. } => {
        saida.push_str(&format!(
          "<w:p><w:pPr><w:pStyle w:val=\"CecistudyVinculo\"/></w:pPr>{}</w:p>",
          run("", &format!("[fragmento: {fragmento_id}]"))
        ));
      }
    }
  }
}

/// As propriedades de parágrafo do corpo: justificado e entrelinha 1,5.
///
/// A entrelinha sai da geometria (`w:line` é em centésimos de linha com
/// `lineRule="auto"`), então DOCX e PDF paginam com a mesma regra declarada.
fn p_pr_corpo() -> String {
  // 1,5 linha = 240 com `lineRule="auto"` (240/240 = 1,0; 360/240 = 1,5).
  "<w:pPr><w:jc w:val=\"both\"/><w:spacing w:after=\"120\" w:line=\"360\" w:lineRule=\"auto\"/></w:pPr>".to_string()
}

fn r_pr(marcas: &[MarcaInline]) -> String {
  if marcas.is_empty() {
    return String::new();
  }
  let mut partes: Vec<&str> = Vec::new();
  for marca in marcas {
    partes.push(match marca {
      MarcaInline::Negrito => "<w:b/>",
      MarcaInline::Italico => "<w:i/>",
      MarcaInline::Sublinhado => "<w:u w:val=\"single\"/>",
      MarcaInline::Sobrescrito => "<w:vertAlign w:val=\"superscript\"/>",
      MarcaInline::Subscrito => "<w:vertAlign w:val=\"subscript\"/>",
    });
  }
  format!("<w:rPr>{}</w:rPr>", partes.join(""))
}

fn run(r_pr: &str, texto: &str) -> String {
  format!(
    "<w:r>{r_pr}<w:t xml:space=\"preserve\">{}</w:t></w:r>",
    escapar(texto)
  )
}

/// A geometria da página em `w:sectPr`, na mesma unidade que o Word usa.
fn sect_pr(geometria: &Geometria) -> String {
  let twips = |mm: f64| (mm * MM_PARA_TWIPS).round() as u64;
  format!(
    "<w:sectPr><w:pgSz w:w=\"{}\" w:h=\"{}\"/><w:pgMar w:top=\"{}\" w:right=\"{}\" w:bottom=\"{}\" w:left=\"{}\" w:header=\"708\" w:footer=\"708\" w:gutter=\"0\"/></w:sectPr>",
    twips(geometria.largura_mm),
    twips(geometria.altura_mm),
    twips(geometria.margem_sup_mm),
    twips(geometria.margem_dir_mm),
    twips(geometria.margem_inf_mm),
    twips(geometria.margem_esq_mm),
  )
}

/// Escapa o texto para XML. Caractere de controle fora do XML é **removido** e
/// isso é declarado no contrato de tolerância, nunca silenciado.
fn escapar(texto: &str) -> String {
  let mut saida = String::with_capacity(texto.len());
  for caractere in texto.chars() {
    match caractere {
      '&' => saida.push_str("&amp;"),
      '<' => saida.push_str("&lt;"),
      '>' => saida.push_str("&gt;"),
      '\t' | '\n' | '\r' => saida.push(' '),
      c if (c as u32) < 0x20 => {}
      c => saida.push(c),
    }
  }
  saida
}

fn content_types() -> String {
  format!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Types xmlns=\"{NS_CT}\">\
<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
<Default Extension=\"xml\" ContentType=\"application/xml\"/>\
<Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/>\
<Override PartName=\"/word/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/>\
<Override PartName=\"/docProps/core.xml\" ContentType=\"application/vnd.openxmlformats-package.core-properties+xml\"/>\
<Override PartName=\"/docProps/app.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.extended-properties+xml\"/>\
</Types>"
  )
}

fn rels_raiz() -> String {
  format!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Relationships xmlns=\"{NS_REL}\">\
<Relationship Id=\"rId1\" Type=\"{NS_R}/officeDocument\" Target=\"word/document.xml\"/>\
<Relationship Id=\"rId2\" Type=\"{NS_REL}/metadata/core-properties\" Target=\"docProps/core.xml\"/>\
<Relationship Id=\"rId3\" Type=\"{NS_REL}/extended-properties\" Target=\"docProps/app.xml\"/>\
</Relationships>"
  )
}

fn rels_documento() -> String {
  format!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Relationships xmlns=\"{NS_REL}\">\
<Relationship Id=\"rId1\" Type=\"{NS_R}/styles\" Target=\"styles.xml\"/>\
</Relationships>"
  )
}

fn styles_xml() -> String {
  let mut estilos = String::new();
  estilos.push_str("<w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\"><w:name w:val=\"Normal\"/></w:style>");
  for nivel in 1..=9u32 {
    estilos.push_str(&format!(
      "<w:style w:type=\"paragraph\" w:styleId=\"Heading{nivel}\"><w:name w:val=\"heading {nivel}\"/><w:rPr><w:b/></w:rPr></w:style>"
    ));
  }
  estilos.push_str("<w:style w:type=\"paragraph\" w:styleId=\"CecistudyVinculo\"><w:name w:val=\"Cecistudy Vinculo\"/><w:pPr><w:ind w:left=\"720\"/></w:pPr></w:style>");
  format!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<w:styles xmlns:w=\"{NS_W}\">{estilos}</w:styles>"
  )
}

fn core_xml(titulo: &str, autor: &str, agora: DateTime<Utc>) -> String {
  format!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<cp:coreProperties xmlns:cp=\"{NS_CP}\" xmlns:dc=\"{NS_DC}\" xmlns:dcterms=\"{NS_DCTERMS}\" xmlns:xsi=\"{NS_XSI}\">\
<dc:title>{titulo}</dc:title>\
<dc:creator>{autor}</dc:creator>\
<cp:lastModifiedBy>{autor}</cp:lastModifiedBy>\
<dcterms:created xsi:type=\"dcterms:W3CDTF\">{criado}</dcterms:created>\
<dcterms:modified xsi:type=\"dcterms:W3CDTF\">{criado}</dcterms:modified>\
</cp:coreProperties>",
    titulo = escapar(titulo),
    autor = escapar(autor),
    criado = agora.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
  )
}

/// O `docProps/app.xml`: o aplicativo que gravou, para o relatório de metadados
/// do contrato de tolerância (`D78`). É reportado, nunca vira dado do projeto.
fn app_xml() -> String {
  "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Properties xmlns=\"http://schemas.openxmlformats.org/officeDocument/2006/extended-properties\">\
<Application>cecistudy</Application>\
<AppVersion>1.0000</AppVersion>\
</Properties>"
    .to_string()
}

// ---------------------------------------------------------------------------
// Leitura
// ---------------------------------------------------------------------------

/// O que a importação produziu: a fila de decisão e o relatório.
///
/// Os dois andam juntos porque `D78` exige que **perder seja declarado**: um
/// relatório sem contagem é a forma de esconder que algo não encaixou.
#[derive(Debug, Clone, PartialEq)]
pub struct Importacao {
  pub itens: Vec<ItemAResolver>,
  pub relatorio: Relatorio,
}

/// O relatório do contrato de tolerância. Cada campo é um item da lista fechada
/// de `D78`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Relatorio {
  /// Comentários (`w:comment`) lidos.
  pub comentarios: u32,
  /// Alterações (`w:ins` + `w:del`) lidas como sugestão.
  pub sugestoes: u32,
  /// Itens cujo trecho citado não existe mais no texto atual.
  pub nao_ancorados: u32,
  /// Imagens embutidas — não suportadas na v1.
  pub imagens: u32,
  /// Tabelas do Word — a sequência de texto entra, a estrutura é perdida.
  pub tabelas: u32,
  /// Notas de rodapé e de fim — reportadas, não importadas.
  pub notas: u32,
  /// Constructos que o contrato não cobre.
  pub nao_reconhecidos: u32,
  /// Autor declarado nos metadados do arquivo.
  pub autor_do_arquivo: Option<String>,
  /// Aplicativo que gravou o arquivo, quando declarado.
  pub aplicativo: Option<String>,
}

/// Lê um DOCX e devolve **itens a resolver**. Nunca devolve árvore (`D78`).
///
/// `texto_do_documento` é o texto atual do modelo ([`texto_plano`]): é contra ele
/// que a âncora por texto citado é verificada, e é por isso que a assinatura o
/// recebe em vez de lê-lo do banco — o motor não conhece banco.
pub fn ler(bytes: &[u8], texto_do_documento: &str, agora_ms: i64) -> Result<Importacao, DocxErro> {
  let mut pacote =
    ZipArchive::new(Cursor::new(bytes)).map_err(|e| DocxErro::PacoteInvalido(e.to_string()))?;

  let documento = ler_parte(&mut pacote, "word/document.xml")?;
  let comentarios = ler_parte_opcional(&mut pacote, "word/comments.xml")?;
  let core = ler_parte_opcional(&mut pacote, "docProps/core.xml")?;
  let app = ler_parte_opcional(&mut pacote, "docProps/app.xml")?;

  let mut relatorio = Relatorio {
    autor_do_arquivo: autor_dos_metadados(&core),
    aplicativo: aplicativo_dos_metadados(&app),
    ..Relatorio::default()
  };

  let mut coletor = Coletor::default();
  percorrer_documento(&documento, &mut coletor, &mut relatorio)
    .map_err(|e| DocxErro::Xml(e.to_string()))?;

  let mapa_de_comentarios = if comentarios.is_empty() {
    Vec::new()
  } else {
    ler_comentarios(&comentarios).map_err(|e| DocxErro::Xml(e.to_string()))?
  };

  // A ordem é a do arquivo: comentário e sugestão entram na ordem em que o
  // orientador escreveu, e não reordenados por tipo. Reordenar esconderia a
  // sequência do ciclo.
  let mut itens: Vec<ItemAResolver> = Vec::new();
  for evento in coletor.eventos {
    let (tipo, autor, data, citado, texto) = match evento {
      EventoLido::Comentario {
        id,
        autor,
        data,
        citado,
        texto,
      } => {
        // O autor e a data podem vir do `w:comment` (texto) ou do
        // `commentReference`. O `comments.xml` é a fonte: é ele que guarda o
        // corpo do comentário.
        match mapa_de_comentarios.iter().find(|c| c.id == id) {
          Some(c) => {
            let citado = if citado.is_empty() {
              c.citado.clone()
            } else {
              citado
            };
            let texto = if c.texto.is_empty() {
              Some(texto)
            } else {
              Some(c.texto.clone())
            };
            (TipoItem::Comentario, c.autor.clone(), c.data, citado, texto)
          }
          // Comentário sem corpo legível: entra **sem texto**, nunca é descartado
          // em silêncio (`D78`: o arquivo inteiro não é recusado por um item).
          None => (
            TipoItem::Comentario,
            autor.clone(),
            data,
            citado,
            Some(texto),
          ),
        }
      }
      EventoLido::Sugestao {
        autor,
        data,
        citado,
        texto,
      } => {
        // A âncora viva é o trecho **excluído**; o substituto é o **inserido**.
        // Cada um só existe quando o controle correspondente veio no arquivo:
        // `w:del` sem `w:ins` é sugestão de exclusão sem substituto, e `None` é
        // o jeito honesto de dizer isso.
        (
          TipoItem::Sugestao,
          autor,
          data,
          citado,
          if texto.is_empty() { None } else { Some(texto) },
        )
      }
    };

    let autor_resolvido = !autor.trim().is_empty();
    let encaixou = !citado.trim().is_empty() && texto_do_documento.contains(citado.as_str());
    if encaixou {
      // (a lista abaixo só marca `NaoEncaixou` no caso contrário)
    } else {
      relatorio.nao_ancorados += 1;
    }

    let mut item = ItemAResolver::pendente(
      uuid::Uuid::new_v4().to_string(),
      "",
      tipo,
      autor.trim(),
      autor_resolvido,
      data,
      citado,
      texto,
      Some("<arquivo importado>".to_string()),
      agora_ms,
    );
    if !encaixou {
      item.estado = super::item::EstadoItem::NaoEncaixou;
      item.resolvido_em = Some(agora_ms);
    }
    itens.push(item);
  }

  relatorio.comentarios = coletor.comentarios as u32;
  relatorio.sugestoes = coletor.sugestoes as u32;

  Ok(Importacao { itens, relatorio })
}

fn ler_parte(
  pacote: &mut ZipArchive<Cursor<&[u8]>>,
  nome: &'static str,
) -> Result<String, DocxErro> {
  let mut entrada = pacote
    .by_name(nome)
    .map_err(|_| DocxErro::ParteAusente(nome))?;
  let mut texto = String::new();
  entrada
    .read_to_string(&mut texto)
    .map_err(|e| DocxErro::PacoteInvalido(e.to_string()))?;
  Ok(texto)
}

/// Parte que pode faltar: `comments.xml` não existe quando o arquivo não tem
/// comentário, e `docProps/core.xml` é opcional no formato. Ausente é **vazio**,
/// não é erro — recusar o arquivo por causa de metadado ausente perderia os
/// comentários, e `D78` manda ler o que houver.
fn ler_parte_opcional(
  pacote: &mut ZipArchive<Cursor<&[u8]>>,
  nome: &str,
) -> Result<String, DocxErro> {
  let mut entrada = match pacote.by_name(nome) {
    Ok(entrada) => entrada,
    Err(_) => return Ok(String::new()),
  };
  let mut texto = String::new();
  entrada
    .read_to_string(&mut texto)
    .map_err(|e| DocxErro::PacoteInvalido(e.to_string()))?;
  Ok(texto)
}

/// Um item cru do arquivo, antes de virar item a resolver.
#[derive(Debug, Clone, PartialEq)]
enum EventoLido {
  Comentario {
    id: String,
    autor: String,
    data: Option<i64>,
    citado: String,
    texto: String,
  },
  Sugestao {
    autor: String,
    data: Option<i64>,
    /// O trecho **excluído** (`w:del`) é a âncora; o **inserido** (`w:ins`) é o
    /// substituto. É a cláusula de `D78`, escrita na ordem do arquivo.
    citado: String,
    texto: String,
  },
}

#[derive(Debug, Default)]
struct Coletor {
  eventos: Vec<EventoLido>,
  comentarios: usize,
  sugestoes: usize,
}

/// O que se está lendo **agora**, para fechar a âncora no fim.
#[derive(Debug, Default)]
struct Pilha {
  /// `w:ins`/`w:del` aberta: guarda autor, data e o texto acumulado.
  alteracao: Option<(String, Option<i64>, String, bool)>,
  /// `w:comment` aberta: guarda o texto acumulado desde o range.
  comentario: Option<(String, Option<i64>, String)>,
}

fn percorrer_documento(
  xml: &str,
  coletor: &mut Coletor,
  relatorio: &mut Relatorio,
) -> Result<(), quick_xml::Error> {
  let mut reader = Reader::from_str(xml);
  let mut pilha = Pilha::default();
  let mut texto_citado: Vec<String> = Vec::new();

  loop {
    match reader.read_event()? {
      Event::Eof => break,
      Event::Start(inicio) | Event::Empty(inicio) => match inicio.local_name().as_ref() {
        "commentRangeStart" => {
          let id = atributo(&inicio, "id").unwrap_or_default();
          pilha.comentario = Some((id, None, String::new()));
          texto_citado.clear();
        }
        "commentRangeEnd" => {
          if let Some((id, data, texto)) = pilha.comentario.take() {
            coletor.eventos.push(EventoLido::Comentario {
              id,
              autor: String::new(),
              data,
              citado: texto_citado.join(""),
              texto,
            });
            coletor.comentarios += 1;
          }
          texto_citado.clear();
        }
        "ins" | "del" => {
          let autor = atributo(&inicio, "author").unwrap_or_default();
          let data = data_de(atributo(&inicio, "date").as_deref());
          let eh_del = inicio.local_name().as_ref() == "del";
          pilha.alteracao = Some((autor, data, String::new(), eh_del));
        }
        "drawing" | "pict" => relatorio.imagens += 1,
        "tbl" => relatorio.tabelas += 1,
        "footnoteReference" | "endnoteReference" => relatorio.notas += 1,
        _ => {}
      },
      Event::Text(bruto) => {
        let texto = bruto
          .xml_content(quick_xml::XmlVersion::Explicit1_0)
          .into_owned();
        if let Some((_, _, acumulado, _)) = pilha.alteracao.as_mut() {
          acumulado.push_str(&texto);
        }
        if let Some((_, _, acumulado)) = pilha.comentario.as_mut() {
          acumulado.push_str(&texto);
        }
        texto_citado.push(texto);
      }
      Event::End(fim) => match fim.local_name().as_ref() {
        "ins" | "del" => {
          if let Some((autor, data, acumulado, eh_del)) = pilha.alteracao.take() {
            // `w:del` guarda o trecho excluído em `w:delText`, que chega como
            // evento de texto como qualquer outro; `w:ins` usa `w:t`. A cláusula
            // de `D78` fixa a ordem: o **excluído** é a âncora (`citado`) e o
            // **inserido** é o substituto (`texto`).
            if eh_del {
              coletor.eventos.push(EventoLido::Sugestao {
                autor,
                data,
                citado: acumulado,
                texto: String::new(),
              });
            } else {
              coletor.eventos.push(EventoLido::Sugestao {
                autor,
                data,
                citado: String::new(),
                texto: acumulado,
              });
            }
            coletor.sugestoes += 1;
          }
        }
        "t" | "delText" => {}
        _ => {}
      },
      // `Decl`, `PI`, `DocType`, `CData` e `Comment` não abrem nem fecham
      // âncora nem alteração: entram aqui sem efeito.
      _ => {}
    }
  }
  Ok(())
}

/// Lê `word/comments.xml`: id, autor, data e o corpo de cada comentário.
fn ler_comentarios(xml: &str) -> Result<Vec<ComentarioDoArquivo>, quick_xml::Error> {
  let mut reader = Reader::from_str(xml);
  let mut saida: Vec<ComentarioDoArquivo> = Vec::new();
  let mut atual: Option<ComentarioDoArquivo> = None;

  loop {
    match reader.read_event()? {
      Event::Eof => break,
      Event::Start(inicio) | Event::Empty(inicio) => {
        if inicio.local_name().as_ref() == "comment" {
          atual = Some(ComentarioDoArquivo {
            id: atributo(&inicio, "id").unwrap_or_default(),
            autor: atributo(&inicio, "author").unwrap_or_default(),
            data: data_de(atributo(&inicio, "date").as_deref()),
            texto: String::new(),
            citado: String::new(),
          });
        }
      }
      Event::Text(bruto) => {
        if let Some(c) = atual.as_mut() {
          c.texto
            .push_str(&bruto.xml_content(quick_xml::XmlVersion::Explicit1_0));
        }
      }
      Event::End(fim) => {
        if fim.local_name().as_ref() == "comment"
          && let Some(c) = atual.take()
        {
          saida.push(c);
        }
      }
      // `Decl`, `PI`, `DocType`, `CData` e `Comment` não carrega id, autor nem
      // data de comentário: entram aqui e não mudam nada.
      _ => {}
    }
  }
  Ok(saida)
}

/// Um comentário como veio em `word/comments.xml`.
#[derive(Debug, Clone, PartialEq)]
struct ComentarioDoArquivo {
  id: String,
  autor: String,
  data: Option<i64>,
  texto: String,
  /// Reservado: o Word não guarda a âncora no `comments.xml`, e sim no
  /// `document.xml`, por `commentRangeStart`/`End`. O campo existe para deixar
  /// isso escrito em vez de implícito.
  citado: String,
}

fn atributo(inicio: &quick_xml::events::BytesStart<'_>, chave: &str) -> Option<String> {
  for atributo in inicio.attributes().flatten() {
    if atributo.key.local_name().as_ref() == chave {
      return atributo
        .normalized_value(quick_xml::XmlVersion::Explicit1_0)
        .ok()
        .map(|v| v.into_owned());
    }
  }
  None
}

/// `w:date` é RFC 3339. Data ausente é `None`, nunca "agora" (`D88`).
fn data_de(bruto: Option<&str>) -> Option<i64> {
  let bruto = bruto?;
  DateTime::parse_from_rfc3339(bruto)
    .ok()
    .map(|d| d.timestamp_millis())
}

fn autor_dos_metadados(core: &str) -> Option<String> {
  metadado(core, "creator").or_else(|| metadado(core, "lastModifiedBy"))
}

fn aplicativo_dos_metadados(core: &str) -> Option<String> {
  metadado(core, "Application")
}

/// Lê o texto de uma etiqueta de metadados, sem `quick-xml` para um campo só.
fn metadado(xml: &str, etiqueta: &str) -> Option<String> {
  // O fechamento pode vir com prefixo de namespace (`</dc:creator>`) ou sem
  // (`</Application>`), então o que casa é o **nome local**.
  let mut de = 0usize;
  while let Some(pos) = xml[de..].find("</") {
    let inicio = de + pos;
    let Some(fim_tag) = xml[inicio..].find('>') else {
      break;
    };
    let nome = &xml[inicio + 2..inicio + fim_tag];
    let local = nome.rsplit(':').next().unwrap_or(nome);
    if local == etiqueta {
      // Achou o fechamento. A abertura é o último `<` antes dele — os metadados
      // de pacote são chatos (`dc:creator`, `cp:lastModifiedBy`), sem aninhamento.
      let antes = &xml[..inicio];
      let abertura = antes.rfind('<')?;
      let fim_abre = antes[abertura..].find('>')? + abertura + 1;
      let valor = antes[fim_abre..].trim();
      if valor.is_empty() {
        return None;
      }
      return Some(valor.to_string());
    }
    de = inicio + fim_tag + 1;
  }
  None
}

#[cfg(test)]
mod test {
  use super::super::documento::{Inline, ModoVinculo};
  use super::super::medidor::MedidorDeterministico;
  use super::super::paginacao::Geometria;
  use super::*;

  /// Um documento de teste com os quatro tipos de nó.
  fn arvore_de_teste() -> Vec<NoDocumento> {
    vec![
      NoDocumento::Capitulo {
        id: "c1".into(),
        titulo: "Introdução".into(),
        filhos: vec![NoDocumento::Paragrafo {
          id: "p1".into(),
          inline: vec![Inline::simples("A pesquisa parte da leitura.")],
        }],
      },
      NoDocumento::Citacao {
        id: "cit".into(),
        referencia_id: "ref-42".into(),
        modo: ModoVinculo::Vivo,
      },
      NoDocumento::ReferenciaViva {
        id: "frag".into(),
        fragmento_id: "frag-7".into(),
      },
    ]
  }

  fn exportar(arvore: &[NoDocumento]) -> Vec<u8> {
    escrever(
      arvore,
      OpcoesDeExportacao {
        titulo: "TCC da usuária",
        autor: "usuária",
        geometria: Geometria::a4_abnt(),
        agora: chrono::DateTime::from_timestamp(1_800_000_000, 0).unwrap(),
      },
    )
    .expect("escreve")
  }

  /// Monta um pacote OOXML de teste, com as partes que o leitor usa.
  fn pacote_de_teste(documento: &str, comentarios: Option<&str>, core: Option<&str>) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let opcoes = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    for (nome, conteudo) in [
      ("[Content_Types].xml", content_types().as_str()),
      ("_rels/.rels", rels_raiz().as_str()),
      ("word/document.xml", documento),
      ("word/_rels/document.xml.rels", rels_documento().as_str()),
      (
        "docProps/core.xml",
        core.unwrap_or("<cp:coreProperties xmlns:cp=\"x\"><dc:creator>Orientador</dc:creator></cp:coreProperties>"),
      ),
    ] {
      writer.start_file(nome, opcoes).expect("abre parte");
      writer.write_all(conteudo.as_bytes()).expect("grava parte");
    }
    if let Some(comentarios) = comentarios {
      writer
        .start_file("word/comments.xml", opcoes)
        .expect("abre");
      writer.write_all(comentarios.as_bytes()).expect("grava");
    }
    writer.finish().expect("fecha").into_inner()
  }

  /// O texto que o **escritor** pôs no `word/document.xml`, lido de volta por um
  /// caminho que não é o do modelo: varre os `<w:t>` do XML.
  ///
  /// Não usa `texto_plano` de propósito — o teste de paridade (`I3`) só prova
  /// alguma quando os dois lados são lidos por caminhos independentes.
  fn texto_do_docx(bytes: &[u8]) -> Vec<String> {
    let mut pacote = ZipArchive::new(Cursor::new(bytes)).expect("zip");
    let mut documento = String::new();
    pacote
      .by_name("word/document.xml")
      .expect("documento")
      .read_to_string(&mut documento)
      .expect("lê");
    let mut saida = Vec::new();
    let mut resto = documento.as_str();
    while let Some(inicio) = resto.find("<w:t") {
      let depois_tag = &resto[inicio..];
      let abertura_fim = depois_tag.find('>').expect("fecha a tag") + 1;
      let conteudo = &depois_tag[abertura_fim..];
      let fim = conteudo.find("</w:t>").expect("fecha w:t");
      saida.push(desescapar(&conteudo[..fim]));
      resto = &conteudo[fim + "</w:t>".len()..];
    }
    saida
  }

  fn desescapar(texto: &str) -> String {
    // Inversa de `escapar` (`docx.rs:253`). A ordem importa: `&amp;` vem por
    // último, senão o texto literal `&lt;` viraria `<` em duas passadas.
    texto
      .replace("&lt;", "<")
      .replace("&gt;", ">")
      .replace("&amp;", "&")
  }

  #[test]
  fn o_pacote_escrito_e_relido_sem_itens() {
    let arvore = arvore_de_teste();
    let bytes = exportar(&arvore);
    let texto = texto_plano(&arvore);
    let importacao = ler(&bytes, &texto, 1_000).expect("lê o que ela mesma escreveu");

    assert!(
      importacao.itens.is_empty(),
      "sem comentário nem revisão no arquivo"
    );
    assert_eq!(importacao.relatorio.comentarios, 0);
    assert_eq!(importacao.relatorio.sugestoes, 0);
    assert_eq!(importacao.relatorio.nao_ancorados, 0);
    assert_eq!(
      importacao.relatorio.autor_do_arquivo.as_deref(),
      Some("usuária"),
      "os metadados do arquivo são lidos e reportados"
    );
    assert_eq!(
      importacao.relatorio.aplicativo.as_deref(),
      Some("cecistudy")
    );
  }

  #[test]
  fn i3_docx_e_pdf_tem_a_mesma_sequencia_de_texto() {
    let arvore = arvore_de_teste();
    let bytes = exportar(&arvore);
    let (_, _) = super::super::pdf::escrever(
      &arvore,
      &Geometria::a4_abnt(),
      &MedidorDeterministico::novo(),
      "TCC da usuária",
      chrono::DateTime::from_timestamp(1_800_000_000, 0).unwrap(),
    )
    .expect("renderiza");

    // O texto do DOCX, pelo varredor de `<w:t>` acima.
    let do_docx = texto_do_docx(&bytes);
    // O texto do PDF, lido do arquivo como um leitor leria.
    let do_pdf = texto_do_pdf(&bytes_pdf(&arvore));

    assert_eq!(do_docx, do_pdf, "I3: os dois saem do mesmo modelo");
    assert!(do_docx.contains(&"Introdução".to_string()));
    assert!(do_docx.contains(&"A pesquisa parte da leitura.".to_string()));
    assert!(
      do_docx.contains(&"[citação: ref-42 · vivo]".to_string()),
      "o marcador de vínculo é texto renderizado, e a ida e volta dele é testada"
    );
  }

  fn bytes_pdf(arvore: &[NoDocumento]) -> Vec<u8> {
    super::super::pdf::escrever(
      arvore,
      &Geometria::a4_abnt(),
      &MedidorDeterministico::novo(),
      "TCC da usuária",
      chrono::DateTime::from_timestamp(1_800_000_000, 0).unwrap(),
    )
    .expect("renderiza")
    .0
  }

  /// O texto que o PDF carrega, lido do arquivo gerado.
  fn texto_do_pdf(bytes: &[u8]) -> Vec<String> {
    let bruto = String::from_utf8_lossy(bytes);
    bruto
      .split(") Tj")
      .filter_map(|fatia| {
        let inicio = fatia.rfind('(')?;
        Some(desescapar_pdf(fatia[inicio + 1..].to_string()))
      })
      .collect()
  }

  fn desescapar_pdf(texto: String) -> String {
    texto
      .replace("\\(", "(")
      .replace("\\)", ")")
      .replace("\\\\", "\\")
  }

  #[test]
  fn i4_a_importacao_nao_altera_o_texto_do_documento() {
    let arvore = arvore_de_teste();
    let antes = texto_plano(&arvore);

    // Um arquivo com comentário **e** revisão de alterações: o caso que mais
    // tentaria reescrever o texto se a importação fosse substituição.
    let documento = "<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>\
<w:p><w:r><w:t>A pesquisa parte da leitura.</w:t></w:r></w:p>\
<w:p><w:ins w:id=\"1\" w:author=\"Orientador\" w:date=\"2026-10-05T14:23:00Z\"><w:r><w:t>melhor</w:t></w:r></w:ins></w:p>\
</w:body></w:document>";
    let bytes = pacote_de_teste(documento, None, None);

    let importacao = ler(&bytes, &antes, 1_000).expect("lê");
    let depois = texto_plano(&arvore);
    assert_eq!(antes, depois, "I4: importar é anexar, nunca sobrescrever");
    assert!(
      !importacao.itens.is_empty(),
      "o comentário do arquivo entrou"
    );
  }

  #[test]
  fn i5_comentario_ancora_pelo_texto_citado() {
    let documento = "<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>\
<w:p><w:commentRangeStart w:id=\"7\"/><w:r><w:t>A pesquisa parte da leitura.</w:t></w:r><w:commentRangeEnd w:id=\"7\"/></w:p>\
</w:body></w:document>";
    let comentarios = "<w:comments xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\
<w:comment w:id=\"7\" w:author=\"Orientador\" w:date=\"2026-10-05T14:23:00Z\"><w:p><w:r><w:t>Reescreva a frase.</w:t></w:r></w:p></w:comment>\
</w:comments>";
    let bytes = pacote_de_teste(documento, Some(comentarios), None);
    let texto = "Introdução\nA pesquisa parte da leitura.";

    let importacao = ler(&bytes, texto, 1_000).expect("lê");
    assert_eq!(importacao.itens.len(), 1);
    let item = &importacao.itens[0];
    assert_eq!(item.tipo, TipoItem::Comentario);
    assert_eq!(item.autor, "Orientador");
    assert!(item.autor_resolvido, "autor com nome é resolvido");
    assert_eq!(item.citado, "A pesquisa parte da leitura.");
    assert_eq!(item.texto.as_deref(), Some("Reescreva a frase."));
    assert_eq!(
      item.data_autor,
      Some(
        chrono::DateTime::parse_from_rfc3339("2026-10-05T14:23:00Z")
          .unwrap()
          .timestamp_millis()
      )
    );
    assert!(item.esta_pendente());
    assert_eq!(
      importacao.relatorio.nao_ancorados, 0,
      "a âncora existe no texto"
    );
  }

  #[test]
  fn i6_trecho_inexistente_e_reportado_nao_silenciado() {
    let documento = "<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>\
<w:p><w:commentRangeStart w:id=\"1\"/><w:r><w:t>trecho que o orientador reescreveu</w:t></w:r><w:commentRangeEnd w:id=\"1\"/></w:p>\
</w:body></w:document>";
    let comentarios = "<w:comments xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\
<w:comment w:id=\"1\" w:author=\"Orientador\" w:date=\"2026-10-05T14:23:00Z\"><w:p><w:r><w:t>Onde isso foi parar?</w:t></w:r></w:p></w:comment>\
</w:comments>";
    let bytes = pacote_de_teste(documento, Some(comentarios), None);
    let texto = "O texto atual não tem esse trecho.";

    let importacao = ler(&bytes, texto, 1_000).expect("lê");
    assert_eq!(importacao.itens.len(), 1, "o item entra, mesmo órfão");
    assert_eq!(
      importacao.itens[0].estado,
      super::super::item::EstadoItem::NaoEncaixou
    );
    assert_eq!(
      importacao.relatorio.nao_ancorados, 1,
      "I6: a âncora perdida é contada, nunca silenciada"
    );
  }

  #[test]
  fn i7_revisao_de_alteracoes_vira_sugestao_com_autor_e_data() {
    let documento = "<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>\
<w:p><w:del w:id=\"1\" w:author=\"Orientador\" w:date=\"2026-10-05T09:00:00Z\"><w:r><w:delText>texto velho</w:delText></w:r></w:del></w:p>\
<w:p><w:ins w:id=\"2\" w:author=\"Orientador\" w:date=\"2026-10-05T09:01:00Z\"><w:r><w:t>texto novo</w:t></w:r></w:ins></w:p>\
</w:body></w:document>";
    let bytes = pacote_de_teste(documento, None, None);
    let texto = "texto velho e mais nada";

    let importacao = ler(&bytes, texto, 1_000).expect("lê");
    assert_eq!(importacao.itens.len(), 2, "uma exclusão e uma inserção");
    assert_eq!(importacao.relatorio.sugestoes, 2);
    assert!(
      importacao
        .itens
        .iter()
        .all(|i| i.tipo == TipoItem::Sugestao),
      "I7: revisão é sugestão, tipo distinto"
    );

    let por_citado: Vec<&ItemAResolver> = importacao
      .itens
      .iter()
      .filter(|i| i.citado == "texto velho")
      .collect();
    assert_eq!(por_citado.len(), 1, "o excluído é a âncora (D78)");
    assert_eq!(por_citado[0].texto.as_deref(), None);

    let por_texto: Vec<&ItemAResolver> = importacao
      .itens
      .iter()
      .filter(|i| i.texto.as_deref() == Some("texto novo"))
      .collect();
    assert_eq!(por_texto.len(), 1, "o inserido é o substituto");
    assert!(por_texto[0].citado.is_empty());
    assert_eq!(por_texto[0].autor, "Orientador");
    assert!(
      por_texto[0].data_autor.is_some(),
      "a data do controle é preservada"
    );
  }

  #[test]
  fn i8_reimportar_o_mesmo_arquivo_acha_o_mesmo_item() {
    // A duplicata é decidida contra o que o projeto já tem (`D88`), e a chave da
    // comparação é autor + data + texto citado. O motor tem de devolver a mesma
    // chave para o mesmo arquivo — senão a checagem do comando nunca casa.
    let documento = "<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>\
<w:p><w:commentRangeStart w:id=\"7\"/><w:r><w:t>mesmo trecho</w:t></w:r><w:commentRangeEnd w:id=\"7\"/></w:p>\
</w:body></w:document>";
    let comentarios = "<w:comments xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\
<w:comment w:id=\"7\" w:author=\"Orientador\" w:date=\"2026-10-05T14:23:00Z\"><w:p><w:r><w:t>De novo.</w:t></w:r></w:p></w:comment>\
</w:comments>";
    let bytes = pacote_de_teste(documento, Some(comentarios), None);
    let texto = "mesmo trecho";

    let primeira = ler(&bytes, texto, 1_000).expect("lê");
    let segunda = ler(&bytes, texto, 2_000).expect("lê");
    let chave = |i: &ItemAResolver| (i.autor.clone(), i.data_autor, i.citado.clone());
    assert_eq!(primeira.itens.len(), 1);
    assert_eq!(segunda.itens.len(), 1);
    assert_eq!(chave(&primeira.itens[0]), chave(&segunda.itens[0]));
  }

  #[test]
  fn autor_desconhecido_entra_nao_resolvido_e_o_arquivo_nao_e_recusado() {
    let documento = "<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>\
<w:p><w:commentRangeStart w:id=\"1\"/><w:r><w:t>trecho</w:t></w:r><w:commentRangeEnd w:id=\"1\"/></w:p>\
<w:p><w:commentRangeStart w:id=\"2\"/><w:r><w:t>outro</w:t></w:r><w:commentRangeEnd w:id=\"2\"/></w:p>\
</w:body></w:document>";
    let comentarios = "<w:comments xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\
<w:comment w:id=\"1\" w:author=\"\" w:date=\"\"><w:p><w:r><w:t>sem autor</w:t></w:r></w:p></w:comment>\
<w:comment w:id=\"2\" w:author=\"Orientador\" w:date=\"\"><w:p><w:r><w:t>com autor</w:t></w:r></w:p></w:comment>\
</w:comments>";
    let bytes = pacote_de_teste(documento, Some(comentarios), None);
    let texto = "trecho\noutro";

    let importacao = ler(&bytes, texto, 1_000).expect("lê");
    assert_eq!(
      importacao.itens.len(),
      2,
      "o arquivo inteiro não é recusado por um item"
    );
    let sem_autor = importacao
      .itens
      .iter()
      .find(|i| i.texto.as_deref() == Some("sem autor"))
      .expect("o item sem autor entra");
    assert!(
      !sem_autor.autor_resolvido,
      "autor não identificado é visível"
    );
    assert_eq!(
      sem_autor.data_autor, None,
      "sem data é sem data, nunca \"agora\""
    );
  }

  #[test]
  fn imagens_tabelas_e_notas_sao_contadas_no_relatorio() {
    let documento = "<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>\
<w:p><w:drawing/></w:p><w:tbl><w:tr><w:tc><w:p><w:r><w:t>célula</w:t></w:r></w:p></w:tc></w:tr></w:tbl>\
<w:p><w:footnoteReference w:id=\"1\"/></w:p>\
</w:body></w:document>";
    let bytes = pacote_de_teste(documento, None, None);
    let importacao = ler(&bytes, "", 1_000).expect("lê");
    assert_eq!(importacao.relatorio.imagens, 1);
    assert_eq!(importacao.relatorio.tabelas, 1);
    assert_eq!(importacao.relatorio.notas, 1);
  }

  #[test]
  fn arquivo_que_nao_e_zip_e_recusado_com_mensagem_clara() {
    let err = ler(b"isto nao e um docx", "texto", 1_000).expect_err("recusa");
    assert!(err.to_string().contains("DOCX"), "diz o porquê: {err}");
  }

  #[test]
  fn docx_sem_document_xml_diz_qual_parte_falta() {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let opcoes = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    writer.start_file("outra/coisa.txt", opcoes).expect("abre");
    writer.write_all(b"x").expect("grava");
    let bytes = writer.finish().expect("fecha").into_inner();

    let err = ler(&bytes, "texto", 1_000).expect_err("recusa");
    assert!(
      err.to_string().contains("word/document.xml"),
      "diz a parte: {err}"
    );
  }

  #[test]
  fn marcadores_de_vinculo_ida_e_volta() {
    for modo in [
      ModoVinculo::Vivo,
      ModoVinculo::Snapshot,
      ModoVinculo::CopiaEditorial,
    ] {
      let (id, modo_de_volta) =
        ler_marcador_de_citacao(&marcador_de_citacao("ref-9", modo)).expect("marcador reconhecido");
      assert_eq!(id, "ref-9");
      assert_eq!(modo_de_volta, modo);
    }
    assert_eq!(
      ler_marcador_de_fragmento(&texto_do_no(&arvore_de_teste()[2])),
      Some("frag-7".into())
    );
    assert_eq!(ler_marcador_de_citacao("[citação: ]"), None);
    assert_eq!(ler_marcador_de_fragmento("[fragmento: ]"), None);
  }

  #[test]
  fn texto_plano_e_um_paragrafo_por_linha() {
    let texto = texto_plano(&arvore_de_teste());
    assert_eq!(
      texto,
      "Introdução\nA pesquisa parte da leitura.\n[citação: ref-42 · vivo]\n[fragmento: frag-7]"
    );
  }
}
