//! Os testes do caminho de escrita do TCC (SPEC-D-015 D99).
//!
//! O `use super::*` é obrigatório aqui como nos outros dois: sem ele o
//! compilador acusa `cannot find macro params` e `cannot find type
//! NoDocumento`. Uma medição anterior disse que o glob era dispensável no TCC
//! e que o `clippy` o marcava como redundante — **essa medição estava errada**,
//! porque o build estava em cache e o arquivo alterado não foi recompilado. Com
//! a recompilação forçada, os mesmos erros aparecem. Valeu registrar: um gate
//! verde sobre build velho é pior que nenhum gate.

use super::*;

fn paragrafo(id: &str, texto: &str) -> NoDocumento {
  NoDocumento::Paragrafo {
    id: id.into(),
    inline: vec![Inline::simples(texto)],
  }
}

fn projeto_tcc(db: &Db) -> (String, String) {
  let projeto = create_projeto_core(
    db,
    NovoProjeto {
      titulo: "Meu TCC".into(),
      tipo: "tcc".into(),
      descricao: None,
    },
  )
  .expect("projeto");
  let documento_id: String = db
    .conn()
    .unwrap()
    .query_row(
      "SELECT id FROM documento WHERE projeto_id = ?1",
      params![projeto.id],
      |r| r.get(0),
    )
    .expect("documento");
  (projeto.id, documento_id)
}

#[test]
fn tcc_projeto_nasce_com_documento() {
  let db = Db::open_in_memory().expect("banco");
  let (_, documento_id) = projeto_tcc(&db);
  let doc = get_documento_core(&db, DocumentoRef { documento_id }).expect("documento");
  assert!(doc.arvore.is_empty());
  assert_eq!(doc.ultima_versao, None, "nenhuma versão antes de salvar");
}

#[test]
fn tcc_documento_do_projeto_le_o_mesmo_documento() {
  let db = Db::open_in_memory().expect("banco");
  let (projeto_id, documento_id) = projeto_tcc(&db);

  let por_projeto =
    get_documento_do_projeto_core(&db, ProjetoRef { projeto_id }).expect("lê pelo projeto");
  let por_id = get_documento_core(&db, DocumentoRef { documento_id }).expect("lê pelo documento");

  assert_eq!(
    por_projeto.id, por_id.id,
    "os dois caminhos são o mesmo documento"
  );
  assert_eq!(por_projeto.projeto_id, por_id.projeto_id);

  let err = get_documento_do_projeto_core(
    &db,
    ProjetoRef {
      projeto_id: "nao-existe".into(),
    },
  )
  .expect_err("projeto sem documento");
  assert!(err.to_string().contains("documento"), "diz o porquê: {err}");
}

#[test]
fn tcc_salvar_versiona_e_escrita_sem_mudanca_nao_versiona() {
  let db = Db::open_in_memory().expect("banco");
  let (_, documento_id) = projeto_tcc(&db);
  let arvore = vec![paragrafo("p1", "Introdução")];

  let v1 = salvar_documento_core(
    &db,
    SalvarDocumento {
      documento_id: documento_id.clone(),
      arvore: arvore.clone(),
      autor: "eu".into(),
      motivo: Some("rascunho".into()),
    },
  )
  .expect("salva");
  assert!(v1.versionada);
  assert_eq!(v1.numero, 1);

  // D87: mesmo conteúdo, sem mudança → não cria versão.
  let v2 = salvar_documento_core(
    &db,
    SalvarDocumento {
      documento_id: documento_id.clone(),
      arvore: arvore.clone(),
      autor: "eu".into(),
      motivo: None,
    },
  )
  .expect("salva de novo");
  assert!(!v2.versionada);
  assert_eq!(v2.numero, 1);

  let doc = get_documento_core(
    &db,
    DocumentoRef {
      documento_id: documento_id.clone(),
    },
  )
  .expect("lê");
  assert_eq!(doc.arvore, arvore, "a árvore volta igual");
  assert_eq!(doc.ultima_versao, Some(1));

  // Mudou → versão 2.
  let mut outra = arvore.clone();
  outra.push(paragrafo("p2", "Método"));
  let v3 = salvar_documento_core(
    &db,
    SalvarDocumento {
      documento_id: documento_id.clone(),
      arvore: outra.clone(),
      autor: "eu".into(),
      motivo: None,
    },
  )
  .expect("salva a mudança");
  assert!(v3.versionada);
  assert_eq!(v3.numero, 2);

  let doc = get_documento_core(&db, DocumentoRef { documento_id }).expect("lê");
  assert_eq!(doc.arvore, outra);
}

