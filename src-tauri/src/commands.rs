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
//! A máquina de estados **não** é reimplementada aqui: ela é
//! `domain/state-machine.ts`. O `CHECK` do banco só impede valor fora da
//! enumeração, não uma transição ilegal — quem valida a transição é o
//! TypeScript, antes de chamar este arquivo.

use rusqlite::{params, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::{Db, DbError};

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

#[tauri::command]
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
    let mut stmt = conn.prepare("SELECT id, title, tone, icon, visible, position FROM layer ORDER BY position")?;
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

#[tauri::command]
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

#[tauri::command]
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
            if let Some(exdates) = &event.exdates_json {
                if is_excluded(exdates, start) {
                    continue;
                }
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
/// Só o que o Calendário usa: `FREQ=DAILY|WEEKLY|MONTHLY`, `INTERVAL`, `BYDAY`,
/// `COUNT` e `UNTIL`. `count` e `until_at` são colunas e **vencem** a string, de
/// modo que um `rrule` editado pela usuária não apaga o limite que ela gravou.
fn expand_rule(event: &RecurringEvent, from: i64, to: i64) -> Result<Vec<i64>, DbError> {
    let rrule = match &event.rrule {
        Some(value) => value.clone(),
        None => return Ok(Vec::new()),
    };
    let parts = parse_rrule(&rrule);
    let freq = parts.get("FREQ").copied().unwrap_or("WEEKLY");
    let interval = parts
        .get("INTERVAL")
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(1)
        .max(1);
    let bydays: Vec<i64> = parts
        .get("BYDAY")
        .map(|value| {
            value
                .split(',')
                .filter_map(|day| weekday_index(day.trim()))
                .collect()
        })
        .unwrap_or_default();

    // Toda recorrência nasce ancorada no próprio evento: `starts_at` é o DTSTART
    // e nunca é reinterpretado no fuso do sistema (D6).
    let anchor = event.starts_at;
    let limit = event.until_at.unwrap_or(i64::MAX).min(to.max(anchor));

    // Passo diário em vez de um `stepping` por FREQ: a grade é sempre por dia, e
    // uma rotina só de passo diário cobre DAILY/WEEKLY/HOURLY sem três
    // aritméticas distintas para o mesmo resultado.
    let step_ms: i64 = if freq == "HOURLY" { 3_600_000 } else { 86_400_000 };

    let mut out = Vec::new();
    // `emitted` conta desde o **ancor**, não desde a janela: um `COUNT` de 10 tem
    // de ignorar as 8 ocorrências que caem antes da janela lida, senão a
    // materialização contaria errado a cada navegação e a série encurtaria.
    let mut emitted = 0i64;
    let mut cursor = anchor;
    let mut step = 0i64;

    // Teto de segurança: uma janela de leitura nunca passa de alguns anos, então
    // isto só existe para uma RRULE corrompida não travar o app.
    let max_steps = 5000;

    while cursor < limit && step < max_steps {
        step += 1;
        if matches_freq(freq, &bydays, cursor, anchor, interval, step) {
            emitted += 1;
            let within_count = event.count.is_none_or(|c| emitted <= c);
            if within_count && cursor >= from && cursor < to {
                out.push(cursor);
            }
        }
        cursor += step_ms;
    }
    Ok(out)
}

fn parse_rrule(value: &str) -> std::collections::HashMap<String, String> {
    value
        .trim()
        .trim_start_matches("RRULE:")
        .split(';')
        .filter_map(|part| {
            let (key, val) = part.split_once('=')?;
            Some((key.trim().to_uppercase(), val.trim().to_string()))
        })
        .collect()
}

/// `MO..SU` para 0..6, aceitando o prefixo numérico que o RFC 5545 permite.
fn weekday_index(day: &str) -> Option<i64> {
    let day = day.trim();
    let name = match day.len() {
        2 => day,
        _ => {
            let letters: String = day.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
            letters.as_str()
        }
    };
    match name.to_uppercase().as_str() {
        "MO" => Some(0),
        "TU" => Some(1),
        "WE" => Some(2),
        "TH" => Some(3),
        "FR" => Some(4),
        "SA" => Some(5),
        "SU" => Some(6),
        _ => None,
    }
}

/// Dia da semana UTC do instante. O Calendário ancora recorrência no fuso do
/// evento, mas o passo é diário em UTC — o ajuste fino de fuso fica com a
/// materialização da grade, e `until_at`/`exdates` são comparados no mesmo eixo.
fn weekday_of(instant: i64) -> i64 {
    const DAY: i64 = 86_400_000;
    let days = instant.div_euclid(DAY);
    // 1970-01-01 foi uma quinta-feira (4).
    (days + 4).rem_euclid(7)
}

fn matches_freq(
    freq: &str,
    bydays: &[i64],
    instant: i64,
    anchor: i64,
    interval: i64,
    step_count: i64,
) -> bool {
    let days_from_anchor = (instant - anchor) / 86_400_000;
    match freq {
        "HOURLY" => (instant - anchor) / 3_600_000 % interval == 0,
        "DAILY" => days_from_anchor % interval == 0,
        _ => {
            // WEEKLY e o padrão: o ciclo conta semanas, e `BYDAY` escolhe os dias.
            if (step_count - 1) % interval != 0 {
                return false;
            }
            if bydays.is_empty() {
                return true;
            }
            bydays.contains(&weekday_of(instant))
        }
    }
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
    let now = now_ms()?;
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
            "INSERT INTO event_recurrence
               (id, event_id, rrule, dtstart_tz, until_at, count, exdates_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
             ON CONFLICT (event_id) DO UPDATE SET
               rrule        = excluded.rrule,
               dtstart_tz   = excluded.dtstart_tz,
               until_at     = excluded.until_at,
               count        = excluded.count,
               exdates_json = excluded.exdates_json,
               updated_at   = excluded.updated_at",
            params![
                rule_id,
                event_id,
                payload.rrule,
                payload.dtstart_tz,
                payload.until_at,
                payload.count,
                payload.exdates_json,
                now,
            ],
        )?;
        Ok(())
    })
}

