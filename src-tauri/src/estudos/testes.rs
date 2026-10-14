//! Os testes do caminho de escrita de Estudos (SPEC-D-015 D99).
//!
//! O `use super::*` dá acesso ao `X_core` e às structs de entrada, que são do
//! módulo de escrita.

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
  let cartao =
    create_cartao_core(&db, cartao_novo(&conteudo.id, "O que faz o hippocampo?")).expect("cartão");

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
  let cartao =
    create_cartao_core(&db, cartao_novo(&conteudo.id, "O que faz o hippocampo?")).expect("cartão");

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
  let cartao =
    create_cartao_core(&db, cartao_novo(&conteudo.id, "O que faz o hippocampo?")).expect("cartão");

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
