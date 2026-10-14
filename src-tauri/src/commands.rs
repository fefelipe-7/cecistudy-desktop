//! O caminho de escrita de Estudos e TCC — em migração (SPEC-D-015).
//!
//! **Este arquivo não deveria existir.** F1 tirou o Calendário, F2 tirou a
//! Faculdade, e F3 e F4 tiram os dois que restam. F5 apaga o arquivo.
//!
//! O cabeçalho que ficava aqui documentava só o Calendário — INV-1, INV-3, D12
//! e o débito C5 — e por isso mentia sobre dois terços do próprio arquivo: I7
//! (semestre encerrado não aceita regra nem estrutura) e D85..D88 (árvore,
//! paginação, versão e item a resolver) não eram citados em lugar nenhum. A
//! prosa de I7 e D85..D88 não se perdeu: ela foi com o bloco que a justificava,
//! para `faculdade/comandos.rs` e `tcc/comandos.rs`.

use rusqlite::{Connection, OptionalExtension, params};

use serde::{Deserialize, Serialize};

use crate::estudos::fsrs::{
  Avaliacao as AvaliacaoFsrs, EstadoCartao, FsrsError, dia_de_vencimento, esta_devido, revisar,
};
use crate::nucleo::{Db, DbError, new_id, now_ms};
use crate::tcc::documento::{
  DocError, Inline, ModoVinculo, NoDocumento, NoLinha, TipoNo, arvore_de_linhas, linhas_da_arvore,
  marcas_de_coluna, marcas_para_coluna,
};
use crate::tcc::item::{EstadoItem, ItemAResolver, TipoItem};
use crate::tcc::medidor::MedidorDeterministico;
use crate::tcc::paginacao::{Geometria, Paginacao, paginar};
use crate::tcc::versao::snapshot_canonico;

// 21. Estudos — conteúdo, cartão e a fila de revisão (SPEC-D-010, D76)
//
// O motor é `crate::estudos::fsrs` (a crate `fsrs` + a nossa adaptação); este
// bloco só lê e grava — nenhum cálculo aqui. O vencimento é **derivado na
// leitura** (cláusula do vencimento, D76): nunca é coluna, nunca é gravado.
//
// A zona do vencimento entra como parâmetro opcional com o literal único do
// produto ("America/Sao_Paulo", o mesmo de `domain/zone.ts`): a fila é por dia
// civil na zona da usuária, e o fuso da máquina nunca é consultado (I11). É
// parâmetro — e não ambiente — para o dia em que o produto sair do Brasil.

/// O literal de fuso do produto, o mesmo de `src/features/calendar/domain/zone.ts`.
const ZONA_DO_PRODUTO: &str = "America/Sao_Paulo";

fn zona_ou_padrao(zona: Option<&str>) -> &str {
  zona.unwrap_or(ZONA_DO_PRODUTO)
}