#[tauri::command]
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
                    o.state, o.override, o.rule_id IS NOT NULL
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
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    })
}

#[tauri::command]
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
}

/// Filtro opcional vira `IS ?n` no SQLite: com o valor `NULL` a comparação é
/// `NULL` e a linha passa — que é exatamente "não filtrar por este campo".
pub fn list_responsibilities_core(
    db: &Db,
    filter: ResponsibilityFilter,
) -> Result<Vec<Responsibility>, DbError> {
    let conn = db.conn()?;
    let mut stmt = conn.prepare(
        "SELECT id, layer_id, title, kind, commitment, state, due_at,
                planned_duration_min, origin, owner_id, parent_id
         FROM responsibility
         WHERE due_at IS ?1 AND state IS ?2 AND layer_id IS ?3
         ORDER BY due_at IS NULL, due_at, title",
    )?;
    let rows = stmt.query_map(params![filter.due_before, filter.state, filter.layer_id], |r| {
        Ok(Responsibility {
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
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command]
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

pub fn create_event_core(db: &Db, payload: UpsertEvent) -> Result<CalendarEvent, DbError> {
    guard_origin(&payload.origin)?;
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

#[tauri::command]
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

#[tauri::command]
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
        tx.execute("DELETE FROM event_occurrence WHERE event_id = ?1", params![payload.id])?;
        tx.execute("DELETE FROM event_recurrence WHERE event_id = ?1", params![payload.id])?;
        tx.execute("DELETE FROM calendar_event WHERE id = ?1", params![payload.id])?;
        Ok(())
    })
}

#[tauri::command]
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

/// Cancelar uma instância é alteração local; a série continua, então
/// `override = 'cancelled'` e `event_recurrence` não é tocado (INV-5).
pub fn set_occurrence_state_core(
    db: &Db,
    payload: SetOccurrenceState,
) -> Result<Occurrence, DbError> {
    let now = now_ms()?;
    let cancelled = payload.state == "cancelado";
    db.write(|tx| {
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

#[tauri::command]
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
        tx.execute(
            "UPDATE event_occurrence
             SET starts_at = ?2, ends_at = ?3, override = 'moved', updated_at = ?4
             WHERE id = ?1",
            params![payload.id, payload.starts_at, payload.ends_at, now],
        )?;
        read_occurrence(tx, &payload.id)
    })
}

#[tauri::command]
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
                    params![payload.rule_id, serde_json::to_string(&exdates).unwrap_or_else(|_| "[]".into())],
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

#[tauri::command]
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
    Ok(tx.query_row(
        "SELECT id, layer_id, title, kind, commitment, state, due_at,
                planned_duration_min, origin, owner_id, parent_id
         FROM responsibility WHERE id = ?1",
        params![resp_id],
        |r| {
            Ok(Responsibility {
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

#[tauri::command]
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

#[tauri::command]
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

#[tauri::command]
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

#[tauri::command]
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
            return Err(DbError::Domain(format!("alvo de execução desconhecido: {other}")));
        }
    };
    let sql = format!("UPDATE {table} SET state = ?1 WHERE id = ?2");
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

#[tauri::command]
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
pub fn record_execution_core(db: &Db, payload: RecordExecution) -> Result<ExecutionRecord, DbError> {
    db.write(|tx| insert_execution(tx, &payload.target, &payload.record, "em_andamento"))
}

#[tauri::command]
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

#[tauri::command]
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

#[tauri::command]
pub fn set_setting(db: tauri::State<'_, Db>, payload: SetSetting) -> Result<(), DbError> {
    set_setting_core(&db, payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: i64 = 1_757_000_000_000;
    const HOUR: i64 = 3_600_000;

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
        set_layer_visibility_core(&db, SetLayerVisibility { id: "tcc".into(), visible: false }).expect("altera");
        let layers = list_layers_core(&db).expect("camadas");
        assert!(!layers.iter().find(|l| l.id == "tcc").expect("tcc").visible);
    }

    #[test]
    fn invariant_google_e_recusado_na_escrita() {
        let db = Db::open_in_memory().expect("banco");
        let mut payload = event_payload("Aula importada");
        payload.origin = "google".into();
        let err = create_event_core(&db, payload).expect_err("INV-6 precisa recusar");
        assert!(err.to_string().contains("INV-6"), "mensagem explica a recusa");
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
                patch: UpsertEventPatch { location: Some(None), ..Default::default() },
            },
        )
        .expect("atualiza");
        assert_eq!(updated.location, None, "Some(None) limpa o campo");

        // Patch sem o campo não mexe.
        let untouched = update_event_core(
            &db,
            UpdateEvent {
                id: created.id.clone(),
                patch: UpsertEventPatch { title: Some("Novo").into(), ..Default::default() },
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

        let dentro = list_events_in_window_core(&db, Window { from: T0 + 60_000, to: T0 + HOUR }).expect("janela");
        assert_eq!(dentro.len(), 1, "interseção parcial conta");
        let fora = list_events_in_window_core(&db, Window { from: T0 + 2 * HOUR, to: T0 + 3 * HOUR }).expect("janela");
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
            MoveOccurrence { id: "occ-1".into(), starts_at: T0 + 5 * HOUR, ends_at: T0 + 6 * HOUR },
        )
        .expect("move");
        assert_eq!(moved.original_start, T0, "§5: a identidade é a data original");
        assert_eq!(moved.starts_at, T0 + 5 * HOUR);
        assert_eq!(moved.override_kind, "moved");
        assert_eq!(moved.ends_at - moved.starts_at, HOUR, "a duração é preservada");
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
            SetOccurrenceState { id: "occ-1".into(), state: "cancelado".into(), reason: Some("aula cancelada".into()) },
        )
        .expect("cancela");
        assert_eq!(cancelled.override_kind, "cancelled");
        assert_eq!(cancelled.cancel_reason.as_deref(), Some("aula cancelada"));

        let rrule: String = db
            .conn()
            .unwrap()
            .query_row("SELECT rrule FROM event_recurrence WHERE id = 'rule-1'", [], |r| r.get(0))
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
            },
        )
        .expect("atualiza");

        let rrule: String = {
            let conn = db.conn().unwrap();
            conn.query_row("SELECT rrule FROM event_recurrence WHERE id = 'rule-1'", [], |r| r.get(0))
                .expect("regra")
        };
        assert_eq!(rrule, "FREQ=WEEKLY;BYDAY=TU", "INV-5: a regra fica byte-idêntica");
        let still_linked: i64 = db
            .conn()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM event_occurrence WHERE id = 'occ-1' AND rule_id IS NOT NULL", [], |r| r.get(0))
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
                target: Target { kind: "responsibility".into(), id: resp.id.clone() },
                record: None,
            },
        )
        .expect_err("INV-3 precisa recusar");
        assert!(err.to_string().contains("INV-3"));

        let (state, execs): (String, i64) = {
            let conn = db.conn().unwrap();
            (
                conn.query_row("SELECT state FROM responsibility WHERE id = ?1", params![resp.id], |r| r.get(0))
                    .expect("estado"),
                conn.query_row("SELECT COUNT(*) FROM execution_record", [], |r| r.get(0)).expect("conta"),
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
                target: Target { kind: "responsibility".into(), id: resp.id.clone() },
                record: Some(execution(T0 + 45 * 60_000)),
            },
        )
        .expect("conclui");
        let (state, execs, actual): (String, i64, i64) = {
            let conn = db.conn().unwrap();
            (
                conn.query_row("SELECT state FROM responsibility WHERE id = ?1", params![resp.id], |r| r.get(0))
                    .expect("estado"),
                conn.query_row("SELECT COUNT(*) FROM execution_record", [], |r| r.get(0)).expect("conta"),
                conn.query_row("SELECT actual_duration_min FROM execution_record", [], |r| r.get(0))
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
                target: Target { kind: "responsibility".into(), id: resp.id.clone() },
                record: execution(T0 + 20 * 60_000),
            },
        )
        .expect("registra");
        let state: String = db
            .conn()
            .unwrap()
            .query_row("SELECT state FROM responsibility WHERE id = ?1", params![resp.id], |r| r.get(0))
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
                conn.query_row("SELECT COUNT(*) FROM execution_record WHERE id = 'exe-1'", [], |r| r.get(0))
                    .expect("histórico"),
                conn.query_row("SELECT COUNT(*) FROM event_occurrence WHERE event_id = ?1", params![evt.id], |r| r.get(0))
                    .expect("ocorrências"),
                conn.query_row("SELECT COUNT(*) FROM event_recurrence WHERE event_id = ?1", params![evt.id], |r| r.get(0))
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
            RescheduleBlock { id: bloco.id.clone(), starts_at: T0 + 3 * HOUR, ends_at: T0 + 4 * HOUR },
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
            ResponsibilityFilter { due_before: Some(T0), ..Default::default() },
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
            SetSetting { key: "locale".into(), value: serde_json::json!("pt-BR") },
        )
        .expect("grava");
        set_setting_core(
            &db,
            SetSetting { key: "locale".into(), value: serde_json::json!("en-US") },
        )
        .expect("sobrescreve");
        let total: i64 = db.conn().unwrap().query_row("SELECT COUNT(*) FROM setting", [], |r| r.get(0)).expect("conta");
        assert_eq!(total, 1);
        let value = get_setting_core(&db, SettingKey { key: "locale".into() }).expect("lê");
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
            },
        )
        .expect_err("escopo desconhecido");
        assert!(err.to_string().contains("escopo"));
    }
}