#[test]
fn tcc_citacao_sem_alvo_e_recusada_na_escrita() {
  let db = Db::open_in_memory().expect("banco");
  let (_, documento_id) = projeto_tcc(&db);
  let arvore = vec![NoDocumento::Citacao {
    id: "c".into(),
    referencia_id: String::new(),
    modo: ModoVinculo::Vivo,
  }];
  let err = salvar_documento_core(
    &db,
    SalvarDocumento {
      documento_id,
      arvore,
      autor: "eu".into(),
      motivo: None,
    },
  )
  .expect_err("citação sem alvo");
  assert!(err.to_string().contains("vínculo"), "diz o porquê: {err}");
}

#[test]
fn tcc_item_duplicado_exige_substituto() {
  let db = Db::open_in_memory().expect("banco");
  let (_, documento_id) = projeto_tcc(&db);
  let item = registrar_item_core(
    &db,
    NovoItem {
      documento_id: documento_id.clone(),
      tipo: "comentario".into(),
      autor: "Orientador".into(),
      autor_resolvido: false,
      data_autor_ms: None,
      citado: "este trecho".into(),
      texto: None,
      origem_arquivo: None,
    },
  )
  .expect("registra");
  assert!(item.esta_pendente());
  assert!(!item.autor_resolvido, "D78: autor desconhecido");

  let err = resolver_item_core(
    &db,
    ResolverItem {
      item_id: item.id.clone(),
      estado: "duplicado".into(),
      substituto: None,
    },
  )
  .expect_err("duplicado sem substituto");
  assert!(
    err.to_string().contains("substituto"),
    "diz o porquê: {err}"
  );

  resolver_item_core(
    &db,
    ResolverItem {
      item_id: item.id.clone(),
      estado: "nao-encaixou".into(),
      substituto: None,
    },
  )
  .expect("não-encaixou");

  let itens = list_itens_core(&db, DocumentoRef { documento_id }).expect("lista");
  assert_eq!(itens.len(), 1);
  assert_eq!(itens[0].estado, EstadoItem::NaoEncaixou);
  assert!(itens[0].resolvido_em.is_some());
}

#[test]
fn tcc_limite_de_projetos_ativos_recusa_o_sexto() {
  let db = Db::open_in_memory().expect("banco");
  let mut ids = Vec::new();
  for n in 1..=LIMITE_DE_PROJETOS_ATIVOS {
    let p = create_projeto_core(
      &db,
      NovoProjeto {
        titulo: format!("Projeto {n}"),
        tipo: "tcc".into(),
        descricao: None,
      },
    )
    .expect("os cinco primeiros cabem");
    ids.push(p.id);
  }

  let err = create_projeto_core(
    &db,
    NovoProjeto {
      titulo: "Sexto".into(),
      tipo: "tcc".into(),
      descricao: None,
    },
  )
  .expect_err("o sexto é recusado");
  assert!(
    err
      .to_string()
      .contains(&LIMITE_DE_PROJETOS_ATIVOS.to_string()),
    "nomeia o limite: {err}"
  );

  // O limite é de **ativos**: encerrar um libera o lugar.
  set_estado_projeto_core(
    &db,
    ProjetoEstadoInput {
      projeto_id: ids[0].clone(),
      estado: "encerrado".into(),
    },
  )
  .expect("encerra um");
  create_projeto_core(
    &db,
    NovoProjeto {
      titulo: "Sexto".into(),
      tipo: "tcc".into(),
      descricao: None,
    },
  )
  .expect("cabe depois de encerrar");
}

