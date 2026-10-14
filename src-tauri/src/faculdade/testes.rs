//! Os testes do caminho de escrita da Faculdade (SPEC-D-015 D99).
//!
//! O `use super::*` dá a este arquivo acesso ao `X_core` e às structs de
//! entrada, que são do módulo de escrita. Um módulo aninhado enxerga o ancestral
//! por caminho, não pelo nome solto.

use super::*;

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
