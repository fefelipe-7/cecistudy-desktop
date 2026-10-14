//! O motor de repetição espaçada do Estudos (D76, SPEC-D-010).
//!
//! Fronteira (a mesma de D50, SPEC-D-002): a **crate** `fsrs` decide o número
//! — estabilidade, dificuldade, intervalo; o que é **nosso** é a forma do
//! estado ([`EstadoCartao`]), a regra de relearning, o vencimento por **dia**
//! com âncora de fuso e o relógio injetado. Nenhum parâmetro é inventado
//! aqui: a retenção desejada é parâmetro, e o padrão dela é **[P]** (§4.4).
//!
//! ## Cláusula do vencimento (D76)
//!
//! O estado por cartão é **persistido**; o vencimento é **calculado**, nunca
//! gravado. `dia_de_vencimento` soma o intervalo à data civil da última revisão
//! **na zona da usuária** — a fila é por dia, e o instante exato dentro do dia
//! não decide nada. A zona entra como parâmetro em toda função que a usa: o
//! fuso da máquina nunca é consultado (I11), e trocá-lo no sistema não desloca
//! a fila de ninguém.
//!
//! ## Relearning
//!
//! A crate não tem o conceito — ela responde quatro agendamentos e pronto. O
//! produto pede o marcador, então ele é **dado do cartão**: a última revisão
//! ter sido `again` num cartão que já tinha memória. É persistido junto com o
//! estado, e uma avaliação melhor (`hard` ou acima) sai dele.

use chrono::{Duration, TimeZone, Utc};
use fsrs::{FSRS, MemoryState};
use std::fmt;
use std::str::FromStr;

/// A nota dada na revisão, na ordem e nos valores da crate (1..4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Avaliacao {
  Again = 1,
  Hard = 2,
  Good = 3,
  Easy = 4,
}

impl Avaliacao {
  /// Converte o rating persistido (1..4). Fora da faixa é `None`.
  pub fn de_rating(rating: i64) -> Option<Self> {
    match rating {
      1 => Some(Avaliacao::Again),
      2 => Some(Avaliacao::Hard),
      3 => Some(Avaliacao::Good),
      4 => Some(Avaliacao::Easy),
      _ => None,
    }
  }
}

/// O estado FSRS que o banco persiste por cartão (D76).
///
/// `None` em `estabilidade`/`dificuldade` é o cartão novo, que nunca foi
/// revisado; a crate trata esse caso como primeiro agendamento. O vencimento
/// não mora aqui nem no banco: é [`dia_de_vencimento`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EstadoCartao {
  pub estabilidade: Option<f64>,
  pub dificuldade: Option<f64>,
  /// Instante da última revisão, epoch ms UTC.
  pub ultima_revisao: Option<i64>,
  /// Intervalo do último agendamento, em dias.
  pub intervalo_dias: Option<i64>,
  /// Marcador de relearning — dado, não derivado da estabilidade.
  pub relearning: bool,
}

impl EstadoCartao {
  /// O estado de nascimento: sem memória, sem revisão, sem intervalo.
  pub fn novo() -> Self {
    EstadoCartao {
      estabilidade: None,
      dificuldade: None,
      ultima_revisao: None,
      intervalo_dias: None,
      relearning: false,
    }
  }
}

/// O que uma revisão produz: o estado novo e o intervalo agendado.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResultadoRevisao {
  pub estado: EstadoCartao,
  /// Dias até o próximo vencimento. Mínimo 1: a fila é por dia, e "revisar de
  /// novo hoje" não é agendamento.
  pub intervalo_dias: i64,
}

#[derive(Debug)]
pub enum FsrsError {
  /// A retenção desejada tem de estar em (0, 1).
  RetencaoInvalida(f64),
  /// Erro do motor (crate ou fuso desconhecido). A mensagem é a causa.
  Motor(String),
}

impl fmt::Display for FsrsError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      FsrsError::RetencaoInvalida(r) => write!(f, "retencao desejada fora de (0, 1): {r}"),
      FsrsError::Motor(m) => write!(f, "{m}"),
    }
  }
}

impl std::error::Error for FsrsError {}

