//! O caminho de escrita da Faculdade (SPEC-D-010, D74/D75; SPEC-D-015 D96).
//!
//! Regra que governa este arquivo: **nenhum comando aceita SQL, nome de tabela
//! ou string de consulta.** Todo parâmetro é uma struct `serde` com campos
//! nomeados, e o SQL vive aqui, escrito à mão.
//!
//! A casca `#[tauri::command]` só desembrulha o `State` e delega ao `X_core`, e
//! a separação existe para que as invariantes sejam testáveis sem subir um app.
//!
//! Onde as invariantes do Faculdade são garantidas:
//!
//! - **I1** — não há coluna de média em nenhuma migration; `media_da_disciplina`
//!   deriva e devolve, sem `UPDATE` de valor calculado.
//! - **I7** — `set_regra_disciplina` chama `exigir_semestre_aberto` **antes** de
//!   escrever. O mesmo vale para `create_disciplina` e `create_avaliacao`:
//!   acrescentar estrutura a um semestre encerrado faria a regra efetiva dessa
//!   estrutura ser **viva**, e o histórico passaria a mudar — o que I7 existe
//!   para impedir.
//! - **I9** — `lancar_nota` **não** tem guarda: nota é dado, e dado chega
//!   tarde. É a única escrita liberada depois do encerramento.
//!
//! As regras de verdade — a média (D74) e a resolução da regra efetiva (D75) —
//! moram nos módulos [`media`] e [`regra`], e são puras: não tocam SQL nem
//! abrem relógio. Este arquivo faz o que só ele pode, que é ler, guardar e
//! fechar o semestre.

use rusqlite::{Connection, OptionalExtension, params};

use serde::{Deserialize, Serialize};

use crate::nucleo::{Db, DbError, new_id, now_ms};

use super::media::{
  Avaliacao as AvaliacaoDoCalculo, EntradaMedia, Frequencia, NOTA_MAXIMA,
  calcular as calcular_media,
};
use super::regra::{
  Camadas, ModoExame, Regra, RegraAplicada, TipoMedia, exigir_semestre_aberto, resolver,
};

// O `#[path]` mantém o teste em `faculdade/testes.rs`, ao lado de `comandos.rs`,
// e ainda assim filho deste módulo — o que dá acesso aos nomes privados.
#[cfg(test)]
#[path = "testes.rs"]
mod testes;

// 20. Faculdade — semestre, disciplina e o motor de notas (SPEC-D-010, D74/D75)
//
// As regras são do módulo `materia/`: o motor (`media.rs`) e a resolução de
// regra (`regra.rs`). Este arquivo faz o que só ele pode fazer — SQL, guarda de
// escrita e IPC. O que é **derivado** (média, situação, frequência restante)
// nunca é gravado (I1): nasce aqui, na leitura, e morre na resposta — o mesmo
// desenho de `Responsibility.is_overdue` (INV-4).
//
// Onde as invariantes do Faculdade são garantidas:
//
// - **I1** — não há coluna de média em nenhuma migration; `media_da_disciplina`
//   deriva e devolve, sem `UPDATE` de valor calculado.
// - **I7** — `set_regra_disciplina` chama `exigir_semestre_aberto` **antes** de
//   escrever. O mesmo vale para `create_disciplina` e `create_avaliacao`:
//   acrescentar estrutura a um semestre encerrado faria a regra efetiva dessa
//   estrutura ser **viva**, e o histórico passaria a mudar — o que I7 existe
//   para impedir. A leitura de uma disciplina sem regra congelada resolveria
//   pelo padrão corrente, e o passado se moveria.
// - **I9** — `lancar_nota` **não** tem guarda: nota é dado, e dado chega
//   tarde. É a única escrita liberada depois do encerramento.
// ---------------------------------------------------------------------------

/// Colunas de regra, na ordem que [`ler_regra`] lê. As três tabelas de regra
/// (`regra_padrao`, `regra_disciplina`, `regra_congelada`) têm as mesmas
/// colunas — a diferença é a chave, não a forma.
const COLUNAS_REGRA: &str = "tipo_de_calculo, nota_minima, arredondamento, frequencia_minima, \
  exame_modo, exame_minima, exame_elegivel_minima";