/// Erro do motor vira erro legível do caminho de escrita.
fn erro_fsrs(e: FsrsError) -> DbError {
  DbError::Domain(e.to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NovoConteudo {
  pub titulo: String,
  pub texto: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Conteudo {
  pub id: String,
  pub titulo: String,
  pub texto: Option<String>,
}

pub fn create_conteudo_core(db: &Db, payload: NovoConteudo) -> Result<Conteudo, DbError> {
  if payload.titulo.trim().is_empty() {
    return Err(DbError::Domain("o conteúdo precisa de um título".into()));
  }
  let now = now_ms()?;
  let id = new_id();
  let titulo = payload.titulo.trim().to_string();
  db.write(|tx| {
    tx.execute(
      "INSERT INTO conteudo (id, titulo, texto, created_at, updated_at)
              VALUES (?1, ?2, ?3, ?4, ?4)",
      params![id, titulo, payload.texto, now],
    )?;
    Ok(())
  })?;
  Ok(Conteudo {
    id,
    titulo,
    texto: payload.texto,
  })
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn create_conteudo(
  db: tauri::State<'_, Db>,
  payload: NovoConteudo,
) -> Result<Conteudo, DbError> {
  create_conteudo_core(&db, payload)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NovoCartao {
  pub conteudo_id: String,
  pub frente: String,
  pub verso: String,
}

/// O cartão como a tela vê: o estado FSRS que o banco guarda (D76) e o
/// vencimento derivado na leitura. `vencimento` é `None` para o cartão novo,
/// que nunca foi agendado — e é devido de imediato.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cartao {
  pub id: String,
  pub conteudo_id: String,
  pub frente: String,
  pub verso: String,
  pub estabilidade: Option<f64>,
  pub dificuldade: Option<f64>,
  pub ultima_revisao: Option<i64>,
  pub intervalo_dias: Option<i64>,
  pub relearning: bool,
  pub vencimento: Option<String>,
}

/// Cartão com estado: a ponte entre as colunas do banco e `EstadoCartao` do
/// domínio. `estabilidade`/`dificuldade` `None` é o cartão novo.
fn estado_do_cartao(
  estabilidade: Option<f64>,
  dificuldade: Option<f64>,
  ultima_revisao: Option<i64>,
  intervalo_dias: Option<i64>,
  relearning: bool,
) -> EstadoCartao {
  EstadoCartao {
    estabilidade,
    dificuldade,
    ultima_revisao,
    intervalo_dias,
    relearning,
  }
}

fn cartao_com_vencimento(
  id: String,
  conteudo_id: String,
  frente: String,
  verso: String,
  estado: &EstadoCartao,
  zona: &str,
) -> Result<Cartao, DbError> {
  let vencimento = match (estado.ultima_revisao, estado.intervalo_dias) {
    (Some(ultima), Some(intervalo)) => {
      Some(dia_de_vencimento(ultima, intervalo, zona).map_err(erro_fsrs)?)
    }
    _ => None,
  };
  Ok(Cartao {
    id,
    conteudo_id,
    frente,
    verso,
    estabilidade: estado.estabilidade,
    dificuldade: estado.dificuldade,
    ultima_revisao: estado.ultima_revisao,
    intervalo_dias: estado.intervalo_dias,
    relearning: estado.relearning,
    vencimento,
  })
}

pub fn create_cartao_core(db: &Db, payload: NovoCartao) -> Result<Cartao, DbError> {
  if payload.frente.trim().is_empty() {
    return Err(DbError::Domain("o cartão precisa de uma frente".into()));
  }
  if payload.verso.trim().is_empty() {
    return Err(DbError::Domain("o cartão precisa de um verso".into()));
  }
  let now = now_ms()?;
  let id = new_id();
  db.write(|tx| {
    let existe: Option<i64> = tx
      .query_row(
        "SELECT 1 FROM conteudo WHERE id = ?1",
        params![payload.conteudo_id],
        |r| r.get(0),
      )
      .optional()?;
    if existe.is_none() {
      return Err(DbError::Domain(format!(
        "conteudo {} não existe",
        payload.conteudo_id
      )));
    }
    // I10/§7 de SPEC-D-010: o cartão nasce **sem passado** — a struct não tem
    // campo de estado, e as colunas de estado ficam NULL. Criar cartão com
    // estado não é recusado: é impossível de pedir.
    tx.execute(
      "INSERT INTO cartao (id, conteudo_id, frente, verso, created_at, updated_at)
              VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
      params![id, payload.conteudo_id, payload.frente, payload.verso, now],
    )?;
    Ok(())
  })?;
  cartao_com_vencimento(
    id,
    payload.conteudo_id,
    payload.frente,
    payload.verso,
    &EstadoCartao::novo(),
    ZONA_DO_PRODUTO,
  )
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn create_cartao(db: tauri::State<'_, Db>, payload: NovoCartao) -> Result<Cartao, DbError> {
  create_cartao_core(&db, payload)
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FilaDoDia {
  pub zona: Option<String>,
}

/// Um cartão devido, com o título do conteúdo para a fila dizer do que é.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CartaoDaFila {
  pub id: String,
  pub conteudo_id: String,
  pub titulo_conteudo: String,
  pub frente: String,
  pub verso: String,
  /// `None` = cartão novo: devido hoje, sem agendamento anterior.
  pub vencimento: Option<String>,
  pub relearning: bool,
}

/// O que a leitura traz antes de decidir `due`: o bruto da linha + o estado.
struct CartaoBruto {
  id: String,
  conteudo_id: String,
  titulo_conteudo: String,
  frente: String,
  verso: String,
  estado: EstadoCartao,
}

/// A fila do dia: cartões cujo dia local de vencimento já chegou (D76). Toda
/// leitura recalcula os vencimentos a partir do estado — nada é gravado. Os
/// novos vêm primeiro, e os vencidos há mais tempo antes dos vencidos ontem.
pub fn fila_do_dia_core(db: &Db, payload: FilaDoDia) -> Result<Vec<CartaoDaFila>, DbError> {
  let zona = zona_ou_padrao(payload.zona.as_deref());
  let agora = now_ms()?;
  let conn = db.conn()?;
  let mut stmt = conn.prepare(
    "SELECT c.id, c.conteudo_id, ct.titulo, c.frente, c.verso,
            c.estabilidade, c.dificuldade, c.ultima_revisao, c.intervalo_dias, c.relearning
       FROM cartao c
       JOIN conteudo ct ON ct.id = c.conteudo_id",
  )?;
  let rows = stmt.query_map([], |r| {
    Ok(CartaoBruto {
      id: r.get(0)?,
      conteudo_id: r.get(1)?,
      titulo_conteudo: r.get(2)?,
      frente: r.get(3)?,
      verso: r.get(4)?,
      estado: estado_do_cartao(
        r.get(5)?,
        r.get(6)?,
        r.get(7)?,
        r.get(8)?,
        r.get::<_, i64>(9)? != 0,
      ),
    })
  })?;
  let mut fila: Vec<CartaoDaFila> = Vec::new();
  for bruto in rows {
    let bruto = bruto?;
    if !esta_devido(&bruto.estado, agora, zona).map_err(erro_fsrs)? {
      continue;
    }
    let vencimento = match (bruto.estado.ultima_revisao, bruto.estado.intervalo_dias) {
      (Some(ultima), Some(intervalo)) => {
        Some(dia_de_vencimento(ultima, intervalo, zona).map_err(erro_fsrs)?)
      }
      _ => None,
    };
    fila.push(CartaoDaFila {
      id: bruto.id,
      conteudo_id: bruto.conteudo_id,
      titulo_conteudo: bruto.titulo_conteudo,
      frente: bruto.frente,
      verso: bruto.verso,
      vencimento,
      relearning: bruto.estado.relearning,
    });
  }
  fila.sort_by(|a, b| a.vencimento.cmp(&b.vencimento));
  Ok(fila)
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn fila_do_dia(
  db: tauri::State<'_, Db>,
  payload: FilaDoDia,
) -> Result<Vec<CartaoDaFila>, DbError> {
  fila_do_dia_core(&db, payload)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisaoInput {
  pub cartao_id: String,
  /// 1 (again) · 2 (hard) · 3 (good) · 4 (easy) — a ordem da crate.
  pub rating: i64,
  /// Retenção desejada, em (0, 1). O padrão dela é **[P]** — a dona decide.
  pub retencao: f64,
  #[serde(default)]
  pub zona: Option<String>,
}

/// A revisão que move o cartão: lê o estado, o motor (`estudos::fsrs`)
/// calcula o novo, e a escrita grava o estado em `cartao` e o **evento** em
/// `revisao` — o mesmo desenho do Calendário: estado e registro são coisas
/// diferentes. A resposta devolve o cartão atualizado com o vencimento novo.
pub fn revisar_cartao_core(db: &Db, payload: RevisaoInput) -> Result<Cartao, DbError> {
  let avaliacao = AvaliacaoFsrs::de_rating(payload.rating).ok_or_else(|| {
    DbError::Domain(format!(
      "rating {} não existe: use 1 (again), 2 (hard), 3 (good) ou 4 (easy)",
      payload.rating
    ))
  })?;
  let zona = zona_ou_padrao(payload.zona.as_deref());
  let agora = now_ms()?;

  let (id, conteudo_id, frente, verso, estado) = {
    let conn = db.conn()?;
    conn
      .query_row(
        "SELECT id, conteudo_id, frente, verso, estabilidade, dificuldade,
                ultima_revisao, intervalo_dias, relearning
           FROM cartao WHERE id = ?1",
        params![payload.cartao_id],
        |r| {
          Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            estado_do_cartao(
              r.get(4)?,
              r.get(5)?,
              r.get(6)?,
              r.get(7)?,
              r.get::<_, i64>(8)? != 0,
            ),
          ))
        },
      )
      .optional()?
      .ok_or_else(|| DbError::Domain(format!("cartao {} não existe", payload.cartao_id)))?
  };

  // O cálculo é puro e fora da transação: a escrita é curta e o motor não
  // conhece banco.
  let resultado = revisar(&estado, avaliacao, payload.retencao, agora).map_err(erro_fsrs)?;
  let novo = resultado.estado;

  db.write(|tx| {
    tx.execute(
      "UPDATE cartao
          SET estabilidade = ?1, dificuldade = ?2, ultima_revisao = ?3,
              intervalo_dias = ?4, relearning = ?5, updated_at = ?3
        WHERE id = ?6",
      params![
        novo.estabilidade,
        novo.dificuldade,
        novo.ultima_revisao,
        novo.intervalo_dias,
        i64::from(novo.relearning),
        payload.cartao_id,
      ],
    )?;
    // O evento: o que a usuária avaliou e quando. O estado resultante vive em
    // `cartao`; aqui mora o histórico (matriz D65).
    tx.execute(
      "INSERT INTO revisao (id, cartao_id, rating, revisado_em, created_at)
              VALUES (?1, ?2, ?3, ?4, ?4)",
      params![new_id(), payload.cartao_id, payload.rating, agora],
    )?;
    Ok(())
  })?;

  cartao_com_vencimento(id, conteudo_id, frente, verso, &novo, zona)
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn revisar_cartao(db: tauri::State<'_, Db>, payload: RevisaoInput) -> Result<Cartao, DbError> {
  revisar_cartao_core(&db, payload)
}

// ---------------------------------------------------------------------------
// 22. TCC & Projetos — projeto, documento e versão (SPEC-D-013, D85..D88)
// ---------------------------------------------------------------------------
//
// O modelo, a paginação e a serialização canônica vivem em `crate::tcc` (motor
// puro). Este bloco é só o caminho de escrita e leitura: valida a árvore com
// `linhas_da_arvore` (as mesmas garantias da CHECK e do índice único) e grava.
//
// Regras que este bloco impõe, e onde:
// - **D87** — a versão é snapshot canônico; `salvar_documento` só versiona se o
//   snapshot mudou. Escrita sem mudança não cria versão.
// - **D85** — a árvore é substituída inteira numa transação; nenhuma coluna de
//   página ou posição é gravada.
// - **D88** — item duplicado exige substituto; o estado é lista fechada.

fn erro_doc(e: DocError) -> DbError {
  DbError::Domain(e.to_string())
}

fn modo_token(modo: ModoVinculo) -> &'static str {
  match modo {
    ModoVinculo::Vivo => "vivo",
    ModoVinculo::Snapshot => "snapshot",
    ModoVinculo::CopiaEditorial => "copia-editorial",
  }
}

fn modo_de_str(valor: &str) -> Result<ModoVinculo, DbError> {
  match valor {
    "vivo" => Ok(ModoVinculo::Vivo),
    "snapshot" => Ok(ModoVinculo::Snapshot),
    "copia-editorial" => Ok(ModoVinculo::CopiaEditorial),
    outro => Err(DbError::Domain(format!(
      "modo de vínculo inválido: {outro}"
    ))),
  }
}

fn tipo_no_de_str(valor: &str) -> Result<TipoNo, DbError> {
  match valor {
    "capitulo" => Ok(TipoNo::Capitulo),
    "paragrafo" => Ok(TipoNo::Paragrafo),
    "citacao" => Ok(TipoNo::Citacao),
    "referencia-viva" => Ok(TipoNo::ReferenciaViva),
    outro => Err(DbError::Domain(format!("tipo de nó inválido: {outro}"))),
  }
}

fn tipo_item_de_str(valor: &str) -> Result<TipoItem, DbError> {
  match valor {
    "comentario" => Ok(TipoItem::Comentario),
    "sugestao" => Ok(TipoItem::Sugestao),
    outro => Err(DbError::Domain(format!("tipo de item inválido: {outro}"))),
  }
}

fn estado_item_de_str(valor: &str) -> Result<EstadoItem, DbError> {
  match valor {
    "pendente" => Ok(EstadoItem::Pendente),
    "resolvido" => Ok(EstadoItem::Resolvido),
    "nao-encaixou" => Ok(EstadoItem::NaoEncaixou),
    "duplicado" => Ok(EstadoItem::Duplicado),
    outro => Err(DbError::Domain(format!("estado de item inválido: {outro}"))),
  }
}

/// Os sete tipos fixos de projeto (§4.7 [D]). A CHECK do schema impõe; aqui é
/// para o erro ser legível antes do SQL.
const TIPOS_DE_PROJETO: [&str; 7] = [
  "tcc",
  "artigo",
  "iniciacao-cientifica",
  "relatorio-de-estagio",
  "projeto-de-pesquisa",
  "revisao-bibliografica",
  "estudo-de-caso",
];

const ESTADOS_DE_PROJETO: [&str; 3] = ["ativo", "encerrado", "arquivado"];

const ESTADOS_DE_ITEM: [&str; 4] = ["pendente", "resolvido", "nao-encaixou", "duplicado"];

/// O limite de projetos **ativos** de §4.7 [D]: até cinco simultâneos. Encerrar
/// ou arquivar libera um lugar. O limite é conferido no caminho de **escrita**,
/// não só na tela — a tela avisa antes, o Rust recusa depois, e é o Rust que
/// grava.
const LIMITE_DE_PROJETOS_ATIVOS: i64 = 5;

/// O projeto como a tela vê.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Projeto {
  pub id: String,
  pub titulo: String,
  pub tipo: String,
  pub estado: String,
  pub descricao: Option<String>,
  pub created_at: i64,
  pub updated_at: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NovoProjeto {
  pub titulo: String,
  pub tipo: String,
  pub descricao: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentoRef {
  pub documento_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjetoRef {
  pub projeto_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjetoEstadoInput {
  pub projeto_id: String,
  pub estado: String,
}

/// O documento completo: a árvore remontada por `arvore_de_linhas`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Documento {
  pub id: String,
  pub projeto_id: String,
  pub titulo: String,
  pub arvore: Vec<NoDocumento>,
  pub ultima_versao: Option<i64>,
}

/// O resultado de salvar. `versionada = false` quando a árvore não mudou
/// (D87: escrita sem mudança não versiona).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentoSalvo {
  pub documento_id: String,
  pub numero: i64,
  pub versionada: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SalvarDocumento {
  pub documento_id: String,
  pub arvore: Vec<NoDocumento>,
  pub autor: String,
  pub motivo: Option<String>,
}

/// Entrada de `paginar_documento`. A geometria é **opcional**: sem ela vale o
/// A4 ABNT (`Geometria::a4_abnt`), o default do produto. A geometria é
/// **parâmetro** do motor (`D85`/`D86`), nunca coluna — mudá-la reprocessa o
/// documento sem tocar na árvore.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginarDocumento {
  pub documento_id: String,
  #[serde(default)]
  pub geometria: Option<Geometria>,
}

/// O resultado de paginar, como a tela vê.
///
/// `Paginacao` é o cálculo puro (`paginas`, `alturasPorNo`) e é achatado no
/// envelope. O envelope acrescenta o que a tela precisa para **não mentir**
/// (`D86`): qual `medidor` produziu o número e qual `geometria` foi aplicada.
/// `D86` proíbe paginação **silenciosamente** aproximada — o número sai com o
/// medidor declarado, e não como se fosse o layout do Word.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginacaoDaTela {
  #[serde(flatten)]
  pub paginacao: Paginacao,
  /// Qual medidor produziu o número. Em produção, o medidor real de fontes é
  /// dependência declarada (`D86`); aqui é o determinístico.
  pub medidor: &'static str,
  /// A geometria que foi de fato aplicada, para a tela mostrá-la.
  pub geometria: Geometria,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NovoItem {
  pub documento_id: String,
  pub tipo: String,
  pub autor: String,
  #[serde(default = "default_true")]
  pub autor_resolvido: bool,
  pub data_autor_ms: Option<i64>,
  pub citado: String,
  pub texto: Option<String>,
  pub origem_arquivo: Option<String>,
}

fn default_true() -> bool {
  true
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolverItem {
  pub item_id: String,
  pub estado: String,
  pub substituto: Option<String>,
}

pub fn list_projetos_core(db: &Db) -> Result<Vec<Projeto>, DbError> {
  let conn = db.conn()?;
  let mut stmt = conn.prepare(
    "SELECT id, titulo, tipo, estado, descricao, created_at, updated_at
        FROM projeto ORDER BY created_at DESC",
  )?;
  let projetos = stmt
    .query_map([], |r| {
      Ok(Projeto {
        id: r.get(0)?,
        titulo: r.get(1)?,
        tipo: r.get(2)?,
        estado: r.get(3)?,
        descricao: r.get(4)?,
        created_at: r.get(5)?,
        updated_at: r.get(6)?,
      })
    })?
    .collect::<Result<Vec<_>, _>>()?;
  Ok(projetos)
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn list_projetos(db: tauri::State<'_, Db>) -> Result<Vec<Projeto>, DbError> {
  list_projetos_core(&db)
}

/// Cria o projeto **e** o documento dele. Um projeto sem documento não existe
/// na v1 (`documento.projeto_id` é `UNIQUE`), então os dois nascem juntos, numa
/// transação — ou nascem os dois, ou não nasce nenhum.
pub fn create_projeto_core(db: &Db, payload: NovoProjeto) -> Result<Projeto, DbError> {
  let titulo = payload.titulo.trim().to_string();
  if titulo.is_empty() {
    return Err(DbError::Domain("o projeto precisa de um título".into()));
  }
  if !TIPOS_DE_PROJETO.contains(&payload.tipo.as_str()) {
    return Err(DbError::Domain(format!(
      "tipo de projeto inválido: {}",
      payload.tipo
    )));
  }

  let agora = now_ms()?;
  let projeto_id = new_id();
  let documento_id = new_id();
  let tipo = payload.tipo.clone();
  let descricao = payload.descricao.clone();

  db.write(|tx| {
    let ativos: i64 = tx.query_row(
      "SELECT COUNT(*) FROM projeto WHERE estado = 'ativo'",
      [],
      |r| r.get(0),
    )?;
    if ativos >= LIMITE_DE_PROJETOS_ATIVOS {
      return Err(DbError::Domain(format!(
        "limite de {LIMITE_DE_PROJETOS_ATIVOS} projetos ativos atingido (§4.7); \
         encerre ou arquive um projeto antes de criar outro"
      )));
    }
    tx.execute(
      "INSERT INTO projeto (id, titulo, tipo, estado, descricao, created_at, updated_at)
              VALUES (?1, ?2, ?3, 'ativo', ?4, ?5, ?5)",
      params![projeto_id, titulo, tipo, descricao, agora],
    )?;
    tx.execute(
      "INSERT INTO documento (id, projeto_id, titulo, created_at, updated_at)
              VALUES (?1, ?2, ?3, ?4, ?4)",
      params![documento_id, projeto_id, titulo, agora],
    )?;
    Ok(())
  })?;

  Ok(Projeto {
    id: projeto_id,
    titulo,
    tipo,
    estado: "ativo".into(),
    descricao,
    created_at: agora,
    updated_at: agora,
  })
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn create_projeto(db: tauri::State<'_, Db>, payload: NovoProjeto) -> Result<Projeto, DbError> {
  create_projeto_core(&db, payload)
}

pub fn set_estado_projeto_core(db: &Db, payload: ProjetoEstadoInput) -> Result<(), DbError> {
  if !ESTADOS_DE_PROJETO.contains(&payload.estado.as_str()) {
    return Err(DbError::Domain(format!(
      "estado de projeto inválido: {}",
      payload.estado
    )));
  }
  let agora = now_ms()?;
  let afetadas = db.write(|tx| {
    let n = tx.execute(
      "UPDATE projeto SET estado = ?1, updated_at = ?2 WHERE id = ?3",
      params![payload.estado, agora, payload.projeto_id],
    )?;
    Ok(n)
  })?;
  if afetadas == 0 {
    return Err(DbError::Domain(format!(
      "projeto {} não existe",
      payload.projeto_id
    )));
  }
  Ok(())
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn set_estado_projeto(
  db: tauri::State<'_, Db>,
  payload: ProjetoEstadoInput,
) -> Result<(), DbError> {
  set_estado_projeto_core(&db, payload)
}

/// Lê o documento e remonta a árvore. A remontagem valida o que o banco garante:
/// pai existente, ordem única, colunas casando com o tipo, sem ciclo. Um banco
/// correto nunca falha aqui.
pub fn get_documento_core(db: &Db, payload: DocumentoRef) -> Result<Documento, DbError> {
  let conn = db.conn()?;
  let (id, projeto_id, titulo): (String, String, String) = conn
    .query_row(
      "SELECT id, projeto_id, titulo FROM documento WHERE id = ?1",
      params![payload.documento_id],
      |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )
    .optional()?
    .ok_or_else(|| DbError::Domain(format!("documento {} não existe", payload.documento_id)))?;
  montar_documento(&conn, id, projeto_id, titulo)
}

/// Lê o documento **do projeto**. O vínculo é `documento.projeto_id` (`UNIQUE` na
/// v1): um projeto tem exatamente um documento, então a tela não precisa carregar
/// o `documento_id` — `list_projetos` não o expõe, e expô-lo seria vazar a forma
/// de persistência para a lista.
pub fn get_documento_do_projeto_core(db: &Db, payload: ProjetoRef) -> Result<Documento, DbError> {
  let conn = db.conn()?;
  let (id, projeto_id, titulo): (String, String, String) = conn
    .query_row(
      "SELECT id, projeto_id, titulo FROM documento WHERE projeto_id = ?1",
      params![payload.projeto_id],
      |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )
    .optional()?
    .ok_or_else(|| DbError::Domain(format!("projeto {} não tem documento", payload.projeto_id)))?;
  montar_documento(&conn, id, projeto_id, titulo)
}

/// Remonta um `Documento` a partir da linha base e das linhas de nó. É
/// compartilhada pelas duas leituras — por id do documento e por id do projeto —
/// para a remontagem da árvore ter **um dono só**.
fn montar_documento(
  conn: &Connection,
  id: String,
  projeto_id: String,
  titulo: String,
) -> Result<Documento, DbError> {
  let linhas_base = {
    let mut stmt = conn.prepare(
      "SELECT id, pai_id, ordem, tipo, titulo, referencia_id, modo, fragmento_id
          FROM no_documento WHERE documento_id = ?1",
    )?;
    stmt
      .query_map(params![id], |r| {
        Ok((
          r.get::<_, String>(0)?,
          r.get::<_, Option<String>>(1)?,
          r.get::<_, i64>(2)?,
          r.get::<_, String>(3)?,
          r.get::<_, Option<String>>(4)?,
          r.get::<_, Option<String>>(5)?,
          r.get::<_, Option<String>>(6)?,
          r.get::<_, Option<String>>(7)?,
        ))
      })?
      .collect::<Result<Vec<_>, _>>()?
  };

  let mut linhas = Vec::with_capacity(linhas_base.len());
  {
    let mut inline_stmt =
      conn.prepare("SELECT texto, marcas FROM inline_run WHERE no_id = ?1 ORDER BY ordem")?;
    for (no_id, pai_id, ordem, tipo, titulo, referencia_id, modo, fragmento_id) in linhas_base {
      let runs = inline_stmt
        .query_map(params![no_id], |r| {
          Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
      let mut inline = Vec::with_capacity(runs.len());
      for (texto, coluna) in runs {
        inline.push(Inline {
          texto,
          marcas: marcas_de_coluna(&coluna).map_err(erro_doc)?,
        });
      }
      linhas.push(NoLinha {
        id: no_id,
        pai_id,
        ordem: u32::try_from(ordem)
          .map_err(|_| DbError::Domain(format!("ordem fora do range: {ordem}")))?,
        tipo: tipo_no_de_str(&tipo)?,
        titulo,
        referencia_id,
        modo: modo.as_deref().map(modo_de_str).transpose()?,
        fragmento_id,
        inlines: inline,
      });
    }
  }

  let arvore = arvore_de_linhas(linhas).map_err(erro_doc)?;
  let ultima_versao: Option<i64> = conn.query_row(
    "SELECT MAX(numero) FROM versao WHERE documento_id = ?1",
    params![id],
    |r| r.get::<_, Option<i64>>(0),
  )?;

  Ok(Documento {
    id,
    projeto_id,
    titulo,
    arvore,
    ultima_versao,
  })
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn get_documento(
  db: tauri::State<'_, Db>,
  payload: DocumentoRef,
) -> Result<Documento, DbError> {
  get_documento_core(&db, payload)
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn get_documento_do_projeto(
  db: tauri::State<'_, Db>,
  payload: ProjetoRef,
) -> Result<Documento, DbError> {
  get_documento_do_projeto_core(&db, payload)
}

/// Pagina o documento **sem gravar nada**: a página é **derivada** (`D77`,
/// `D86`).
///
/// A árvore vem do banco (`get_documento_core`), o motor é puro
/// ([`crate::tcc::paginacao::paginar`]) e o medidor é o determinístico — em
/// produção, o medidor real de fontes é dependência declarada (`D86`). O
/// envelope devolve o `medidor` porque `D86` proíbe paginação **silenciosamente**
/// aproximada: a tela mostra que o número é do medidor determinístico, e não do
/// Word. Nenhuma linha de página, altura ou posição é gravada (I1).
pub fn paginar_documento_core(
  db: &Db,
  payload: PaginarDocumento,
) -> Result<PaginacaoDaTela, DbError> {
  let documento = get_documento_core(
    db,
    DocumentoRef {
      documento_id: payload.documento_id,
    },
  )?;
  let geometria = payload.geometria.unwrap_or_else(Geometria::a4_abnt);
  let paginacao = paginar(
    &documento.arvore,
    &geometria,
    &MedidorDeterministico::novo(),
  );
  Ok(PaginacaoDaTela {
    paginacao,
    medidor: "deterministico",
    geometria,
  })
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn paginar_documento(
  db: tauri::State<'_, Db>,
  payload: PaginarDocumento,
) -> Result<PaginacaoDaTela, DbError> {
  paginar_documento_core(&db, payload)
}

/// Substitui a árvore do documento e, se ela mudou, grava uma versão (D87).
///
/// A árvore é substituída inteira: os nós antigos saem em cascata (o inline vai
/// junto) e a nova lista entra na ordem pai-antes-de-filho que `linhas_da_arvore`
/// produz — o FK de `pai_id` exige essa ordem.
pub fn salvar_documento_core(db: &Db, payload: SalvarDocumento) -> Result<DocumentoSalvo, DbError> {
  let snapshot = snapshot_canonico(&payload.arvore).map_err(erro_doc)?;
  let linhas = linhas_da_arvore(&payload.arvore).map_err(erro_doc)?;
  let agora = now_ms()?;

  db.write(|tx| {
    let existe: Option<i64> = tx
      .query_row(
        "SELECT 1 FROM documento WHERE id = ?1",
        params![payload.documento_id],
        |r| r.get(0),
      )
      .optional()?;
    if existe.is_none() {
      return Err(DbError::Domain(format!(
        "documento {} não existe",
        payload.documento_id
      )));
    }

    let ultimo: Option<(i64, String)> = tx
      .query_row(
        "SELECT numero, snapshot FROM versao
            WHERE documento_id = ?1 ORDER BY numero DESC LIMIT 1",
        params![payload.documento_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
      )
      .optional()?;

    if let Some((numero, anterior)) = &ultimo
      && *anterior == snapshot
    {
      // D87: nada mudou — não versiona e não reescreve a árvore.
      return Ok(DocumentoSalvo {
        documento_id: payload.documento_id.clone(),
        numero: *numero,
        versionada: false,
      });
    }

    tx.execute(
      "DELETE FROM no_documento WHERE documento_id = ?1",
      params![payload.documento_id],
    )?;

    for linha in &linhas {
      tx.execute(
        "INSERT INTO no_documento
           (id, documento_id, pai_id, ordem, tipo, titulo, referencia_id, modo,
            fragmento_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
        params![
          linha.id,
          payload.documento_id,
          linha.pai_id,
          i64::from(linha.ordem),
          linha.tipo.token(),
          linha.titulo,
          linha.referencia_id,
          linha.modo.map(modo_token),
          linha.fragmento_id,
          agora,
        ],
      )?;
      for (indice, run) in linha.inlines.iter().enumerate() {
        tx.execute(
          "INSERT INTO inline_run (id, no_id, ordem, texto, marcas)
             VALUES (?1, ?2, ?3, ?4, ?5)",
          params![
            new_id(),
            linha.id,
            indice as i64,
            run.texto,
            marcas_para_coluna(&run.marcas),
          ],
        )?;
      }
    }

    let numero = ultimo.map(|(n, _)| n).unwrap_or(0) + 1;
    tx.execute(
      "INSERT INTO versao (id, documento_id, numero, snapshot, autor, motivo, criado_em)
              VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
      params![
        new_id(),
        payload.documento_id,
        numero,
        snapshot,
        payload.autor,
        payload.motivo,
        agora
      ],
    )?;
    tx.execute(
      "UPDATE documento SET updated_at = ?1 WHERE id = ?2",
      params![agora, payload.documento_id],
    )?;

    Ok(DocumentoSalvo {
      documento_id: payload.documento_id.clone(),
      numero,
      versionada: true,
    })
  })
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn salvar_documento(
  db: tauri::State<'_, Db>,
  payload: SalvarDocumento,
) -> Result<DocumentoSalvo, DbError> {
  salvar_documento_core(&db, payload)
}

/// Registra um item a resolver vindo da importação de DOCX (D88). A importação
/// **anexa**: este comando nunca toca em `no_documento`.
pub fn registrar_item_core(db: &Db, payload: NovoItem) -> Result<ItemAResolver, DbError> {
  let tipo = tipo_item_de_str(&payload.tipo)?;
  let agora = now_ms()?;
  let id = new_id();

  db.write(|tx| {
    let existe: Option<i64> = tx
      .query_row(
        "SELECT 1 FROM documento WHERE id = ?1",
        params![payload.documento_id],
        |r| r.get(0),
      )
      .optional()?;
    if existe.is_none() {
      return Err(DbError::Domain(format!(
        "documento {} não existe",
        payload.documento_id
      )));
    }
    tx.execute(
      "INSERT INTO item_a_resolver
         (id, documento_id, tipo, autor, autor_resolvido, data_autor_ms, citado,
          substituto, texto, origem_arquivo, importado_em, estado, resolvido_em, created_at)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, ?8, ?9, ?10, 'pendente', NULL, ?10)",
      params![
        id,
        payload.documento_id,
        payload.tipo,
        payload.autor,
        i64::from(payload.autor_resolvido),
        payload.data_autor_ms,
        payload.citado,
        payload.texto,
        payload.origem_arquivo,
        agora,
      ],
    )?;
    Ok(())
  })?;

  Ok(ItemAResolver {
    id,
    documento_id: payload.documento_id,
    tipo,
    autor: payload.autor,
    autor_resolvido: payload.autor_resolvido,
    data_autor: payload.data_autor_ms,
    citado: payload.citado,
    substituto: None,
    texto: payload.texto,
    origem_arquivo: payload.origem_arquivo,
    importado_em: agora,
    estado: EstadoItem::Pendente,
    resolvido_em: None,
  })
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn registrar_item(
  db: tauri::State<'_, Db>,
  payload: NovoItem,
) -> Result<ItemAResolver, DbError> {
  registrar_item_core(&db, payload)
}

/// Decide um item: resolvido, não-encaixou ou duplicado. `duplicado` exige o
/// `substituto` — sem ele o estado não decide nada (D88).
pub fn resolver_item_core(db: &Db, payload: ResolverItem) -> Result<(), DbError> {
  if !ESTADOS_DE_ITEM.contains(&payload.estado.as_str()) {
    return Err(DbError::Domain(format!(
      "estado de item inválido: {}",
      payload.estado
    )));
  }
  if payload.estado == "duplicado"
    && !payload
      .substituto
      .as_deref()
      .is_some_and(|s| !s.trim().is_empty())
  {
    return Err(DbError::Domain(
      "item duplicado exige o substituto (D88)".into(),
    ));
  }

  let agora = now_ms()?;
  let resolvido_em = if payload.estado == "pendente" {
    None
  } else {
    Some(agora)
  };
  let afetadas = db.write(|tx| {
    let n = tx.execute(
      "UPDATE item_a_resolver SET estado = ?1, resolvido_em = ?2, substituto = ?3
         WHERE id = ?4",
      params![
        payload.estado,
        resolvido_em,
        payload.substituto,
        payload.item_id
      ],
    )?;
    Ok(n)
  })?;
  if afetadas == 0 {
    return Err(DbError::Domain(format!(
      "item {} não existe",
      payload.item_id
    )));
  }
  Ok(())
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn resolver_item(db: tauri::State<'_, Db>, payload: ResolverItem) -> Result<(), DbError> {
  resolver_item_core(&db, payload)
}

pub fn list_itens_core(db: &Db, payload: DocumentoRef) -> Result<Vec<ItemAResolver>, DbError> {
  let conn = db.conn()?;
  let mut stmt = conn.prepare(
    "SELECT id, documento_id, tipo, autor, autor_resolvido, data_autor_ms, citado,
            substituto, texto, origem_arquivo, importado_em, estado, resolvido_em
        FROM item_a_resolver WHERE documento_id = ?1 ORDER BY importado_em DESC",
  )?;
  let brutos = stmt
    .query_map(params![payload.documento_id], |r| {
      Ok((
        r.get::<_, String>(0)?,
        r.get::<_, String>(1)?,
        r.get::<_, String>(2)?,
        r.get::<_, String>(3)?,
        r.get::<_, i64>(4)? != 0,
        r.get::<_, Option<i64>>(5)?,
        r.get::<_, String>(6)?,
        r.get::<_, Option<String>>(7)?,
        r.get::<_, Option<String>>(8)?,
        r.get::<_, Option<String>>(9)?,
        r.get::<_, i64>(10)?,
        r.get::<_, String>(11)?,
        r.get::<_, Option<i64>>(12)?,
      ))
    })?
    .collect::<Result<Vec<_>, _>>()?;

  let mut itens = Vec::with_capacity(brutos.len());
  for (
    id,
    documento_id,
    tipo,
    autor,
    autor_resolvido,
    data_autor,
    citado,
    substituto,
    texto,
    origem_arquivo,
    importado_em,
    estado,
    resolvido_em,
  ) in brutos
  {
    itens.push(ItemAResolver {
      id,
      documento_id,
      tipo: tipo_item_de_str(&tipo)?,
      autor,
      autor_resolvido,
      data_autor,
      citado,
      substituto,
      texto,
      origem_arquivo,
      importado_em,
      estado: estado_item_de_str(&estado)?,
      resolvido_em,
    });
  }
  Ok(itens)
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn list_itens(
  db: tauri::State<'_, Db>,
  payload: DocumentoRef,
) -> Result<Vec<ItemAResolver>, DbError> {
  list_itens_core(&db, payload)
}

/// Os dois alvos possíveis de um vínculo da Base de pesquisa (o `CHECK` de
/// `base_de_pesquisa`). Referência é da **Biblioteca**; Fragmento é do
/// **Conhecimento** (D65/D66).
const TIPOS_DE_ALVO: [&str; 2] = ["referencia", "fragmento"];

/// Um vínculo da Base de pesquisa: o projeto **aponta** para um item do acervo,
/// sem copiar nada (D66). `alvo_id` é referência viva sem FK enquanto a
/// Biblioteca e o Conhecimento não existem — o vínculo órfão é possível e é
/// sinalizado na tela, não escondido.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Vinculo {
  pub id: String,
  pub projeto_id: String,
  pub alvo_tipo: String,
  pub alvo_id: String,
  pub criado_em: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NovoVinculo {
  pub projeto_id: String,
  pub alvo_tipo: String,
  pub alvo_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VinculoRef {
  pub vinculo_id: String,
}

/// Lista os vínculos do projeto, na ordem em que entraram.
pub fn list_base_core(db: &Db, payload: ProjetoRef) -> Result<Vec<Vinculo>, DbError> {
  let conn = db.conn()?;
  let mut stmt = conn.prepare(
    "SELECT id, projeto_id, alvo_tipo, alvo_id, criado_em
        FROM base_de_pesquisa WHERE projeto_id = ?1 ORDER BY criado_em, id",
  )?;
  let vinculos = stmt
    .query_map(params![payload.projeto_id], |r| {
      Ok(Vinculo {
        id: r.get(0)?,
        projeto_id: r.get(1)?,
        alvo_tipo: r.get(2)?,
        alvo_id: r.get(3)?,
        criado_em: r.get(4)?,
      })
    })?
    .collect::<Result<Vec<_>, _>>()?;
  Ok(vinculos)
}

/// Vincula o projeto a um alvo do acervo. Idempotente: o mesmo
/// (`projeto`, `alvo_tipo`, `alvo_id`) não vira duas linhas — vínculo repetido é
/// o mesmo vínculo, e a lista de referências é gerada daqui (I11).
pub fn vincular_core(db: &Db, payload: NovoVinculo) -> Result<Vinculo, DbError> {
  let alvo_id = payload.alvo_id.trim().to_string();
  if alvo_id.is_empty() {
    return Err(DbError::Domain("o vínculo precisa do id do alvo".into()));
  }
  if !TIPOS_DE_ALVO.contains(&payload.alvo_tipo.as_str()) {
    return Err(DbError::Domain(format!(
      "tipo de alvo inválido: {}",
      payload.alvo_tipo
    )));
  }
  let agora = now_ms()?;
  let novo_id = new_id();
  let projeto_id = payload.projeto_id;
  let alvo_tipo = payload.alvo_tipo;

  db.write(|tx| {
    let dono: i64 = tx.query_row(
      "SELECT COUNT(*) FROM projeto WHERE id = ?1",
      params![projeto_id],
      |r| r.get(0),
    )?;
    if dono == 0 {
      return Err(DbError::Domain(format!("projeto {projeto_id} não existe")));
    }
    let ja_existe: Option<String> = tx
      .query_row(
        "SELECT id FROM base_de_pesquisa
            WHERE projeto_id = ?1 AND alvo_tipo = ?2 AND alvo_id = ?3",
        params![projeto_id, alvo_tipo, alvo_id],
        |r| r.get(0),
      )
      .optional()?;
    if ja_existe.is_none() {
      tx.execute(
        "INSERT INTO base_de_pesquisa (id, projeto_id, alvo_tipo, alvo_id, criado_em)
                VALUES (?1, ?2, ?3, ?4, ?5)",
        params![novo_id, projeto_id, alvo_tipo, alvo_id, agora],
      )?;
    }
    Ok(())
  })?;

  let conn = db.conn()?;
  let vinculo = conn.query_row(
    "SELECT id, projeto_id, alvo_tipo, alvo_id, criado_em
        FROM base_de_pesquisa
        WHERE projeto_id = ?1 AND alvo_tipo = ?2 AND alvo_id = ?3",
    params![projeto_id, alvo_tipo, alvo_id],
    |r| {
      Ok(Vinculo {
        id: r.get(0)?,
        projeto_id: r.get(1)?,
        alvo_tipo: r.get(2)?,
        alvo_id: r.get(3)?,
        criado_em: r.get(4)?,
      })
    },
  )?;
  Ok(vinculo)
}

/// Remove um vínculo pelo id. O alvo (Biblioteca/Conhecimento) não é tocado:
/// vínculo é ponteiro, e apagar a ponte não apaga a fonte.
pub fn desvincular_core(db: &Db, payload: VinculoRef) -> Result<(), DbError> {
  let afetadas = db.write(|tx| {
    let n = tx.execute(
      "DELETE FROM base_de_pesquisa WHERE id = ?1",
      params![payload.vinculo_id],
    )?;
    Ok(n)
  })?;
  if afetadas == 0 {
    return Err(DbError::Domain(format!(
      "vínculo {} não existe",
      payload.vinculo_id
    )));
  }
  Ok(())
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn list_base(db: tauri::State<'_, Db>, payload: ProjetoRef) -> Result<Vec<Vinculo>, DbError> {
  list_base_core(&db, payload)
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn vincular(db: tauri::State<'_, Db>, payload: NovoVinculo) -> Result<Vinculo, DbError> {
  vincular_core(&db, payload)
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn desvincular(db: tauri::State<'_, Db>, payload: VinculoRef) -> Result<(), DbError> {
  desvincular_core(&db, payload)
}

#[cfg(test)]
mod testes_tcc {
  use super::*;

  fn paragrafo(id: &str, texto: &str) -> NoDocumento {
    NoDocumento::Paragrafo {
      id: id.into(),
      inline: vec![Inline::simples(texto)],
    }
  }

  fn projeto_tcc(db: &Db) -> (String, String) {
    let projeto = create_projeto_core(
      db,
      NovoProjeto {
        titulo: "Meu TCC".into(),
        tipo: "tcc".into(),
        descricao: None,
      },
    )
    .expect("projeto");
    let documento_id: String = db
      .conn()
      .unwrap()
      .query_row(
        "SELECT id FROM documento WHERE projeto_id = ?1",
        params![projeto.id],
        |r| r.get(0),
      )
      .expect("documento");
    (projeto.id, documento_id)
  }

  #[test]
  fn tcc_projeto_nasce_com_documento() {
    let db = Db::open_in_memory().expect("banco");
    let (_, documento_id) = projeto_tcc(&db);
    let doc = get_documento_core(&db, DocumentoRef { documento_id }).expect("documento");
    assert!(doc.arvore.is_empty());
    assert_eq!(doc.ultima_versao, None, "nenhuma versão antes de salvar");
  }

  #[test]
  fn tcc_documento_do_projeto_le_o_mesmo_documento() {
    let db = Db::open_in_memory().expect("banco");
    let (projeto_id, documento_id) = projeto_tcc(&db);

    let por_projeto =
      get_documento_do_projeto_core(&db, ProjetoRef { projeto_id }).expect("lê pelo projeto");
    let por_id = get_documento_core(&db, DocumentoRef { documento_id }).expect("lê pelo documento");

    assert_eq!(
      por_projeto.id, por_id.id,
      "os dois caminhos são o mesmo documento"
    );
    assert_eq!(por_projeto.projeto_id, por_id.projeto_id);

    let err = get_documento_do_projeto_core(
      &db,
      ProjetoRef {
        projeto_id: "nao-existe".into(),
      },
    )
    .expect_err("projeto sem documento");
    assert!(err.to_string().contains("documento"), "diz o porquê: {err}");
  }

  #[test]
  fn tcc_salvar_versiona_e_escrita_sem_mudanca_nao_versiona() {
    let db = Db::open_in_memory().expect("banco");
    let (_, documento_id) = projeto_tcc(&db);
    let arvore = vec![paragrafo("p1", "Introdução")];

    let v1 = salvar_documento_core(
      &db,
      SalvarDocumento {
        documento_id: documento_id.clone(),
        arvore: arvore.clone(),
        autor: "eu".into(),
        motivo: Some("rascunho".into()),
      },
    )
    .expect("salva");
    assert!(v1.versionada);
    assert_eq!(v1.numero, 1);

    // D87: mesmo conteúdo, sem mudança → não cria versão.
    let v2 = salvar_documento_core(
      &db,
      SalvarDocumento {
        documento_id: documento_id.clone(),
        arvore: arvore.clone(),
        autor: "eu".into(),
        motivo: None,
      },
    )
    .expect("salva de novo");
    assert!(!v2.versionada);
    assert_eq!(v2.numero, 1);

    let doc = get_documento_core(
      &db,
      DocumentoRef {
        documento_id: documento_id.clone(),
      },
    )
    .expect("lê");
    assert_eq!(doc.arvore, arvore, "a árvore volta igual");
    assert_eq!(doc.ultima_versao, Some(1));

    // Mudou → versão 2.
    let mut outra = arvore.clone();
    outra.push(paragrafo("p2", "Método"));
    let v3 = salvar_documento_core(
      &db,
      SalvarDocumento {
        documento_id: documento_id.clone(),
        arvore: outra.clone(),
        autor: "eu".into(),
        motivo: None,
      },
    )
    .expect("salva a mudança");
    assert!(v3.versionada);
    assert_eq!(v3.numero, 2);

    let doc = get_documento_core(&db, DocumentoRef { documento_id }).expect("lê");
    assert_eq!(doc.arvore, outra);
  }

  #[test]
  fn tcc_citacao_sem_alvo_e_recusada_na_escrita() {
    let db = Db::open_in_memory().expect("banco");
    let (_, documento_id) = projeto_tcc(&db);
    let arvore = vec![NoDocumento::Citacao {
      id: "c".into(),
      referencia_id: String::new(),
      modo: ModoVinculo::Vivo,
    }];
    let err = salvar_documento_core(
      &db,
      SalvarDocumento {
        documento_id,
        arvore,
        autor: "eu".into(),
        motivo: None,
      },
    )
    .expect_err("citação sem alvo");
    assert!(err.to_string().contains("vínculo"), "diz o porquê: {err}");
  }

  #[test]
  fn tcc_item_duplicado_exige_substituto() {
    let db = Db::open_in_memory().expect("banco");
    let (_, documento_id) = projeto_tcc(&db);
    let item = registrar_item_core(
      &db,
      NovoItem {
        documento_id: documento_id.clone(),
        tipo: "comentario".into(),
        autor: "Orientador".into(),
        autor_resolvido: false,
        data_autor_ms: None,
        citado: "este trecho".into(),
        texto: None,
        origem_arquivo: None,
      },
    )
    .expect("registra");
    assert!(item.esta_pendente());
    assert!(!item.autor_resolvido, "D78: autor desconhecido");

    let err = resolver_item_core(
      &db,
      ResolverItem {
        item_id: item.id.clone(),
        estado: "duplicado".into(),
        substituto: None,
      },
    )
    .expect_err("duplicado sem substituto");
    assert!(
      err.to_string().contains("substituto"),
      "diz o porquê: {err}"
    );

    resolver_item_core(
      &db,
      ResolverItem {
        item_id: item.id.clone(),
        estado: "nao-encaixou".into(),
        substituto: None,
      },
    )
    .expect("não-encaixou");

    let itens = list_itens_core(&db, DocumentoRef { documento_id }).expect("lista");
    assert_eq!(itens.len(), 1);
    assert_eq!(itens[0].estado, EstadoItem::NaoEncaixou);
    assert!(itens[0].resolvido_em.is_some());
  }

  #[test]
  fn tcc_limite_de_projetos_ativos_recusa_o_sexto() {
    let db = Db::open_in_memory().expect("banco");
    let mut ids = Vec::new();
    for n in 1..=LIMITE_DE_PROJETOS_ATIVOS {
      let p = create_projeto_core(
        &db,
        NovoProjeto {
          titulo: format!("Projeto {n}"),
          tipo: "tcc".into(),
          descricao: None,
        },
      )
      .expect("os cinco primeiros cabem");
      ids.push(p.id);
    }

    let err = create_projeto_core(
      &db,
      NovoProjeto {
        titulo: "Sexto".into(),
        tipo: "tcc".into(),
        descricao: None,
      },
    )
    .expect_err("o sexto é recusado");
    assert!(
      err
        .to_string()
        .contains(&LIMITE_DE_PROJETOS_ATIVOS.to_string()),
      "nomeia o limite: {err}"
    );

    // O limite é de **ativos**: encerrar um libera o lugar.
    set_estado_projeto_core(
      &db,
      ProjetoEstadoInput {
        projeto_id: ids[0].clone(),
        estado: "encerrado".into(),
      },
    )
    .expect("encerra um");
    create_projeto_core(
      &db,
      NovoProjeto {
        titulo: "Sexto".into(),
        tipo: "tcc".into(),
        descricao: None,
      },
    )
    .expect("cabe depois de encerrar");
  }

  #[test]
  fn tcc_base_guarda_vinculo_sem_copia_e_deduplica() {
    let db = Db::open_in_memory().expect("banco");
    let (projeto_id, _) = projeto_tcc(&db);

    let v = vincular_core(
      &db,
      NovoVinculo {
        projeto_id: projeto_id.clone(),
        alvo_tipo: "referencia".into(),
        alvo_id: "ref-42".into(),
      },
    )
    .expect("vincula");
    assert_eq!(v.alvo_id, "ref-42");
    assert_eq!(v.alvo_tipo, "referencia");

    // O vínculo é referência viva: guarda o id, não copia conteúdo (D66/I11).
    let base = list_base_core(
      &db,
      ProjetoRef {
        projeto_id: projeto_id.clone(),
      },
    )
    .expect("lista");
    assert_eq!(base.len(), 1);
    assert_eq!(base[0].alvo_id, "ref-42");

    // Citar o mesmo alvo de novo não duplica: vínculo repetido é o mesmo vínculo.
    vincular_core(
      &db,
      NovoVinculo {
        projeto_id: projeto_id.clone(),
        alvo_tipo: "referencia".into(),
        alvo_id: "ref-42".into(),
      },
    )
    .expect("de novo");
    assert_eq!(
      list_base_core(
        &db,
        ProjetoRef {
          projeto_id: projeto_id.clone()
        }
      )
      .expect("lista")
      .len(),
      1,
      "o mesmo alvo não duplica"
    );

    // Fragmento é o outro tipo de alvo e conta separado.
    vincular_core(
      &db,
      NovoVinculo {
        projeto_id: projeto_id.clone(),
        alvo_tipo: "fragmento".into(),
        alvo_id: "ref-42".into(),
      },
    )
    .expect("fragmento");
    assert_eq!(
      list_base_core(
        &db,
        ProjetoRef {
          projeto_id: projeto_id.clone(),
        },
      )
      .expect("lista")
      .len(),
      2
    );

    desvincular_core(
      &db,
      VinculoRef {
        vinculo_id: v.id.clone(),
      },
    )
    .expect("desvincula");
    assert_eq!(
      list_base_core(&db, ProjetoRef { projeto_id })
        .expect("lista")
        .len(),
      1
    );

    let err = desvincular_core(
      &db,
      VinculoRef {
        vinculo_id: "nao-existe".into(),
      },
    )
    .expect_err("id inexistente");
    assert!(err.to_string().contains("vínculo"), "diz o porquê: {err}");
  }

  #[test]
  fn tcc_base_recusa_alvo_tipo_invalido() {
    let db = Db::open_in_memory().expect("banco");
    let (projeto_id, _) = projeto_tcc(&db);
    let err = vincular_core(
      &db,
      NovoVinculo {
        projeto_id,
        alvo_tipo: "site".into(),
        alvo_id: "x".into(),
      },
    )
    .expect_err("tipo inválido");
    assert!(err.to_string().contains("alvo"), "diz o porquê: {err}");
  }

  #[test]
  fn tcc_paginar_documento_vazio_tem_uma_pagina_e_declara_o_medidor() {
    let db = Db::open_in_memory().expect("banco");
    let (_, documento_id) = projeto_tcc(&db);
    let pag = paginar_documento_core(
      &db,
      PaginarDocumento {
        documento_id,
        geometria: None,
      },
    )
    .expect("pagina");

    assert_eq!(pag.paginacao.paginas, 1, "documento vazio tem uma página");
    assert!(pag.paginacao.alturas_por_no.is_empty());
    assert_eq!(
      pag.medidor, "deterministico",
      "D86: o medidor é declarado, nunca silencioso"
    );
    assert_eq!(
      pag.geometria,
      Geometria::a4_abnt(),
      "sem geometria, vale o A4 ABNT"
    );
  }

  #[test]
  fn tcc_paginar_muda_a_geometria_sem_mudar_o_conteudo() {
    let db = Db::open_in_memory().expect("banco");
    let (_, documento_id) = projeto_tcc(&db);

    // Conteúdo suficiente para a página estreita quebrar em mais de uma.
    let mut arvore = Vec::new();
    for i in 0..30 {
      arvore.push(paragrafo(&format!("p{i}"), &"palavra ".repeat(24)));
    }
    salvar_documento_core(
      &db,
      SalvarDocumento {
        documento_id: documento_id.clone(),
        arvore: arvore.clone(),
        autor: "eu".into(),
        motivo: None,
      },
    )
    .expect("salva");

    let estreita = Geometria {
      largura_mm: 120.0,
      ..Geometria::a4_abnt()
    };
    let pag_estreita = paginar_documento_core(
      &db,
      PaginarDocumento {
        documento_id: documento_id.clone(),
        geometria: Some(estreita),
      },
    )
    .expect("pagina estreita");
    let pag_larga = paginar_documento_core(
      &db,
      PaginarDocumento {
        documento_id: documento_id.clone(),
        geometria: None,
      },
    )
    .expect("pagina larga");

    // A geometria move texto de página; o conjunto de nós é o mesmo (I2).
    let mut chaves_estreita: Vec<&String> = pag_estreita.paginacao.alturas_por_no.keys().collect();
    let mut chaves_larga: Vec<&String> = pag_larga.paginacao.alturas_por_no.keys().collect();
    chaves_estreita.sort();
    chaves_larga.sort();
    assert_eq!(chaves_estreita, chaves_larga);
    assert!(
      pag_estreita.paginacao.paginas >= pag_larga.paginacao.paginas,
      "a geometria estreita não pode paginar em menos páginas"
    );

    // O conteúdo não mudou: o documento relido é idêntico.
    let doc = get_documento_core(&db, DocumentoRef { documento_id }).expect("lê");
    assert_eq!(doc.arvore, arvore, "paginar não toca na árvore");
  }
}

#[cfg(test)]
mod tests {
  // Os testes de Estudos. Os de Faculdade sairam em F2, os do Calendario em
  // F1, e os do TCC sao `mod testes_tcc`, acima, e saem em F4.
  use super::*;
  // -- Estudos (SPEC-D-010, D76) -------------------------------------------

  fn conteudo_novo(titulo: &str) -> NovoConteudo {
    NovoConteudo {
      titulo: titulo.into(),
      texto: None,
    }
  }

  fn cartao_novo(conteudo_id: &str, frente: &str) -> NovoCartao {
    NovoCartao {
      conteudo_id: conteudo_id.into(),
      frente: frente.into(),
      verso: "a resposta".into(),
    }
  }

  #[test]
  fn estudos_cartao_nasce_sem_passado_e_devido_na_fila() {
    let db = Db::open_in_memory().expect("banco");
    let conteudo = create_conteudo_core(&db, conteudo_novo("Hippocampo")).expect("conteúdo");
    let cartao = create_cartao_core(&db, cartao_novo(&conteudo.id, "O que faz o hippocampo?"))
      .expect("cartão");

    // I10/§7: o estado nasce vazio — criar cartão com passado nem existe na
    // struct, e as colunas ficam NULL.
    assert_eq!(cartao.estabilidade, None);
    assert_eq!(cartao.dificuldade, None);
    assert_eq!(cartao.ultima_revisao, None);
    assert_eq!(cartao.intervalo_dias, None);
    assert!(!cartao.relearning);
    assert_eq!(cartao.vencimento, None);

    let fila = fila_do_dia_core(&db, FilaDoDia::default()).expect("fila");
    assert_eq!(fila.len(), 1, "cartão novo é devido de imediato");
    assert_eq!(fila[0].id, cartao.id);
    assert_eq!(fila[0].titulo_conteudo, "Hippocampo");
    assert_eq!(fila[0].vencimento, None);
  }

  #[test]
  fn estudos_revisar_grava_o_estado_e_o_evento_e_tira_da_fila() {
    let db = Db::open_in_memory().expect("banco");
    let conteudo = create_conteudo_core(&db, conteudo_novo("Hippocampo")).expect("conteúdo");
    let cartao = create_cartao_core(&db, cartao_novo(&conteudo.id, "O que faz o hippocampo?"))
      .expect("cartão");

    let revisado = revisar_cartao_core(
      &db,
      RevisaoInput {
        cartao_id: cartao.id.clone(),
        rating: 3,
        retencao: 0.9,
        zona: None,
      },
    )
    .expect("revisa");
    assert!(revisado.estabilidade.is_some(), "após revisar há memória");
    assert!(revisado.intervalo_dias.unwrap() >= 1);
    assert!(!revisado.relearning);
    assert!(revisado.vencimento.is_some(), "agendado tem vencimento");

    // O evento mora em `revisao`, o estado em `cartao` — coisas diferentes.
    let (rating, _revisado_em): (i64, i64) = db
      .conn()
      .unwrap()
      .query_row(
        "SELECT rating, revisado_em FROM revisao WHERE cartao_id = ?1",
        params![cartao.id],
        |r| Ok((r.get(0)?, r.get(1)?)),
      )
      .expect("um evento por revisão");
    assert_eq!(rating, 3);

    // O golden pinia `good` 0,9 em cartão novo: intervalo 2 dias. Vencimento
    // é daqui a dois dias — hoje o cartão não é devido.
    let fila = fila_do_dia_core(&db, FilaDoDia::default()).expect("fila");
    assert!(
      fila.is_empty(),
      "revisado hoje com intervalo > 1 dia sai da fila"
    );
  }

  #[test]
  fn estudos_revisar_recusa_rating_retencao_e_cartao_inexistente() {
    let db = Db::open_in_memory().expect("banco");
    let conteudo = create_conteudo_core(&db, conteudo_novo("Hippocampo")).expect("conteúdo");
    let cartao = create_cartao_core(&db, cartao_novo(&conteudo.id, "O que faz o hippocampo?"))
      .expect("cartão");

    let input = |rating: i64, retencao: f64| RevisaoInput {
      cartao_id: cartao.id.clone(),
      rating,
      retencao,
      zona: None,
    };

    let err = revisar_cartao_core(&db, input(5, 0.9)).expect_err("rating 5 não existe");
    assert!(err.to_string().contains("rating"), "diz o porquê: {err}");
    let err = revisar_cartao_core(&db, input(3, 1.5)).expect_err("retenção fora de (0, 1)");
    assert!(err.to_string().contains("retencao"), "diz o porquê: {err}");
    let err = revisar_cartao_core(
      &db,
      RevisaoInput {
        cartao_id: "inexistente".into(),
        rating: 3,
        retencao: 0.9,
        zona: None,
      },
    )
    .expect_err("cartão inexistente");
    assert!(
      err.to_string().contains("não existe"),
      "diz o porquê: {err}"
    );

    let total: i64 = db
      .conn()
      .unwrap()
      .query_row("SELECT COUNT(*) FROM revisao", [], |r| r.get(0))
      .expect("conta");
    assert_eq!(total, 0, "nada foi escrito");
  }

  #[test]
  fn estudos_create_cartao_recusa_campos_vazios_e_conteudo_inexistente() {
    let db = Db::open_in_memory().expect("banco");

    let err = create_cartao_core(
      &db,
      NovoCartao {
        conteudo_id: "qualquer".into(),
        frente: "   ".into(),
        verso: "verso".into(),
      },
    )
    .expect_err("frente vazia");
    assert!(err.to_string().contains("frente"), "diz o porquê: {err}");

    let err = create_cartao_core(&db, cartao_novo("inexistente", "frente")).expect_err("conteúdo");
    assert!(
      err.to_string().contains("não existe"),
      "diz o porquê: {err}"
    );
  }
}
