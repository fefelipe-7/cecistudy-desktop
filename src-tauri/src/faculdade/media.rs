//! Motor de notas (D74): cálculo puro, média **nunca** persistida.
//!
//! As entradas são as notas lançadas, o tipo de avaliação, o peso, a regra
//! efetiva (de [`super::regra`]) e a frequência. As saídas — média, situação e
//! "quanto falta" — **não** são gravadas em nenhuma tabela (I1) e são derivadas
//! na leitura, como `domain/overdue.ts` já faz com o atraso.
//!
//! Não há relógio aqui: o motor é determinístico e não depende de `now`. O
//! `Clock` injetado que o §5 pede mora no chamador; nenhuma das saídas desta
//! função muda com o tempo.
//!
//! Fronteira (I4): `arredondamento` decide a fronteira. Com média crua 6,95 e
//! `nota_minima` 7,0, o passo 0,1 aprova (7,0) e o passo 0,01 não (6,95).

use super::regra::{ModoExame, OrigemRegra, Regra, RegraAplicada, TipoMedia};

/// Nota máxima do produto. Não é configurável na v1.
pub const NOTA_MAXIMA: f64 = 10.0;

/// Tolerância de comparação de `f64`. Sem ela, `7.0 >= 7.0` ainda passa, mas
/// somas de `f64` produzem 6,999999999 e reprovariam uma média aprovada.
const EPS: f64 = 1e-9;

/// Uma avaliação no cálculo. Só `vale_nota` entra na média (§4.3 **[D]**).
#[derive(Debug, Clone, PartialEq)]
pub struct Avaliacao {
  pub peso: f64,
  pub vale_nota: bool,
  /// `None` é avaliação pendente, ainda sem nota lançada.
  pub nota: Option<f64>,
}

/// Frequência da disciplina. As faltas são dado; o limite é derivado (I5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frequencia {
  pub total_aulas: u32,
  pub faltas: u32,
}

/// Entrada do motor.
#[derive(Debug, Clone, PartialEq)]
pub struct EntradaMedia {
  pub avaliacoes: Vec<Avaliacao>,
  pub frequencia: Option<Frequencia>,
}

/// A situação como **enum** (I15): o filtro é o enum, a string é apresentação.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Situacao {
  /// Nenhuma nota lançada. Não é "não alcança": sem nota não se reprova.
  SemNotas,
  /// Média alcançou a nota mínima.
  Aprovada,
  /// Ainda há avaliação pendente e dá para alcançar.
  PrecisaTirar,
  /// Média abaixo da mínima, mas com direito a exame.
  SoComExame,
  /// Não alcança: reprovado por frequência, sem pendente, ou exame impossível.
  NaoAlcanca,
}

impl Situacao {
  /// A string do contrato (§5). Apresentação, nunca filtro.
  pub fn as_str(self) -> &'static str {
    match self {
      Situacao::SemNotas => "sem-notas",
      Situacao::Aprovada => "aprovada",
      Situacao::PrecisaTirar => "precisa-tirar",
      Situacao::SoComExame => "so-com-exame",
      Situacao::NaoAlcanca => "nao-alcanca",
    }
  }
}

/// Saída do motor, espelhando o contrato §5.
#[derive(Debug, Clone, PartialEq)]
pub struct ResultadoMedia {
  /// Média já arredondada conforme a regra. `None` quando não há nota.
  pub media: Option<f64>,
  /// Média sem arredondar, base do simulador "e se".
  pub media_sem_arredondamento: Option<f64>,
  pub situacao: Situacao,
  /// Nota que falta: na próxima avaliação, ou no exame. `None` fora de
  /// `precisa-tirar` e `so-com-exame`.
  pub nota_necessaria: Option<f64>,
  /// Faltas ainda possíveis antes de reprovar por frequência.
  pub frequencia_restante: Option<i64>,
  pub regra_aplicada: OrigemRegra,
}