fn tipo_media_de(valor: &str) -> TipoMedia {
  match valor {
    "simples" => TipoMedia::Simples,
    // O CHECK do schema garante "ponderada"; cair aqui é defeito de dado, e o
    // fallback é o valor da semente.
    _ => TipoMedia::Ponderada,
  }
}

fn tipo_media_para(tipo: TipoMedia) -> &'static str {
  match tipo {
    TipoMedia::Simples => "simples",
    TipoMedia::Ponderada => "ponderada",
  }
}

fn modo_exame_de(valor: &str) -> ModoExame {
  match valor {
    "recuperacao" => ModoExame::MediaComExame,
    "substitui" => ModoExame::ExameSubstitui,
    _ => ModoExame::Nenhum,
  }
}

fn modo_exame_para(modo: ModoExame) -> &'static str {
  match modo {
    ModoExame::Nenhum => "nenhum",
    ModoExame::MediaComExame => "recuperacao",
    ModoExame::ExameSubstitui => "substitui",
  }
}

/// Lê uma linha de regra nas colunas de [`COLUNAS_REGRA`].
fn ler_regra(r: &rusqlite::Row<'_>) -> rusqlite::Result<Regra> {
  Ok(Regra {
    tipo_media: tipo_media_de(&r.get::<_, String>(0)?),
    nota_minima: r.get(1)?,
    arredondamento: r.get(2)?,
    frequencia_minima: r.get(3)?,
    exame_modo: modo_exame_de(&r.get::<_, String>(4)?),
    exame_minima: r.get(5)?,
    exame_elegivel_minima: r.get(6)?,
  })
}

/// D75: resolve a regra efetiva de `(semestre, disciplina)` — congelada; senão,
/// personalização; senão, padrão. A ordem é a de `materia::regra::resolver`,
/// o único caminho; aqui só entra o SQL que cada camada pede.
fn resolver_regra(
  conn: &Connection,
  disciplina_id: &str,
  semestre_id: &str,
) -> Result<RegraAplicada, DbError> {
  let padrao = conn.query_row(
    &format!("SELECT {COLUNAS_REGRA} FROM regra_padrao WHERE id = 'padrao'"),
    [],
    ler_regra,
  )?;
  let personalizada = conn
    .query_row(
      &format!("SELECT {COLUNAS_REGRA} FROM regra_disciplina WHERE disciplina_id = ?1"),
      params![disciplina_id],
      ler_regra,
    )
    .optional()?;
  let congelada = conn
    .query_row(
      &format!(
        "SELECT {COLUNAS_REGRA} FROM regra_congelada \
         WHERE semestre_id = ?1 AND disciplina_id = ?2"
      ),
      params![semestre_id, disciplina_id],
      ler_regra,
    )
    .optional()?;
  Ok(resolver(&Camadas {
    congelada_do_semestre: congelada.as_ref(),
    personalizacao_da_disciplina: personalizada.as_ref(),
    padrao_global: &padrao,
  }))
}

/// `true` quando o semestre ainda está aberto. Semestre inexistente é erro:
/// a guarda de I7 nunca decide por ausência.
fn semestre_esta_aberto(conn: &Connection, semestre_id: &str) -> Result<bool, DbError> {
  let encerrado_em: Option<i64> = conn
    .query_row(
      "SELECT encerrado_em FROM semestre WHERE id = ?1",
      params![semestre_id],
      |r| r.get(0),
    )
    .optional()?
    .ok_or_else(|| DbError::Domain(format!("semestre {semestre_id} não existe")))?;
  Ok(encerrado_em.is_none())
}

/// I7 no caminho de escrita: recusa escrever regra ou estrutura em semestre
/// encerrado, **antes** de qualquer SQL de escrita.
///
/// O `!` não é enfeite: `exigir_semestre_aberto` pergunta pelo **encerrado** e
/// [`semestre_esta_aberto`] responde pelo **aberto**. A primeira versão desta
/// função passou um pelo outro e recusou **todo** semestre novo — o bug foi
/// pego pelo teste de caminho, não por leitura.
fn exigir_escrita_do_semestre(conn: &Connection, semestre_id: &str) -> Result<(), DbError> {
  exigir_semestre_aberto(!semestre_esta_aberto(conn, semestre_id)?)
    .map_err(|e| DbError::Domain(e.to_string()))
}

