//! Regras puras de domínio do Calendário.
//!
//! Este módulo é o **dono** das quatro regras que até 2026-10-03 existiam só em
//! TypeScript, em `src/features/calendar/domain/`. A ADR-006 (seção "Resolução da
//! pendência 1") decide que os quatro arquivos viram espelho; este é o dono.
//!
//! | Regra | Espelho em `domain/` | Decisão |
//! |---|---|---|
//! | transição de estado | `state-machine.ts` | §4 da spec 01 |
//! | sobreposição | `overlap.ts` | D7 |
//! | atraso derivado | `overdue.ts` | INV-4 |
//! | compromisso e estado aberto | `commitment.ts` | §3 e §4 |
//!
//! Nenhuma regra aqui toca SQL, relógio ou plataforma. `commands.rs` é quem lê e
//! grava; aqui mora a decisão. É essa separação que permite testar a regra sem
//! banco e sem subir o app.
//!
//! O espelho TypeScript existe para a render e para rodar teste sem o toolchain
//! Rust. Quando os dois divergem, **o Rust está certo e o espelho é o bug**
//! (ADR-006, corolário 2). O contrato que prova que eles concordam é
//! `golden/*.jsonl`, lido pelos dois lados.

// ---------------------------------------------------------------------------
// Estados e compromisso
// ---------------------------------------------------------------------------

/// Tabela fechada de transições do estado de um item.
///
/// Espelha `src/features/calendar/domain/state-machine.ts`, e o espelho é o lado
/// errado quando os dois divergem. A lista de `to` é o texto exato do diagrama
/// da spec 01 §4: nada mais é permitido.
///
/// `cancelado` e `dispensado` voltam para `planejado` porque §4 os descreve como
/// alteração externa e decisão revisável, não como fim. E `concluido` só volta
/// para `em_andamento` — que é o caso que o C5 descreve: `concluido → planejado`
/// é ilegal, e antes desta tabela nada impedia a gravação.
pub const TRANSITIONS: &[(&str, &[&str])] = &[
  (
    "planejado",
    &[
      "em_andamento",
      "concluido",
      "adiado",
      "nao_realizado",
      "cancelado",
      "dispensado",
    ],
  ),
  (
    "em_andamento",
    &[
      "concluido",
      "adiado",
      "nao_realizado",
      "cancelado",
      "dispensado",
    ],
  ),
  (
    "adiado",
    &[
      "planejado",
      "em_andamento",
      "concluido",
      "nao_realizado",
      "cancelado",
      "dispensado",
    ],
  ),
  (
    "nao_realizado",
    &[
      "planejado",
      "em_andamento",
      "concluido",
      "adiado",
      "cancelado",
      "dispensado",
    ],
  ),
  ("concluido", &["em_andamento"]),
  ("cancelado", &["planejado"]),
  ("dispensado", &["planejado"]),
];

/// Transição permitida entre dois estados do item.
///
/// Estado desconhecido é **recusado**, e não tratado como "tudo permitido": um
/// estado novo no schema sem linha aqui tem de falhar alto, não passar.
pub fn transition_allowed(from: &str, to: &str) -> bool {
  destinations_of(from).contains(&to)
}

/// Estados a partir dos quais `to` é alcançável, para a mensagem de recusa.
pub fn destinations_of(from: &str) -> &'static [&'static str] {
  TRANSITIONS
    .iter()
    .find(|(estado, _)| *estado == from)
    .map(|(_, destinos)| *destinos)
    .unwrap_or(&[])
}

/// §3 — só `obrigatorio` e `importante` podem virar atraso.
pub fn can_generate_delay(commitment: &str) -> bool {
  matches!(commitment, "obrigatorio" | "importante")
}

/// §3 e §4 — os quatro estados em que a atividade ainda está em aberto.
pub fn is_open_state(state: &str) -> bool {
  matches!(
    state,
    "planejado" | "em_andamento" | "adiado" | "nao_realizado"
  )
}

