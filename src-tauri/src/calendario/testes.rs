//! Os testes do caminho de escrita do Calendário (SPEC-D-015 D99).
//!
//! Ficam em arquivo próprio, e não no corpo de `comandos.rs`, por duas razões
//! que se reforçam: 37% de `commands.rs` era teste, e o `#[cfg(test)]` no meio
//! do arquivo era **o ponto de corte que o gate usava** para não contar escrita
//! de teste como caminho de escrita — a fragilidade medida em F8b2.
//!
//! O `use super::*` é o que dá a este arquivo acesso ao `X_core`, que é
//! privado: um módulo aninhado enxerga o ancestral por caminho, não pelo nome
//! solto. Tentar remover esse glob dá `E0425 cannot find type` em 94 lugares —
//! foi o que aconteceu em F2, e é a razão de ele estar aqui e documentado.

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