// ---------------------------------------------------------------------------
// Leitura
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Semestre {
  pub id: String,
  pub rotulo: String,
  pub starts_on: String,
  pub ends_on: String,
  pub encerrado_em: Option<i64>,
}

pub fn list_semestres_core(db: &Db) -> Result<Vec<Semestre>, DbError> {
  let conn = db.conn()?;
  let mut stmt = conn.prepare(
    "SELECT id, rotulo, starts_on, ends_on, encerrado_em FROM semestre
      ORDER BY starts_on DESC, rotulo",
  )?;
  let rows = stmt.query_map([], |r| {
    Ok(Semestre {
      id: r.get(0)?,
      rotulo: r.get(1)?,
      starts_on: r.get(2)?,
      ends_on: r.get(3)?,
      encerrado_em: r.get(4)?,
    })
  })?;
  Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn list_semestres(db: tauri::State<'_, Db>) -> Result<Vec<Semestre>, DbError> {
  list_semestres_core(&db)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisciplinaFilter {
  pub semestre_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Disciplina {
  pub id: String,
  pub semestre_id: String,
  pub nome: String,
  pub codigo: Option<String>,
  pub professor: Option<String>,
  pub carga_horaria: Option<i64>,
  pub status: String,
  pub total_aulas: i64,
  pub faltas: i64,
}

/// Filtro ausente não filtra — a mesma lição do C16: `coluna IS ?n` com parâmetro
/// nulo elimina tudo, e a lista vem vazia.
pub fn list_disciplinas_core(
  db: &Db,
  filter: DisciplinaFilter,
) -> Result<Vec<Disciplina>, DbError> {
  let conn = db.conn()?;
  let mut stmt = conn.prepare(
    "SELECT id, semestre_id, nome, codigo, professor, carga_horaria, status,
            total_aulas, faltas
       FROM disciplina
      WHERE (?1 IS NULL OR semestre_id = ?1)
      ORDER BY nome",
  )?;
  let rows = stmt.query_map(params![filter.semestre_id], |r| {
    Ok(Disciplina {
      id: r.get(0)?,
      semestre_id: r.get(1)?,
      nome: r.get(2)?,
      codigo: r.get(3)?,
      professor: r.get(4)?,
      carga_horaria: r.get(5)?,
      status: r.get(6)?,
      total_aulas: r.get(7)?,
      faltas: r.get(8)?,
    })
  })?;
  Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn list_disciplinas(
  db: tauri::State<'_, Db>,
  filter: DisciplinaFilter,
) -> Result<Vec<Disciplina>, DbError> {
  list_disciplinas_core(&db, filter)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisciplinaRef {
  pub disciplina_id: String,
}

/// A saída do motor, no formato do contrato (SPEC-D-010 §5). Tudo derivado:
/// nada disto volta ao banco (I1).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaResultado {
  pub media: Option<f64>,
  pub media_sem_arredondamento: Option<f64>,
  pub situacao: String,
  pub nota_necessaria_proxima: Option<f64>,
  pub frequencia_restante: Option<i64>,
  pub regra_aplicada: String,
}

/// D74: média, situação e frequência restante de uma disciplina, derivadas na
/// leitura. O cálculo mora em `materia::media`; este comando só junta as
/// entradas — avaliações com nota lançada, frequência e a regra resolvida por
/// D75 — e devolve. Nenhum caminho além deste calcula média.
pub fn media_da_disciplina_core(
  db: &Db,
  payload: DisciplinaRef,
) -> Result<MediaResultado, DbError> {
  let conn = db.conn()?;
  let (semestre_id, total_aulas, faltas) = conn
    .query_row(
      "SELECT semestre_id, total_aulas, faltas FROM disciplina WHERE id = ?1",
      params![payload.disciplina_id],
      |r| {
        Ok((
          r.get::<_, String>(0)?,
          r.get::<_, i64>(1)?,
          r.get::<_, i64>(2)?,
        ))
      },
    )
    .optional()?
    .ok_or_else(|| DbError::Domain(format!("disciplina {} não existe", payload.disciplina_id)))?;

  let aplicada = resolver_regra(&conn, &payload.disciplina_id, &semestre_id)?;

  let mut stmt = conn.prepare(
    "SELECT a.peso, a.vale_nota, n.valor
       FROM avaliacao a
       LEFT JOIN nota n ON n.avaliacao_id = a.id
      WHERE a.disciplina_id = ?1
      ORDER BY a.created_at, a.id",
  )?;
  let avaliacoes = stmt
    .query_map(params![payload.disciplina_id], |r| {
      Ok(AvaliacaoDoCalculo {
        peso: r.get(0)?,
        vale_nota: r.get::<_, i64>(1)? == 1,
        nota: r.get(2)?,
      })
    })?
    .collect::<Result<Vec<_>, _>>()?;

  let resultado = calcular_media(
    &EntradaMedia {
      avaliacoes,
      // `total_aulas` 0 é "frequência ainda não registrada", e o motor trata
      // `None` exatamente assim: sem limite derivado, sem reprovação (I5).
      frequencia: (total_aulas > 0).then(|| Frequencia {
        total_aulas: u32::try_from(total_aulas).unwrap_or(0),
        faltas: u32::try_from(faltas).unwrap_or(0),
      }),
    },
    &aplicada,
  );

  Ok(MediaResultado {
    media: resultado.media,
    media_sem_arredondamento: resultado.media_sem_arredondamento,
    situacao: resultado.situacao.as_str().to_string(),
    nota_necessaria_proxima: resultado.nota_necessaria,
    frequencia_restante: resultado.frequencia_restante,
    regra_aplicada: aplicada.rotulo(&semestre_id),
  })
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn media_da_disciplina(
  db: tauri::State<'_, Db>,
  payload: DisciplinaRef,
) -> Result<MediaResultado, DbError> {
  media_da_disciplina_core(&db, payload)
}

/// A avaliação com a nota atual, se houver. A nota entra por `LEFT JOIN`:
/// avaliação sem nota lançada aparece com `nota: null`, e avaliação que não
/// vale nota (§4.2 [D]) também aparece — quem decide se ela entra na média é
/// o motor, não a lista.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AvaliacaoComNota {
  pub id: String,
  pub disciplina_id: String,
  pub titulo: String,
  pub tipo: String,
  pub peso: f64,
  pub vale_nota: bool,
  pub plain_date: Option<String>,
  pub nota: Option<Nota>,
}

pub fn list_avaliacoes_core(
  db: &Db,
  payload: DisciplinaRef,
) -> Result<Vec<AvaliacaoComNota>, DbError> {
  let conn = db.conn()?;
  let mut stmt = conn.prepare(
    "SELECT a.id, a.disciplina_id, a.titulo, a.tipo, a.peso, a.vale_nota, a.plain_date,
            n.id, n.valor, n.lancada_em
       FROM avaliacao a
       LEFT JOIN nota n ON n.avaliacao_id = a.id
      WHERE a.disciplina_id = ?1
      ORDER BY a.plain_date IS NULL, a.plain_date, a.created_at",
  )?;
  let rows = stmt.query_map(params![payload.disciplina_id], |r| {
    // `nota.id` é a prova de que há linha no JOIN: `valor` sozinho não serve,
    // porque nota com valor `0.0` é nota válida e `NULL` de `valor` confundiria
    // os dois casos.
    let id: String = r.get(0)?;
    let nota_id: Option<String> = r.get(7)?;
    let nota = match nota_id {
      Some(nota_id) => {
        let valor: f64 = r.get(8)?;
        let lancada_em: i64 = r.get(9)?;
        Some(Nota {
          id: nota_id,
          avaliacao_id: id.clone(),
          valor,
          lancada_em,
        })
      }
      None => None,
    };
    Ok(AvaliacaoComNota {
      id,
      disciplina_id: r.get(1)?,
      titulo: r.get(2)?,
      tipo: r.get(3)?,
      peso: r.get(4)?,
      vale_nota: r.get::<_, i64>(5)? != 0,
      plain_date: r.get(6)?,
      nota,
    })
  })?;
  Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn list_avaliacoes(
  db: tauri::State<'_, Db>,
  payload: DisciplinaRef,
) -> Result<Vec<AvaliacaoComNota>, DbError> {
  list_avaliacoes_core(&db, payload)
}

// ---------------------------------------------------------------------------
// Escrita
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NovoSemestre {
  pub rotulo: String,
  pub starts_on: String,
  pub ends_on: String,
}

pub fn create_semestre_core(db: &Db, payload: NovoSemestre) -> Result<Semestre, DbError> {
  if payload.rotulo.trim().is_empty() {
    return Err(DbError::Domain("o semestre precisa de um rótulo".into()));
  }
  let now = now_ms()?;
  let id = new_id();
  let rotulo = payload.rotulo.trim().to_string();
  db.write(|tx| {
    tx.execute(
      "INSERT INTO semestre
                (id, rotulo, starts_on, ends_on, encerrado_em, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?5)",
      params![id, rotulo, payload.starts_on, payload.ends_on, now],
    )?;
    Ok(())
  })?;
  Ok(Semestre {
    id,
    rotulo,
    starts_on: payload.starts_on,
    ends_on: payload.ends_on,
    encerrado_em: None,
  })
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn create_semestre(
  db: tauri::State<'_, Db>,
  payload: NovoSemestre,
) -> Result<Semestre, DbError> {
  create_semestre_core(&db, payload)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NovaDisciplina {
  pub semestre_id: String,
  pub nome: String,
  pub codigo: Option<String>,
  pub professor: Option<String>,
  pub carga_horaria: Option<i64>,
  pub total_aulas: Option<i64>,
}

pub fn create_disciplina_core(db: &Db, payload: NovaDisciplina) -> Result<Disciplina, DbError> {
  if payload.nome.trim().is_empty() {
    return Err(DbError::Domain("a disciplina precisa de um nome".into()));
  }
  // CHECK (faltas <= total_aulas) com `faltas` nascendo 0 exige total >= 0.
  let total_aulas = payload.total_aulas.unwrap_or(0).max(0);
  let now = now_ms()?;
  let id = new_id();
  let nome = payload.nome.trim().to_string();
  db.write(|tx| {
    // I7: disciplina nova em semestre encerrado teria regra viva.
    exigir_escrita_do_semestre(tx, &payload.semestre_id)?;
    tx.execute(
      "INSERT INTO disciplina
                (id, semestre_id, nome, codigo, professor, carga_horaria, status,
                 total_aulas, faltas, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'ativa', ?7, 0, ?8, ?8)",
      params![
        id,
        payload.semestre_id,
        nome,
        payload.codigo,
        payload.professor,
        payload.carga_horaria,
        total_aulas,
        now,
      ],
    )?;
    Ok(())
  })?;
  Ok(Disciplina {
    id,
    semestre_id: payload.semestre_id,
    nome,
    codigo: payload.codigo,
    professor: payload.professor,
    carga_horaria: payload.carga_horaria,
    status: "ativa".into(),
    total_aulas,
    faltas: 0,
  })
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn create_disciplina(
  db: tauri::State<'_, Db>,
  payload: NovaDisciplina,
) -> Result<Disciplina, DbError> {
  create_disciplina_core(&db, payload)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NovaAvaliacao {
  pub disciplina_id: String,
  pub titulo: String,
  pub tipo: String,
  pub peso: f64,
  pub vale_nota: Option<bool>,
  pub plain_date: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Avaliacao {
  pub id: String,
  pub disciplina_id: String,
  pub titulo: String,
  pub tipo: String,
  pub peso: f64,
  pub vale_nota: bool,
  pub plain_date: Option<String>,
}

/// Tipos que a tela pode oferecer. O CHECK do schema aceita estes cinco; a
/// recusa aqui existe para dar erro legível em vez de `CHECK constraint failed`.
const TIPOS_AVALIACAO: [&str; 5] = ["prova", "trabalho", "apresentacao", "entrega", "outro"];

pub fn create_avaliacao_core(db: &Db, payload: NovaAvaliacao) -> Result<Avaliacao, DbError> {
  if payload.titulo.trim().is_empty() {
    return Err(DbError::Domain("a avaliação precisa de um título".into()));
  }
  if !TIPOS_AVALIACAO.contains(&payload.tipo.as_str()) {
    return Err(DbError::Domain(format!(
      "tipo de avaliação `{}` não existe: use um de {}",
      payload.tipo,
      TIPOS_AVALIACAO.join(", ")
    )));
  }
  if payload.peso <= 0.0 {
    return Err(DbError::Domain(
      "o peso da avaliação precisa ser positivo".into(),
    ));
  }
  let now = now_ms()?;
  let id = new_id();
  let titulo = payload.titulo.trim().to_string();
  let vale_nota = payload.vale_nota.unwrap_or(true);
  db.write(|tx| {
    let semestre_id: String = tx
      .query_row(
        "SELECT semestre_id FROM disciplina WHERE id = ?1",
        params![payload.disciplina_id],
        |r| r.get(0),
      )
      .optional()?
      .ok_or_else(|| DbError::Domain(format!("disciplina {} não existe", payload.disciplina_id)))?;
    // I7: avaliação nova em semestre encerrado muda a conta do histórico.
    exigir_escrita_do_semestre(tx, &semestre_id)?;
    tx.execute(
      "INSERT INTO avaliacao
                (id, disciplina_id, titulo, tipo, peso, vale_nota, plain_date,
                 created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
      params![
        id,
        payload.disciplina_id,
        titulo,
        payload.tipo,
        payload.peso,
        i64::from(vale_nota),
        payload.plain_date,
        now,
      ],
    )?;
    Ok(())
  })?;
  Ok(Avaliacao {
    id,
    disciplina_id: payload.disciplina_id,
    titulo,
    tipo: payload.tipo,
    peso: payload.peso,
    vale_nota,
    plain_date: payload.plain_date,
  })
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn create_avaliacao(
  db: tauri::State<'_, Db>,
  payload: NovaAvaliacao,
) -> Result<Avaliacao, DbError> {
  create_avaliacao_core(&db, payload)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LancarNota {
  pub avaliacao_id: String,
  pub valor: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Nota {
  pub id: String,
  pub avaliacao_id: String,
  pub valor: f64,
  pub lancada_em: i64,
}

/// Lança (ou corrige) a nota de uma avaliação. I9: **sem guarda de semestre** —
/// nota é dado, e a correção real de um semestre encerrado é a nota que
/// faltou. §4.2 **[D]**: avaliação que não vale nota não recebe nota.
pub fn lancar_nota_core(db: &Db, payload: LancarNota) -> Result<Nota, DbError> {
  if !(0.0..=NOTA_MAXIMA).contains(&payload.valor) {
    return Err(DbError::Domain(format!(
      "nota {} fora da faixa 0 a {}",
      payload.valor, NOTA_MAXIMA
    )));
  }
  let now = now_ms()?;
  let id = new_id();
  db.write(|tx| {
    let vale_nota: Option<i64> = tx
      .query_row(
        "SELECT vale_nota FROM avaliacao WHERE id = ?1",
        params![payload.avaliacao_id],
        |r| r.get(0),
      )
      .optional()?;
    let Some(vale_nota) = vale_nota else {
      return Err(DbError::Domain(format!(
        "avaliação {} não existe",
        payload.avaliacao_id
      )));
    };
    if vale_nota == 0 {
      return Err(DbError::Domain(
        "avaliação que não vale nota não recebe nota (§4.2): ela vira \
         responsabilidade"
          .into(),
      ));
    }
    tx.execute(
      "INSERT INTO nota (id, avaliacao_id, valor, lancada_em, created_at)
             VALUES (?1, ?2, ?3, ?4, ?4)
             ON CONFLICT (avaliacao_id) DO UPDATE SET
               valor = excluded.valor,
               lancada_em = excluded.lancada_em",
      params![id, payload.avaliacao_id, payload.valor, now],
    )?;
    Ok(())
  })?;
  // Re-lê: o `id` gravado pode ser o da linha que já existia (UPSERT).
  let conn = db.conn()?;
  Ok(conn.query_row(
    "SELECT id, avaliacao_id, valor, lancada_em FROM nota WHERE avaliacao_id = ?1",
    params![payload.avaliacao_id],
    |r| {
      Ok(Nota {
        id: r.get(0)?,
        avaliacao_id: r.get(1)?,
        valor: r.get(2)?,
        lancada_em: r.get(3)?,
      })
    },
  )?)
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn lancar_nota(db: tauri::State<'_, Db>, payload: LancarNota) -> Result<Nota, DbError> {
  lancar_nota_core(&db, payload)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegraInput {
  pub tipo_de_calculo: String,
  pub nota_minima: f64,
  pub arredondamento: f64,
  pub frequencia_minima: f64,
  pub exame_modo: String,
  pub exame_minima: f64,
  pub exame_elegivel_minima: f64,
}

/// Amigável onde o CHECK do schema responde cru. Espelha `validar_novo_evento`:
/// o schema também protege, mas `CHECK constraint failed` não diz o campo.
fn validar_regra(regra: &RegraInput) -> Result<(), DbError> {
  if !matches!(regra.tipo_de_calculo.as_str(), "simples" | "ponderada") {
    return Err(DbError::Domain(format!(
      "tipo de cálculo `{}` não existe: use 'simples' ou 'ponderada'",
      regra.tipo_de_calculo
    )));
  }
  if !(0.0..=NOTA_MAXIMA).contains(&regra.nota_minima) {
    return Err(DbError::Domain(format!(
      "nota mínima {} fora da faixa 0 a {}",
      regra.nota_minima, NOTA_MAXIMA
    )));
  }
  if !(regra.arredondamento > 0.0 && regra.arredondamento <= 1.0) {
    return Err(DbError::Domain(format!(
      "arredondamento {} fora da faixa (0, 1]",
      regra.arredondamento
    )));
  }
  if !(0.0..=1.0).contains(&regra.frequencia_minima) {
    return Err(DbError::Domain(format!(
      "frequência mínima {} fora da faixa 0 a 1",
      regra.frequencia_minima
    )));
  }
  if !matches!(
    regra.exame_modo.as_str(),
    "nenhum" | "recuperacao" | "substitui"
  ) {
    return Err(DbError::Domain(format!(
      "modo de exame `{}` não existe: use 'nenhum', 'recuperacao' ou 'substitui'",
      regra.exame_modo
    )));
  }
  if !(0.0..=NOTA_MAXIMA).contains(&regra.exame_minima)
    || !(0.0..=NOTA_MAXIMA).contains(&regra.exame_elegivel_minima)
  {
    return Err(DbError::Domain(
      "nota mínima do exame e de elegibilidade ficam na faixa 0 a 10".into(),
    ));
  }
  Ok(())
}

/// Padrão global (D75). Sem guarda de semestre: o padrão não pertence a
/// semestre nenhum — quem ele move são as disciplinas não personalizadas,
/// e é exatamente isso que a herança viva manda (I6).
pub fn set_regra_padrao_core(db: &Db, payload: RegraInput) -> Result<(), DbError> {
  validar_regra(&payload)?;
  let now = now_ms()?;
  db.write(|tx| {
    tx.execute(
      "UPDATE regra_padrao
                 SET tipo_de_calculo = ?1, nota_minima = ?2, arredondamento = ?3,
                     frequencia_minima = ?4, exame_modo = ?5, exame_minima = ?6,
                     exame_elegivel_minima = ?7, atualizado_em = ?8
               WHERE id = 'padrao'",
      params![
        payload.tipo_de_calculo,
        payload.nota_minima,
        payload.arredondamento,
        payload.frequencia_minima,
        payload.exame_modo,
        payload.exame_minima,
        payload.exame_elegivel_minima,
        now,
      ],
    )?;
    Ok(())
  })
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn set_regra_padrao(db: tauri::State<'_, Db>, payload: RegraInput) -> Result<(), DbError> {
  set_regra_padrao_core(&db, payload)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegraDisciplinaInput {
  pub disciplina_id: String,
  pub regra: RegraInput,
}

/// Personalização por disciplina (D75): substituição **por inteiro**, e a
/// guarda do semestre aberto antes de qualquer escrita (I7).
pub fn set_regra_disciplina_core(db: &Db, payload: RegraDisciplinaInput) -> Result<(), DbError> {
  validar_regra(&payload.regra)?;
  let now = now_ms()?;
  db.write(|tx| {
    let semestre_id: String = tx
      .query_row(
        "SELECT semestre_id FROM disciplina WHERE id = ?1",
        params![payload.disciplina_id],
        |r| r.get(0),
      )
      .optional()?
      .ok_or_else(|| DbError::Domain(format!("disciplina {} não existe", payload.disciplina_id)))?;
    exigir_escrita_do_semestre(tx, &semestre_id)?;
    tx.execute(
      "INSERT INTO regra_disciplina
                (disciplina_id, tipo_de_calculo, nota_minima, arredondamento,
                 frequencia_minima, exame_modo, exame_minima, exame_elegivel_minima,
                 atualizado_em)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT (disciplina_id) DO UPDATE SET
               tipo_de_calculo = excluded.tipo_de_calculo,
               nota_minima = excluded.nota_minima,
               arredondamento = excluded.arredondamento,
               frequencia_minima = excluded.frequencia_minima,
               exame_modo = excluded.exame_modo,
               exame_minima = excluded.exame_minima,
               exame_elegivel_minima = excluded.exame_elegivel_minima,
               atualizado_em = excluded.atualizado_em",
      params![
        payload.disciplina_id,
        payload.regra.tipo_de_calculo,
        payload.regra.nota_minima,
        payload.regra.arredondamento,
        payload.regra.frequencia_minima,
        payload.regra.exame_modo,
        payload.regra.exame_minima,
        payload.regra.exame_elegivel_minima,
        now,
      ],
    )?;
    Ok(())
  })
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn set_regra_disciplina(
  db: tauri::State<'_, Db>,
  payload: RegraDisciplinaInput,
) -> Result<(), DbError> {
  set_regra_disciplina_core(&db, payload)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemestreRef {
  pub semestre_id: String,
}

/// D75: encerrar é um **ato que grava**. A regra efetiva de cada disciplina —
/// personalização, se houver; senão, padrão — vira linha em `regra_congelada`
/// e o semestre fecha. Depois disto, mudar o padrão não altera o histórico
/// (I7), e a regra gravada responde "por que essa disciplina reprovou" (I8).
pub fn encerrar_semestre_core(db: &Db, payload: SemestreRef) -> Result<(), DbError> {
  let now = now_ms()?;
  db.write(|tx| {
    let encerrado_em: Option<i64> = tx
      .query_row(
        "SELECT encerrado_em FROM semestre WHERE id = ?1",
        params![payload.semestre_id],
        |r| r.get(0),
      )
      .optional()?
      .ok_or_else(|| DbError::Domain(format!("semestre {} não existe", payload.semestre_id)))?;
    if encerrado_em.is_some() {
      return Err(DbError::Domain(
        "semestre já encerrado: a regra do passado é imutável (D75)".into(),
      ));
    }

    let mut stmt = tx.prepare("SELECT id FROM disciplina WHERE semestre_id = ?1")?;
    let ids = stmt
      .query_map(params![payload.semestre_id], |r| r.get::<_, String>(0))?
      .collect::<Result<Vec<_>, _>>()?;
    drop(stmt);

    for disciplina_id in ids {
      let aplicada = resolver_regra(tx, &disciplina_id, &payload.semestre_id)?;
      tx.execute(
        "INSERT INTO regra_congelada
                   (semestre_id, disciplina_id, tipo_de_calculo, nota_minima,
                    arredondamento, frequencia_minima, exame_modo, exame_minima,
                    exame_elegivel_minima, congelado_em)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
          payload.semestre_id,
          disciplina_id,
          tipo_media_para(aplicada.regra.tipo_media),
          aplicada.regra.nota_minima,
          aplicada.regra.arredondamento,
          aplicada.regra.frequencia_minima,
          modo_exame_para(aplicada.regra.exame_modo),
          aplicada.regra.exame_minima,
          aplicada.regra.exame_elegivel_minima,
          now,
        ],
      )?;
    }

    tx.execute(
      "UPDATE semestre SET encerrado_em = ?1, updated_at = ?1 WHERE id = ?2",
      params![now, payload.semestre_id],
    )?;
    Ok(())
  })
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn encerrar_semestre(db: tauri::State<'_, Db>, payload: SemestreRef) -> Result<(), DbError> {
  encerrar_semestre_core(&db, payload)
}