/// Agenda a próxima revisão a partir do estado atual. Pura: `agora_ms` é o
/// relógio injetado — nenhum `now()` aqui dentro.
pub fn revisar(
  estado: &EstadoCartao,
  avaliacao: Avaliacao,
  retencao: f64,
  agora_ms: i64,
) -> Result<ResultadoRevisao, FsrsError> {
  // Intervalo aberto dos dois lados: retenção 0 ("nunca lembrar") e 1
  // ("nunca esquecer") não são agendamentos.
  if retencao <= 0.0 || retencao >= 1.0 {
    return Err(FsrsError::RetencaoInvalida(retencao));
  }
  // Meio-estado (uma coluna sem a outra) é defeito de dado; a crate recebe
  // `None` e trata como primeira revisão. Nada é inventado para completar.
  let anterior = match (estado.estabilidade, estado.dificuldade) {
    (Some(estabilidade), Some(dificuldade)) => Some(MemoryState {
      stability: estabilidade as f32,
      difficulty: dificuldade as f32,
    }),
    _ => None,
  };
  let dias_decorridos = match estado.ultima_revisao {
    // Relógio que anda para trás não cria dias negativos: a crate pede u32.
    Some(ultima) => (((agora_ms - ultima) / 86_400_000).max(0)) as u32,
    None => 0,
  };
  let fsrs = FSRS::default();
  let proximos = fsrs
    .next_states(anterior, retencao as f32, dias_decorridos)
    .map_err(|e| FsrsError::Motor(e.to_string()))?;
  let escolhido = match avaliacao {
    Avaliacao::Again => proximos.again,
    Avaliacao::Hard => proximos.hard,
    Avaliacao::Good => proximos.good,
    Avaliacao::Easy => proximos.easy,
  };
  let intervalo = escolhido.interval.round().max(1.0) as i64;
  let estado_novo = EstadoCartao {
    estabilidade: Some(f64::from(escolhido.memory.stability)),
    dificuldade: Some(f64::from(escolhido.memory.difficulty)),
    ultima_revisao: Some(agora_ms),
    intervalo_dias: Some(intervalo),
    // `again` num cartão com memória entra em relearning; cartão novo não,
    // porque não havia memória a perder.
    relearning: avaliacao == Avaliacao::Again && estado.estabilidade.is_some(),
  };
  Ok(ResultadoRevisao {
    estado: estado_novo,
    intervalo_dias: intervalo,
  })
}

fn zona(zona: &str) -> Result<chrono_tz::Tz, FsrsError> {
  chrono_tz::Tz::from_str(zona)
    .map_err(|_| FsrsError::Motor(format!("fuso IANA desconhecido: {zona}")))
}

/// A data civil de `instante_ms` na zona dada, `YYYY-MM-DD`.
pub fn dia_local(instante_ms: i64, zona_str: &str) -> Result<String, FsrsError> {
  let tz = zona(zona_str)?;
  let dia = Utc
    .timestamp_millis_opt(instante_ms)
    .single()
    .ok_or_else(|| FsrsError::Motor("instante fora do range suportado".into()))?;
  Ok(
    dia
      .with_timezone(&tz)
      .date_naive()
      .format("%Y-%m-%d")
      .to_string(),
  )
}

/// O dia local de vencimento, `YYYY-MM-DD`: a data civil da última revisão na
/// zona da usuária, mais o intervalo. A fila é por dia (D76) — o instante
/// exato dentro do dia não decide nada, e o vencimento não é gravado.
pub fn dia_de_vencimento(
  ultima_revisao_ms: i64,
  intervalo_dias: i64,
  zona_str: &str,
) -> Result<String, FsrsError> {
  let tz = zona(zona_str)?;
  let dia = Utc
    .timestamp_millis_opt(ultima_revisao_ms)
    .single()
    .ok_or_else(|| FsrsError::Motor("instante fora do range suportado".into()))?;
  let vencimento = dia.with_timezone(&tz).date_naive() + Duration::days(intervalo_dias);
  Ok(vencimento.format("%Y-%m-%d").to_string())
}

