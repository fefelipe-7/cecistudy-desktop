// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// A casca só existe com a feature `tauri-shell`. Sem ela — que é como
// `cargo test --lib --no-default-features` compila — o binário é compilado
// apenas para satisfazer o alvo, e não sobe nada.
#[cfg(feature = "tauri-shell")]
fn main() {
  cecistudy_lib::run();
}

#[cfg(not(feature = "tauri-shell"))]
fn main() {}