/// INV-4 — o atraso é **derivado**, nunca persistido (D11).
///
/// Nenhum outro lugar no backend compara `due_at` com o relógio, e não existe
/// coluna de atraso no schema. `cancelado`, `dispensado` e `concluido` não atrasam
/// mesmo com prazo vencido, e `recomendado` ou `opcional` nunca é lido como atraso
/// (§3) — as duas condições são o que impede `recomendado` de virar obrigação.
pub fn is_overdue(commitment: &str, state: &str, due_at: Option<i64>, now: i64) -> bool {
  // Prazo ausente não atrasa, e prazo no futuro não atrasa.
  let Some(due_at) = due_at else {
    return false;
  };
  if due_at >= now {
    return false;
  }
  can_generate_delay(commitment) && is_open_state(state)
}

// ---------------------------------------------------------------------------
// D7 · Sobreposição
// ---------------------------------------------------------------------------

/// D7 — sobreposição é derivada, sinalizada e **nunca bloqueante**.
///
/// Recebe os intervalos **já ordenados por início** e devolve, para cada item,
/// se ele participa de algum grupo de sobreposição. Item isolado recebe `false`.
///
/// **Não** há `overlaps_pair` aqui, e a ausência é deliberada: o espelho
/// `overlap.ts` exporta o atalho booleano porque a UI tem dois itens em mãos e
/// quer a resposta de um par. O backend não tem esse caso — ele lê uma janela
/// inteira e responde sobre todos os itens de uma vez. Deixar a função sem
/// chamador seria uma regra escrita e não conectada, que é a classe de dívida
/// que a ADR-006 fecha.
///
/// Intervalos são **semiabertos** `[início, fim)`: o que termina às 10:00 e o
/// que começa às 10:00 não se sobrepõem, e é por isso que o corte de grupo usa
/// `>=` e não `>`.
///
/// A varredura carrega o fim mais tarde do grupo corrente, o que junta uma
/// cadeia a-b-c num grupo só em tempo linear. É por isso que a assinatura é
/// "já ordenado": ordenar aqui esconderia o custo, e ordenar é uma escolha de
/// quem chama, não da regra. O espelho `overlap.ts` ordena dentro; aqui o
/// chamador ordena, e o golden prova que o resultado é o mesmo.
pub fn mark_overlaps(ordenado: &[(i64, i64)]) -> Vec<bool> {
  let n = ordenado.len();
  let mut marcas = vec![false; n];
  let mut inicio = 0usize;
  let mut alcance: Option<i64> = None;

  for (i, &(comeca, termina)) in ordenado.iter().enumerate() {
    // Tocar não é sobrepor: um item que começa no alcance fecha o grupo.
    if alcance.is_some_and(|reach| comeca >= reach) {
      if i - inicio > 1 {
        for marca in marcas.iter_mut().take(i).skip(inicio) {
          *marca = true;
        }
      }
      inicio = i;
      alcance = Some(termina);
      continue;
    }
    alcance = Some(alcance.map_or(termina, |r| r.max(termina)));
  }

  // Fecha o último grupo, que nunca passou pelo corpo do laço.
  if n > inicio + 1 {
    for marca in marcas.iter_mut().take(n).skip(inicio) {
      *marca = true;
    }
  }
  marcas
}

#[cfg(test)]
mod tests {
  use super::*;

  fn h(hora: i64) -> i64 {
    hora * 3_600_000
  }

  // -----------------------------------------------------------------------
  // Transições
  // -----------------------------------------------------------------------

  #[test]
  fn tabela_bate_com_a_spec() {
    assert!(transition_allowed("planejado", "concluido"));
    assert!(transition_allowed("concluido", "em_andamento"));
    assert!(transition_allowed("cancelado", "planejado"));
    assert!(transition_allowed("dispensado", "planejado"));

    assert!(!transition_allowed("concluido", "planejado"));
    assert!(!transition_allowed("concluido", "adiado"));
    assert!(!transition_allowed("em_andamento", "planejado"));

    // Estado desconhecido recusa, em vez de passar como "tudo permitido".
    assert!(!transition_allowed("inexistente", "planejado"));
    assert!(!transition_allowed("planejado", "inexistente"));
  }

  // -----------------------------------------------------------------------
  // D7 · Sobreposição — as mesmas expectativas do espelho TS
  // -----------------------------------------------------------------------

  #[test]
  fn que_se_tocam_nao_conflitam() {
    // Dois intervalos que se tocam: marcar por `mark_overlaps` é o mesmo que
    // perguntar pelo par, e é a API que a produção usa.
    assert_eq!(
      mark_overlaps(&[(h(8), h(10)), (h(10), h(12))]),
      [false, false]
    );
    assert_eq!(mark_overlaps(&[(h(8), h(10)), (h(9), h(11))]), [true, true]);
  }