/// O cartão é devido quando o dia de vencimento já chegou. Cartão sem
/// histórico é devido de imediato. A comparação é entre **datas locais** na
/// zona dada, nunca entre instantes (I11).
pub fn esta_devido(
  estado: &EstadoCartao,
  agora_ms: i64,
  zona_str: &str,
) -> Result<bool, FsrsError> {
  match (estado.ultima_revisao, estado.intervalo_dias) {
    (Some(ultima), Some(intervalo)) => {
      Ok(dia_de_vencimento(ultima, intervalo, zona_str)? <= dia_local(agora_ms, zona_str)?)
    }
    _ => Ok(true),
  }
}

#[cfg(test)]
mod test {
  use super::*;

  /// 2026-10-10T12:00:00Z — instante fixo de todos os casos.
  const AGORA: i64 = 1791633600000;
  const SP: &str = "America/Sao_Paulo";
  const RETENCAO: f64 = 0.9;

  #[test]
  fn retencao_fora_do_intervalo_e_recusada() {
    for invalida in [1.5, 1.0, 0.0, -0.2] {
      let erro = revisar(&EstadoCartao::novo(), Avaliacao::Good, invalida, AGORA);
      assert!(
        matches!(erro, Err(FsrsError::RetencaoInvalida(_))),
        "{invalida}"
      );
    }
  }

  #[test]
  fn cartao_novo_avaliado_again_nao_entra_em_relearning() {
    let resultado = revisar(&EstadoCartao::novo(), Avaliacao::Again, RETENCAO, AGORA)
      .expect("revisar em cartão novo");
    // Não havia memória a perder: o primeiro `again` é o primeiro agendamento,
    // não uma queda. O marcador é para quem já tinha memória.
    assert!(!resultado.estado.relearning);
    assert!(resultado.estado.estabilidade.is_some());
  }

  #[test]
  fn vencimento_e_data_civil_na_ancora_nao_em_utc() {
    // 2026-10-10T02:00:00Z é 2026-10-09 23:00 em São Paulo. O vencimento
    // some o intervalo à data LOCAL: 2026-10-09 + 3 = 2026-10-12. Somar em
    // UTC daria 2026-10-13 — e é exatamente o deslocamento que I11 proíbe.
    let revisao_em = 1791597600000;
    assert_eq!(
      dia_de_vencimento(revisao_em, 3, SP).expect("vencimento"),
      "2026-10-12"
    );
    // A zona é parâmetro: o mesmo instante, lido em UTC, vence um dia depois.
    assert_eq!(
      dia_de_vencimento(revisao_em, 3, "UTC").expect("vencimento"),
      "2026-10-13"
    );
  }

  #[test]
  fn fuso_desconhecido_e_recusado() {
    let erro = dia_de_vencimento(AGORA, 3, "Zona/Inexistente");
    assert!(matches!(erro, Err(FsrsError::Motor(_))));
  }

  #[test]
  fn cartao_sem_historico_e_devido_imediato() {
    let agora_qualquer = 1791633600000;
    assert!(esta_devido(&EstadoCartao::novo(), agora_qualquer, SP).expect("devido"));
  }

  #[test]
  fn devido_e_comparado_por_dia_local_na_zona() {
    // Revisado em 2026-10-01T00:00Z = 2026-09-30 21:00 em SP; intervalo 9.
    // Vencimento: 2026-09-30 + 9 = 2026-10-09 (data civil de SP).
    let estado = EstadoCartao {
      estabilidade: Some(7.0),
      dificuldade: Some(5.0),
      ultima_revisao: Some(1790812800000),
      intervalo_dias: Some(9),
      relearning: false,
    };
    let antes_do_dia = 1791385200000; // 2026-10-07T15:00Z = 12:00 de 2026-10-07 em SP
    assert!(!esta_devido(&estado, antes_do_dia, SP).expect("devido"));
    let no_dia = 1791558000000; // 2026-10-09T15:00Z = 12:00 de 2026-10-09 em SP
    assert!(esta_devido(&estado, no_dia, SP).expect("devido"));
    // Vencido não sai da fila: no dia seguinte ele continua devido.
    let atrasado = 1791644400000; // 2026-10-10T15:00Z = 12:00 de 2026-10-10 em SP
    assert!(esta_devido(&estado, atrasado, SP).expect("devido"));
  }

  // -- golden ------------------------------------------------------------

  use serde::{Deserialize, Serialize};

