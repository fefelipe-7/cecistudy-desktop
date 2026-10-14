//! Comandos de domínio do Calendário (spec 01, §3.4).
//!
//! Regra que governa este arquivo (§3.4): **nenhum comando aceita SQL, nome de
//! tabela ou string de consulta.** Todo parâmetro é uma struct `serde` com
//! campos nomeados, e o SQL vive aqui, escrito à mão.
//!
//! Estrutura de cada comando, em duas camadas:
//!
//! - `fn algo_core(db: &Db, …)` — a regra, sem Tauri. É o que os testes chamam.
//! - `#[tauri::command] fn algo(db: State<Db>, …)` — a casca, que só desembrulha
//!   o `State` e delega.
//!
//! A separação existe para que INV-3, INV-5, INV-6 e D12 sejam testáveis sem
//! subir um app. `State<Db>` é `&Db`, e o núcleo só precisa de `&Db` — por isso
//! `Db` guarda a conexão atrás de um `Mutex`.
//!
//! Onde as invariantes são garantidas:
//!
//! - **INV-1** — `plan_block.responsibility_id NOT NULL` no schema; a struct
//!   `UpsertBlock` exige o campo, então o compilador já impede a ausência.
//! - **INV-3** — `complete_item` exige `record`; sem ele, nada é escrito.
//! - **INV-4** — nada de atraso é persistido; este arquivo nunca compara
//!   `due_at` com o relógio. O atraso é lido em `domain/overdue.ts`.
//! - **INV-5** — `update_recurrence` com `scope != "serie"` não toca
//!   `event_recurrence`.
//! - **INV-6** — `guard_origin` recusa `origin: "google"` em escrita.
//! - **D12** — `execution_record` não tem FK para o alvo e nenhum comando de
//!   deleção o apaga.
//!
//! A máquina de estados é de `domain.rs`, e é imposta no caminho de escrita.
//! Até 2026-10-03 ela não era: o cabeçalho dizia "quem valida a transição é o
//! TypeScript, antes de chamar este arquivo", e esse "antes" não existia — não há
//! chamador de `transitionAllowed` em produção, e `concluido → planejado` era
//! gravável de ponta a ponta (débito C5). A ADR-006 (seção "Resolução da
//! pendência 1") decide que os quatro arquivos de `domain/` viram espelho; o
//! dono é `domain.rs`.
use rusqlite::{Connection, OptionalExtension, Transaction, params};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use chrono::{DateTime, TimeZone};
use rrule::{RRuleError, RRuleSet, Tz};

use crate::db::{Db, DbError};
use crate::domain::{destinations_of, is_overdue, mark_overlaps, transition_allowed};
use crate::estudos::fsrs::{
  Avaliacao as AvaliacaoFsrs, EstadoCartao, FsrsError, dia_de_vencimento, esta_devido, revisar,
};
use crate::faculdade::media::{
  Avaliacao as AvaliacaoDoCalculo, EntradaMedia, Frequencia, NOTA_MAXIMA,
  calcular as calcular_media,
};
use crate::faculdade::regra::{
  Camadas, ModoExame, Regra, RegraAplicada, TipoMedia, exigir_semestre_aberto, resolver,
};
use crate::tcc::documento::{
  DocError, Inline, ModoVinculo, NoDocumento, NoLinha, TipoNo, arvore_de_linhas, linhas_da_arvore,
  marcas_de_coluna, marcas_para_coluna,
};
use crate::tcc::item::{EstadoItem, ItemAResolver, TipoItem};
use crate::tcc::medidor::MedidorDeterministico;
use crate::tcc::paginacao::{Geometria, Paginacao, paginar};
use crate::tcc::versao::snapshot_canonico;

/// Lê o estado atual e recusa a transição se a tabela não a permite.
///
/// `table` vem sempre de um literal do próprio código, nunca de entrada da
/// usuária — é o mesmo cuidado de `insert_execution`. Fica aqui, e não em
/// `domain.rs`, porque exige SQL: `domain.rs` mora só a regra.
fn exigir_transicao(tx: &Transaction<'_>, table: &str, id: &str, to: &str) -> Result<(), DbError> {
  let sql = format!("SELECT state FROM {table} WHERE id = ?1");
  let from: Option<String> = tx.query_row(&sql, params![id], |r| r.get(0)).optional()?;

  // Alvo inexistente: quem valida é o `UPDATE` seguinte, com contagem de
  // linhas afetadas. Aqui a ausência não é erro de transição.
  let Some(from) = from else {
    return Ok(());
  };

  if transition_allowed(&from, to) {
    return Ok(());
  }

  let destinos = destinations_of(&from);
  let lista = if destinos.is_empty() {
    "nenhum estado".to_string()
  } else {
    destinos.join(", ")
  };
  Err(DbError::Domain(format!(
    "transição ilegal `{from}` → `{to}`: a partir de `{from}` só é {lista} (ou a mesma)"
  )))
}

// ---------------------------------------------------------------------------
// Janela e utilitários
// ---------------------------------------------------------------------------

/// Janela visível. `from` e `to` são exclusivos, para não repetir o item da
/// borda entre duas janelas consecutivas.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Window {
  pub from: i64,
  pub to: i64,
}

fn new_id() -> String {
  Uuid::new_v4().to_string()
}

fn now_ms() -> Result<i64, DbError> {
  // O `Clock` injetável é do lado TypeScript. Aqui o relógio do sistema é a
  // única fonte possível, e nenhuma regra de domínio depende deste valor.
  let ms = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map_err(|e| DbError::Domain(format!("relógio antes da época Unix: {e}")))?;
  i64::try_from(ms.as_millis()).map_err(|_| DbError::Domain("instante fora do range de i64".into()))
}

/// INV-6: `google` é leitura. A camada de binding nunca envia isso; mesmo que
/// envie, a escrita é recusada aqui — a restrição fica no caminho de escrita,
/// não na disciplina de quem chama.
fn guard_origin(origin: &str) -> Result<(), DbError> {
  if origin == "google" {
    return Err(DbError::Domain(
      "item do Google Calendar é somente leitura (INV-6)".into(),
    ));
  }
  Ok(())
}

// ---------------------------------------------------------------------------
// 1. migrate
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrateResult {
  pub version: u32,
}

