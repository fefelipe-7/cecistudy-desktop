//! O caminho de escrita do TCC & Projetos (SPEC-D-011, SPEC-D-013, D85..D88;
//! SPEC-D-015 D96).
//!
//! Regra que governa este arquivo: **nenhum comando aceita SQL, nome de tabela
//! ou string de consulta.** A casca `#[tauri::command]` só desembrulha o
//! `State` e delega ao `X_core`.
//!
//! O modelo, a paginação e a serialização canônica vivem em [`super`], que é
//! motor puro: não toca SQL. Este arquivo é o caminho de escrita e leitura —
//! valida a árvore com `linhas_da_arvore` (as mesmas garantias da CHECK e do
//! índice único) e grava.
//!
//! Onde as regras são impostas:
//! - **D85** — a árvore é substituída inteira numa transação; nenhuma coluna de
//!   página ou posição é gravada, e a geometria é **parâmetro** do motor, nunca
//!   coluna.
//! - **D86** — a paginação é derivada. O envelope devolve o `medidor` porque a
//!   regra proíbe paginação **silenciosamente** aproximada: a tela mostra que o
//!   número é do medidor determinístico, e não do Word.
//! - **D87** — a versão é snapshot canônico; `salvar_documento` só versiona se
//!   o snapshot mudou. Escrita sem mudança não cria versão.
//! - **D88** — item duplicado exige substituto; o estado é lista fechada.
//! - **D66** — o vínculo da Base de pesquisa guarda o id do alvo, não copia
//!   conteúdo. Referência é da Biblioteca, Fragmento é do Conhecimento, e
//!   `vincular` é idempotente.

use rusqlite::{Connection, OptionalExtension, params};

use serde::{Deserialize, Serialize};

use super::documento::{
  DocError, Inline, ModoVinculo, NoDocumento, NoLinha, TipoNo, arvore_de_linhas, linhas_da_arvore,
  marcas_de_coluna, marcas_para_coluna,
};
use super::item::{EstadoItem, ItemAResolver, TipoItem};
use super::medidor::MedidorDeterministico;
use super::paginacao::{Geometria, Paginacao, paginar};
use super::versao::snapshot_canonico;
use crate::nucleo::{Db, DbError, new_id, now_ms};

// O `#[path]` mantém o teste em `tcc/testes.rs`, ao lado de `comandos.rs`, e
// ainda assim filho deste módulo — o que dá acesso ao `X_core` e ao `params!`
// pelo `use super::*` do arquivo de teste.
#[cfg(test)]
#[path = "testes.rs"]
mod testes;

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
