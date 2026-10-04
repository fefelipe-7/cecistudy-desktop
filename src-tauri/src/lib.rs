//! Casca do Tauri: abre o banco e registra os comandos do Calendário.
//!
//! Este arquivo é deliberadamente fino. Ele sabe duas coisas — onde fica o
//! `app_data_dir` e quais comandos existem. A regra de domínio está em
//! `commands.rs`, e a conexão em `db.rs`; nenhum dos dois importa `tauri`.

mod commands;
mod db;

use tauri::Manager;

use db::{StoreAcademico, StoreClinico, StoreSalaTreino};

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

            // Os bancos são criados no primeiro `setup`. Se o diretório ou o schema
            // falhar, o app não sobe em um estado meio inicializado: é melhor erro
            // na abertura do que tela vazia sem persistência.
            let dir = app.path().app_data_dir()?;

            // `SPEC-D-013` `D88`: três stores, três tipos, três arquivos. O
            // `manage` só aceita tipos distintos, então trocar um pelo outro em
            // runtime não compila — a regra do §1.6 linha 23 é imposta pelo
            // compilador, e não por uma lista de prefixo de tabela.
            let academico = StoreAcademico::open_in(&dir)?;
            let clinico = StoreClinico::open_in(&dir)?;
            let sala_treino = StoreSalaTreino::open_in(&dir)?;

            log::info!(
                "SQLite pronto em {} (acadêmico {} v{}, clínico {}, Sala de treino {})",
                dir.display(),
                academico.arquivo(),
                academico.schema_version().unwrap_or(0),
                clinico.arquivo(),
                sala_treino.arquivo(),
            );

            app.manage(academico);
            app.manage(clinico);
            app.manage(sala_treino);

            Ok(())
        })
        // §3.4: a fronteira entre Rust e TypeScript. `SPEC-D-013` `D85` diz que
        // esta lista, o manifest em `contracts/*.json` e o que o TypeScript invoca
        // são **três conjuntos que têm de ser iguais**, e `npm run gate:contracts`
        // falha se divergirem em qualquer sentido. comentário antigo ("os mesmos 20
        // nomes") estava errado: são 21.
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
            commands::set_item_state,
            commands::record_execution,
            commands::list_layers,
            commands::set_layer_visibility,
            commands::get_setting,
            commands::set_setting,
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar a aplicação Tauri");
}