  /// O estado de entrada de um caso, como vive no banco. Campos ausentes
  /// (ou `null`) valem `None`.
  #[derive(Debug, Serialize, Deserialize)]
  #[serde(rename_all = "camelCase")]
  struct EstadoCaso {
    estabilidade: Option<f64>,
    dificuldade: Option<f64>,
    ultima_revisao: Option<i64>,
    intervalo_dias: Option<i64>,
    relearning: bool,
  }

  #[derive(Debug, Serialize, Deserialize)]
  #[serde(rename_all = "camelCase")]
  struct EntradaCaso {
    /// `null` = cartão novo.
    estado: Option<EstadoCaso>,
    /// 1..4, a ordem de [`Avaliacao`].
    rating: i64,
    retencao: f64,
    agora_ms: i64,
    zona: String,
  }

  #[derive(Debug, Serialize, Deserialize)]
  #[serde(rename_all = "camelCase")]
  struct SaidaCaso {
    estabilidade: f64,
    dificuldade: f64,
    intervalo_dias: i64,
    relearning: bool,
    vencimento: String,
  }

  #[derive(Debug, Serialize, Deserialize)]
  #[serde(rename_all = "camelCase")]
  struct CasoFsrs {
    caso: String,
    entrada: EntradaCaso,
    saida: SaidaCaso,
  }

  fn estado_do_caso(caso: &EstadoCaso) -> EstadoCartao {
    EstadoCartao {
      estabilidade: caso.estabilidade,
      dificuldade: caso.dificuldade,
      ultima_revisao: caso.ultima_revisao,
      intervalo_dias: caso.intervalo_dias,
      relearning: caso.relearning,
    }
  }

  /// Um caso, do começo ao fim: revisa e devolve a saída esperada do golden.
  fn processar(entrada: &EntradaCaso) -> SaidaCaso {
    let estado = entrada
      .estado
      .as_ref()
      .map(estado_do_caso)
      .unwrap_or_else(EstadoCartao::novo);
    let avaliacao = Avaliacao::de_rating(entrada.rating).expect("rating entre 1 e 4");
    let resultado = revisar(&estado, avaliacao, entrada.retencao, entrada.agora_ms)
      .expect("revisar não falha em caso de golden");
    let vencimento = dia_de_vencimento(entrada.agora_ms, resultado.intervalo_dias, &entrada.zona)
      .expect("vencimento não falha em caso de golden");
    SaidaCaso {
      estabilidade: resultado
        .estado
        .estabilidade
        .expect("após revisar há memória"),
      dificuldade: resultado
        .estado
        .dificuldade
        .expect("após revisar há memória"),
      intervalo_dias: resultado.intervalo_dias,
      relearning: resultado.estado.relearning,
      vencimento,
    }
  }

  /// As entradas dos casos do golden. É o gerador documentado: o arquivo é a
  /// entrada + a saída que o motor produziu, e este vetor é a única fonte das
  /// entradas quando o golden precisa ser regenerado (bump da crate).
  fn entradas_do_golden() -> Vec<(&'static str, EntradaCaso)> {
    // Construtores, não valores: cada caso precisa da própria `String` de
    // zona, e o estado não é `Copy`.
    let nova = |rating: i64| EntradaCaso {
      estado: None,
      rating,
      retencao: RETENCAO,
      agora_ms: AGORA,
      zona: SP.into(),
    };
    let com_estado = |estado: EstadoCaso, rating: i64| EntradaCaso {
      estado: Some(estado),
      rating,
      retencao: RETENCAO,
      agora_ms: AGORA,
      zona: SP.into(),
    };
    let revisada = |rating: i64| {
      com_estado(
        EstadoCaso {
          estabilidade: Some(7.0),
          dificuldade: Some(5.0),
          ultima_revisao: Some(AGORA - 7 * 86_400_000),
          intervalo_dias: Some(7),
          relearning: false,
        },
        rating,
      )
    };
    let em_relearning = |rating: i64| {
      com_estado(
        EstadoCaso {
          estabilidade: Some(2.0),
          dificuldade: Some(7.0),
          ultima_revisao: Some(AGORA - 86_400_000),
          intervalo_dias: Some(1),
          relearning: true,
        },
        rating,
      )
    };
    vec![
      ("nova-e-good-090", nova(3)),
      ("nova-e-hard-090", nova(2)),
      ("nova-e-easy-090", nova(4)),
      ("nova-e-again-090", nova(1)),
      ("revisada-good-090", revisada(3)),
      ("revisada-again-090", revisada(1)),
      ("relearning-good-090", em_relearning(3)),
      (
        "vencimento-na-ancora-SP",
        EntradaCaso {
          // 2026-10-10T02:00Z = 2026-10-09 23:00 em SP: pinar que o
          // vencimento some o intervalo à data civil LOCAL, não à de UTC.
          agora_ms: 1791597600000,
          ..nova(3)
        },
      ),
    ]
  }