  #[test]
  fn duracao_zero_nao_se_sobrepoe_a_si_mesmo() {
    assert_eq!(mark_overlaps(&[(h(8), h(8))]), [false]);
  }

  #[test]
  fn par_sobreposto_marca_os_dois() {
    let m = mark_overlaps(&[(h(8), h(10)), (h(9), h(11))]);
    assert_eq!(m, vec![true, true]);
  }

  #[test]
  fn item_isolado_nao_marca() {
    let m = mark_overlaps(&[(h(8), h(9)), (h(10), h(11)), (h(12), h(13))]);
    assert_eq!(m, vec![false, false, false]);
  }

  #[test]
  fn cadeia_a_b_c_vira_um_grupo_so() {
    let m = mark_overlaps(&[(h(8), h(10)), (h(9), h(11)), (h(10), h(12))]);
    assert_eq!(m, vec![true, true, true]);
  }

  #[test]
  fn dois_grupos_separados_marcam_so_os_deletes() {
    let m = mark_overlaps(&[(h(8), h(10)), (h(9), h(11)), (h(14), h(16)), (h(15), h(17))]);
    assert_eq!(m, vec![true, true, true, true]);
  }

  #[test]
  fn grupo_que_comeca_por_toque_nao_vaza_para_o_seguinte() {
    // a-b se sobrepõem, c-d apenas se tocam: o grupo tem de fechar em c.
    let m = mark_overlaps(&[(h(8), h(10)), (h(9), h(11)), (h(11), h(12)), (h(12), h(13))]);
    assert_eq!(m, vec![true, true, false, false]);
  }

  #[test]
  fn lista_vazia_e_lista_de_um_so() {
    assert_eq!(mark_overlaps(&[]), Vec::<bool>::new());
    assert_eq!(mark_overlaps(&[(h(8), h(10))]), vec![false]);
  }

  #[test]
  fn item_contido_em_outro_conta_como_sobreposicao() {
    let m = mark_overlaps(&[(h(8), h(12)), (h(9), h(10))]);
    assert_eq!(m, vec![true, true]);
  }

  // -----------------------------------------------------------------------
  // INV-4 · Atraso derivado
  // -----------------------------------------------------------------------

  #[test]
  fn atraso_exige_prazo_vencido_compromisso_e_estado_aberto() {
    let agora = h(12);
    let vencido = Some(h(11));

    assert!(is_overdue("obrigatorio", "planejado", vencido, agora));
    assert!(is_overdue("importante", "em_andamento", vencido, agora));
    assert!(is_overdue("obrigatorio", "adiado", vencido, agora));
    assert!(is_overdue("importante", "nao_realizado", vencido, agora));

    // §3: `recomendado` e `opcional` nunca viram obrigação, logo nunca atrasam.
    assert!(!is_overdue("recomendado", "planejado", vencido, agora));
    assert!(!is_overdue("opcional", "planejado", vencido, agora));

    // Estado fechado não atrasa, mesmo com prazo vencido.
    assert!(!is_overdue("obrigatorio", "concluido", vencido, agora));
    assert!(!is_overdue("obrigatorio", "cancelado", vencido, agora));
    assert!(!is_overdue("obrigatorio", "dispensado", vencido, agora));

    // Sem prazo, ou prazo no futuro, não atrasa.
    assert!(!is_overdue("obrigatorio", "planejado", None, agora));
    assert!(!is_overdue("obrigatorio", "planejado", Some(h(13)), agora));
    assert!(!is_overdue("obrigatorio", "planejado", Some(agora), agora));
  }

  // -----------------------------------------------------------------------
  // D61 · O golden é o contrato
  //
  // O espelho TypeScript (`state-machine.golden.test.ts` e
  // `overlap-overdue.golden.test.ts`) lê os **mesmos** arquivos. Divergência
  // entre os dois lados é falha, e a falha nomeia o caso.
  // -----------------------------------------------------------------------

  fn caminho_golden(nome: &str) -> String {
    // `CARGO_MANIFEST_DIR` é `src-tauri/`, e o golden mora na raiz do app.
    format!("{}/../golden/{nome}", env!("CARGO_MANIFEST_DIR"))
  }

