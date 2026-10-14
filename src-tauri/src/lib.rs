//! Casca do Tauri: abre o banco e registra os comandos do Calendário.
//!
//! Este arquivo é deliberadamente fino. Ele sabe duas coisas — onde fica o
//! `app_data_dir` e quais comandos existem. A regra de domínio está em
//! `commands.rs`, e a conexão em `db.rs`; nenhum dos dois importa `tauri`.
//!
//! A casca inteira é `#[cfg(feature = "tauri-shell")]`. Sem essa feature o crate
//! compila só o núcleo de domínio, e é assim que `cargo test --lib
//! --no-default-features` roda a suíte sem carregar WebView2 nem a pilha de GUI.
//! A feature é padrão, então `tauri build` e `tauri:dev` continuam iguais.

// Sem a casca (`--no-default-features`, que é como `cargo test` compila), as
// funções `*_core` que só a casca chama e as structs que só eladeserialize são
// inalcançáveis, e o lint de `dead_code` reclama. Não é código morto: é código
// que a feature desliga. O `allow` é condicional, então com a casca ligada o
// lint continua valendo e não há como esconder código morto de verdade.
#![cfg_attr(
  not(feature = "tauri-shell"),
  allow(
    dead_code,
    reason = "sem a casca do Tauri, só o teste alcança estas funções"
  )
)]

mod commands;
mod db;
mod domain;
// O motor de notas (D74/D75) nasce aqui e é consumido pelo caminho de escrita em
// `commands.rs`. É `pub` porque é a API de domínio do módulo Faculdade — o mesmo
// papel que `domain.rs` tem para o Calendário, mas exposto para o IPC.
pub mod materia;
// O motor de repetição espaçada (D76, SPEC-D-010) segue o mesmo desenho: puro,
// com relógio injetado, e consumido pelo caminho de escrita quando a tela de
// Estudos existir.
pub mod estudos;
// TCC & Projetos (SPEC-D-013, D85..D88): o modelo do documento, o motor de
// paginação puro e a lista de decisão da importação. Fonte de verdade ao lado da
// migração `0005_tcc.sql`; consumido pelo caminho de escrita em `commands.rs`.
pub mod tcc;

// `db` só é usado pela casca; sem a feature ele compila, mas não é referenciado.
#[cfg(feature = "tauri-shell")]
use tauri::Manager;

#[cfg(feature = "tauri-shell")]
use db::Db;

#[cfg(feature = "tauri-shell")]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }

      // O banco é criado no primeiro `setup`. Se o diretório ou o schema
      // falhar, o app não sobe em um estado meio inicializado: é melhor
      // erro na abertura do que tela vazia sem persistência.
      let dir = app.path().app_data_dir()?;
      let db = Db::open_in(&dir)?;
      let version = db.migrate()?;
      log::info!("SQLite pronto em {} (schema v{version})", dir.display());
      app.manage(db);

      Ok(())
    })
    // §3.4 + SPEC-D-010 + SPEC-D-013: os nomes registrados. Os de Calendário,
    // Faculdade, Estudos e TCC têm par numa `data/bridge.ts`; `get_setting` e
    // `set_setting` ainda não têm chamador no front-end. A lista é a fronteira
    // entre Rust e TypeScript — se um nome divergir, o `invoke` falha em
    // runtime, então qualquer mudança aqui muda lá também.
    .invoke_handler(tauri::generate_handler![
      commands::migrate,
      commands::list_events_in_window,
      commands::list_responsibilities,
      commands::create_event,
      commands::update_event,
      commands::delete_event,
      commands::set_occurrence_state,
      commands::move_occurrence,
      commands::upsert_recurrence,
      commands::update_recurrence,
      commands::create_responsibility,
      commands::set_step_state,
      commands::plan_block,
      commands::reschedule_block,
      commands::complete_item,
      commands::record_execution,
      commands::list_layers,
      commands::set_layer_visibility,
      commands::get_setting,
      commands::set_setting,
      // Faculdade (SPEC-D-010, D74/D75)
      commands::list_semestres,
      commands::list_disciplinas,
      commands::media_da_disciplina,
      commands::list_avaliacoes,
      commands::create_semestre,
      commands::create_disciplina,
      commands::create_avaliacao,
      commands::lancar_nota,
      commands::set_regra_padrao,
      commands::set_regra_disciplina,
      commands::encerrar_semestre,
      // Estudos (SPEC-D-010, D76)
      commands::create_conteudo,
      commands::create_cartao,
      commands::fila_do_dia,
      commands::revisar_cartao,
      // TCC & Projetos (SPEC-D-013, D85..D88)
      commands::list_projetos,
      commands::create_projeto,
      commands::set_estado_projeto,
      commands::get_documento,
      commands::get_documento_do_projeto,
      commands::paginar_documento,
      commands::salvar_documento,
      commands::registrar_item,
      commands::resolver_item,
      commands::list_itens,
      commands::list_base,
      commands::vincular,
      commands::desvincular,
    ])
    .run(tauri::generate_context!())
    .expect("erro ao iniciar a aplicação Tauri");
}
