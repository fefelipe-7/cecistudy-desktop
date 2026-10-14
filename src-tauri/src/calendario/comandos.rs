//! O caminho de escrita do Calendário (spec 01, §3.4; SPEC-D-015 D96).
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
//! A máquina de estados é de [`super::dominio`], e é imposta no caminho de
//! escrita. Até 2026-10-03 ela não era: o cabeçalho dizia "quem valida a transição
//! é o TypeScript, antes de chamar este arquivo", e esse "antes" não existia — não
//! há chamador de `transitionAllowed` em produção, e `concluido → planejado` era
//! gravável de ponta a ponta (débito C5). A ADR-006 (seção "Resolução da
//! pendência 1") decide que os quatro arquivos de `domain/` viram espelho; o
//! dono é `calendario/dominio.rs`.

use rusqlite::{OptionalExtension, Transaction, params};

use serde::{Deserialize, Serialize};

use chrono::{DateTime, TimeZone};
use rrule::{RRuleError, RRuleSet, Tz};

use super::dominio::{destinations_of, is_overdue, mark_overlaps, transition_allowed};
use crate::nucleo::{Db, DbError, new_id, now_ms};

// O `#[path]` mantém o teste em `calendario/testes.rs`, ao lado de
// `comandos.rs`, e ainda assim filho deste módulo — que é o que permite
// `use super::*` alcançar o `X_core` privado. Sem ele, o Rust procuraria
// `comandos/testes.rs` e o layout documentado em D99 (um diretório de módulo
// com `dominio`, `comandos` e `testes`) não existiria.
#[cfg(test)]
#[path = "testes.rs"]
mod testes;

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
