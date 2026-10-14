//! Casca do Tauri: abre o banco e registra os comandos.
//!
//! Este arquivo é deliberadamente fino. Ele sabe três coisas — onde fica o
//! `app_data_dir`, quais módulos existem e quais comandos cada um registra.
//! Nenhum dos módulos importa `tauri`.
//!
//! **O registro é a leitura da fronteira.** Os nomes vão com o caminho do módulo
//! que os possui (`calendario::comandos::list_events_in_window`), agrupados por
//! dono e comentados, de modo que os quatro donos do produto fiquem visíveis na
//! própria lista de IPC. A regra 9 de `check-boundaries.mjs` cruza esta lista com
//! as funções que existem: núcleo sem casca, casca sem registro, registro
//! apontando para nada e nome duplicado são falha do gate — porque divergência
//! aqui só aparece em runtime, na tela (SPEC-D-015 D97, I4).
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

pub mod calendario;
// O motor de notas (D74/D75) é puro e não toca SQL; é a API de domínio de
// Faculdade, e o caminho de escrita do módulo está em `faculdade/comandos.rs`
// (SPEC-D-015 F2). O diretório chama `faculdade`, e não `materia`, porque **o
// nome do diretório é o id do dono** (D97): o gate tira o dono de cada escrita
// de `DONOS_RUST`, e um diretório fora da lista é violação. O id já era
// `faculdade` em `DONO_DA_TABELA`, em `src/shell/navigation.ts` e na migration
// `0003_faculdade.sql` — a pasta era a única peça fora do padrão.
pub mod faculdade;
// O motor de repetição espaçada (D76, SPEC-D-010) segue o mesmo desenho: puro,
// com relógio injetado, e consumido pelo caminho de escrita do módulo de Estudos.
pub mod estudos;
// TCC & Projetos (SPEC-D-013, D85..D88): o modelo do documento, o motor de
// paginação puro e a lista de decisão da importação. Fonte de verdade ao lado da
// migração `0005_tcc.sql`; consumido pelo caminho de escrita em `tcc/comandos.rs`.
mod commands;
pub mod nucleo;
pub mod tcc;

// `nucleo::Db` só é usado pela casca; sem a feature ele compila, mas não é
// referenciado.
#[cfg(feature = "tauri-shell")]
use tauri::Manager;

#[cfg(feature = "tauri-shell")]
use nucleo::Db;

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
    // §3.4 + SPEC-D-010 + SPEC-D-013: os nomes registrados.
    //
    // **O caminho do módulo é o que torna esta lista a leitura da fronteira.**
    // Os quatro donos do produto aparecem em quatro blocos, e quem lê a lista
    // sabe quem é dono de qual comando sem abrir mais nada. Os nomes de
    // Calendário, Faculdade, Estudos e TCC têm par numa `data/bridge.ts`;
    // `get_setting` e `set_setting` ainda não têm chamador no front-end.
    //
    // Esta lista é a fronteira entre Rust e TypeScript: se um nome divergir,
    // o `invoke` falha em runtime, na tela, e nada acusa antes. A regra 9 de
    // `check-boundaries.mjs` cruza esta lista com as funções que existem, e
    // recusa registro sem casca, casca sem registro e nome duplicado.
    //
    // Estudos e TCC ainda apontam para `commands::`, que é o caminho em
    // migração: F3 e F4 dão a cada um o seu diretório, e F5 apaga
    // `commands.rs`.
    .invoke_handler(tauri::generate_handler![
      // Calendário (spec 01, §3.4)
      calendario::comandos::migrate,
      calendario::comandos::list_events_in_window,
      calendario::comandos::list_responsibilities,
      calendario::comandos::create_event,
      calendario::comandos::update_event,
      calendario::comandos::delete_event,
      calendario::comandos::set_occurrence_state,
      calendario::comandos::move_occurrence,
      calendario::comandos::upsert_recurrence,
      calendario::comandos::update_recurrence,
      calendario::comandos::create_responsibility,
      calendario::comandos::set_step_state,
      calendario::comandos::plan_block,
      calendario::comandos::reschedule_block,
      calendario::comandos::complete_item,
      calendario::comandos::record_execution,
      calendario::comandos::list_layers,
      calendario::comandos::set_layer_visibility,
      calendario::comandos::get_setting,
      calendario::comandos::set_setting,
      // Faculdade (SPEC-D-010, D74/D75)
      faculdade::comandos::list_semestres,
      faculdade::comandos::list_disciplinas,
      faculdade::comandos::media_da_disciplina,
      faculdade::comandos::list_avaliacoes,
      faculdade::comandos::create_semestre,
      faculdade::comandos::create_disciplina,
      faculdade::comandos::create_avaliacao,
      faculdade::comandos::lancar_nota,
      faculdade::comandos::set_regra_padrao,
      faculdade::comandos::set_regra_disciplina,
      faculdade::comandos::encerrar_semestre,
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