  /// Linhas de dados do golden: descarta vazio e comentário.
  fn linhas_do_golden(nome: &str) -> Vec<String> {
    std::fs::read_to_string(caminho_golden(nome))
      .unwrap_or_else(|e| panic!("golden/{nome} não pode ser lido: {e}"))
      .lines()
      .map(str::trim)
      .filter(|l| !l.is_empty() && !l.starts_with('#'))
      .map(str::to_string)
      .collect()
  }

  #[test]
  fn golden_da_transicao_bate_com_a_regra_em_rust() {
    let linhas = linhas_do_golden("state-machine.jsonl");
    let mut total = 0;
    for linha in &linhas {
      let caso: serde_json::Value = serde_json::from_str(linha).expect("caso é JSON");
      total += 1;
      let de = caso["entrada"]["from"].as_str().expect("entrada.from");
      let para = caso["entrada"]["to"].as_str().expect("entrada.to");
      let esperado = caso["saida"]["aceito"].as_bool().expect("saida.aceito");
      assert_eq!(
        transition_allowed(de, para),
        esperado,
        "caso `{}`: o Rust discorda do golden",
        caso["caso"]
      );
    }
    assert_eq!(
      total, 49,
      "7 estados × 7 destinos — o golden tem de ser completo"
    );
  }

  #[test]
  fn golden_da_sobreposicao_bate_com_a_regra_em_rust() {
    let linhas = linhas_do_golden("overlap.jsonl");
    let mut total = 0;
    for linha in &linhas {
      let caso: serde_json::Value = serde_json::from_str(linha).expect("caso é JSON");
      total += 1;
      let itens = caso["entrada"]["items"].as_array().expect("entrada.items");

      // A regra exige entrada ordenada por início — é o que a consulta da
      // grade garante. O golden traz um caso propositalmente desordenado, e
      // ordená-lo aqui é o que prova que os dois ladosagnosticam a ordem.
      let mut intervalos: Vec<(String, i64, i64)> = itens
        .iter()
        .map(|item| {
          (
            item["id"].as_str().expect("id").to_string(),
            item["inicioHora"].as_i64().expect("inicioHora") * 3_600_000,
            item["fimHora"].as_i64().expect("fimHora") * 3_600_000,
          )
        })
        .collect();
      intervalos.sort_by_key(|(_, ini, fim)| (*ini, *fim));

      let vazio: Vec<(i64, i64)> = intervalos.iter().map(|(_, i, f)| (*i, *f)).collect();
      let marcas = mark_overlaps(&vazio);

      for (indice, (id, _, _)) in intervalos.iter().enumerate() {
        let esperado = caso["saida"]["emConflito"][id]
          .as_bool()
          .unwrap_or_else(|| panic!("caso `{}`: sem saida para `{id}`", caso["caso"]));
        assert_eq!(
          marcas[indice], esperado,
          "caso `{}`, item `{id}`: o Rust discorda do golden",
          caso["caso"]
        );
      }
    }
    assert_eq!(total, 10, "o golden da sobreposição cobre as 10 situações");
  }

  #[test]
  fn golden_do_atraso_bate_com_a_regra_em_rust() {
    let linhas = linhas_do_golden("overdue.jsonl");
    let mut total = 0;
    for linha in &linhas {
      let caso: serde_json::Value = serde_json::from_str(linha).expect("caso é JSON");
      total += 1;
      let compromisso = caso["entrada"]["compromisso"]
        .as_str()
        .expect("compromisso");
      let estado = caso["entrada"]["estado"].as_str().expect("estado");
      let due_at = caso["entrada"]["dueAt"].as_i64();
      let agora = caso["entrada"]["agoraHoras"].as_i64().expect("agoraHoras") * 3_600_000;
      let esperado = caso["saida"]["atrasado"].as_bool().expect("saida.atrasado");
      assert_eq!(
        is_overdue(compromisso, estado, due_at, agora),
        esperado,
        "caso `{}`: o Rust discorda do golden",
        caso["caso"]
      );
    }
    assert_eq!(
      total, 84,
      "4 compromissos × 7 estados × 3 prazos — o golden tem de ser completo"
    );
  }
}
