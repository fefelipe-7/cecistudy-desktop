//! Regra de média (D75): viva no semestre corrente, congelada no passado.
//!
//! A spec referencial §4.3 é **[D]** em duas frases que precisam valer juntas:
//! *"Herança: Viva: disciplinas não personalizadas seguem o padrão"* e
//! *"Semestres passados: Regra congelada por semestre: mudar o padrão depois não
//! altera o histórico"*. A resolução abaixo é a ordem declarada em D75 e não tem
//! outro caminho.
//!
//! O que este módulo **não** faz: gravar. Ele resolve qual regra vale e devolve
//! de onde ela veio. Quem grava a regra congelada ao encerrar o semestre é o
//! caminho de escrita (`commands.rs`, quando ele chegar).

/// Tipo de média (§4.3 **[D]**).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoMedia {
  /// Todas as avaliações pesam igual. O peso da avaliação é **ignorado** (I3).
  Simples,
  /// Cada avaliação pesa pelo seu `peso`.
  Ponderada,
}

/// Como o exame/recuperação entra na conta.
///
/// §4.3 diz que *"a regra do exame é da disciplina"* e não dá a fórmula. Por
/// isso o modo é configurável por disciplina — a decisão de produto é o
/// **modo**, e cada disciplina escolhe o seu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModoExame {
  /// Não há exame: a média decide sozinha.
  Nenhum,
  /// Recuperação por média: `(media + exame) / 2` precisa alcançar
  /// `exame_minima`.
  MediaComExame,
  /// O exame substitui a média: alcançar `exame_minima` no exame aprova.
  ExameSubstitui,
}

/// Uma camada de configuração de regra. A efetiva é a resolução de quatro
/// camadas (ver [`resolver`]).
#[derive(Debug, Clone, PartialEq)]
pub struct Regra {
  pub tipo_media: TipoMedia,
  /// Nota mínima para aprovação direta.
  pub nota_minima: f64,
  /// Passo de arredondamento (0,1 ou 0,01). Afeta a **fronteira** (I4).
  pub arredondamento: f64,
  /// Fração mínima de presença (ex.: 0,75).
  pub frequencia_minima: f64,
  pub exame_modo: ModoExame,
  /// Nota mínima do exame, ou da média após o exame.
  pub exame_minima: f64,
  /// Média mínima para **ter direito** ao exame. Abaixo dela, não alcança.
  pub exame_elegivel_minima: f64,
}

/// De onde veio a regra efetiva. Vira o rótulo `regraAplicada` do contrato (§5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrigemRegra {
  Padrao,
  Personalizada,
  Congelada,
}

/// A regra efetiva e a sua origem.
#[derive(Debug, Clone, PartialEq)]
pub struct RegraAplicada {
  pub regra: Regra,
  pub origem: OrigemRegra,
}

impl RegraAplicada {
  /// O rótulo do contrato: `padrao` · `personalizada-<semestre>` ·
  /// `congelada-<semestre>`.
  pub fn rotulo(&self, semestre_id: &str) -> String {
    match self.origem {
      OrigemRegra::Padrao => "padrao".to_string(),
      OrigemRegra::Personalizada => format!("personalizada-{semestre_id}"),
      OrigemRegra::Congelada => format!("congelada-{semestre_id}"),
    }
  }
}

/// As três camadas, na ordem de precedência de D75.
///
/// D75 enuncia quatro candidatos ("personalização do semestre · congelada do
/// semestre · personalização corrente · padrão"), mas "personalização do
/// semestre" e "personalização corrente" são a **mesma** personalização por
/// disciplina: uma disciplina pertence a um semestre só, e a personalização vive
/// nele enquanto ele está aberto. Persistir as duas seria duplicar a mesma
/// linha, então elas colapsam em `personalizacao_da_disciplina`. O referencial
/// (§4.3) descreve exatamente estas três: herança viva no presente, congelado no
/// passado.
///
/// A ordem **importa**: personalização antes de congelada seria o bug de inverter
/// a precedência, que é a armadilha de risco alto registrada em `SPEC-D-010` §10.
pub struct Camadas<'a> {
  pub congelada_do_semestre: Option<&'a Regra>,
  pub personalizacao_da_disciplina: Option<&'a Regra>,
  pub padrao_global: &'a Regra,
}

/// Resolve a regra efetiva, na ordem declarada em D75.
pub fn resolver(camadas: &Camadas<'_>) -> RegraAplicada {
  if let Some(r) = camadas.congelada_do_semestre {
    return RegraAplicada {
      regra: r.clone(),
      origem: OrigemRegra::Congelada,
    };
  }
  if let Some(r) = camadas.personalizacao_da_disciplina {
    return RegraAplicada {
      regra: r.clone(),
      origem: OrigemRegra::Personalizada,
    };
  }
  RegraAplicada {
    regra: camadas.padrao_global.clone(),
    origem: OrigemRegra::Padrao,
  }
}

