//! Casca do Tauri: abre o banco e registra os comandos do Calendário.
//!
//! Este arquivo é deliberadamente fino. Ele sabe duas coisas — onde fica o
//! `app_data_dir` e quais comandos existem. A regra de domínio está em
//! `commands.rs`, e a conexão em `db.rs`; nenhum dos dois importa `tauri`.

mod commands;
mod db;

use tauri::Manager;

use db::Db;

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
        // §3.4: os mesmos 20 nomes que `data/bridge.ts` invoca. A lista é a
        // fronteira entre Rust e TypeScript — se um nome divergir, o `invoke`
        // falha em runtime, então qualquer mudança aqui muda lá também.
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
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar a aplicação Tauri");
}