/// Arredonda para o passo. `f64::round` arredonda meio para longe do zero.
fn arredondar(valor: f64, passo: f64) -> f64 {
  if passo <= 0.0 {
    return valor;
  }
  (valor / passo).round() * passo
}

/// Arredonda **para cima** no passo. É o certo para a nota que falta: prometer
/// 6,8 quando 6,83 era o necessário é prometer uma nota que reprova.
fn teto(valor: f64, passo: f64) -> f64 {
  if passo <= 0.0 {
    return valor;
  }
  (valor / passo).ceil() * passo
}

/// O peso que a avaliação tem no cálculo. Em média simples o peso é 1, e o peso
/// da avaliação é ignorado (I3).
fn peso_efetivo(regra: &Regra, avaliacao: &Avaliacao) -> f64 {
  match regra.tipo_media {
    TipoMedia::Simples => 1.0,
    TipoMedia::Ponderada => avaliacao.peso,
  }
}

/// Calcula média, situação e frequência restante. Puro e determinístico (I2).
pub fn calcular(entrada: &EntradaMedia, aplicada: &RegraAplicada) -> ResultadoMedia {
  let regra = &aplicada.regra;

  let mut soma_notas = 0.0;
  let mut soma_pesos = 0.0;
  let mut pendentes: Vec<&Avaliacao> = Vec::new();

  for avaliacao in entrada.avaliacoes.iter().filter(|a| a.vale_nota) {
    let peso = peso_efetivo(regra, avaliacao);
    match avaliacao.nota {
      Some(nota) => {
        soma_notas += nota * peso;
        soma_pesos += peso;
      }
      None => pendentes.push(avaliacao),
    }
  }

  let media_crua = if soma_pesos > 0.0 {
    Some(soma_notas / soma_pesos)
  } else {
    None
  };

  let (frequencia_restante, reprovado_frequencia) = match entrada.frequencia {
    Some(f) => {
      let limite = ((f.total_aulas as f64) * (1.0 - regra.frequencia_minima)).floor();
      let limite = limite.max(0.0);
      let restante = limite as i64 - f.faltas as i64;
      (Some(restante.max(0)), f.faltas as i64 > limite as i64)
    }
    None => (None, false),
  };

  let Some(crua) = media_crua else {
    return ResultadoMedia {
      media: None,
      media_sem_arredondamento: None,
      situacao: Situacao::SemNotas,
      nota_necessaria: None,
      frequencia_restante,
      regra_aplicada: aplicada.origem,
    };
  };

  let media = arredondar(crua, regra.arredondamento);

  let (situacao, nota_necessaria) = if reprovado_frequencia {
    (Situacao::NaoAlcanca, None)
  } else if media + EPS >= regra.nota_minima {
    (Situacao::Aprovada, None)
  } else {
    match regra.exame_modo {
      ModoExame::Nenhum => {
        if let Some(proxima) = pendentes.first() {
          let peso = peso_efetivo(regra, proxima);
          if peso <= 0.0 {
            // Avaliação de peso zero não move a média; não há como alcançar.
            (Situacao::NaoAlcanca, None)
          } else {
            let x = (regra.nota_minima * (soma_pesos + peso) - soma_notas) / peso;
            if x > NOTA_MAXIMA + EPS {
              (Situacao::NaoAlcanca, None)
            } else {
              (
                Situacao::PrecisaTirar,
                Some(teto(x.max(0.0), regra.arredondamento)),
              )
            }
          }
        } else {
          (Situacao::NaoAlcanca, None)
        }
      }
      modo => {
        if media + EPS < regra.exame_elegivel_minima {
          (Situacao::NaoAlcanca, None)
        } else {
          let precisa = match modo {
            ModoExame::MediaComExame => regra.exame_minima * 2.0 - crua,
            ModoExame::ExameSubstitui => regra.exame_minima,
            ModoExame::Nenhum => unreachable!("tratado no braço acima"),
          };
          if precisa > NOTA_MAXIMA + EPS {
            (Situacao::NaoAlcanca, None)
          } else {
            (
              Situacao::SoComExame,
              Some(teto(precisa.max(0.0), regra.arredondamento)),
            )
          }
        }
      }
    }
  };

  ResultadoMedia {
    media: Some(media),
    media_sem_arredondamento: Some(crua),
    situacao,
    nota_necessaria,
    frequencia_restante,
    regra_aplicada: aplicada.origem,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::faculdade::regra::Regra;

  fn av(nota: Option<f64>, peso: f64) -> Avaliacao {
    Avaliacao {
      peso,
      vale_nota: true,
      nota,
    }
  }

  fn aplicada(tipo: TipoMedia, nota_minima: f64, arredondamento: f64) -> RegraAplicada {
    RegraAplicada {
      regra: Regra {
        tipo_media: tipo,
        nota_minima,
        arredondamento,
        frequencia_minima: 0.75,
        exame_modo: ModoExame::Nenhum,
        exame_minima: 6.0,
        exame_elegivel_minima: 4.0,
      },
      origem: OrigemRegra::Padrao,
    }
  }

  fn com_exame(
    mut a: RegraAplicada,
    modo: ModoExame,
    exame_minima: f64,
    elegivel: f64,
  ) -> RegraAplicada {
    a.regra.exame_modo = modo;
    a.regra.exame_minima = exame_minima;
    a.regra.exame_elegivel_minima = elegivel;
    a
  }

  fn entrada(avaliacoes: Vec<Avaliacao>) -> EntradaMedia {
    EntradaMedia {
      avaliacoes,
      frequencia: None,
    }
  }

  #[test]
  fn simples_ignora_o_peso_e_ponderada_usa() {
    let avs = vec![av(Some(8.0), 1.0), av(Some(6.0), 9.0)];

    let simples = calcular(
      &entrada(avs.clone()),
      &aplicada(TipoMedia::Simples, 5.0, 0.1),
    );
    assert_eq!(simples.media, Some(7.0));
    assert_eq!(simples.situacao, Situacao::Aprovada);

    let ponderada = calcular(&entrada(avs), &aplicada(TipoMedia::Ponderada, 5.0, 0.1));
    assert_eq!(ponderada.media, Some(6.2));
  }

  #[test]
  fn arredondamento_afeta_a_fronteira_de_6_95() {
    // 6.0 e 7.9 → crua 6.95.
    let avs = vec![av(Some(6.0), 1.0), av(Some(7.9), 1.0)];

    let passo_1 = calcular(
      &entrada(avs.clone()),
      &aplicada(TipoMedia::Simples, 7.0, 0.1),
    );
    assert_eq!(passo_1.media_sem_arredondamento, Some(6.95));
    assert_eq!(passo_1.media, Some(7.0));
    assert_eq!(passo_1.situacao, Situacao::Aprovada);

    let passo_001 = calcular(&entrada(avs), &aplicada(TipoMedia::Simples, 7.0, 0.01));
    assert_eq!(passo_001.media, Some(6.95));
    assert_eq!(
      passo_001.situacao,
      Situacao::NaoAlcanca,
      "sem pendente e sem exame, 6,95 não alcança 7,0"
    );
  }

  #[test]
  fn nenhuma_nota_lancada_e_sem_notas() {
    let avs = vec![av(None, 1.0), av(None, 2.0)];
    let r = calcular(&entrada(avs), &aplicada(TipoMedia::Ponderada, 6.0, 0.1));
    assert_eq!(r.situacao, Situacao::SemNotas);
    assert_eq!(r.media, None);
    assert_eq!(r.media_sem_arredondamento, None);
  }

  #[test]
  fn precisa_tirar_calcula_a_nota_da_proxima() {
    // 4,0 lançada + próxima: (4 + x) / 2 >= 6 → x >= 8.
    let avs = vec![av(Some(4.0), 1.0), av(None, 1.0)];
    let r = calcular(&entrada(avs), &aplicada(TipoMedia::Simples, 6.0, 0.1));
    assert_eq!(r.situacao, Situacao::PrecisaTirar);
    assert_eq!(r.nota_necessaria, Some(8.0));
  }

  #[test]
  fn precisa_tirar_com_media_ponderada() {
    // S = 4*3 = 12, W = 3; próxima peso 2: (12 + 2x)/5 >= 6 → x = 9.
    let avs = vec![av(Some(4.0), 3.0), av(None, 2.0)];
    let r = calcular(&entrada(avs), &aplicada(TipoMedia::Ponderada, 6.0, 0.1));
    assert_eq!(r.situacao, Situacao::PrecisaTirar);
    assert_eq!(r.nota_necessaria, Some(9.0));
  }

  #[test]
  fn sem_pendente_e_abaixo_nao_alcanca() {
    let avs = vec![av(Some(3.0), 1.0)];
    let r = calcular(&entrada(avs), &aplicada(TipoMedia::Simples, 6.0, 0.1));
    assert_eq!(r.situacao, Situacao::NaoAlcanca);
    assert_eq!(r.nota_necessaria, None);
  }

  #[test]
  fn exame_media_com_exame_gera_so_com_exame() {
    let avs = vec![av(Some(5.0), 1.0)];
    let regra = com_exame(
      aplicada(TipoMedia::Simples, 6.0, 0.1),
      ModoExame::MediaComExame,
      6.0,
      4.0,
    );
    let r = calcular(&entrada(avs), &regra);
    assert_eq!(r.situacao, Situacao::SoComExame);
    assert_eq!(r.nota_necessaria, Some(7.0)); // 2*6 - 5
  }

  #[test]
  fn exame_substitui_a_media() {
    let avs = vec![av(Some(5.0), 1.0)];
    let regra = com_exame(
      aplicada(TipoMedia::Simples, 6.0, 0.1),
      ModoExame::ExameSubstitui,
      6.0,
      4.0,
    );
    let r = calcular(&entrada(avs), &regra);
    assert_eq!(r.situacao, Situacao::SoComExame);
    assert_eq!(r.nota_necessaria, Some(6.0));
  }

  #[test]
  fn abaixo_da_elegibilidade_nao_alcanca_mesmo_com_exame() {
    let avs = vec![av(Some(3.0), 1.0)];
    let regra = com_exame(
      aplicada(TipoMedia::Simples, 6.0, 0.1),
      ModoExame::MediaComExame,
      6.0,
      4.0,
    );
    let r = calcular(&entrada(avs), &regra);
    assert_eq!(r.situacao, Situacao::NaoAlcanca);
  }

  #[test]
  fn exame_impossivel_nao_alcanca() {
    // Crua 1,0; elegível 1,0 (tem direito); precisa = 2*6 - 1 = 11 > 10.
    let avs = vec![av(Some(1.0), 1.0)];
    let regra = com_exame(
      aplicada(TipoMedia::Simples, 6.0, 0.1),
      ModoExame::MediaComExame,
      6.0,
      1.0,
    );
    let r = calcular(&entrada(avs), &regra);
    assert_eq!(r.situacao, Situacao::NaoAlcanca);
  }

  #[test]
  fn frequencia_reprova_e_a_restante_e_derivada() {
    let avs = vec![av(Some(8.0), 1.0)];
    let regra = aplicada(TipoMedia::Simples, 6.0, 0.1);

    // 40 aulas, 25% de faltas → limite 10.
    let ok = calcular(
      &EntradaMedia {
        avaliacoes: avs.clone(),
        frequencia: Some(Frequencia {
          total_aulas: 40,
          faltas: 6,
        }),
      },
      &regra,
    );
    assert_eq!(ok.situacao, Situacao::Aprovada);
    assert_eq!(ok.frequencia_restante, Some(4));

    let reprovado = calcular(
      &EntradaMedia {
        avaliacoes: avs,
        frequencia: Some(Frequencia {
          total_aulas: 40,
          faltas: 11,
        }),
      },
      &regra,
    );
    assert_eq!(reprovado.situacao, Situacao::NaoAlcanca);
    assert_eq!(reprovado.frequencia_restante, Some(0));
  }

  #[test]
  fn avaliacao_que_nao_vale_nota_fica_fora_da_media() {
    let avs = vec![
      av(Some(8.0), 1.0),
      Avaliacao {
        peso: 100.0,
        vale_nota: false,
        nota: Some(0.0),
      },
    ];
    let r = calcular(&entrada(avs), &aplicada(TipoMedia::Ponderada, 6.0, 0.1));
    assert_eq!(r.media, Some(8.0));
  }

  #[test]
  fn situacao_string_do_contrato() {
    assert_eq!(Situacao::SemNotas.as_str(), "sem-notas");
    assert_eq!(Situacao::Aprovada.as_str(), "aprovada");
    assert_eq!(Situacao::PrecisaTirar.as_str(), "precisa-tirar");
    assert_eq!(Situacao::SoComExame.as_str(), "so-com-exame");
    assert_eq!(Situacao::NaoAlcanca.as_str(), "nao-alcanca");
  }

  // ---------------------------------------------------------------------
  // D74 · O golden é o contrato (golden/media.jsonl)
  //
  // Diferente do golden de recorrência, aqui há UMA implementação só (D74
  // rejeitou o recalculo no TypeScript). O papel deste golden é pinar o
  // **valor**: qualquer mudança de fronteira — arredondamento, elegibilidade
  // do exame, limite de frequência — falha alto, em vez de passar silenciosa.
  // Os valores esperados são aritmética de mão, documentada no próprio
  // arquivo — nunca saída de rodar o motor.
  // ---------------------------------------------------------------------

  #[derive(serde::Deserialize)]
  #[serde(rename_all = "camelCase")]
  struct CasoMedia {
    caso: String,
    entrada: EntradaCaso,
    saida: SaidaCaso,
  }

  #[derive(serde::Deserialize)]
  #[serde(rename_all = "camelCase")]
  struct EntradaCaso {
    origem: String,
    semestre_id: String,
    regra: RegraCaso,
    avaliacoes: Vec<AvaliacaoCaso>,
    frequencia: Option<FrequenciaCaso>,
  }

  #[derive(serde::Deserialize)]
  #[serde(rename_all = "camelCase")]
  struct RegraCaso {
    tipo_de_calculo: String,
    nota_minima: f64,
    arredondamento: f64,
    frequencia_minima: f64,
    exame_modo: String,
    exame_minima: f64,
    exame_elegivel_minima: f64,
  }

  #[derive(serde::Deserialize)]
  #[serde(rename_all = "camelCase")]
  struct AvaliacaoCaso {
    peso: f64,
    vale_nota: bool,
    nota: Option<f64>,
  }

  #[derive(serde::Deserialize)]
  #[serde(rename_all = "camelCase")]
  struct FrequenciaCaso {
    total_aulas: u32,
    faltas: u32,
  }

  #[derive(serde::Deserialize)]
  #[serde(rename_all = "camelCase")]
  struct SaidaCaso {
    media: Option<f64>,
    media_sem_arredondamento: Option<f64>,
    situacao: String,
    nota_necessaria_proxima: Option<f64>,
    frequencia_restante: Option<i64>,
    regra_aplicada: String,
  }

  fn tipo_de(valor: &str) -> TipoMedia {
    match valor {
      "simples" => TipoMedia::Simples,
      _ => TipoMedia::Ponderada,
    }
  }

  fn modo_de(valor: &str) -> ModoExame {
    match valor {
      "recuperacao" => ModoExame::MediaComExame,
      "substitui" => ModoExame::ExameSubstitui,
      _ => ModoExame::Nenhum,
    }
  }

  fn origem_de(valor: &str) -> OrigemRegra {
    match valor {
      "congelada" => OrigemRegra::Congelada,
      "personalizada" => OrigemRegra::Personalizada,
      _ => OrigemRegra::Padrao,
    }
  }

  /// `f64` não carrega igualdade exata entre o JSON e a conta; a tolerância é a
  /// mesma ordem de grandeza do `EPS` do motor.
  fn quase(a: Option<f64>, b: Option<f64>) -> bool {
    match (a, b) {
      (None, None) => true,
      (Some(a), Some(b)) => (a - b).abs() < 1e-9,
      _ => false,
    }
  }

  #[test]
  fn golden_da_media_bate_com_o_motor() {
    let bruto = std::fs::read_to_string(format!(
      "{}/../golden/media.jsonl",
      env!("CARGO_MANIFEST_DIR")
    ))
    .expect("golden/media.jsonl existe");
    let mut total = 0;
    for linha in bruto.lines() {
      let t = linha.trim();
      if t.is_empty() || t.starts_with('#') {
        continue;
      }
      total += 1;
      let caso: CasoMedia = serde_json::from_str(t).expect("caso do golden é JSON");

      let aplicada = RegraAplicada {
        regra: Regra {
          tipo_media: tipo_de(&caso.entrada.regra.tipo_de_calculo),
          nota_minima: caso.entrada.regra.nota_minima,
          arredondamento: caso.entrada.regra.arredondamento,
          frequencia_minima: caso.entrada.regra.frequencia_minima,
          exame_modo: modo_de(&caso.entrada.regra.exame_modo),
          exame_minima: caso.entrada.regra.exame_minima,
          exame_elegivel_minima: caso.entrada.regra.exame_elegivel_minima,
        },
        origem: origem_de(&caso.entrada.origem),
      };
      let entrada = EntradaMedia {
        avaliacoes: caso
          .entrada
          .avaliacoes
          .iter()
          .map(|a| Avaliacao {
            peso: a.peso,
            vale_nota: a.vale_nota,
            nota: a.nota,
          })
          .collect(),
        frequencia: caso.entrada.frequencia.map(|f| Frequencia {
          total_aulas: f.total_aulas,
          faltas: f.faltas,
        }),
      };

      let r = calcular(&entrada, &aplicada);
      assert_eq!(
        r.situacao.as_str(),
        caso.saida.situacao,
        "caso `{}`: situação diverge",
        caso.caso
      );
      assert!(
        quase(r.media, caso.saida.media),
        "caso `{}`: media {:?} != {:?}",
        caso.caso,
        r.media,
        caso.saida.media
      );
      assert!(
        quase(
          r.media_sem_arredondamento,
          caso.saida.media_sem_arredondamento
        ),
        "caso `{}`: média sem arredondamento diverge",
        caso.caso
      );
      assert!(
        quase(r.nota_necessaria, caso.saida.nota_necessaria_proxima),
        "caso `{}`: nota necessária diverge",
        caso.caso
      );
      assert_eq!(
        r.frequencia_restante, caso.saida.frequencia_restante,
        "caso `{}`: frequência restante diverge",
        caso.caso
      );
      assert_eq!(
        aplicada.rotulo(&caso.entrada.semestre_id),
        caso.saida.regra_aplicada,
        "caso `{}`: rótulo da regra diverge",
        caso.caso
      );
    }
    // SPEC-D-010 §9: >= 5 casos, incluindo o limite de arredondamento e a
    // regra congelada. Os dois casos obrigatórios existem no arquivo; a
    // contagem garante que ninguém os apaga em silêncio.
    assert!(total >= 5, "SPEC-D-010 §9 exige >= 5 casos; há {total}");
    assert!(
      bruto.contains("limite-695") && bruto.contains("congelada-"),
      "os casos obrigatórios (6,95 e congelada) têm de existir"
    );
  }
}