  /// O golden pina o motor: mesmas entradas, mesmas saídas, e um bump da
  /// crate `fsrs` que mude agendamento **fala alto aqui** — é o I13 de
  /// SPEC-D-010. Os valores vêm da própria crate (6.6.2) — não há segunda
  /// implementação independente de FSRS, e hand-rolar a estabilidade seria
  /// reimplementar o modelo, o que D76 rejeita. O pin é deliberado.
  #[test]
  fn golden_do_fsrs_pina_o_motor() {
    let caminho = concat!(env!("CARGO_MANIFEST_DIR"), "/../golden/fsrs.jsonl");
    let conteudo = std::fs::read_to_string(caminho).expect("golden/fsrs.jsonl legível");
    let linhas: Vec<&str> = conteudo
      .lines()
      .filter(|l| !l.trim_start().starts_with('#'))
      .collect();
    assert!(linhas.len() >= 6, "SPEC-D-010 §9 manda pelo menos 6 casos");
    assert_eq!(
      linhas.len(),
      entradas_do_golden().len(),
      "golden e gerador divergem"
    );
    for (linha, (nome, entrada)) in linhas.iter().zip(entradas_do_golden()) {
      let caso: CasoFsrs = serde_json::from_str(linha).expect("linha JSON válida");
      assert_eq!(
        caso.caso, nome,
        "ordem e nome dos casos são parte do contrato"
      );
      // A entrada do arquivo é a do gerador — sem isso, o arquivo e a
      // regeneração contariam histórias diferentes.
      let entrada_do_arquivo = serde_json::to_string(&caso.entrada).expect("serializa");
      let entrada_do_gerador = serde_json::to_string(&entrada).expect("serializa");
      assert_eq!(
        entrada_do_arquivo, entrada_do_gerador,
        "entrada: {}",
        caso.caso
      );
      let saida = processar(&caso.entrada);
      assert_eq!(
        saida.estabilidade, caso.saida.estabilidade,
        "estabilidade: {}",
        caso.caso
      );
      assert_eq!(
        saida.dificuldade, caso.saida.dificuldade,
        "dificuldade: {}",
        caso.caso
      );
      assert_eq!(
        saida.intervalo_dias, caso.saida.intervalo_dias,
        "intervalo: {}",
        caso.caso
      );
      assert_eq!(
        saida.relearning, caso.saida.relearning,
        "relearning: {}",
        caso.caso
      );
      assert_eq!(
        saida.vencimento, caso.saida.vencimento,
        "vencimento: {}",
        caso.caso
      );
    }
  }

  /// Gerador do golden, fora do gate comum. Regenerar (por exemplo num bump
  /// da crate) é rodar:
  ///
  /// ```text
  /// cargo test --lib --no-default-features regenerar -- --ignored --nocapture
  /// ```
  ///
  /// e escrever a saída em `golden/fsrs.jsonl` com as cabeceiras de comentário
  /// (ver o cabeçalho do arquivo). O bump é **deliberado**: o golden só muda
  /// quando alguém decide mudá-lo.
  #[test]
  #[ignore = "gerador: roda com --ignored --nocapture; a saída vira golden/fsrs.jsonl"]
  fn regenerar_o_golden_do_fsrs() {
    for (nome, entrada) in entradas_do_golden() {
      let caso = CasoFsrs {
        caso: nome.into(),
        saida: processar(&entrada),
        entrada,
      };
      println!("{}", serde_json::to_string(&caso).expect("serializa"));
    }
  }
}