/// Erros de escrita da regra.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegraError {
  /// I7: a regra de um semestre encerrado é imutável.
  SemestreEncerrado,
}

impl std::fmt::Display for RegraError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      RegraError::SemestreEncerrado => {
        write!(f, "a regra de um semestre encerrado é imutável (D75)")
      }
    }
  }
}

impl std::error::Error for RegraError {}

/// Guarda do caminho de escrita: a regra de um semestre encerrado não muda (I7).
///
/// **A nota continua lançável** em semestre encerrado (I9): lançar nota é dado,
/// mudar regra é regra. Por isso esta guarda só protege a regra — quem protege a
/// nota é não existir guarda de nota.
pub fn exigir_semestre_aberto(encerrado: bool) -> Result<(), RegraError> {
  if encerrado {
    return Err(RegraError::SemestreEncerrado);
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  fn regra(nota_minima: f64) -> Regra {
    Regra {
      tipo_media: TipoMedia::Ponderada,
      nota_minima,
      arredondamento: 0.1,
      frequencia_minima: 0.75,
      exame_modo: ModoExame::Nenhum,
      exame_minima: 6.0,
      exame_elegivel_minima: 4.0,
    }
  }

  #[test]
  fn sem_personalizacao_herda_o_padrao_e_mudar_o_padrao_move() {
    let padrao = regra(6.0);
    let a = resolver(&Camadas {
      congelada_do_semestre: None,
      personalizacao_da_disciplina: None,
      padrao_global: &padrao,
    });
    assert_eq!(a.origem, OrigemRegra::Padrao);
    assert_eq!(a.regra.nota_minima, 6.0);

    // I6: mudar o padrão global move a disciplina não personalizada.
    let novo = regra(7.0);
    let b = resolver(&Camadas {
      congelada_do_semestre: None,
      personalizacao_da_disciplina: None,
      padrao_global: &novo,
    });
    assert_eq!(b.regra.nota_minima, 7.0);
  }

  #[test]
  fn personalizacao_substitui_o_padrao_por_inteiro() {
    let padrao = regra(6.0);
    let mut propria = regra(8.0);
    propria.tipo_media = TipoMedia::Simples;
    let a = resolver(&Camadas {
      congelada_do_semestre: None,
      personalizacao_da_disciplina: Some(&propria),
      padrao_global: &padrao,
    });
    assert_eq!(a.origem, OrigemRegra::Personalizada);
    assert_eq!(a.regra.nota_minima, 8.0);
    assert_eq!(a.regra.tipo_media, TipoMedia::Simples);
    // O padrão mudou depois e a disciplina personalizada não se move.
    let padrao_novo = regra(5.0);
    let b = resolver(&Camadas {
      congelada_do_semestre: None,
      personalizacao_da_disciplina: Some(&propria),
      padrao_global: &padrao_novo,
    });
    assert_eq!(b.regra.nota_minima, 8.0);
  }

  #[test]
  fn congelada_vence_a_personalizacao_da_disciplina() {
    let padrao = regra(6.0);
    let congelada = regra(4.0);
    let corrente = regra(9.0);
    let a = resolver(&Camadas {
      congelada_do_semestre: Some(&congelada),
      personalizacao_da_disciplina: Some(&corrente),
      padrao_global: &padrao,
    });
    assert_eq!(a.origem, OrigemRegra::Congelada);
    assert_eq!(
      a.regra.nota_minima, 4.0,
      "histórico não muda com o presente"
    );
  }

  #[test]
  fn rotulos_do_contrato() {
    let r = regra(6.0);
    let padrao = RegraAplicada {
      regra: r.clone(),
      origem: OrigemRegra::Padrao,
    };
    assert_eq!(padrao.rotulo("2026-2"), "padrao");

    let propria = RegraAplicada {
      regra: r.clone(),
      origem: OrigemRegra::Personalizada,
    };
    assert_eq!(propria.rotulo("2026-2"), "personalizada-2026-2");

    let congelada = RegraAplicada {
      regra: r,
      origem: OrigemRegra::Congelada,
    };
    assert_eq!(congelada.rotulo("2026-1"), "congelada-2026-1");
  }

  #[test]
  fn caso_negativo_alterar_regra_de_semestre_encerrado_e_recusado() {
    assert_eq!(exigir_semestre_aberto(false), Ok(()));
    assert_eq!(
      exigir_semestre_aberto(true),
      Err(RegraError::SemestreEncerrado)
    );
  }
}
