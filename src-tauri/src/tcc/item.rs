//! O item a resolver da importação de DOCX (SPEC-D-013, `D88`).
//!
//! `item_a_resolver` é **tabela própria**, não um par de campos no nós. Ela
//! guarda o que o motor de DOCX não conseguiu transformar sozinho: um comentário
//! ou uma sugestão (controle `w:ins`/`w:del`) que precisa de decisão humana.
//! Decisão registrada em `D88`; a alternativa rejeitada foi guardar isso como
//! coluna do nó, que perderia a lista fechada de estados.
//!
//! Regra dura da importação (`D88`): **importar DOCX anexa e nunca altera**
//! `no_documento`. O que não casa vira linha aqui.

use serde::{Deserialize, Serialize};

/// O tipo de item a resolver: a lista é fechada (`D88`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TipoItem {
  Comentario,
  Sugestao,
}

/// O estado do item na lista de decisão. Lista fechada (`D88`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EstadoItem {
  /// Ainda não decidido.
  Pendente,
  /// A usuária aceitou.
  Resolvido,
  /// Não havia onde encaixar no documento.
  NaoEncaixou,
  /// Já existia um item equivalente; o `substituto` aponta para o que ficou.
  Duplicado,
}

/// Um item da lista de decisão da importação.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemAResolver {
  pub id: String,
  pub documento_id: String,
  pub tipo: TipoItem,
  /// Autor do comentário ou dos controles de alteração, como veio no arquivo.
  /// Vazio quando o arquivo não o traz — nunca inventado.
  pub autor: String,
  /// Se o autor do item é a própria usuária (`w:author` casando com o perfil).
  pub autor_resolvido: bool,
  /// Quando o autor marcou a data, em epoch-ms. `None` é "sem data", não "agora".
  pub data_autor: Option<i64>,
  /// Trecho citado do documento, como contexto.
  pub citado: String,
  /// Quando o item é duplicado, o id do item que ficou.
  pub substituto: Option<String>,
  /// O texto da sugestão, para os controles de alteração.
  pub texto: Option<String>,
  /// De qual arquivo importado o item veio.
  pub origem_arquivo: Option<String>,
  /// Quando a importação o criou, em epoch-ms.
  pub importado_em: i64,
  pub estado: EstadoItem,
  /// Quando foi decidido, em epoch-ms. `None` enquanto pendente.
  pub resolvido_em: Option<i64>,
}

impl ItemAResolver {
  /// Cria um item pendente. A importação é o único caminho que cria.
  #[allow(clippy::too_many_arguments)]
  pub fn pendente(
    id: impl Into<String>,
    documento_id: impl Into<String>,
    tipo: TipoItem,
    autor: impl Into<String>,
    autor_resolvido: bool,
    data_autor: Option<i64>,
    citado: impl Into<String>,
    texto: Option<String>,
    origem_arquivo: Option<String>,
    importado_em: i64,
  ) -> Self {
    ItemAResolver {
      id: id.into(),
      documento_id: documento_id.into(),
      tipo,
      autor: autor.into(),
      autor_resolvido,
      data_autor,
      citado: citado.into(),
      substituto: None,
      texto,
      origem_arquivo,
      importado_em,
      estado: EstadoItem::Pendente,
      resolvido_em: None,
    }
  }

  /// Marca o item como resolvido num instante.
  pub fn resolver(&mut self, agora_ms: i64) {
    self.estado = EstadoItem::Resolvido;
    self.resolvido_em = Some(agora_ms);
  }

  /// Marca o item como duplicado, apontando para o que ficou.
  ///
  /// Um item duplicado sem substituto não decide nada; o substituto é
  /// obrigatório, e é a razão de o estado existir.
  pub fn marcar_duplicado(&mut self, substituto: impl Into<String>, agora_ms: i64) {
    self.estado = EstadoItem::Duplicado;
    self.substituto = Some(substituto.into());
    self.resolvido_em = Some(agora_ms);
  }

  /// Diz se o item ainda espera decisão.
  pub fn esta_pendente(&self) -> bool {
    self.estado == EstadoItem::Pendente
  }
}

#[cfg(test)]
mod test {
  use super::*;

  fn item() -> ItemAResolver {
    ItemAResolver::pendente(
      "i1",
      "doc1",
      TipoItem::Comentario,
      "Orientador",
      false,
      None,
      "este trecho",
      None,
      None,
      1_000,
    )
  }

  #[test]
  fn item_nasce_pendente_e_sem_data_de_autoria() {
    let item = item();
    assert!(item.esta_pendente());
    assert_eq!(item.estado, EstadoItem::Pendente);
    assert_eq!(item.data_autor, None);
    assert_eq!(item.resolvido_em, None);
  }

  #[test]
  fn resolver_carimba_o_instante() {
    let mut item = item();
    item.resolver(2_000);
    assert_eq!(item.estado, EstadoItem::Resolvido);
    assert_eq!(item.resolvido_em, Some(2_000));
    assert!(!item.esta_pendente());
  }

  #[test]
  fn duplicado_aponta_para_o_substituto() {
    let mut item = item();
    item.marcar_duplicado("i2", 3_000);
    assert_eq!(item.estado, EstadoItem::Duplicado);
    assert_eq!(item.substituto.as_deref(), Some("i2"));
    assert_eq!(item.resolvido_em, Some(3_000));
  }

  #[test]
  fn os_tokens_do_contrato_sao_estaveis() {
    assert_eq!(
      serde_json::to_string(&EstadoItem::NaoEncaixou).unwrap(),
      "\"nao-encaixou\""
    );
    assert_eq!(
      serde_json::to_string(&TipoItem::Sugestao).unwrap(),
      "\"sugestao\""
    );
  }
}