pub fn migrate_core(db: &Db) -> Result<MigrateResult, DbError> {
  Ok(MigrateResult {
    version: db.migrate()?,
  })
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn migrate(db: tauri::State<'_, Db>) -> Result<MigrateResult, DbError> {
  migrate_core(&db)
}

// ---------------------------------------------------------------------------
// 2-3. Camadas
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Layer {
  pub id: String,
  pub label: String,
  pub tone: String,
  pub icon: String,
  pub visible: bool,
  pub position: i64,
}

pub fn list_layers_core(db: &Db) -> Result<Vec<Layer>, DbError> {
  let conn = db.conn()?;
  let mut stmt =
    conn.prepare("SELECT id, title, tone, icon, visible, position FROM layer ORDER BY position")?;
  let rows = stmt.query_map([], |r| {
    Ok(Layer {
      id: r.get(0)?,
      label: r.get(1)?,
      tone: r.get(2)?,
      icon: r.get(3)?,
      visible: r.get::<_, i64>(4)? == 1,
      position: r.get(5)?,
    })
  })?;
  Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn list_layers(db: tauri::State<'_, Db>) -> Result<Vec<Layer>, DbError> {
  list_layers_core(&db)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetLayerVisibility {
  pub id: String,
  pub visible: bool,
}

pub fn set_layer_visibility_core(db: &Db, payload: SetLayerVisibility) -> Result<(), DbError> {
  db.write(|tx| {
    tx.execute(
      "UPDATE layer SET visible = ?1 WHERE id = ?2",
      params![i64::from(payload.visible), payload.id],
    )?;
    Ok(())
  })
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn set_layer_visibility(
  db: tauri::State<'_, Db>,
  payload: SetLayerVisibility,
) -> Result<(), DbError> {
  set_layer_visibility_core(&db, payload)
}

// ---------------------------------------------------------------------------
// 4-5. Leitura de evento e responsabilidade
// ---------------------------------------------------------------------------

/// Item da grade. `kind` separa o que a spec §13 manda nunca misturar: o que
/// está agendado, o que precisa ser feito e o que aconteceu.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GridItem {
  pub id: String,
  pub event_id: String,
  pub layer_id: String,
  pub title: String,
  pub starts_at: i64,
  pub ends_at: i64,
  pub state: String,
  pub override_kind: String,
  pub is_recurring: bool,
  /// D7 — sobreposição é derivada e **sinalizada, nunca bloqueante**. A regra é
  /// de `domain.rs`; este campo é o resultado, e é o que a grade usa para
  /// desenhar o aviso. Antes de 2026-10-03 não existia, e por isso o app não
  /// detectava conflito nenhum, embora `D7` e `02-grade-desktop.md` prometessem.
  pub has_conflict: bool,
  /// INV-2 / INV-6 — a origem viaja **na leitura** porque a regra é
  /// "Google é somente leitura", e essa regra só é respeitada se o cliente
  /// souber a origem **antes** de oferecer a ação. Sem este campo a tela não
  /// tem como distinguir um item próprio de um item do Google, e acabaria
  /// oferecendo "Concluir" num evento do Google para só receber `guard_origin`
  /// recusando a escrita.
  pub origin: String,
  /// D1.3 — os destinos de estado válidos a partir de `state`, derivados de
  /// `domain::destinations_of`. A tela decide quais ações oferecer sem
  /// reimplementar a máquina de estados; e se ela oferecer algo ilegal, o
  /// `exigir_transicao` do caminho de escrita continua recusando. É o que tira
  /// `state-machine.ts` do caminho de produção.
  pub allowed_transitions: Vec<String>,
}

/// Materializa as instâncias de um evento na janela pedida (D10).
///
/// A regra de recorrência mora **aqui**, no backend: `AGENTS.md` determina que a
/// verdade de domínio é o Rust. O espelho em `domain/recurrence.ts` existe para a
/// UI renderizar e para testar sem o toolchain — quando os dois divergem, o Rust
/// está certo e o espelho é o bug.
///
/// A gravação é idempotente por `UNIQUE (event_id, original_start)`: materializar
/// duas vezes a mesma janela não duplica nada, e por isso a chamada pode acontecer
/// em toda leitura de janela sem custo correto a pagar.
fn materialize_occurrences(
  tx: &rusqlite::Transaction<'_>,
  from: i64,
  to: i64,
) -> Result<usize, DbError> {
  let mut stmt = tx.prepare(
    "SELECT e.id, e.starts_at, e.ends_at, e.state,
                r.id, r.rrule, r.until_at, r.count, r.exdates_json
         FROM calendar_event e
         LEFT JOIN event_recurrence r ON r.event_id = e.id
         WHERE (e.starts_at < ?1 AND e.ends_at > ?2) OR r.id IS NOT NULL",
  )?;
  let rows = stmt.query_map(params![to, from], |r| {
    Ok(RecurringEvent {
      event_id: r.get(0)?,
      starts_at: r.get(1)?,
      ends_at: r.get(2)?,
      event_state: r.get(3)?,
      rule_id: r.get(4)?,
      rrule: r.get(5)?,
      until_at: r.get(6)?,
      count: r.get(7)?,
      exdates_json: r.get(8)?,
    })
  })?;

  let mut now = None;
  let mut written = 0usize;
  for row in rows {
    let event = row?;
    let stamp = match now {
      Some(value) => value,
      None => {
        let value = now_ms()?;
        now = Some(value);
        value
      }
    };
    let duration = event.ends_at - event.starts_at;
    let state = event
      .event_state
      .clone()
      .unwrap_or_else(|| "planejado".to_string());

    // Evento sem regra: ele **é** a própria instância. Sem esta linha o
    // evento nunca apareceria na grade, porque a grade lê `event_occurrence`.
    let starts = if event.rule_id.is_none() {
      vec![event.starts_at]
    } else {
      expand_rule(&event, from, to)?
    };

    for start in starts {
      if event
        .exdates_json
        .as_deref()
        .is_some_and(|exdates| is_excluded(exdates, start))
      {
        continue;
      }
      let inserted = tx.execute(
        "INSERT INTO event_occurrence
                   (id, event_id, rule_id, original_start, starts_at, ends_at,
                    state, override, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'none', ?8, ?8)
                 ON CONFLICT (event_id, original_start) DO NOTHING",
        params![
          new_id(),
          event.event_id,
          event.rule_id,
          start,
          start,
          start + duration,
          state,
          stamp,
        ],
      )?;
      written += inserted;
    }
  }
  Ok(written)
}

/// Uma regra já lida do banco, pronta para ser expandida.
struct RecurringEvent {
  event_id: String,
  starts_at: i64,
  ends_at: i64,
  event_state: Option<String>,
  rule_id: Option<String>,
  rrule: Option<String>,
  until_at: Option<i64>,
  count: Option<i64>,
  exdates_json: Option<String>,
}

/// Expande a RRULE dentro de `[from, to)`.
///
/// **F1.2 — o motor é o crate `rrule`, implementação de RFC 5545.** O parser de
/// 6 chaves escrito à mão foi removido em 2026-10-03, e com ele os três bugs que
/// a C6 descrevia: `FREQ=MONTHLY` caía no ramo `_` e virava semanal; `FREQ`
/// inválido virava `unwrap_or("WEEKLY")`; e `FREQ=WEEKLY` **sem `BYDAY`**
/// produzia uma ocorrência por dia, porque o motor nunca checava o dia da semana.
///
/// O que `rrule` traz e o parser não traz: `BYSETPOS`, `BYMONTHDAY`,
/// `BYYEARDAY`, `WKST`, `BYMONTH`, `BYSECOND`, `BYMINUTE`, `BYHOUR`, e
/// `FREQ=YEARLY`. `golden/recurrence.jsonl` é o contrato que prova que este
/// lado e o espelho `domain/recurrence.ts` concordam.
///
/// `count` e `until_at` são colunas e **vencem** a string: um `rrule` editado
/// pela usuária não apaga o limite que ela gravou.
fn expand_rule(event: &RecurringEvent, from: i64, to: i64) -> Result<Vec<i64>, DbError> {
  let rrule = match &event.rrule {
    Some(value) if !value.trim().is_empty() => value.trim(),
    _ => return Ok(Vec::new()),
  };

  // Toda recorrência nasce ancorada no próprio evento: `starts_at` é o DTSTART
  // e nunca é reinterpretado no fuso do sistema (D6). A âncora vai em UTC
  // porque o passo é UTC — `dtstart_tz` fica como informação do evento.
  let dtstart = ms_para_tz(event.starts_at)
    .ok_or_else(|| DbError::Domain(format!("âncora inválida: {}", event.starts_at)))?;
  let rrule_efetiva = rrule_efetiva(rrule, event.count, event.until_at);

  // `DTSTART` e `RRULE` entram na mesma string: o parser do crate lê as duas
  // juntas, e montar o conjunto em duas passos exigiria um método de retorno
  // diferente para erro.
  let conjunto = format!(
    "DTSTART:{}\nRRULE:{rrule_efetiva}",
    dtstart.format("%Y%m%dT%H%M%SZ")
  );
  let mut set: RRuleSet = conjunto
    .parse()
    .map_err(|e: RRuleError| DbError::Domain(format!("RRULE inválida: {e}")))?;

  // EXDATE é nativo do crate. Antes, `exdates_json` era filtrado à mão depois da
  // expansão, e a ordem contava: filtrar depois de materializar é o que fazia a
  // materialização re-tentar a mesma instance.
  if let Some(json) = &event.exdates_json {
    let exdates: Vec<i64> = serde_json::from_str(json).unwrap_or_default();
    let convertidos: Vec<_> = exdates.iter().filter_map(|ms| ms_para_tz(*ms)).collect();
    set = set.set_exdates(convertidos);
  }

  // `before` é exclusivo, e a janela `[from, to)` é semiaberta: `from - 1`
  // inclui a ocorrência que começa exatamente em `from`.
  let teto = event.until_at.unwrap_or(i64::MAX).min(to);
  if let Some(ate) = ms_para_tz(teto.saturating_sub(1)) {
    set = set.before(ate);
  }
  if let Some(depois) = ms_para_tz(from.saturating_sub(1)) {
    set = set.after(depois);
  }

  // Teto de segurança: uma janela de leitura nunca passa de alguns anos, e
  // `all()` recusa limite acima de 65535. `u16` é o maior valor que a API
  // aceita, e é o que impede uma RRULE corrompida de travar o app.
  const MAX: u16 = 65535;
  let saida: Vec<i64> = set
    .all(MAX)
    .dates
    .into_iter()
    .map(|dt| dt.timestamp_millis())
    .filter(|ms| *ms >= from && *ms < to)
    .collect();
  Ok(saida)
}

/// Aplica o `COUNT` e o `UNTIL` das colunas por cima da string.
///
/// A coluna vence porque é o que a usuária gravou: um `rrule` editado pela mão
/// não pode apagar o limite que o app já tinha. Devolver a string efetiva
/// montada — e não post-processar a lista — é o que mantém `COUNT` contando a
/// partir da âncora e não a partir da janela lida.
fn rrule_efetiva(rrule: &str, count: Option<i64>, until_at: Option<i64>) -> String {
  let corpo = rrule
    .trim()
    .trim_start_matches("RRULE:")
    .split(';')
    .map(str::trim)
    .filter(|parte| !parte.is_empty())
    .filter(|parte| {
      let chave = parte.split('=').next().unwrap_or("").to_ascii_uppercase();
      chave != "COUNT" && chave != "UNTIL"
    })
    .collect::<Vec<_>>()
    .join(";");

  let mut partes = vec![corpo];
  if let Some(n) = count.filter(|n| *n > 0) {
    partes.push(format!("COUNT={n}"));
  }
  if let Some(ate) = until_at.and_then(ms_para_tz) {
    partes.push(format!("UNTIL={}", ate.format("%Y%m%dT%H%M%SZ")));
  }
  partes.join(";")
}

/// Epoch-ms para `DateTime<Tz>` em UTC. `None` fora da faixa representável.
fn ms_para_tz(ms: i64) -> Option<DateTime<Tz>> {
  let dt = chrono::Utc.timestamp_millis_opt(ms).single()?;
  Some(Tz::UTC.from_utc_datetime(&dt.naive_utc()))
}

/// (ano, mês, dia) a partir de dias desde 1970-01-01. Aritmética civil de
/// Howard Hinnant: funciona para datas negativas, ao contrário de dividir por 30
/// ou por 365. Só o teste de recorrência usa — o motor não faz mais data civil.
#[cfg(test)]
fn civil_from_days(z: i64) -> (i64, i64, i64) {
  let z = z + 719_468;
  let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
  let doe = z - era * 146_097;
  let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
  let ano = yoe + era * 400;
  let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
  let mp = (5 * doy + 2) / 153;
  let dia = doy - (153 * mp + 2) / 5 + 1;
  let mes = if mp < 10 { mp + 3 } else { mp - 9 };
  (if mes <= 2 { ano + 1 } else { ano }, mes, dia)
}

/// Inverso de `civil_from_days`. Só o teste de recorrência usa.
#[cfg(test)]
fn days_from_civil(ano: i64, mes: i64, dia: i64) -> i64 {
  let ano = if mes <= 2 { ano - 1 } else { ano };
  let era = if ano >= 0 { ano } else { ano - 399 } / 400;
  let yoe = ano - era * 400;
  let mp = if mes > 2 { mes - 3 } else { mes + 9 };
  let doy = (153 * mp + 2) / 5 + dia - 1;
  let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
  era * 146_097 + doe - 719_468
}

fn is_excluded(exdates_json: &str, start: i64) -> bool {
  serde_json::from_str::<Vec<i64>>(exdates_json)
    .map(|dates| dates.contains(&start))
    .unwrap_or(false)
}

/// Cria ou substitui a regra de recorrência de um evento (o comando que faltava
/// para a spec 01 ter criação de série).
fn upsert_recurrence_core(db: &Db, payload: UpsertRecurrence) -> Result<(), DbError> {
  let event_id = payload.event_id.clone();
  let rule_id = payload.id.clone().unwrap_or_else(new_id);
  db.write(|tx| {
    // A origem é a do **evento**, não a do comando: uma série do Google é
    // somente leitura (INV-6), e quem sabe disso é a linha do evento.
    let origin: Option<String> = tx
      .query_row(
        "SELECT origin FROM calendar_event WHERE id = ?1",
        params![event_id],
        |r| r.get(0),
      )
      .map_err(|err| match err {
        rusqlite::Error::QueryReturnedNoRows => {
          DbError::Domain(format!("evento inexistente: {event_id}"))
        }
        other => DbError::Sqlite(other),
      })?;
    if let Some(value) = origin {
      guard_origin(&value)?;
    }
    tx.execute(
      // `event_recurrence` **não tem** `created_at`/`updated_at`. A tabela
      // carrega `event_id`, `rrule`, `dtstart_tz`, `until_at`, `count` e
      // `exdates_json`, e nada mais — o `INSERT` citava as duas colunas
      // ausentes, o que fazia **toda** criação de recorrência falhar com
      // "no column named created_at". Como nenhum chamador usava este
      // comando, o erro nunca apareceu: foi o formulário "Novo" que
      // expôs. Uma regra de recorrência vive ou na tabela ou no Rust, e
      // `updated_at` aqui seria metadado sem dono.
      "INSERT INTO event_recurrence
               (id, event_id, rrule, dtstart_tz, until_at, count, exdates_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT (event_id) DO UPDATE SET
               rrule        = excluded.rrule,
               dtstart_tz   = excluded.dtstart_tz,
               until_at     = excluded.until_at,
               count        = excluded.count,
               exdates_json = excluded.exdates_json",
      params![
        rule_id,
        event_id,
        payload.rrule,
        payload.dtstart_tz,
        payload.until_at,
        payload.count,
        // `exdates_json` é `NOT NULL DEFAULT '[]'`, e o default só age
        // quando a coluna é **omitida** — passar `NULL` explícito viola
        // o `NOT NULL` e a regra nem era criada. A ausência de exclusão
        // é a lista vazia, que é o que o default queria dizer.
        payload.exdates_json.unwrap_or_else(|| "[]".to_string()),
      ],
    )?;
    Ok(())
  })
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn upsert_recurrence(
  db: tauri::State<'_, Db>,
  payload: UpsertRecurrence,
) -> Result<(), DbError> {
  upsert_recurrence_core(&db, payload)
}

pub fn list_events_in_window_core(db: &Db, window: Window) -> Result<Vec<GridItem>, DbError> {
  // Materializa e lê na mesma transação: ler uma janela e depois materializar
  // deixaria a grade mostrando um estado que a próxima leitura desfaz.
  db.write(|tx| {
    materialize_occurrences(tx, window.from, window.to)?;
    let mut stmt = tx.prepare(
      "SELECT o.id, o.event_id, e.layer_id, e.title, o.starts_at, o.ends_at,
                    o.state, o.override, o.rule_id IS NOT NULL, e.origin
             FROM event_occurrence o
             JOIN calendar_event e ON e.id = o.event_id
             WHERE o.starts_at < ?1 AND o.ends_at > ?2
             ORDER BY o.starts_at, e.title",
    )?;
    let rows = stmt.query_map(params![window.to, window.from], |r| {
      Ok(GridItem {
        id: r.get(0)?,
        event_id: r.get(1)?,
        layer_id: r.get(2)?,
        title: r.get(3)?,
        starts_at: r.get(4)?,
        ends_at: r.get(5)?,
        state: r.get(6)?,
        override_kind: r.get(7)?,
        is_recurring: r.get::<_, i64>(8)? == 1,
        has_conflict: false,
        origin: r.get(9)?,
        allowed_transitions: Vec::new(),
      })
    })?;
    let mut itens = rows.collect::<Result<Vec<_>, _>>()?;

    // D1.3 — os destinos válidos viajam na leitura para a tela não reimplementar
    // a máquina de estados (`state-machine.ts` deixa de ser caminho de produção).
    for item in itens.iter_mut() {
      item.allowed_transitions = destinations_of(&item.state)
        .iter()
        .map(|destino| (*destino).to_string())
        .collect();
    }

    // D7 — a marca de sobreposição entra aqui, na leitura da grade, e é a razão
    // de `domain.rs::mark_overlaps` existir no caminho de produção. A consulta já
    // vem em `ORDER BY o.starts_at, e.title`, que é a ordenação que a regra
    // exige — reordenar aqui custaria e não mudaria o resultado.
    if itens.len() > 1 {
      let intervalos: Vec<(i64, i64)> = itens.iter().map(|i| (i.starts_at, i.ends_at)).collect();
      for (item, conflito) in itens.iter_mut().zip(mark_overlaps(&intervalos)) {
        item.has_conflict = conflito;
      }
    }
    Ok(itens)
  })
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn list_events_in_window(
  db: tauri::State<'_, Db>,
  window: Window,
) -> Result<Vec<GridItem>, DbError> {
  list_events_in_window_core(&db, window)
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponsibilityFilter {
  pub due_before: Option<i64>,
  pub state: Option<String>,
  pub layer_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Responsibility {
  pub id: String,
  pub layer_id: String,
  pub title: String,
  pub kind: String,
  pub commitment: String,
  pub state: String,
  pub due_at: Option<i64>,
  pub planned_duration: Option<i64>,
  pub origin: String,
  pub owner_id: Option<String>,
  pub parent_id: Option<String>,
  /// INV-4 — o atraso é **derivado**, nunca persistido. Não existe coluna de
  /// atraso no schema; este campo é o resultado de `domain::is_overdue` no
  /// momento da leitura. É o que dá à lista de responsabilidades o sinal de
  /// atraso sem que nada seja gravado para isso.
  pub is_overdue: bool,
}

/// Filtro ausente não filtra.
///
/// A forma é `(?n IS NULL OU coluna ...)` e **não** `coluna IS ?n`. A segunda
/// parece equivalente e não é: em SQLite `x IS NULL` só é verdadeiro quando `x`
/// **é** NULL, então `due_at IS ?1` com `?1` nulo só passa para as linhas sem
/// prazo — e as três condições juntas faziam a lista voltar **vazia**. Era o que
/// acontecia: `list_responsibilities` devolvia zero linhas para filtro vazio.
pub fn list_responsibilities_core(
  db: &Db,
  filter: ResponsibilityFilter,
) -> Result<Vec<Responsibility>, DbError> {
  let conn = db.conn()?;
  // INV-4: o "agora" é do relógio do sistema, e só é lido aqui, uma vez, para
  // derivar o atraso. Nenhuma regra de domínio depende deste valor — ele é
  // insumo, não estado.
  let agora = now_ms()?;
  let mut stmt = conn.prepare(
    "SELECT id, layer_id, title, kind, commitment, state, due_at,
                planned_duration_min, origin, owner_id, parent_id
         FROM responsibility
         WHERE (?1 IS NULL OR due_at <= ?1)
           AND (?2 IS NULL OR state = ?2)
           AND (?3 IS NULL OR layer_id = ?3)
         ORDER BY due_at IS NULL, due_at, title",
  )?;
  let rows = stmt.query_map(
    params![filter.due_before, filter.state, filter.layer_id],
    |r| {
      let responsibility = Responsibility {
        id: r.get(0)?,
        layer_id: r.get(1)?,
        title: r.get(2)?,
        kind: r.get(3)?,
        commitment: r.get(4)?,
        state: r.get(5)?,
        due_at: r.get(6)?,
        planned_duration: r.get(7)?,
        origin: r.get(8)?,
        owner_id: r.get(9)?,
        parent_id: r.get(10)?,
        is_overdue: false,
      };
      Ok(Responsibility {
        // INV-4: derivado na leitura, nunca persistido. A regra mora em
        // `domain::is_overdue`, e o espelho `domain/overdue.ts` é o mesmo cálculo
        // do outro lado — `golden/overdue.jsonl` é o contrato que prova que os
        // dois concordam.
        is_overdue: is_overdue(
          &responsibility.commitment,
          &responsibility.state,
          responsibility.due_at,
          agora,
        ),
        ..responsibility
      })
    },
  )?;
  Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn list_responsibilities(
  db: tauri::State<'_, Db>,
  filter: ResponsibilityFilter,
) -> Result<Vec<Responsibility>, DbError> {
  list_responsibilities_core(&db, filter)
}

// ---------------------------------------------------------------------------
// 6-9. Escrita de evento
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertEvent {
  pub id: Option<String>,
  pub layer_id: String,
  pub title: String,
  pub notes: Option<String>,
  pub starts_at: i64,
  pub ends_at: i64,
  pub all_day: bool,
  pub plain_date: Option<String>,
  pub time_zone: String,
  pub location: Option<String>,
  pub commitment: String,
  pub state: Option<String>,
  pub origin: String,
  pub owner_type: Option<String>,
  pub owner_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarEvent {
  pub id: String,
  pub layer_id: String,
  pub title: String,
  pub notes: Option<String>,
  pub starts_at: i64,
  pub ends_at: i64,
  pub all_day: bool,
  pub plain_date: Option<String>,
  pub time_zone: String,
  pub location: Option<String>,
  pub commitment: String,
  pub state: String,
  pub origin: String,
  pub owner_id: Option<String>,
}

fn read_event(tx: &Transaction<'_>, event_id: &str) -> Result<CalendarEvent, DbError> {
  Ok(tx.query_row(
    "SELECT id, layer_id, title, notes, starts_at, ends_at, all_day, plain_date,
                time_zone, location, commitment, state, origin, owner_id
         FROM calendar_event WHERE id = ?1",
    params![event_id],
    |r| {
      Ok(CalendarEvent {
        id: r.get(0)?,
        layer_id: r.get(1)?,
        title: r.get(2)?,
        notes: r.get(3)?,
        starts_at: r.get(4)?,
        ends_at: r.get(5)?,
        all_day: r.get::<_, i64>(6)? == 1,
        plain_date: r.get(7)?,
        time_zone: r.get(8)?,
        location: r.get(9)?,
        commitment: r.get(10)?,
        state: r.get(11)?,
        origin: r.get(12)?,
        owner_id: r.get(13)?,
      })
    },
  )?)
}

/// Recusa o que a tela pode mandar e o schema só pegaria depois, como erro cru.
///
/// O `CHECK (ends_at >= starts_at)` do schema existe e funciona — mas ele
/// responde `CHECK constraint failed: ends_at >= starts_at`, que não diz o que a
/// usuária fez de errado nem qual campo. Título vazio o `CHECK` **não** pegava:
/// `''` passa em qualquer `TEXT NOT NULL`, e um evento sem título aparece na
/// grade como uma caixa vazia que ocupa espaço.
///
/// `end == start` é recusado, e não aceito como "evento instantâneo": a grade
/// posiciona por `(top, height)` e `height: 0` não é clicável.
fn validar_novo_evento(payload: &UpsertEvent) -> Result<(), DbError> {
  if payload.title.trim().is_empty() {
    return Err(DbError::Domain("o evento precisa de um título".into()));
  }
  if payload.ends_at <= payload.starts_at {
    return Err(DbError::Domain(format!(
      "o evento termina em {} e começa em {}: duração {}",
      payload.ends_at,
      payload.starts_at,
      payload.ends_at - payload.starts_at
    )));
  }
  Ok(())
}

pub fn create_event_core(db: &Db, payload: UpsertEvent) -> Result<CalendarEvent, DbError> {
  guard_origin(&payload.origin)?;
  validar_novo_evento(&payload)?;
  let now = now_ms()?;
  let event_id = payload.id.clone().unwrap_or_else(new_id);
  let state = payload
    .state
    .clone()
    .unwrap_or_else(|| "planejado".to_string());
  db.write(|tx| {
    tx.execute(
      "INSERT INTO calendar_event
               (id, layer_id, title, notes, starts_at, ends_at, all_day, plain_date,
                time_zone, location, commitment, state, origin, owner_type, owner_id,
                created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?16)",
      params![
        event_id,
        payload.layer_id,
        payload.title,
        payload.notes,
        payload.starts_at,
        payload.ends_at,
        i64::from(payload.all_day),
        payload.plain_date,
        payload.time_zone,
        payload.location,
        payload.commitment,
        state,
        payload.origin,
        payload.owner_type,
        payload.owner_id,
        now,
      ],
    )?;
    read_event(tx, &event_id)
  })
}

/// Criação/edição da regra de recorrência de um evento.
///
/// `until_at` e `count` são colunas e **vencem** a string `rrule`: são elas que a
/// usuária gravou com um clique, e editá-la não pode apagá-las. `exdates_json` é o
/// caminho de D10 e `rrule` é a definição — a materialização lê os dois.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertRecurrence {
  pub id: Option<String>,
  pub event_id: String,
  pub rrule: String,
  pub dtstart_tz: String,
  pub until_at: Option<i64>,
  pub count: Option<i64>,
  pub exdates_json: Option<String>,
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn create_event(
  db: tauri::State<'_, Db>,
  payload: UpsertEvent,
) -> Result<CalendarEvent, DbError> {
  create_event_core(&db, payload)
}

/// Patch parcial. `Option<Option<T>>` distingue "não mexe" de "define como
/// vazio" — o mesmo cuidado de `exactOptionalPropertyTypes` do TypeScript.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertEventPatch {
  pub layer_id: Option<String>,
  pub title: Option<String>,
  pub notes: Option<Option<String>>,
  pub starts_at: Option<i64>,
  pub ends_at: Option<i64>,
  pub all_day: Option<bool>,
  pub plain_date: Option<Option<String>>,
  pub time_zone: Option<String>,
  pub location: Option<Option<String>>,
  pub commitment: Option<String>,
  pub state: Option<String>,
  pub origin: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEvent {
  pub id: String,
  pub patch: UpsertEventPatch,
}

pub fn update_event_core(db: &Db, payload: UpdateEvent) -> Result<CalendarEvent, DbError> {
  if let Some(origin) = &payload.patch.origin {
    guard_origin(origin)?;
  }
  let now = now_ms()?;
  db.write(|tx| {
    // C5: o patch pode mudar `state`, então é uma transição e passa pela
    // tabela. Patch sem `state` não é transição — e não é validado.
    if let Some(estado) = &payload.patch.state {
      exigir_transicao(tx, "calendar_event", &payload.id, estado)?;
    }
    // `COALESCE` mantém o valor atual quando o patch não traz o campo. Para
    // limpar um campo, o patch traz `Some(None)` e um flag diz que a
    // intenção é apagar — daí o par `?n THEN ?n+1 ELSE coluna`.
    tx.execute(
      "UPDATE calendar_event SET
               layer_id   = COALESCE(?2, layer_id),
               title      = COALESCE(?3, title),
               notes      = CASE WHEN ?4 THEN ?5 ELSE notes END,
               starts_at  = COALESCE(?6, starts_at),
               ends_at    = COALESCE(?7, ends_at),
               all_day    = COALESCE(?8, all_day),
               plain_date = CASE WHEN ?9 THEN ?10 ELSE plain_date END,
               time_zone  = COALESCE(?11, time_zone),
               location   = CASE WHEN ?12 THEN ?13 ELSE location END,
               commitment = COALESCE(?14, commitment),
               state      = COALESCE(?15, state),
               origin     = COALESCE(?16, origin),
               updated_at = ?17
             WHERE id = ?1",
      params![
        payload.id,
        payload.patch.layer_id,
        payload.patch.title,
        payload.patch.notes.is_some(),
        payload.patch.notes.clone().flatten(),
        payload.patch.starts_at,
        payload.patch.ends_at,
        payload.patch.all_day.map(i64::from),
        payload.patch.plain_date.is_some(),
        payload.patch.plain_date.clone().flatten(),
        payload.patch.time_zone,
        payload.patch.location.is_some(),
        payload.patch.location.clone().flatten(),
        payload.patch.commitment,
        payload.patch.state,
        payload.patch.origin,
        now,
      ],
    )?;
    read_event(tx, &payload.id)
  })
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn update_event(
  db: tauri::State<'_, Db>,
  payload: UpdateEvent,
) -> Result<CalendarEvent, DbError> {
  update_event_core(&db, payload)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityId {
  pub id: String,
}

/// Apaga evento, Instances e regra. **Não** apaga `execution_record` (D12): o
/// registro do que aconteceu sobrevive ao item.
pub fn delete_event_core(db: &Db, payload: EntityId) -> Result<(), DbError> {
  db.write(|tx| {
    tx.execute(
      "DELETE FROM event_occurrence WHERE event_id = ?1",
      params![payload.id],
    )?;
    tx.execute(
      "DELETE FROM event_recurrence WHERE event_id = ?1",
      params![payload.id],
    )?;
    tx.execute(
      "DELETE FROM calendar_event WHERE id = ?1",
      params![payload.id],
    )?;
    Ok(())
  })
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn delete_event(db: tauri::State<'_, Db>, payload: EntityId) -> Result<(), DbError> {
  delete_event_core(&db, payload)
}

// ---------------------------------------------------------------------------
// 10-12. Ocorrência
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Occurrence {
  pub id: String,
  pub event_id: String,
  pub original_start: i64,
  pub starts_at: i64,
  pub ends_at: i64,
  pub state: String,
  pub override_kind: String,
  pub cancel_reason: Option<String>,
}

fn read_occurrence(tx: &Transaction<'_>, occurrence_id: &str) -> Result<Occurrence, DbError> {
  Ok(tx.query_row(
    "SELECT id, event_id, original_start, starts_at, ends_at, state, override, cancel_reason
         FROM event_occurrence WHERE id = ?1",
    params![occurrence_id],
    |r| {
      Ok(Occurrence {
        id: r.get(0)?,
        event_id: r.get(1)?,
        original_start: r.get(2)?,
        starts_at: r.get(3)?,
        ends_at: r.get(4)?,
        state: r.get(5)?,
        override_kind: r.get(6)?,
        cancel_reason: r.get(7)?,
      })
    },
  )?)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetOccurrenceState {
  pub id: String,
  pub state: String,
  pub reason: Option<String>,
}

/// INV-2 / INV-6 — a origem que vale aqui é a do **evento da ocorrência**, e a
/// linha que a guarda é o `JOIN` com `calendar_event`.
///
/// Sem isto, `set_occurrence_state` e `move_occurrence` gravavam sobre um item do
/// Google: as duas escreviam em `event_occurrence` sem nunca perguntar de quem
/// ela era. As outras cinco escritas (`upsert_recurrence`, `create_event`,
/// `update_event`, `create_responsibility`, `plan_block`) já guardavam; estas
/// duas eram o buraco.
fn exigir_origem_editavel(
  tx: &rusqlite::Transaction<'_>,
  occurrence_id: &str,
) -> Result<(), DbError> {
  let origin: String = tx
    .query_row(
      "SELECT e.origin
         FROM event_occurrence o
         JOIN calendar_event e ON e.id = o.event_id
        WHERE o.id = ?1",
      params![occurrence_id],
      |r| r.get(0),
    )
    .map_err(|err| match err {
      rusqlite::Error::QueryReturnedNoRows => {
        DbError::Domain(format!("ocorrência inexistente: {occurrence_id}"))
      }
      other => DbError::Sqlite(other),
    })?;
  guard_origin(&origin)
}

/// Cancelar uma instância é alteração local; a série continua, então
/// `override = 'cancelled'` e `event_recurrence` não é tocado (INV-5).
pub fn set_occurrence_state_core(
  db: &Db,
  payload: SetOccurrenceState,
) -> Result<Occurrence, DbError> {
  let now = now_ms()?;
  let cancelled = payload.state == "cancelado";
  db.write(|tx| {
    // C5: a transição é validada **antes** do UPDATE. Sem esta linha,
    // `concluido → planejado` era gravável de ponta a ponta: o `CHECK` do
    // schema só impede valor fora da enumeração, não transição ilegal.
    exigir_transicao(tx, "event_occurrence", &payload.id, &payload.state)?;
    // INV-6: um item do Google é somente leitura, e concluir é escrever nele.
    exigir_origem_editavel(tx, &payload.id)?;
    tx.execute(
      "UPDATE event_occurrence
             SET state = ?2,
                 override = CASE WHEN ?3 THEN 'cancelled' ELSE 'none' END,
                 cancel_reason = CASE WHEN ?3 THEN ?4 ELSE NULL END,
                 updated_at = ?5
             WHERE id = ?1",
      params![payload.id, payload.state, cancelled, payload.reason, now],
    )?;
    read_occurrence(tx, &payload.id)
  })
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn set_occurrence_state(
  db: tauri::State<'_, Db>,
  payload: SetOccurrenceState,
) -> Result<Occurrence, DbError> {
  set_occurrence_state_core(&db, payload)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveOccurrence {
  pub id: String,
  pub starts_at: i64,
  pub ends_at: i64,
}

/// Remarca preserva `original_start` (§5): a identidade da instância é a data
/// original, então histórico e o par `recurringEventId`/`originalStartTime` do
/// Google continuam válidos.
pub fn move_occurrence_core(db: &Db, payload: MoveOccurrence) -> Result<Occurrence, DbError> {
  let now = now_ms()?;
  db.write(|tx| {
    // INV-6: remarcar é escrever, e um evento do Google não é nosso.
    exigir_origem_editavel(tx, &payload.id)?;
    // Reagendar não muda estado, então não passa por `exigir_transicao`. O que
    // não pode é inverter a duração: um item que termina antes de começar
    // atravessa a meia-noite ao contrário e quebra a coluna do dia na grade.
    if payload.ends_at <= payload.starts_at {
      return Err(DbError::Domain(format!(
        "remarca com duração não positiva: {} → {}",
        payload.starts_at, payload.ends_at
      )));
    }
    tx.execute(
      "UPDATE event_occurrence
             SET starts_at = ?2, ends_at = ?3, override = 'moved', updated_at = ?4
             WHERE id = ?1",
      params![payload.id, payload.starts_at, payload.ends_at, now],
    )?;
    read_occurrence(tx, &payload.id)
  })
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn move_occurrence(
  db: tauri::State<'_, Db>,
  payload: MoveOccurrence,
) -> Result<Occurrence, DbError> {
  move_occurrence_core(&db, payload)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRecurrence {
  pub rule_id: String,
  pub rrule: String,
  pub scope: String,
  /// A spec 01 §3.4 lista apenas `ruleId`, `rrule` e `scope`, o que **não
  /// identifica a instância**: "esta ocorrência" só faz sentido com a
  /// ocorrência em mãos. O campo é obrigatório nos escopos que não são
  /// `serie`; se faltar, o comando recusa em vez de adivinhar.
  pub occurrence_id: Option<String>,
}

/// Resolve a ocorrência de um escopo local, recusando quando ela não vem.
fn require_occurrence(payload: &UpdateRecurrence) -> Result<String, DbError> {
  payload.occurrence_id.clone().ok_or_else(|| {
    DbError::Domain(format!(
      "escopo `{}` exige `occurrenceId`: a instância é identificada pela data original",
      payload.scope
    ))
  })
}

/// Lê a data original de uma ocorrência.
fn occurrence_start(tx: &Transaction<'_>, occurrence_id: &str) -> Result<i64, DbError> {
  Ok(tx.query_row(
    "SELECT original_start FROM event_occurrence WHERE id = ?1",
    params![occurrence_id],
    |r| r.get(0),
  )?)
}

/// INV-5: um escopo local nunca reescreve o padrão da série.
///
/// - `serie` — regra nova para a série inteira. Única escrita que muda `rrule`.
/// - `esta_ocorrencia` — a instância vira avulsa: entra como `EXDATE` na regra
///   e é solta (`rule_id = NULL`). A `rrule` não muda, e o `EXDATE` não renumera
///   as seguintes (§5), então a materialização continua idempotente: sem o
///   exdate, a próxima expansão tentaria reinserir a mesma `original_start` e
///   esbarraria no `UNIQUE (event_id, original_start)`.
/// - `esta_e_seguintes` — divide a série: nasce um evento novo com a regra nova,
///   as instâncias a partir de `original_start` migram para ele, e a regra
///   original é limitada a `until_at = original_start - 1`.
///
/// Nos dois escopos locais a série anterior continua válida **para trás**.
pub fn update_recurrence_core(db: &Db, payload: UpdateRecurrence) -> Result<(), DbError> {
  match payload.scope.as_str() {
    "serie" => db.write(|tx| {
      tx.execute(
        "UPDATE event_recurrence SET rrule = ?2 WHERE id = ?1",
        params![payload.rule_id, payload.rrule],
      )?;
      Ok(())
    }),

    "esta_ocorrencia" => {
      let occurrence_id = require_occurrence(&payload)?;
      db.write(|tx| {
        let start = occurrence_start(tx, &occurrence_id)?;
        let current: String = tx.query_row(
          "SELECT exdates_json FROM event_recurrence WHERE id = ?1",
          params![payload.rule_id],
          |r| r.get(0),
        )?;
        // Somar o exdate é o que impede a regra de regenerar a instância.
        let mut exdates: Vec<i64> = serde_json::from_str(&current).unwrap_or_default();
        if !exdates.contains(&start) {
          exdates.push(start);
          exdates.sort_unstable();
        }
        tx.execute(
          "UPDATE event_recurrence SET exdates_json = ?2 WHERE id = ?1",
          params![
            payload.rule_id,
            serde_json::to_string(&exdates).unwrap_or_else(|_| "[]".into())
          ],
        )?;
        tx.execute(
          "UPDATE event_occurrence
                     SET rule_id = NULL, override = 'moved', updated_at = ?2
                     WHERE id = ?1",
          params![occurrence_id, now_ms()?],
        )?;
        Ok(())
      })
    }

    "esta_e_seguintes" => {
      let occurrence_id = require_occurrence(&payload)?;
      let new_event = new_id();
      let new_rule = new_id();
      let now = now_ms()?;
      db.write(|tx| {
        let start = occurrence_start(tx, &occurrence_id)?;
        let old_event: String = tx.query_row(
          "SELECT event_id FROM event_occurrence WHERE id = ?1",
          params![occurrence_id],
          |r| r.get(0),
        )?;
        let tz: String = tx.query_row(
          "SELECT dtstart_tz FROM event_recurrence WHERE id = ?1",
          params![payload.rule_id],
          |r| r.get(0),
        )?;

        // A série nova é uma cópia do evento: título, camada, fuso e
        // origem continuam; o que muda é a regra e a identidade.
        tx.execute(
          "INSERT INTO calendar_event
                       (id, layer_id, title, notes, starts_at, ends_at, all_day, plain_date,
                        time_zone, location, commitment, state, origin, owner_type, owner_id,
                        created_at, updated_at)
                     SELECT ?1, layer_id, title, notes, starts_at, ends_at, all_day, plain_date,
                            time_zone, location, commitment, state, origin, owner_type, owner_id,
                            ?2, ?2
                     FROM calendar_event WHERE id = ?3",
          params![new_event, now, old_event],
        )?;
        tx.execute(
          "INSERT INTO event_recurrence (id, event_id, rrule, dtstart_tz, exdates_json)
                     VALUES (?1, ?2, ?3, ?4, '[]')",
          params![new_rule, new_event, payload.rrule, tz],
        )?;
        // A regra antiga vale só até a data anterior à divisão.
        tx.execute(
          "UPDATE event_recurrence SET until_at = ?2 WHERE id = ?1",
          params![payload.rule_id, start - 1],
        )?;
        // As instâncias já materializadas a partir da divisão migram.
        tx.execute(
          "UPDATE event_occurrence
                     SET event_id = ?1, rule_id = ?2, updated_at = ?3
                     WHERE event_id = ?4 AND original_start >= ?5",
          params![new_event, new_rule, now, old_event, start],
        )?;
        Ok(())
      })
    }

    other => Err(DbError::Domain(format!(
      "escopo de recorrência desconhecido: {other}"
    ))),
  }
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn update_recurrence(
  db: tauri::State<'_, Db>,
  payload: UpdateRecurrence,
) -> Result<(), DbError> {
  update_recurrence_core(&db, payload)
}

// ---------------------------------------------------------------------------
// 13-14. Responsabilidade
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertResponsibility {
  pub id: Option<String>,
  pub layer_id: String,
  pub title: String,
  pub notes: Option<String>,
  pub kind: String,
  pub commitment: String,
  pub state: Option<String>,
  pub due_at: Option<i64>,
  pub planned_duration: Option<i64>,
  pub objective: Option<String>,
  pub origin: String,
  pub owner_type: Option<String>,
  pub owner_id: Option<String>,
  pub parent_id: Option<String>,
}

fn read_responsibility(tx: &Transaction<'_>, resp_id: &str) -> Result<Responsibility, DbError> {
  // O relógio é lido aqui, e não dentro do closure: o closure devolve
  // `rusqlite::Result`, e `DbError` não converte para `rusqlite::Error`.
  let agora = now_ms()?;
  Ok(tx.query_row(
    "SELECT id, layer_id, title, kind, commitment, state, due_at,
                planned_duration_min, origin, owner_id, parent_id
         FROM responsibility WHERE id = ?1",
    params![resp_id],
    |r| {
      let resp = Responsibility {
        id: r.get(0)?,
        layer_id: r.get(1)?,
        title: r.get(2)?,
        kind: r.get(3)?,
        commitment: r.get(4)?,
        state: r.get(5)?,
        due_at: r.get(6)?,
        planned_duration: r.get(7)?,
        origin: r.get(8)?,
        owner_id: r.get(9)?,
        parent_id: r.get(10)?,
        is_overdue: false,
      };
      // INV-4: o atraso é derivado na leitura, nos dois lugares que devolvem
      // `Responsibility`. Sem isto, `is_overdue` ficaria `false` aqui e a regra
      // pareceria viva só na listagem.
      Ok(Responsibility {
        is_overdue: is_overdue(&resp.commitment, &resp.state, resp.due_at, agora),
        ..resp
      })
    },
  )?)
}

pub fn create_responsibility_core(
  db: &Db,
  payload: UpsertResponsibility,
) -> Result<Responsibility, DbError> {
  guard_origin(&payload.origin)?;
  let now = now_ms()?;
  let resp_id = payload.id.clone().unwrap_or_else(new_id);
  let state = payload
    .state
    .clone()
    .unwrap_or_else(|| "planejado".to_string());
  db.write(|tx| {
    tx.execute(
      "INSERT INTO responsibility
               (id, layer_id, title, notes, kind, commitment, state, due_at,
                planned_duration_min, objective, origin, owner_type, owner_id,
                parent_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?15)",
      params![
        resp_id,
        payload.layer_id,
        payload.title,
        payload.notes,
        payload.kind,
        payload.commitment,
        state,
        payload.due_at,
        payload.planned_duration,
        payload.objective,
        payload.origin,
        payload.owner_type,
        payload.owner_id,
        payload.parent_id,
        now,
      ],
    )?;
    read_responsibility(tx, &resp_id)
  })
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn create_responsibility(
  db: tauri::State<'_, Db>,
  payload: UpsertResponsibility,
) -> Result<Responsibility, DbError> {
  create_responsibility_core(&db, payload)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponsibilityStep {
  pub id: String,
  pub responsibility_id: String,
  pub title: String,
  pub position: i64,
  pub state: String,
  pub due_at: Option<i64>,
  pub completed_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetStepState {
  pub id: String,
  pub state: String,
}

/// Concluir uma etapa carimba `completed_at`; reabrir limpa o carimbo.
pub fn set_step_state_core(db: &Db, payload: SetStepState) -> Result<ResponsibilityStep, DbError> {
  let now = now_ms()?;
  let done = payload.state == "concluida";
  db.write(|tx| {
    tx.execute(
      "UPDATE responsibility_step
             SET state = ?2, completed_at = CASE WHEN ?3 THEN ?4 ELSE NULL END
             WHERE id = ?1",
      params![payload.id, payload.state, done, now],
    )?;
    Ok(tx.query_row(
      "SELECT id, responsibility_id, title, position, state, due_at, completed_at
             FROM responsibility_step WHERE id = ?1",
      params![payload.id],
      |r| {
        Ok(ResponsibilityStep {
          id: r.get(0)?,
          responsibility_id: r.get(1)?,
          title: r.get(2)?,
          position: r.get(3)?,
          state: r.get(4)?,
          due_at: r.get(5)?,
          completed_at: r.get(6)?,
        })
      },
    )?)
  })
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn set_step_state(
  db: tauri::State<'_, Db>,
  payload: SetStepState,
) -> Result<ResponsibilityStep, DbError> {
  set_step_state_core(&db, payload)
}

// ---------------------------------------------------------------------------
// 15-16. Bloco de planejamento
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertBlock {
  pub id: Option<String>,
  /// INV-1: sem responsável, não há bloco. O tipo já não aceita a ausência.
  pub responsibility_id: String,
  pub layer_id: String,
  pub starts_at: i64,
  pub ends_at: i64,
  pub time_zone: String,
  pub planned_duration: i64,
  pub state: Option<String>,
  pub origin: String,
  pub timer_state: Option<String>,
  pub accumulated: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanBlock {
  pub id: String,
  pub responsibility_id: String,
  pub layer_id: String,
  pub starts_at: i64,
  pub ends_at: i64,
  pub time_zone: String,
  pub planned_duration: i64,
  pub state: String,
  pub origin: String,
  pub timer_state: String,
  pub accumulated: i64,
}

/// Reserva tempo para uma responsabilidade que já existe. Não cria obrigação:
/// o `responsibility_id` aponta para algo preexistente (INV-1).
pub fn plan_block_core(db: &Db, payload: UpsertBlock) -> Result<PlanBlock, DbError> {
  guard_origin(&payload.origin)?;
  let now = now_ms()?;
  let block_id = payload.id.clone().unwrap_or_else(new_id);
  let state = payload
    .state
    .clone()
    .unwrap_or_else(|| "planejado".to_string());
  let timer = payload
    .timer_state
    .clone()
    .unwrap_or_else(|| "parado".to_string());
  let accumulated = payload.accumulated.unwrap_or(0);
  db.write(|tx| {
    tx.execute(
      "INSERT INTO plan_block
               (id, responsibility_id, layer_id, starts_at, ends_at, time_zone,
                planned_duration_min, state, origin, timer_state, accumulated_min,
                created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)",
      params![
        block_id,
        payload.responsibility_id,
        payload.layer_id,
        payload.starts_at,
        payload.ends_at,
        payload.time_zone,
        payload.planned_duration,
        state,
        payload.origin,
        timer,
        accumulated,
        now,
      ],
    )?;
    read_block(tx, &block_id)
  })
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn plan_block(db: tauri::State<'_, Db>, payload: UpsertBlock) -> Result<PlanBlock, DbError> {
  plan_block_core(&db, payload)
}

fn read_block(tx: &Transaction<'_>, block_id: &str) -> Result<PlanBlock, DbError> {
  Ok(tx.query_row(
    "SELECT id, responsibility_id, layer_id, starts_at, ends_at, time_zone,
                planned_duration_min, state, origin, timer_state, accumulated_min
         FROM plan_block WHERE id = ?1",
    params![block_id],
    |r| {
      Ok(PlanBlock {
        id: r.get(0)?,
        responsibility_id: r.get(1)?,
        layer_id: r.get(2)?,
        starts_at: r.get(3)?,
        ends_at: r.get(4)?,
        time_zone: r.get(5)?,
        planned_duration: r.get(6)?,
        state: r.get(7)?,
        origin: r.get(8)?,
        timer_state: r.get(9)?,
        accumulated: r.get(10)?,
      })
    },
  )?)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RescheduleBlock {
  pub id: String,
  pub starts_at: i64,
  pub ends_at: i64,
}

/// Reagendar muda *quando*, não *quanto*: a duração planejada acompanha.
pub fn reschedule_block_core(db: &Db, payload: RescheduleBlock) -> Result<PlanBlock, DbError> {
  let now = now_ms()?;
  let minutes = (payload.ends_at - payload.starts_at) / 60_000;
  db.write(|tx| {
    tx.execute(
      "UPDATE plan_block
             SET starts_at = ?2, ends_at = ?3, planned_duration_min = ?4, updated_at = ?5
             WHERE id = ?1",
      params![payload.id, payload.starts_at, payload.ends_at, minutes, now],
    )?;
    read_block(tx, &payload.id)
  })
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn reschedule_block(
  db: tauri::State<'_, Db>,
  payload: RescheduleBlock,
) -> Result<PlanBlock, DbError> {
  reschedule_block_core(&db, payload)
}

// ---------------------------------------------------------------------------
// 17-18. Execução
// ---------------------------------------------------------------------------

/// Alvo polimórfico: `kind` + `id`, nunca uma string SQL.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Target {
  pub kind: String,
  pub id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionInput {
  pub started_at: i64,
  pub finished_at: i64,
  pub actual_duration: i64,
  pub result: Option<String>,
  pub notes: Option<String>,
  pub source: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionRecord {
  pub id: String,
  pub target_kind: String,
  pub target_id: String,
  pub started_at: i64,
  pub finished_at: i64,
  pub actual_duration: i64,
  pub result: Option<String>,
  pub corrected_by: Option<String>,
}

/// Grava o registro do que aconteceu e reflete o estado mais recente no alvo.
/// D12: `INSERT` puro, sem `UPDATE` da linha anterior.
fn insert_execution(
  tx: &Transaction<'_>,
  target: &Target,
  input: &ExecutionInput,
  next_state: &str,
) -> Result<ExecutionRecord, DbError> {
  let exec_id = new_id();
  tx.execute(
    "INSERT INTO execution_record
           (id, target_kind, target_id, started_at, finished_at, actual_duration_min,
            result, notes, source, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
    params![
      exec_id,
      target.kind,
      target.id,
      input.started_at,
      input.finished_at,
      input.actual_duration,
      input.result,
      input.notes,
      input.source,
      input.started_at,
    ],
  )?;
  // `table` vem de uma correspondência fechada sobre literais, nunca de
  // entrada do usuário: não há injeção de SQL apesar do `format!`.
  let table = match target.kind.as_str() {
    "occurrence" => "event_occurrence",
    "responsibility" => "responsibility",
    "block" => "plan_block",
    other => {
      return Err(DbError::Domain(format!(
        "alvo de execução desconhecido: {other}"
      )));
    }
  };
  let sql = format!("UPDATE {table} SET state = ?1 WHERE id = ?2");
  // C5: refletir estado no alvo também é uma transição, e passa pela mesma
  // tabela. `plan_block` tem um conjunto menor de estados no `CHECK`, então a
  // tabela aqui é mais permissiva que o schema — o `CHECK` segue sendo o
  // segundo portão, e os dois juntos fecham.
  exigir_transicao(tx, table, &target.id, next_state)?;
  let affected = tx.execute(&sql, params![next_state, target.id])?;
  if affected == 0 && target.kind == "event" {
    // Um evento pode não ter ocorrência materializada; o estado do evento
    // base é o fallback.
    tx.execute(
      "UPDATE calendar_event SET state = ?1 WHERE id = ?2",
      params![next_state, target.id],
    )?;
  }
  Ok(ExecutionRecord {
    id: exec_id,
    target_kind: target.kind.clone(),
    target_id: target.id.clone(),
    started_at: input.started_at,
    finished_at: input.finished_at,
    actual_duration: input.actual_duration,
    result: input.result.clone(),
    corrected_by: None,
  })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteItem {
  pub target: Target,
  pub record: Option<ExecutionInput>,
}

/// INV-3: concluir exige o registro do que foi feito. Sem `record`, a função
/// recusa **antes** de qualquer escrita — nem estado, nem histórico.
pub fn complete_item_core(db: &Db, payload: CompleteItem) -> Result<(), DbError> {
  let record = payload
    .record
    .ok_or_else(|| DbError::Domain("concluir exige o registro de execução (INV-3)".into()))?;
  db.write(|tx| {
    insert_execution(tx, &payload.target, &record, "concluido")?;
    Ok(())
  })
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn complete_item(db: tauri::State<'_, Db>, payload: CompleteItem) -> Result<(), DbError> {
  complete_item_core(&db, payload)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordExecution {
  pub target: Target,
  pub record: ExecutionInput,
}

/// Registrar execução parcial não conclui: o item segue aberto e o registro
/// alimenta o histórico e a comparação planejado × real do §6.
pub fn record_execution_core(
  db: &Db,
  payload: RecordExecution,
) -> Result<ExecutionRecord, DbError> {
  db.write(|tx| insert_execution(tx, &payload.target, &payload.record, "em_andamento"))
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn record_execution(
  db: tauri::State<'_, Db>,
  payload: RecordExecution,
) -> Result<ExecutionRecord, DbError> {
  record_execution_core(&db, payload)
}

// ---------------------------------------------------------------------------
// 19. Settings
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingKey {
  pub key: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingValue {
  pub key: String,
  pub value: serde_json::Value,
}

pub fn get_setting_core(db: &Db, payload: SettingKey) -> Result<Option<SettingValue>, DbError> {
  let conn = db.conn()?;
  let found = conn
    .query_row(
      "SELECT key, value_json FROM setting WHERE key = ?1",
      params![payload.key],
      |r| {
        let raw: String = r.get(1)?;
        Ok(SettingValue {
          key: r.get(0)?,
          value: serde_json::from_str(&raw).unwrap_or(serde_json::Value::Null),
        })
      },
    )
    .optional()?;
  Ok(found)
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn get_setting(
  db: tauri::State<'_, Db>,
  payload: SettingKey,
) -> Result<Option<SettingValue>, DbError> {
  get_setting_core(&db, payload)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetSetting {
  pub key: String,
  pub value: serde_json::Value,
}

pub fn set_setting_core(db: &Db, payload: SetSetting) -> Result<(), DbError> {
  let raw = serde_json::to_string(&payload.value)
    .map_err(|e| DbError::Domain(format!("valor de setting não serializa: {e}")))?;
  db.write(|tx| {
    tx.execute(
      "INSERT INTO setting (key, value_json) VALUES (?1, ?2)
             ON CONFLICT (key) DO UPDATE SET value_json = excluded.value_json",
      params![payload.key, raw],
    )?;
    Ok(())
  })
}

#[cfg_attr(feature = "tauri-shell", tauri::command)]
#[cfg(feature = "tauri-shell")]
pub fn set_setting(db: tauri::State<'_, Db>, payload: SetSetting) -> Result<(), DbError> {
  set_setting_core(&db, payload)
}

// ---------------------------------------------------------------------------
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
  use super::*;

  const T0: i64 = 1_757_000_000_000;
  const HOUR: i64 = 3_600_000;
  /// Dia em ms. O passo da recorrência é diário em UTC (D6), e o eixo dos
  /// goldens é o dia a partir da âncora — por isso a constante existe mesmo
  /// depois de o motor virar `rrule`.
  const DIA_MS: i64 = 86_400_000;

  fn event_payload(title: &str) -> UpsertEvent {
    UpsertEvent {
      id: None,
      layer_id: "faculdade".into(),
      title: title.into(),
      notes: None,
      starts_at: T0,
      ends_at: T0 + HOUR,
      all_day: false,
      plain_date: None,
      time_zone: "America/Sao_Paulo".into(),
      location: None,
      commitment: "obrigatorio".into(),
      state: None,
      origin: "cecistudy".into(),
      owner_type: None,
      owner_id: None,
    }
  }

  fn responsibility_payload(title: &str) -> UpsertResponsibility {
    UpsertResponsibility {
      id: None,
      layer_id: "estudos".into(),
      title: title.into(),
      notes: None,
      kind: "leitura".into(),
      commitment: "importante".into(),
      state: None,
      due_at: Some(T0 - HOUR),
      planned_duration: Some(120),
      objective: None,
      origin: "cecistudy".into(),
      owner_type: None,
      owner_id: None,
      parent_id: None,
    }
  }

  fn execution(finished: i64) -> ExecutionInput {
    ExecutionInput {
      started_at: T0,
      finished_at: finished,
      actual_duration: (finished - T0) / 60_000,
      result: Some("concluido".into()),
      notes: None,
      source: "manual".into(),
    }
  }

  #[test]
  fn seed_traz_as_seis_camadas_de_d4() {
    let db = Db::open_in_memory().expect("banco");
    let layers = list_layers_core(&db).expect("camadas");
    assert_eq!(layers.len(), 6, "D4: 6 camadas");
    assert_eq!(layers.first().map(|l| l.id.as_str()), Some("faculdade"));
    let google = layers.iter().find(|l| l.id == "google").expect("google");
    assert!(!google.visible, "a camada do Google nasce oculta");
    assert_eq!(google.tone, "neutral");
  }

  #[test]
  fn set_layer_visibility_persiste() {
    let db = Db::open_in_memory().expect("banco");
    set_layer_visibility_core(
      &db,
      SetLayerVisibility {
        id: "tcc".into(),
        visible: false,
      },
    )
    .expect("altera");
    let layers = list_layers_core(&db).expect("camadas");
    assert!(!layers.iter().find(|l| l.id == "tcc").expect("tcc").visible);
  }

  #[test]
  fn invariant_google_e_recusado_na_escrita() {
    let db = Db::open_in_memory().expect("banco");
    let mut payload = event_payload("Aula importada");
    payload.origin = "google".into();
    let err = create_event_core(&db, payload).expect_err("INV-6 precisa recusar");
    assert!(
      err.to_string().contains("INV-6"),
      "mensagem explica a recusa"
    );
    let total: i64 = db
      .conn()
      .unwrap()
      .query_row("SELECT COUNT(*) FROM calendar_event", [], |r| r.get(0))
      .expect("conta");
    assert_eq!(total, 0, "nada foi escrito");
  }

  #[test]
  fn update_parcial_muda_somente_o_enviado() {
    let db = Db::open_in_memory().expect("banco");
    let created = create_event_core(&db, event_payload("Aula 1")).expect("cria");
    let updated = update_event_core(
      &db,
      UpdateEvent {
        id: created.id.clone(),
        patch: UpsertEventPatch {
          title: Some("Aula 1 — remake".into()),
          ..Default::default()
        },
      },
    )
    .expect("atualiza");
    assert_eq!(updated.title, "Aula 1 — remake");
    assert_eq!(updated.commitment, "obrigatorio", "o resto foi preservado");
    assert_eq!(updated.starts_at, T0);
  }

  #[test]
  fn limpar_campo_requer_o_marcador_do_patch() {
    let db = Db::open_in_memory().expect("banco");
    let mut payload = event_payload("Com local");
    payload.location = Some("Sala 3".into());
    let created = create_event_core(&db, payload).expect("cria");
    assert_eq!(created.location.as_deref(), Some("Sala 3"));

    let updated = update_event_core(
      &db,
      UpdateEvent {
        id: created.id.clone(),
        patch: UpsertEventPatch {
          location: Some(None),
          ..Default::default()
        },
      },
    )
    .expect("atualiza");
    assert_eq!(updated.location, None, "Some(None) limpa o campo");

    // Patch sem o campo não mexe.
    let untouched = update_event_core(
      &db,
      UpdateEvent {
        id: created.id.clone(),
        patch: UpsertEventPatch {
          title: Some("Novo".into()),
          ..Default::default()
        },
      },
    )
    .expect("atualiza");
    assert_eq!(untouched.location, None, "continua limpo, não foi reposto");
  }

  #[test]
  fn janela_consulta_somente_ocorrencias_que_intersectam() {
    let db = Db::open_in_memory().expect("banco");
    let evt = create_event_core(&db, event_payload("Aula")).expect("cria");
    db.write(|tx| {
      tx.execute(
        "INSERT INTO event_occurrence
                   (id, event_id, original_start, starts_at, ends_at, state, override,
                    created_at, updated_at)
                 VALUES ('occ-1', ?1, ?2, ?2, ?3, 'planejado', 'none', ?2, ?2)",
        params![evt.id, T0, T0 + HOUR],
      )?;
      Ok(())
    })
    .expect("materializa");

    let dentro = list_events_in_window_core(
      &db,
      Window {
        from: T0 + 60_000,
        to: T0 + HOUR,
      },
    )
    .expect("janela");
    assert_eq!(dentro.len(), 1, "interseção parcial conta");
    let fora = list_events_in_window_core(
      &db,
      Window {
        from: T0 + 2 * HOUR,
        to: T0 + 3 * HOUR,
      },
    )
    .expect("janela");
    assert!(fora.is_empty(), "janela sem interseção não traz nada");
  }

  #[test]
  fn mover_ocorrencia_preserva_a_data_original() {
    let db = Db::open_in_memory().expect("banco");
    let evt = create_event_core(&db, event_payload("Aula")).expect("cria");
    db.write(|tx| {
      tx.execute(
        "INSERT INTO event_occurrence
                   (id, event_id, original_start, starts_at, ends_at, state, override,
                    created_at, updated_at)
                 VALUES ('occ-1', ?1, ?2, ?2, ?3, 'planejado', 'none', ?2, ?2)",
        params![evt.id, T0, T0 + HOUR],
      )?;
      Ok(())
    })
    .expect("materializa");

    let moved = move_occurrence_core(
      &db,
      MoveOccurrence {
        id: "occ-1".into(),
        starts_at: T0 + 5 * HOUR,
        ends_at: T0 + 6 * HOUR,
      },
    )
    .expect("move");
    assert_eq!(
      moved.original_start, T0,
      "§5: a identidade é a data original"
    );
    assert_eq!(moved.starts_at, T0 + 5 * HOUR);
    assert_eq!(moved.override_kind, "moved");
    assert_eq!(
      moved.ends_at - moved.starts_at,
      HOUR,
      "a duração é preservada"
    );
  }

  #[test]
  fn cancelar_ocorrencia_preserva_a_regra_da_serie() {
    let db = Db::open_in_memory().expect("banco");
    let evt = create_event_core(&db, event_payload("Aula")).expect("cria");
    db.write(|tx| {
      tx.execute(
        "INSERT INTO event_recurrence (id, event_id, rrule, dtstart_tz, exdates_json)
                 VALUES ('rule-1', ?1, 'FREQ=WEEKLY;BYDAY=TU', 'UTC', '[]')",
        params![evt.id],
      )?;
      tx.execute(
        "INSERT INTO event_occurrence
                   (id, event_id, rule_id, original_start, starts_at, ends_at, state,
                    override, created_at, updated_at)
                 VALUES ('occ-1', ?1, 'rule-1', ?2, ?2, ?3, 'planejado', 'none', ?2, ?2)",
        params![evt.id, T0, T0 + HOUR],
      )?;
      Ok(())
    })
    .expect("prepara");

    let cancelled = set_occurrence_state_core(
      &db,
      SetOccurrenceState {
        id: "occ-1".into(),
        state: "cancelado".into(),
        reason: Some("aula cancelada".into()),
      },
    )
    .expect("cancela");
    assert_eq!(cancelled.override_kind, "cancelled");
    assert_eq!(cancelled.cancel_reason.as_deref(), Some("aula cancelada"));

    let rrule: String = db
      .conn()
      .unwrap()
      .query_row(
        "SELECT rrule FROM event_recurrence WHERE id = 'rule-1'",
        [],
        |r| r.get(0),
      )
      .expect("regra");
    assert_eq!(rrule, "FREQ=WEEKLY;BYDAY=TU", "INV-5: a série não muda");
  }

  #[test]
  fn escopo_esta_ocorrencia_solta_a_instancia_da_regra() {
    let db = Db::open_in_memory().expect("banco");
    let evt = create_event_core(&db, event_payload("Aula")).expect("cria");
    db.write(|tx| {
      tx.execute(
        "INSERT INTO event_recurrence (id, event_id, rrule, dtstart_tz, exdates_json)
                 VALUES ('rule-1', ?1, 'FREQ=WEEKLY;BYDAY=TU', 'UTC', '[]')",
        params![evt.id],
      )?;
      tx.execute(
        "INSERT INTO event_occurrence
                   (id, event_id, rule_id, original_start, starts_at, ends_at, state,
                    override, created_at, updated_at)
                 VALUES ('occ-1', ?1, 'rule-1', ?2, ?2, ?3, 'planejado', 'none', ?2, ?2)",
        params![evt.id, T0, T0 + HOUR],
      )?;
      Ok(())
    })
    .expect("prepara");

    update_recurrence_core(
      &db,
      UpdateRecurrence {
        rule_id: "rule-1".into(),
        rrule: "FREQ=DAILY".into(),
        scope: "esta_ocorrencia".into(),
        // O escopo local identifica a instância pelo id. Sem este campo
        // o comando recusa, e o teste não teria o que provar.
        occurrence_id: Some("occ-1".into()),
      },
    )
    .expect("atualiza");

    let rrule: String = {
      let conn = db.conn().unwrap();
      conn
        .query_row(
          "SELECT rrule FROM event_recurrence WHERE id = 'rule-1'",
          [],
          |r| r.get(0),
        )
        .expect("regra")
    };
    assert_eq!(
      rrule, "FREQ=WEEKLY;BYDAY=TU",
      "INV-5: a regra fica byte-idêntica"
    );
    let still_linked: i64 = db
      .conn()
      .unwrap()
      .query_row(
        "SELECT COUNT(*) FROM event_occurrence WHERE id = 'occ-1' AND rule_id IS NOT NULL",
        [],
        |r| r.get(0),
      )
      .expect("conta");
    assert_eq!(still_linked, 0, "a instância foi solta da regra");
  }

  #[test]
  fn concluir_sem_registro_nao_escreve_nada() {
    let db = Db::open_in_memory().expect("banco");
    let resp = create_responsibility_core(&db, responsibility_payload("Ler")).expect("cria");
    let err = complete_item_core(
      &db,
      CompleteItem {
        target: Target {
          kind: "responsibility".into(),
          id: resp.id.clone(),
        },
        record: None,
      },
    )
    .expect_err("INV-3 precisa recusar");
    assert!(err.to_string().contains("INV-3"));

    let (state, execs): (String, i64) = {
      let conn = db.conn().unwrap();
      (
        conn
          .query_row(
            "SELECT state FROM responsibility WHERE id = ?1",
            params![resp.id],
            |r| r.get(0),
          )
          .expect("estado"),
        conn
          .query_row("SELECT COUNT(*) FROM execution_record", [], |r| r.get(0))
          .expect("conta"),
      )
    };
    assert_eq!(state, "planejado", "o estado não mudou");
    assert_eq!(execs, 0, "nenhum registro foi criado");
  }

  #[test]
  fn concluir_com_registro_grava_os_dois() {
    let db = Db::open_in_memory().expect("banco");
    let resp = create_responsibility_core(&db, responsibility_payload("Ler")).expect("cria");
    complete_item_core(
      &db,
      CompleteItem {
        target: Target {
          kind: "responsibility".into(),
          id: resp.id.clone(),
        },
        record: Some(execution(T0 + 45 * 60_000)),
      },
    )
    .expect("conclui");
    let (state, execs, actual): (String, i64, i64) = {
      let conn = db.conn().unwrap();
      (
        conn
          .query_row(
            "SELECT state FROM responsibility WHERE id = ?1",
            params![resp.id],
            |r| r.get(0),
          )
          .expect("estado"),
        conn
          .query_row("SELECT COUNT(*) FROM execution_record", [], |r| r.get(0))
          .expect("conta"),
        conn
          .query_row(
            "SELECT actual_duration_min FROM execution_record",
            [],
            |r| r.get(0),
          )
          .expect("duração"),
      )
    };
    assert_eq!(state, "concluido");
    assert_eq!(execs, 1, "INV-3: o registro acompanha a conclusão");
    assert_eq!(actual, 45);
  }

  #[test]
  fn registro_parcial_nao_conclui_o_item() {
    let db = Db::open_in_memory().expect("banco");
    let resp = create_responsibility_core(&db, responsibility_payload("Ler")).expect("cria");
    record_execution_core(
      &db,
      RecordExecution {
        target: Target {
          kind: "responsibility".into(),
          id: resp.id.clone(),
        },
        record: execution(T0 + 20 * 60_000),
      },
    )
    .expect("registra");
    let state: String = db
      .conn()
      .unwrap()
      .query_row(
        "SELECT state FROM responsibility WHERE id = ?1",
        params![resp.id],
        |r| r.get(0),
      )
      .expect("estado");
    assert_eq!(state, "em_andamento", "registrar não conclui");
  }

  #[test]
  fn apagar_evento_preserva_o_historico_de_execucao() {
    let db = Db::open_in_memory().expect("banco");
    let evt = create_event_core(&db, event_payload("Aula")).expect("cria");
    db.write(|tx| {
      tx.execute(
        "INSERT INTO event_occurrence
                   (id, event_id, original_start, starts_at, ends_at, state, override,
                    created_at, updated_at)
                 VALUES ('occ-1', ?1, ?2, ?2, ?3, 'planejado', 'none', ?2, ?2)",
        params![evt.id, T0, T0 + HOUR],
      )?;
      tx.execute(
        "INSERT INTO event_recurrence (id, event_id, rrule, dtstart_tz, exdates_json)
                 VALUES ('rule-1', ?1, 'FREQ=WEEKLY', 'UTC', '[]')",
        params![evt.id],
      )?;
      tx.execute(
        "INSERT INTO execution_record
                   (id, target_kind, target_id, started_at, finished_at,
                    actual_duration_min, source, created_at)
                 VALUES ('exe-1', 'event', ?1, ?2, ?3, 50, 'timer', ?2)",
        params![evt.id, T0, T0 + 3_000_000],
      )?;
      Ok(())
    })
    .expect("prepara");

    delete_event_core(&db, EntityId { id: evt.id.clone() }).expect("apaga");

    let (execs, occs, rules): (i64, i64, i64) = {
      let conn = db.conn().unwrap();
      (
        conn
          .query_row(
            "SELECT COUNT(*) FROM execution_record WHERE id = 'exe-1'",
            [],
            |r| r.get(0),
          )
          .expect("histórico"),
        conn
          .query_row(
            "SELECT COUNT(*) FROM event_occurrence WHERE event_id = ?1",
            params![evt.id],
            |r| r.get(0),
          )
          .expect("ocorrências"),
        conn
          .query_row(
            "SELECT COUNT(*) FROM event_recurrence WHERE event_id = ?1",
            params![evt.id],
            |r| r.get(0),
          )
          .expect("regras"),
      )
    };
    assert_eq!(execs, 1, "D12: o histórico sobrevive ao item");
    assert_eq!(occs, 0, "as ocorrências foram limpas");
    assert_eq!(rules, 0, "a regra foi limpa");
  }

  #[test]
  fn bloco_exige_responsabilidade_existente() {
    let db = Db::open_in_memory().expect("banco");
    let err = plan_block_core(
      &db,
      UpsertBlock {
        id: None,
        responsibility_id: "inexistente".into(),
        layer_id: "estudos".into(),
        starts_at: T0,
        ends_at: T0 + HOUR,
        time_zone: "UTC".into(),
        planned_duration: 60,
        state: None,
        origin: "cecistudy".into(),
        timer_state: None,
        accumulated: None,
      },
    )
    .expect_err("INV-1: a FK exige uma responsabilidade real");
    assert!(err.to_string().contains("banco"), "veio do banco: {err}");
  }

  #[test]
  fn reagendar_bloco_move_o_horario_e_mantem_a_duracao() {
    let db = Db::open_in_memory().expect("banco");
    let resp = create_responsibility_core(&db, responsibility_payload("Ler")).expect("cria");
    let bloco = plan_block_core(
      &db,
      UpsertBlock {
        id: None,
        responsibility_id: resp.id,
        layer_id: "estudos".into(),
        starts_at: T0,
        ends_at: T0 + HOUR,
        time_zone: "UTC".into(),
        planned_duration: 60,
        state: None,
        origin: "cecistudy".into(),
        timer_state: None,
        accumulated: None,
      },
    )
    .expect("planeja");

    let movido = reschedule_block_core(
      &db,
      RescheduleBlock {
        id: bloco.id.clone(),
        starts_at: T0 + 3 * HOUR,
        ends_at: T0 + 4 * HOUR,
      },
    )
    .expect("reagenda");
    assert_eq!(movido.starts_at, T0 + 3 * HOUR);
    assert_eq!(movido.planned_duration, 60, "reagendar não muda o quanto");
  }

  #[test]
  fn filtro_de_responsabilidade_e_opcional() {
    let db = Db::open_in_memory().expect("banco");
    create_responsibility_core(&db, responsibility_payload("Com prazo")).expect("cria");
    let mut sem_prazo = responsibility_payload("Sem prazo");
    sem_prazo.due_at = None;
    create_responsibility_core(&db, sem_prazo).expect("cria");

    let todas = list_responsibilities_core(&db, ResponsibilityFilter::default()).expect("lista");
    assert_eq!(todas.len(), 2);
    let vencidas = list_responsibilities_core(
      &db,
      ResponsibilityFilter {
        due_before: Some(T0),
        ..Default::default()
      },
    )
    .expect("filtra");
    assert_eq!(vencidas.len(), 1, "só a que tem prazo vencido");
    assert_eq!(vencidas[0].title, "Com prazo");
  }

  #[test]
  fn setting_sobrescreve_sem_duplicar() {
    let db = Db::open_in_memory().expect("banco");
    set_setting_core(
      &db,
      SetSetting {
        key: "locale".into(),
        value: serde_json::json!("pt-BR"),
      },
    )
    .expect("grava");
    set_setting_core(
      &db,
      SetSetting {
        key: "locale".into(),
        value: serde_json::json!("en-US"),
      },
    )
    .expect("sobrescreve");
    let total: i64 = db
      .conn()
      .unwrap()
      .query_row("SELECT COUNT(*) FROM setting", [], |r| r.get(0))
      .expect("conta");
    assert_eq!(total, 1);
    let value = get_setting_core(
      &db,
      SettingKey {
        key: "locale".into(),
      },
    )
    .expect("lê");
    assert_eq!(value.expect("presente").value, serde_json::json!("en-US"));
  }

  #[test]
  fn escopo_de_recorrencia_invalido_e_recusado() {
    let db = Db::open_in_memory().expect("banco");
    let err = update_recurrence_core(
      &db,
      UpdateRecurrence {
        rule_id: "rule-1".into(),
        rrule: "FREQ=DAILY".into(),
        scope: "sempre".into(),
        // Escopo desconhecido é recusado antes de qualquer exigência de
        // ocorrência, então aqui o `occurrenceId` é mesmo ausente.
        occurrence_id: None,
      },
    )
    .expect_err("escopo desconhecido");
    assert!(err.to_string().contains("escopo"));
  }

  // -----------------------------------------------------------------------
  // C5 · A máquina de estados no caminho de escrita
  //
  // Estes testes são o que o débito C5 pedia e não existia: a tabela tinha
  // arquivo e teste em TypeScript, e **zero** imposição em Rust. Sem eles, a
  // regra pode ser removida de `exigir_transicao` e a suíte continua verde.
  // -----------------------------------------------------------------------

  #[test]
  fn tabela_de_transicoes_bate_com_a_spec() {
    // `cancelado` e `dispensado` voltam para `planejado` (§4: alteração
    // externa, decisão revisável, não fim). `concluido` só volta para
    // `em_andamento` — e é por isso que `concluido → planejado` é ilegal.
    assert!(transition_allowed("planejado", "concluido"));
    assert!(transition_allowed("concluido", "em_andamento"));
    assert!(transition_allowed("cancelado", "planejado"));
    assert!(transition_allowed("dispensado", "planejado"));

    assert!(!transition_allowed("concluido", "planejado"));
    assert!(!transition_allowed("concluido", "adiado"));
    assert!(!transition_allowed("concluido", "cancelado"));
    assert!(!transition_allowed("em_andamento", "planejado"));

    // Estado desconhecido recusa, em vez de passar como "tudo permitido".
    assert!(!transition_allowed("estado_inexistente", "planejado"));
    assert!(!transition_allowed("planejado", "estado_inexistente"));
  }

  #[test]
  fn transicao_ilegal_e_recusada_na_escrita_da_ocorrencia() {
    let db = Db::open_in_memory().expect("banco");
    let evt = create_event_core(&db, event_payload("Aula")).expect("cria");
    db.write(|tx| {
      tx.execute(
        "INSERT INTO event_occurrence
                   (id, event_id, rule_id, original_start, starts_at, ends_at,
                    state, override, created_at, updated_at)
                 VALUES ('occ-c5', ?1, NULL, ?2, ?2, ?3, 'concluido', 'none', ?2, ?2)",
        params![evt.id, T0, T0 + HOUR],
      )?;
      Ok(())
    })
    .expect("semeia");

    // C5: `concluido → planejado` era gravável de ponta a ponta. Agora recusa.
    let err = set_occurrence_state_core(
      &db,
      SetOccurrenceState {
        id: "occ-c5".into(),
        state: "planejado".into(),
        reason: None,
      },
    )
    .expect_err("concluido → planejado é ilegal");
    assert!(err.to_string().contains("transição ilegal"), "{}", err);

    // E a gravação não aconteceu: o estado continua `concluido`.
    let conn = db.conn().unwrap();
    let state: String = conn
      .query_row(
        "SELECT state FROM event_occurrence WHERE id = 'occ-c5'",
        [],
        |r| r.get(0),
      )
      .expect("estado");
    assert_eq!(state, "concluido", "a transição recusada não gravou nada");
  }

  #[test]
  fn transicao_permitida_e_aceita_na_escrita_da_ocorrencia() {
    let db = Db::open_in_memory().expect("banco");
    let evt = create_event_core(&db, event_payload("Aula")).expect("cria");
    db.write(|tx| {
      tx.execute(
        "INSERT INTO event_occurrence
                   (id, event_id, rule_id, original_start, starts_at, ends_at,
                    state, override, created_at, updated_at)
                 VALUES ('occ-ok', ?1, NULL, ?2, ?2, ?3, 'concluido', 'none', ?2, ?2)",
        params![evt.id, T0, T0 + HOUR],
      )?;
      Ok(())
    })
    .expect("semeia");

    // `concluido → em_andamento` é a única saída de `concluido`, e passa.
    let occ = set_occurrence_state_core(
      &db,
      SetOccurrenceState {
        id: "occ-ok".into(),
        state: "em_andamento".into(),
        reason: None,
      },
    )
    .expect("concluido → em_andamento é permitido");
    assert_eq!(occ.state, "em_andamento");
  }

  /// Semeia um evento do Google direto por SQL, porque `create_event_core` já
  /// recusa `origin: "google"` — que é justamente o que se quer num evento que
  /// entra pela sincronização do Google, e não pela tela.
  fn semear_google(db: &Db, occ_id: &str) {
    db.write(|tx| {
      tx.execute(
        "INSERT INTO calendar_event
                   (id, layer_id, title, starts_at, ends_at, all_day, time_zone,
                    commitment, state, origin, created_at, updated_at)
         VALUES ('evt-google', 'google', 'Reunião do Google', ?1, ?2, 0,
                 'America/Sao_Paulo', 'importante', 'planejado', 'google', ?1, ?1)",
        params![T0, T0 + HOUR],
      )?;
      tx.execute(
        "INSERT INTO event_occurrence
                   (id, event_id, rule_id, original_start, starts_at, ends_at,
                    state, override, created_at, updated_at)
         VALUES (?1, 'evt-google', NULL, ?2, ?2, ?3, 'planejado', 'none', ?2, ?2)",
        params![occ_id, T0, T0 + HOUR],
      )?;
      Ok(())
    })
    .expect("semeia evento do Google");
  }

  #[test]
  fn inv6_concluir_item_do_google_e_recusado() {
    // Slice 1. Antes de `exigir_origem_editavel`, este comando gravava: as cinco
    // outras escritas guardavam a origem e estas duas não. A regra é INV-2/INV-6
    // e ela mora no caminho de escrita, não na disciplina de quem chama.
    let db = Db::open_in_memory().expect("banco");
    semear_google(&db, "occ-google");

    let err = set_occurrence_state_core(
      &db,
      SetOccurrenceState {
        id: "occ-google".into(),
        state: "concluido".into(),
        reason: None,
      },
    )
    .expect_err("concluir um item do Google é escrever nele");
    assert!(err.to_string().contains("somente leitura"), "{}", err);

    let conn = db.conn().unwrap();
    let state: String = conn
      .query_row(
        "SELECT state FROM event_occurrence WHERE id = 'occ-google'",
        [],
        |r| r.get(0),
      )
      .expect("estado");
    assert_eq!(state, "planejado", "a escrita recusada não gravou nada");
  }

  #[test]
  fn inv6_remarcar_item_do_google_e_recusado() {
    let db = Db::open_in_memory().expect("banco");
    semear_google(&db, "occ-google-move");

    let err = move_occurrence_core(
      &db,
      MoveOccurrence {
        id: "occ-google-move".into(),
        starts_at: T0 + DIA_MS,
        ends_at: T0 + DIA_MS + HOUR,
      },
    )
    .expect_err("remarcar um item do Google é escrever nele");
    assert!(err.to_string().contains("somente leitura"), "{}", err);

    let conn = db.conn().unwrap();
    let (starts_at, override_kind): (i64, String) = conn
      .query_row(
        "SELECT starts_at, override FROM event_occurrence WHERE id = 'occ-google-move'",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
      )
      .expect("ocorrencia");
    assert_eq!(starts_at, T0, "a remarca recusada não deslocou nada");
    assert_eq!(override_kind, "none", "nem marcou override");
  }

  #[test]
  fn remarca_inverte_a_duracao_e_recusada() {
    // Não passa por `exigir_transicao` — remarca não muda estado — mas um item
    // que termina antes de começar atravessa a meia-noite ao contrário e quebra
    // a coluna do dia na grade.
    let db = Db::open_in_memory().expect("banco");
    let evt = create_event_core(&db, event_payload("Aula")).expect("cria");
    db.write(|tx| {
      tx.execute(
        "INSERT INTO event_occurrence
                   (id, event_id, rule_id, original_start, starts_at, ends_at,
                    state, override, created_at, updated_at)
         VALUES ('occ-dur', ?1, NULL, ?2, ?2, ?3, 'planejado', 'none', ?2, ?2)",
        params![evt.id, T0, T0 + HOUR],
      )?;
      Ok(())
    })
    .expect("semeia");

    let err = move_occurrence_core(
      &db,
      MoveOccurrence {
        id: "occ-dur".into(),
        starts_at: T0 + HOUR,
        ends_at: T0,
      },
    )
    .expect_err("terminar antes de começar não é remarca");
    assert!(err.to_string().contains("duração não positiva"), "{}", err);

    // E o mesmo comando com duração positiva passa, para o teste não ser
    // satisfeito só porque o comando está sempre recusando.
    let occ = move_occurrence_core(
      &db,
      MoveOccurrence {
        id: "occ-dur".into(),
        starts_at: T0 + DIA_MS,
        ends_at: T0 + DIA_MS + HOUR,
      },
    )
    .expect("remarca de verdade passa");
    assert_eq!(occ.override_kind, "moved");
    assert_eq!(occ.starts_at, T0 + DIA_MS);
  }

  #[test]
  fn a_origem_do_evento_viaja_na_leitura_da_grade() {
    // O botão "Concluir" só pode aparecer quando a tela sabe a origem. Sem este
    // campo no wire a UI oferece a ação e leva erro depois do clique.
    let db = Db::open_in_memory().expect("banco");
    let proprio = create_event_core(&db, event_payload("Aula")).expect("cria");
    semear_google(&db, "occ-grade-google");
    db.write(|tx| {
      tx.execute(
        "INSERT INTO event_occurrence
                   (id, event_id, rule_id, original_start, starts_at, ends_at,
                    state, override, created_at, updated_at)
         VALUES ('occ-grade-proprio', ?1, NULL, ?2, ?2, ?3, 'planejado', 'none', ?2, ?2)",
        params![proprio.id, T0 + HOUR, T0 + HOUR + 60_000],
      )?;
      Ok(())
    })
    .expect("semeia");

    let itens = list_events_in_window_core(
      &db,
      Window {
        from: T0 - DIA_MS,
        to: T0 + DIA_MS,
      },
    )
    .expect("lista");
    let google = itens
      .iter()
      .find(|i| i.id == "occ-grade-google")
      .expect("item do google");
    let nosso = itens
      .iter()
      .find(|i| i.id == "occ-grade-proprio")
      .expect("item próprio");

    assert_eq!(
      google.origin, "google",
      "a origem do Google precisa chegar na grade"
    );
    assert_eq!(nosso.origin, "cecistudy");
  }

  /// O ciclo que a tela executa, inteiro, dentro do Rust.
  ///
  /// A tela faz exatamente isto: `setOccurrenceState` → invalida o cache →
  /// `list_events_in_window` de novo → desenha. Se cada passo passa e o
  /// resultado final é o estado novo, a tela funciona; se algum passo só
  /// compila, ela mostra a linha antiga e nada acusa.
  #[test]
  fn o_ciclo_escreve_invalida_e_rele_traz_o_estado_novo() {
    let db = Db::open_in_memory().expect("banco");
    let evt = create_event_core(&db, event_payload("Aula")).expect("cria");
    db.write(|tx| {
      tx.execute(
        "INSERT INTO event_occurrence
                   (id, event_id, rule_id, original_start, starts_at, ends_at,
                    state, override, created_at, updated_at)
         VALUES ('occ-ciclo', ?1, NULL, ?2, ?2, ?3, 'planejado', 'none', ?2, ?2)",
        params![evt.id, T0, T0 + HOUR],
      )?;
      Ok(())
    })
    .expect("semeia");

    let janela = Window {
      from: T0 - DIA_MS,
      to: T0 + DIA_MS,
    };
    let antes = list_events_in_window_core(&db, janela).expect("primeira leitura");
    assert_eq!(
      antes
        .iter()
        .find(|i| i.id == "occ-ciclo")
        .expect("item na grade")
        .state,
      "planejado"
    );

    // Passo 2: a escrita da tela.
    set_occurrence_state_core(
      &db,
      SetOccurrenceState {
        id: "occ-ciclo".into(),
        state: "concluido".into(),
        reason: None,
      },
    )
    .expect("concluir");

    // Passo 3: a releitura. `list_events_in_window_core` materializa e lê na
    // mesma transação, então não existe janela em que a tela veja o estado
    // velho depois de escrever — e é isso que dispensa o cliente de ter um
    // segundo cache para o caso "acabei de escrever".
    let depois = list_events_in_window_core(&db, janela).expect("rele");
    let item = depois
      .iter()
      .find(|i| i.id == "occ-ciclo")
      .expect("item continua na grade");
    assert_eq!(item.state, "concluido", "a releitura trouxe o estado velho");
    assert_eq!(item.origin, "cecistudy");

    // E remarca, pela mesma janela: muda quando acontece e preserva a
    // identidade da instância, que é `original_start`.
    let novo_inicio = T0 + HOUR * 6;
    move_occurrence_core(
      &db,
      MoveOccurrence {
        id: "occ-ciclo".into(),
        starts_at: novo_inicio,
        ends_at: novo_inicio + HOUR,
      },
    )
    .expect("remarca");
    let relido = list_events_in_window_core(&db, janela).expect("rele apos remarca");
    let movido = relido
      .iter()
      .find(|i| i.id == "occ-ciclo")
      .expect("item remarcado");
    assert_eq!(movido.starts_at, novo_inicio);
    assert_eq!(movido.override_kind, "moved");
    assert_eq!(
      movido.state, "concluido",
      "remarcar nao pode apagar a conclusao"
    );

    // A janela e semiaberta, e a consulta e `starts_at < to AND ends_at > from`.
    // Um item remarcado **exatamente** para `to` fica fora dela — e e assim que
    // o Rust trata: o que decide a entrada e a comparacao estrita, nao um
    // `>=` disfrazado. Este teste ja falhou uma vez por causa desta linha, o
    // que e a melhor forma de provar que ela importa.
    move_occurrence_core(
      &db,
      MoveOccurrence {
        id: "occ-ciclo".into(),
        starts_at: janela.to,
        ends_at: janela.to + HOUR,
      },
    )
    .expect("remarca para o limite");
    let na_borda = list_events_in_window_core(&db, janela).expect("rele na borda");
    assert!(
      !na_borda.iter().any(|i| i.id == "occ-ciclo"),
      "um item que comeca exatamente em `to` pertence a janela seguinte, nao a esta"
    );
  }

  #[test]
  fn criar_evento_sem_titulo_e_recusado() {
    // O `CHECK` do schema **não** pega isto: `''` passa em qualquer
    // `TEXT NOT NULL`. Um evento sem título aparece na grade como uma caixa
    // vazia ocupando espaço, e ninguém sabe que houve erro.
    let db = Db::open_in_memory().expect("banco");
    for titulo in ["", "   ", "\t\n"] {
      let err = create_event_core(&db, event_payload(titulo)).expect_err("titulo vazio");
      assert!(
        err.to_string().contains("tulo"),
        "titulo {:?} produziu {}",
        titulo,
        err
      );
    }
    assert_eq!(
      list_events_in_window_core(
        &db,
        Window {
          from: T0 - DIA_MS,
          to: T0 + DIA_MS
        }
      )
      .expect("lista")
      .len(),
      0,
      "nenhum evento sem titulo foi gravado"
    );
  }

  #[test]
  fn criar_evento_com_duracao_nao_positiva_e_recusado() {
    let db = Db::open_in_memory().expect("banco");

    let mut payload = event_payload("Aula");
    payload.ends_at = payload.starts_at;
    let err = create_event_core(&db, payload).expect_err("duracao zero");
    assert!(err.to_string().contains("duração"), "{}", err);

    let mut payload = event_payload("Aula");
    payload.ends_at = payload.starts_at - HOUR;
    create_event_core(&db, payload).expect_err("termina antes de começar");

    // E o caso honesto passa, para o teste não ficar satisfeito só porque o
    // comando recusa sempre.
    let evt = create_event_core(&db, event_payload("Aula")).expect("aula de 1h");
    assert_eq!(evt.ends_at - evt.starts_at, HOUR);
  }

  #[test]
  fn o_ciclo_de_criar_com_recorrencia_e_reler() {
    // O caminho que o formulário "Novo" percorre: `createEvent` →
    // `upsertRecurrence` → a tela invalida → relê. Sem o passo da recorrência o
    // evento entra uma vez e a grade mostra um item onde a usuária pediu uma
    // série.
    let db = Db::open_in_memory().expect("banco");
    let evt = create_event_core(&db, event_payload("Aula")).expect("cria");
    upsert_recurrence_core(
      &db,
      UpsertRecurrence {
        id: None,
        event_id: evt.id.clone(),
        rrule: "FREQ=DAILY".into(),
        dtstart_tz: "America/Sao_Paulo".into(),
        until_at: None,
        count: None,
        exdates_json: None,
      },
    )
    .expect("regra");

    let janela = Window {
      from: T0 - DIA_MS,
      to: T0 + DIA_MS,
    };
    let lidos = list_events_in_window_core(&db, janela).expect("rele");
    assert!(
      !lidos.is_empty(),
      "uma série diária precisa materializar os dias que cruzam a janela"
    );
    assert!(
      lidos.iter().all(|i| i.is_recurring),
      "todo item materializado sabe que é recorrente"
    );
    assert!(lidos.iter().all(|i| i.state == "planejado"));

    // E o item é nosso, que é o que faz o botão Concluir aparecer.
    assert!(lidos.iter().all(|i| i.origin == "cecistudy"));
    assert!(transition_allowed("planejado", "concluido"));
  }

  #[test]
  fn transicao_ilegal_e_recusada_no_patch_do_evento() {
    let db = Db::open_in_memory().expect("banco");
    let evt = create_event_core(&db, event_payload("Aula")).expect("cria");
    db.write(|tx| {
      tx.execute(
        "UPDATE calendar_event SET state = 'concluido' WHERE id = ?1",
        params![evt.id.clone()],
      )?;
      Ok(())
    })
    .expect("semeia");

    let err = update_event_core(
      &db,
      UpdateEvent {
        id: evt.id.clone(),
        patch: UpsertEventPatch {
          state: Some("dispensado".into()),
          ..Default::default()
        },
      },
    )
    .expect_err("concluido → dispensado é ilegal");
    assert!(err.to_string().contains("transição ilegal"), "{}", err);
  }

  #[test]
  fn patch_sem_estado_nao_e_transicao() {
    let db = Db::open_in_memory().expect("banco");
    let evt = create_event_core(&db, event_payload("Aula")).expect("cria");
    // Evento novo é `planejado`. Patch só de título não passa pela tabela, e
    // funciona — a validação não pode barrar edição que não muda estado.
    let atualizado = update_event_core(
      &db,
      UpdateEvent {
        id: evt.id.clone(),
        patch: UpsertEventPatch {
          title: Some("Novo".into()),
          ..Default::default()
        },
      },
    )
    .expect("muda só o título");
    assert_eq!(atualizado.title, "Novo");
  }

  // -----------------------------------------------------------------------
  // F1.4 · Sobreposição tem chamador em produção
  //
  // Antes de 2026-10-03 `findOverlaps` e `overlapsPair` existiam com teste em
  // TypeScript e **zero** chamadores, e o app não detectava conflito nenhum
  // (débito C5). Este teste existe para a regra ter chamador de verdade: ele
  // passa pela **leitura da grade**, que é o caminho que a UI usa.
  // -----------------------------------------------------------------------

  #[test]
  fn leitura_da_grade_marca_sobreposicao_e_nao_bloqueia() {
    let db = Db::open_in_memory().expect("banco");
    // a 08–10 e b 09–11 se sobrepõem; c 11–12 apenas toca b e fica isolado.
    let a = criar_sobreposto(&db, "A", T0, T0 + 2 * HOUR);
    let b = criar_sobreposto(&db, "B", T0 + HOUR, T0 + 3 * HOUR);
    let c = criar_sobreposto(&db, "C", T0 + 3 * HOUR, T0 + 4 * HOUR);

    let itens = list_events_in_window_core(
      &db,
      Window {
        from: T0,
        to: T0 + 12 * HOUR,
      },
    )
    .expect("lê a janela");

    let marca = |id: &str| {
      itens
        .iter()
        .find(|i| i.title == id)
        .unwrap_or_else(|| panic!("{id} não está na janela"))
        .has_conflict
    };

    // D7: sinalizado, nunca bloqueante — os três continuam sendo lidos.
    assert_eq!(itens.len(), 3, "nenhum item foi descartado");
    assert!(marca("A"), "A se sobrepõe com B");
    assert!(marca("B"), "B se sobrepõe com A");
    assert!(!marca("C"), "C só toca B e não se sobrepõe");

    // Os três eventos foram criados; o teste usa os títulos para casar.
    assert!(a.id != b.id && b.id != c.id);
  }

  #[test]
  fn leitura_da_grade_nao_marca_nada_quando_nada_sobrepoe() {
    let db = Db::open_in_memory().expect("banco");
    criar_sobreposto(&db, "A", T0, T0 + HOUR);
    criar_sobreposto(&db, "B", T0 + 2 * HOUR, T0 + 3 * HOUR);

    let itens = list_events_in_window_core(
      &db,
      Window {
        from: T0,
        to: T0 + 12 * HOUR,
      },
    )
    .expect("lê a janela");

    assert_eq!(itens.len(), 2);
    assert!(
      itens.iter().all(|i| !i.has_conflict),
      "intervalos que não se sobrepõem não podem ser marcados"
    );
  }

  // -----------------------------------------------------------------------
  // D1.3 · Os destinos válidos viajam na leitura
  //
  // É o que tira `state-machine.ts` do caminho de produção: a tela pergunta ao
  // backend quais transições existem, em vez de manter a tabela em TypeScript.
  // -----------------------------------------------------------------------

  #[test]
  fn leitura_da_grade_traz_os_destinos_validos() {
    let db = Db::open_in_memory().expect("banco");
    criar_sobreposto(&db, "A", T0, T0 + HOUR);

    let itens = list_events_in_window_core(
      &db,
      Window {
        from: T0,
        to: T0 + HOUR,
      },
    )
    .expect("lê a janela");

    let item = itens.first().expect("um item");
    // Espelha `domain.rs::TRANSITIONS` para `planejado`: tem `concluido`, e não
    // tem `planejado` (um estado não transiciona para si mesmo).
    assert!(
      item.allowed_transitions.iter().any(|d| d == "concluido"),
      "planejado → concluido tem de vir no read-model"
    );
    assert!(
      !item.allowed_transitions.iter().any(|d| d == "planejado"),
      "a leitura não pode oferecer transição para o mesmo estado"
    );
  }

  /// Cria um evento e materializa a ocorrência no intervalo dado.
  fn criar_sobreposto(db: &Db, titulo: &str, comeca: i64, termina: i64) -> CalendarEvent {
    let mut payload = event_payload(titulo);
    payload.starts_at = comeca;
    payload.ends_at = termina;
    let evt = create_event_core(db, payload).expect("cria evento");
    db.write(|tx| {
      tx.execute(
        "INSERT INTO event_occurrence
                   (id, event_id, original_start, starts_at, ends_at, state, override,
                    created_at, updated_at)
                 VALUES (?4, ?1, ?2, ?2, ?3, 'planejado', 'none', ?2, ?2)",
        params![evt.id, comeca, termina, format!("occ-{titulo}")],
      )?;
      Ok(())
    })
    .expect("materializa");
    evt
  }

  // -----------------------------------------------------------------------
  // C6 · O motor de recorrência
  //
  // Antes, `FREQ=MONTHLY` caía no ramo `_` de `matches_freq` e produzia série
  // **semanal**. Estes testes são o golden que faltava.
  // -----------------------------------------------------------------------

  /// Constrói o `RecurringEvent` mínimo para exercitar `expand_rule`.
  fn serie(rrule: &str) -> RecurringEvent {
    RecurringEvent {
      event_id: "evt-1".into(),
      starts_at: T0,
      ends_at: T0 + HOUR,
      event_state: None,
      rule_id: Some("rule-1".into()),
      rrule: Some(rrule.into()),
      until_at: None,
      count: None,
      exdates_json: None,
    }
  }

  /// Instantes como dias desde a âncora, que é legível numa asserção.
  fn dias_relativos(anchor: i64, saida: &[i64]) -> Vec<i64> {
    saida
      .iter()
      .map(|ms| (ms - anchor).div_euclid(DIA_MS))
      .collect()
  }

  #[test]
  fn monthly_expande_por_mes_e_nao_por_semana() {
    // Âncora em dia 15, 6 meses à frente. O passo é mês civil, então as
    // distâncias crescem em ~30/31 dias — nunca em 7.
    let saida = expand_rule(&serie("FREQ=MONTHLY"), T0, T0 + 400 * DIA_MS).expect("expande");
    assert!(
      saida.len() >= 6,
      "6 meses devem produzir 6 ocorrências, veio {}",
      saida.len()
    );

    let mut anterior = None;
    for inicio in &saida {
      let dias = (inicio - T0) / DIA_MS;
      if let Some(prev) = anterior {
        // Nenhum passo pode ser de 7 dias: seria a série semanal.
        assert_ne!(dias - prev, 7, "FREQ=MONTHLY não pode virar semanal");
        assert!(
          (25..=62).contains(&(dias - prev)),
          "passo mensal fora da faixa: {} dias",
          dias - prev
        );
      }
      anterior = Some(dias);
    }
  }

  #[test]
  fn monthly_preserva_o_dia_e_a_hora_da_ancora() {
    let saida = expand_rule(&serie("FREQ=MONTHLY"), T0, T0 + 200 * DIA_MS).expect("expande");
    let hora_ancora = T0.rem_euclid(DIA_MS);
    for inicio in &saida {
      assert_eq!(inicio.rem_euclid(DIA_MS), hora_ancora, "a hora mudau");
    }
  }

  #[test]
  fn monthly_dia31_pula_fevereiro_e_nao_limita() {
    // **RFC 5545 §3.3.10:** "recurrence instances with an invalid date […] MUST be
    // ignored and MUST NOT be counted". Dia 31 em fevereiro **não existe**, então
    // fevereiro é pulado — não vira dia 28.
    //
    // Este teste já afirmava o contrário, e o motor RFC é quem tinha razão. A
    // versão anterior limitava o dia ao último do mês, que é o que a maioria dos
    // calendários faz por decisão de produto e o que a RFC proíbe por omissão.
    // `golden/recurrence.jsonl` fixa a escolha: `[0, 59, 120, 181]`.
    let anchor = 1_769_817_600_000; // 2026-01-31T00:00:00Z
    let mut ev = serie("FREQ=MONTHLY");
    ev.starts_at = anchor;
    ev.ends_at = anchor + HOUR;
    let saida = expand_rule(&ev, anchor, anchor + 300 * DIA_MS).expect("expande");

    assert_eq!(
      dias_relativos(anchor, &saida).first().copied(),
      Some(0),
      "a âncora é a primeira ocorrência"
    );
    // De 31 em 31 dias: janeiro, março, maio, julho… e **nada** em 28/29 de
    // fevereiro, nem em 30 de abril. Se o motor limitar ao último dia do mês, os
    // passos seriam 59, 90, 120 em vez de 59, 120, 181.
    let dias = dias_relativos(anchor, &saida);
    assert_eq!(
      &dias[..4],
      &[0, 59, 120, 181],
      "fevereiro, abril e junho não têm dia 31"
    );
    let (_, mes, _) = civil_from_days(saida[1].div_euclid(DIA_MS));
    assert_eq!(mes, 3, "a segunda ocorrência é em março, não em fevereiro");
  }

  #[test]
  fn monthly_com_intervalo_pula_meses() {
    let saida =
      expand_rule(&serie("FREQ=MONTHLY;INTERVAL=3"), T0, T0 + 400 * DIA_MS).expect("expande");
    // Todo 3 meses ≈ 90 dias. Se fosse semanal, os primeiros passos seriam 7.
    assert_eq!(dias_relativos(T0, &saida).first().copied(), Some(0));
    let segundo = dias_relativos(T0, &saida)[1];
    assert!(
      (85..=95).contains(&segundo),
      "passo de 3 meses, veio {segundo}"
    );
  }

  #[test]
  fn monthly_respeita_count() {
    let mut ev = serie("FREQ=MONTHLY");
    ev.count = Some(3);
    let saida = expand_rule(&ev, T0, T0 + 400 * DIA_MS).expect("expande");
    assert_eq!(saida.len(), 3, "COUNT=3 produz 3");
  }

  #[test]
  fn rrule_invalida_e_recusada() {
    // Antes do crate, `FREQ` inválido virava semanal por queda de caso e a
    // ausência de `FREQ` também. Agora o parse do RFC recusa os dois, e a recusa
    // nomeia a RRULE — o que a usuária precisa ler.
    for rrule in [
      "FREQ=WEEKLYLY",
      "FREQ=mensal",
      "INTERVAL=1",
      "FREQ=WEEKLY;INTERVAL=zero",
      "NAO_E_RRULE",
    ] {
      let err = expand_rule(&serie(rrule), T0, T0 + 400 * DIA_MS)
        .expect_err("RRULE fora de RFC 5545 tem de ser recusada");
      assert!(
        err.to_string().contains("RRULE"),
        "a recusa tem de dizer que a RRULE é inválida: {}",
        err
      );
    }
  }

  #[test]
  fn rrule_valida_com_valores_que_o_parser_antigo_nao_sabia() {
    // As capacidades que justificaram a troca: `BYSETPOS`, `BYDAY` com ordinal,
    // `BYMONTHDAY` e `FREQ=YEARLY` não existiam no parser de 6 chaves.
    for rrule in [
      "FREQ=MONTHLY;BYDAY=1MO",
      "FREQ=MONTHLY;BYDAY=FR;BYSETPOS=-1",
      "FREQ=MONTHLY;BYMONTHDAY=31",
      "FREQ=YEARLY",
      "FREQ=WEEKLY;BYDAY=MO,WE,FR",
      "FREQ=MONTHLY;BYSETPOS=2;BYDAY=MO,TU,WE,TH,FR",
    ] {
      let saida = expand_rule(&serie(rrule), T0, T0 + 800 * DIA_MS)
        .unwrap_or_else(|e| panic!("`{rrule}` é RFC 5545 válida e tem de expandir: {e}"));
      assert!(!saida.is_empty(), "`{rrule}` produziu nada");
    }
  }

  // -----------------------------------------------------------------------
  // D61 · O golden é o contrato
  //
  // O teste abaixo lê `golden/*.jsonl` e compara a saída do Rust com a
  // esperada. O espelho TypeScript lê o **mesmo** arquivo. Divergência entre
  // os dois lados é falha, e a falha nomeia o caso.
  // -----------------------------------------------------------------------

  /// Lê um caso do golden. Devolve None para linha vazia e para comentário.
  fn caso_do_golden(linha: &str) -> Option<Golden> {
    let t = linha.trim();
    if t.is_empty() || t.starts_with('#') {
      return None;
    }
    Some(serde_json::from_str(t).expect("caso do golden é JSON"))
  }

  #[derive(serde::Deserialize)]
  #[serde(rename_all = "camelCase")]
  struct Golden {
    caso: String,
    entrada: serde_json::Value,
    #[serde(default)]
    saida: serde_json::Value,
  }

  fn caminho_golden(nome: &str) -> String {
    // `CARGO_MANIFEST_DIR` é `src-tauri/`, e o golden mora na raiz do app.
    format!("{}/../golden/{nome}", env!("CARGO_MANIFEST_DIR"))
  }

  // O golden da máquina de estados mora em `domain.rs`, junto da regra que ele
  // prova. Aqui só o da recorrência, que é a regra deste arquivo.

  #[test]
  fn golden_da_recorrencia_bate_com_a_regra_em_rust() {
    let bruto = std::fs::read_to_string(caminho_golden("recurrence.jsonl"))
      .expect("golden/recurrence.jsonl existe");
    let mut total = 0;
    for linha in bruto.lines() {
      let Some(caso) = caso_do_golden(linha) else {
        continue;
      };
      total += 1;
      let rrule = caso.entrada["rrule"].as_str().expect("entrada.rrule");
      let ancora_iso = caso.entrada["ancora"].as_str().expect("entrada.ancora");
      let esperado: Vec<i64> = caso.saida["diasDaAncora"]
        .as_array()
        .expect("saida.diasDaAncora")
        .iter()
        .map(|v| v.as_i64().expect("dia inteiro"))
        .collect();

      let ancora = iso_para_ms(ancora_iso);
      let mut ev = serie(rrule);
      ev.starts_at = ancora;
      ev.ends_at = ancora + HOUR;
      // `COUNT` limita a expansão ao número de casos do golden. Isso não
      // mascara bug de passo: o que o golden fixa é o VALOR de cada
      // instante, e um passo errado faz o valor divergir mesmo com COUNT.
      ev.count = Some(esperado.len() as i64);

      let saida = expand_rule(&ev, ancora, i64::MAX)
        .unwrap_or_else(|e| panic!("caso `{}`: a recusa foi inesperada: {e}", caso.caso));
      // Traz a saída para o eixo do golden: dias desde a âncora.
      let obtido: Vec<i64> = saida.iter().map(|ms| (ms - ancora) / DIA_MS).collect();
      assert_eq!(
        obtido, esperado,
        "caso `{}`: o Rust discorda do golden",
        caso.caso
      );
    }
    assert!(total >= 6, "o golden da recorrência tem casos");
  }

  /// `2026-01-15T12:00:00Z` para epoch-ms. Só o formato que o golden usa, e
  /// propositalmente sem `chrono`: o golden tem de ser lido sem dependência.
  fn iso_para_ms(iso: &str) -> i64 {
    let corpo = iso.trim_end_matches('Z');
    let (data, hora) = corpo.split_once('T').expect("ISO com T");
    let mut partes = data.split('-');
    let ano: i64 = partes.next().expect("ano").parse().expect("ano");
    let mes: i64 = partes.next().expect("mes").parse().expect("mes");
    let dia: i64 = partes.next().expect("dia").parse().expect("dia");
    let h: i64 = hora.split(':').next().unwrap_or("0").parse().expect("hora");
    days_from_civil(ano, mes, dia) * DIA_MS + h * HOUR
  }

  // -----------------------------------------------------------------------
  // Faculdade (SPEC-D-010, D74/D75) — os testes do caminho de IPC
  // -----------------------------------------------------------------------

  fn regra_input(nota_minima: f64) -> RegraInput {
    RegraInput {
      tipo_de_calculo: "ponderada".into(),
      nota_minima,
      arredondamento: 0.1,
      frequencia_minima: 0.75,
      exame_modo: "nenhum".into(),
      exame_minima: 6.0,
      exame_elegivel_minima: 4.0,
    }
  }

  fn semestre_novo(rotulo: &str) -> NovoSemestre {
    NovoSemestre {
      rotulo: rotulo.into(),
      starts_on: "2026-08-01".into(),
      ends_on: "2026-12-15".into(),
    }
  }

  fn disciplina_nova(semestre_id: &str, nome: &str) -> NovaDisciplina {
    NovaDisciplina {
      semestre_id: semestre_id.into(),
      nome: nome.into(),
      codigo: None,
      professor: None,
      carga_horaria: Some(60),
      total_aulas: Some(40),
    }
  }

  fn avaliacao_nova(disciplina_id: &str, titulo: &str, vale_nota: Option<bool>) -> NovaAvaliacao {
    NovaAvaliacao {
      disciplina_id: disciplina_id.into(),
      titulo: titulo.into(),
      tipo: "prova".into(),
      peso: 3.0,
      vale_nota,
      plain_date: None,
    }
  }

  #[test]
  fn faculdade_media_derivada_na_leitura_respeita_o_padrao() {
    let db = Db::open_in_memory().expect("banco");
    let sem = create_semestre_core(&db, semestre_novo("2026-2")).expect("semestre");
    let disc =
      create_disciplina_core(&db, disciplina_nova(&sem.id, "Neuropsicologia")).expect("disciplina");
    let av =
      create_avaliacao_core(&db, avaliacao_nova(&disc.id, "Prova 1", None)).expect("avaliação");
    lancar_nota_core(
      &db,
      LancarNota {
        avaliacao_id: av.id,
        valor: 8.0,
      },
    )
    .expect("lança nota");

    let media = media_da_disciplina_core(
      &db,
      DisciplinaRef {
        disciplina_id: disc.id,
      },
    )
    .expect("média");
    assert_eq!(media.media, Some(8.0));
    assert_eq!(media.media_sem_arredondamento, Some(8.0));
    assert_eq!(media.situacao, "aprovada");
    assert_eq!(media.nota_necessaria_proxima, None);
    // 40 aulas × (1 − 0,75) = 10 faltas possíveis (I5: derivado, não gravado).
    assert_eq!(media.frequencia_restante, Some(10));
    assert_eq!(
      media.regra_aplicada, "padrao",
      "sem personalização herda o padrão"
    );
  }

  #[test]
  fn faculdade_encerrar_congela_a_regra_e_o_padrao_nao_move_o_historico() {
    let db = Db::open_in_memory().expect("banco");
    let sem = create_semestre_core(&db, semestre_novo("2026-1")).expect("semestre");
    let disc =
      create_disciplina_core(&db, disciplina_nova(&sem.id, "Avaliação")).expect("disciplina");
    // Personaliza para nota mínima 8,0 — substituição por inteiro (D75).
    set_regra_disciplina_core(
      &db,
      RegraDisciplinaInput {
        disciplina_id: disc.id.clone(),
        regra: regra_input(8.0),
      },
    )
    .expect("personaliza");
    let av =
      create_avaliacao_core(&db, avaliacao_nova(&disc.id, "Prova 1", None)).expect("avaliação");
    lancar_nota_core(
      &db,
      LancarNota {
        avaliacao_id: av.id,
        valor: 7.5,
      },
    )
    .expect("lança nota");

    encerrar_semestre_core(
      &db,
      SemestreRef {
        semestre_id: sem.id.clone(),
      },
    )
    .expect("encerra e congela");

    // Depois de encerrado, o padrão cai para 5,0 — e o histórico não se move.
    set_regra_padrao_core(&db, regra_input(5.0)).expect("muda o padrão");

    let media = media_da_disciplina_core(
      &db,
      DisciplinaRef {
        disciplina_id: disc.id,
      },
    )
    .expect("média");
    assert_eq!(
      media.regra_aplicada,
      format!("congelada-{}", sem.id),
      "I8: a regra do semestre responde, não a corrente"
    );
    assert_eq!(
      media.situacao, "nao-alcanca",
      "7,5 não alcança a nota mínima 8,0 da regra congelada; com o padrão novo \
       (5,0) seria aprovada — e é isso que I7 proíbe"
    );
  }

  #[test]
  fn faculdade_regra_e_estrutura_de_semestre_encerrado_sao_recusadas() {
    let db = Db::open_in_memory().expect("banco");
    let sem = create_semestre_core(&db, semestre_novo("2026-1")).expect("semestre");
    let disc = create_disciplina_core(&db, disciplina_nova(&sem.id, "Ética")).expect("disciplina");
    encerrar_semestre_core(
      &db,
      SemestreRef {
        semestre_id: sem.id.clone(),
      },
    )
    .expect("encerra");

    // I7: personalização recusada.
    let err = set_regra_disciplina_core(
      &db,
      RegraDisciplinaInput {
        disciplina_id: disc.id.clone(),
        regra: regra_input(5.0),
      },
    )
    .expect_err("regra de encerrado não muda");
    assert!(err.to_string().contains("imutável"), "diz o porquê: {err}");

    // I7: estrutura nova também não — a regra efetiva dela seria viva.
    let err = create_disciplina_core(&db, disciplina_nova(&sem.id, "Tardia"))
      .expect_err("disciplina nova em encerrado");
    assert!(err.to_string().contains("imutável"), "diz o porquê: {err}");

    let err = create_avaliacao_core(&db, avaliacao_nova(&disc.id, "Prova tardia", None))
      .expect_err("avaliação nova em encerrado");
    assert!(err.to_string().contains("imutável"), "diz o porquê: {err}");

    // Encerrar de novo também é recusado: é ato explícito, não idempotente.
    let err = encerrar_semestre_core(
      &db,
      SemestreRef {
        semestre_id: sem.id,
      },
    )
    .expect_err("encerrar duas vezes");
    assert!(
      err.to_string().contains("já encerrado"),
      "diz o porquê: {err}"
    );
  }

  #[test]
  fn faculdade_nota_continua_lancavel_em_semestre_encerrado() {
    let db = Db::open_in_memory().expect("banco");
    let sem = create_semestre_core(&db, semestre_novo("2026-1")).expect("semestre");
    let disc = create_disciplina_core(&db, disciplina_nova(&sem.id, "TCC I")).expect("disciplina");
    let av =
      create_avaliacao_core(&db, avaliacao_nova(&disc.id, "Entrega", None)).expect("avaliação");
    encerrar_semestre_core(
      &db,
      SemestreRef {
        semestre_id: sem.id,
      },
    )
    .expect("encerra");

    // I9: nota é dado — a correção real de um semestre encerrado é a nota que
    // faltou, e não há guarda aqui.
    let nota = lancar_nota_core(
      &db,
      LancarNota {
        avaliacao_id: av.id,
        valor: 9.0,
      },
    )
    .expect("I9: lança em encerrado");
    assert_eq!(nota.valor, 9.0);

    let media = media_da_disciplina_core(
      &db,
      DisciplinaRef {
        disciplina_id: disc.id,
      },
    )
    .expect("recalcula com a nota nova");
    assert_eq!(media.media, Some(9.0));
    assert_eq!(media.situacao, "aprovada");
  }

  #[test]
  fn faculdade_avaliacao_que_nao_vale_nota_nao_recebe_nota() {
    let db = Db::open_in_memory().expect("banco");
    let sem = create_semestre_core(&db, semestre_novo("2026-2")).expect("semestre");
    let disc =
      create_disciplina_core(&db, disciplina_nova(&sem.id, "Seminário")).expect("disciplina");
    // §4.2 [D]: avaliação que não vale nota vira responsabilidade, e lançar
    // nota nela é erro — e o erro diz isso.
    let av = create_avaliacao_core(&db, avaliacao_nova(&disc.id, "Seminário", Some(false)))
      .expect("avaliação");
    assert!(!av.vale_nota);

    let err = lancar_nota_core(
      &db,
      LancarNota {
        avaliacao_id: av.id.clone(),
        valor: 7.0,
      },
    )
    .expect_err("não vale nota");
    assert!(
      err.to_string().contains("não vale nota"),
      "diz o porquê: {err}"
    );

    let total: i64 = db
      .conn()
      .unwrap()
      .query_row("SELECT COUNT(*) FROM nota", [], |r| r.get(0))
      .expect("conta");
    assert_eq!(total, 0, "nada foi escrito");
  }

  #[test]
  fn faculdade_list_avaliacoes_traz_a_nota_atual() {
    let db = Db::open_in_memory().expect("banco");
    let sem = create_semestre_core(&db, semestre_novo("2026-2")).expect("semestre");
    let disc =
      create_disciplina_core(&db, disciplina_nova(&sem.id, "Neuropsicologia")).expect("disciplina");
    let prova =
      create_avaliacao_core(&db, avaliacao_nova(&disc.id, "Prova 1", None)).expect("avaliação");
    let seminario = create_avaliacao_core(&db, avaliacao_nova(&disc.id, "Seminário", Some(false)))
      .expect("avaliação sem nota");

    lancar_nota_core(
      &db,
      LancarNota {
        avaliacao_id: prova.id.clone(),
        valor: 0.0,
      },
    )
    .expect("lança nota zero");

    let lista = list_avaliacoes_core(
      &db,
      DisciplinaRef {
        disciplina_id: disc.id.clone(),
      },
    )
    .expect("lista");
    assert_eq!(lista.len(), 2);

    // Nota `0.0` é nota: o JOIN distingue por `nota.id`, não por valor.
    let da_prova = lista.iter().find(|a| a.id == prova.id).expect("prova");
    let nota = da_prova.nota.as_ref().expect("nota zero conta como nota");
    assert_eq!(nota.valor, 0.0);
    assert_eq!(nota.avaliacao_id, prova.id);

    // Sem nota lançada: `null`, e não um `valor` caído em zero.
    let do_seminario = lista
      .iter()
      .find(|a| a.id == seminario.id)
      .expect("seminário");
    assert!(do_seminario.nota.is_none());
    assert!(!do_seminario.vale_nota);
  }

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
