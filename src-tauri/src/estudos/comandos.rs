//! O caminho de escrita de Estudos (SPEC-D-010, D76; SPEC-D-015 D96).
//!
//! Regra que governa este arquivo: **nenhum comando aceita SQL, nome de tabela
//! ou string de consulta.** A casca `#[tauri::command]` só desembrulha o
//! `State` e delega ao `X_core`.
//!
//! O motor é a crate `fsrs` (ver `Cargo.toml` para versão e licença); o que é
//! **nosso** mora em [`super::fsrs`]: a forma do estado, a regra de relearning,
//! o vencimento por dia com âncora de fuso e o relógio injetado. Mesma fronteira
//! de D50: a crate decide o número, o app decide a forma.
//!
//! **O vencimento é derivado na leitura** (cláusula do vencimento, D76): nunca é
//! coluna, nunca é gravado. Toda leitura recalcula a partir do estado.
//!
//! A zona do vencimento entra como parâmetro opcional com o literal único do
//! produto ("America/Sao_Paulo", o mesmo de `domain/zone.ts`): a fila é por dia
//! civil na zona da usuária, e o fuso da máquina nunca é consultado (I11). É
//! parâmetro — e não ambiente — para o dia em que o produto sair do Brasil.
//!
//! O que está fora daqui, e por quê (§4.4 do referencial):
//! - a retenção desejada é **parâmetro**, e o padrão dela é **[P]** — a dona do
//!   produto decide, e nenhum default é inventado aqui;
//! - o otimizador da crate não é chamado ([P]);
//! - Conteúdos, Trilhas, materiais e sessões são a tela (F8), não o motor.

use rusqlite::{OptionalExtension, params};

use serde::{Deserialize, Serialize};

use super::fsrs::{
  Avaliacao as AvaliacaoFsrs, EstadoCartao, FsrsError, dia_de_vencimento, esta_devido, revisar,
};
use crate::nucleo::{Db, DbError, new_id, now_ms};

// O `#[path]` mantém o teste em `estudos/testes.rs`, ao lado de `comandos.rs`, e
// ainda assim filho deste módulo — o que dá acesso pelo `use super::*`.
#[cfg(test)]
#[path = "testes.rs"]
mod testes;

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