#[test]
fn tcc_base_guarda_vinculo_sem_copia_e_deduplica() {
  let db = Db::open_in_memory().expect("banco");
  let (projeto_id, _) = projeto_tcc(&db);

  let v = vincular_core(
    &db,
    NovoVinculo {
      projeto_id: projeto_id.clone(),
      alvo_tipo: "referencia".into(),
      alvo_id: "ref-42".into(),
    },
  )
  .expect("vincula");
  assert_eq!(v.alvo_id, "ref-42");
  assert_eq!(v.alvo_tipo, "referencia");

  // O vínculo é referência viva: guarda o id, não copia conteúdo (D66/I11).
  let base = list_base_core(
    &db,
    ProjetoRef {
      projeto_id: projeto_id.clone(),
    },
  )
  .expect("lista");
  assert_eq!(base.len(), 1);
  assert_eq!(base[0].alvo_id, "ref-42");

  // Citar o mesmo alvo de novo não duplica: vínculo repetido é o mesmo vínculo.
  vincular_core(
    &db,
    NovoVinculo {
      projeto_id: projeto_id.clone(),
      alvo_tipo: "referencia".into(),
      alvo_id: "ref-42".into(),
    },
  )
  .expect("de novo");
  assert_eq!(
    list_base_core(
      &db,
      ProjetoRef {
        projeto_id: projeto_id.clone()
      }
    )
    .expect("lista")
    .len(),
    1,
    "o mesmo alvo não duplica"
  );

  // Fragmento é o outro tipo de alvo e conta separado.
  vincular_core(
    &db,
    NovoVinculo {
      projeto_id: projeto_id.clone(),
      alvo_tipo: "fragmento".into(),
      alvo_id: "ref-42".into(),
    },
  )
  .expect("fragmento");
  assert_eq!(
    list_base_core(
      &db,
      ProjetoRef {
        projeto_id: projeto_id.clone(),
      },
    )
    .expect("lista")
    .len(),
    2
  );

  desvincular_core(
    &db,
    VinculoRef {
      vinculo_id: v.id.clone(),
    },
  )
  .expect("desvincula");
  assert_eq!(
    list_base_core(&db, ProjetoRef { projeto_id })
      .expect("lista")
      .len(),
    1
  );

  let err = desvincular_core(
    &db,
    VinculoRef {
      vinculo_id: "nao-existe".into(),
    },
  )
  .expect_err("id inexistente");
  assert!(err.to_string().contains("vínculo"), "diz o porquê: {err}");
}

#[test]
fn tcc_base_recusa_alvo_tipo_invalido() {
  let db = Db::open_in_memory().expect("banco");
  let (projeto_id, _) = projeto_tcc(&db);
  let err = vincular_core(
    &db,
    NovoVinculo {
      projeto_id,
      alvo_tipo: "site".into(),
      alvo_id: "x".into(),
    },
  )
  .expect_err("tipo inválido");
  assert!(err.to_string().contains("alvo"), "diz o porquê: {err}");
}

#[test]
fn tcc_paginar_documento_vazio_tem_uma_pagina_e_declara_o_medidor() {
  let db = Db::open_in_memory().expect("banco");
  let (_, documento_id) = projeto_tcc(&db);
  let pag = paginar_documento_core(
    &db,
    PaginarDocumento {
      documento_id,
      geometria: None,
    },
  )
  .expect("pagina");

  assert_eq!(pag.paginacao.paginas, 1, "documento vazio tem uma página");
  assert!(pag.paginacao.alturas_por_no.is_empty());
  assert_eq!(
    pag.medidor, "deterministico",
    "D86: o medidor é declarado, nunca silencioso"
  );
  assert_eq!(
    pag.geometria,
    Geometria::a4_abnt(),
    "sem geometria, vale o A4 ABNT"
  );
}

#[test]
fn tcc_paginar_muda_a_geometria_sem_mudar_o_conteudo() {
  let db = Db::open_in_memory().expect("banco");
  let (_, documento_id) = projeto_tcc(&db);

  // Conteúdo suficiente para a página estreita quebrar em mais de uma.
  let mut arvore = Vec::new();
  for i in 0..30 {
    arvore.push(paragrafo(&format!("p{i}"), &"palavra ".repeat(24)));
  }
  salvar_documento_core(
    &db,
    SalvarDocumento {
      documento_id: documento_id.clone(),
      arvore: arvore.clone(),
      autor: "eu".into(),
      motivo: None,
    },
  )
  .expect("salva");

  let estreita = Geometria {
    largura_mm: 120.0,
    ..Geometria::a4_abnt()
  };
  let pag_estreita = paginar_documento_core(
    &db,
    PaginarDocumento {
      documento_id: documento_id.clone(),
      geometria: Some(estreita),
    },
  )
  .expect("pagina estreita");
  let pag_larga = paginar_documento_core(
    &db,
    PaginarDocumento {
      documento_id: documento_id.clone(),
      geometria: None,
    },
  )
  .expect("pagina larga");

  // A geometria move texto de página; o conjunto de nós é o mesmo (I2).
  let mut chaves_estreita: Vec<&String> = pag_estreita.paginacao.alturas_por_no.keys().collect();
  let mut chaves_larga: Vec<&String> = pag_larga.paginacao.alturas_por_no.keys().collect();
  chaves_estreita.sort();
  chaves_larga.sort();
  assert_eq!(chaves_estreita, chaves_larga);
  assert!(
    pag_estreita.paginacao.paginas >= pag_larga.paginacao.paginas,
    "a geometria estreita não pode paginar em menos páginas"
  );

  // O conteúdo não mudou: o documento relido é idêntico.
  let doc = get_documento_core(&db, DocumentoRef { documento_id }).expect("lê");
  assert_eq!(doc.arvore, arvore, "paginar não toca na árvore");
}
