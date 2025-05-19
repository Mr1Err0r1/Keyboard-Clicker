// In Cargo.toml

[package]
name = "gaming-overlay"
version = "0.1.0"
edition = "2021"

[dependencies]
tauri = "1.5"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

[build-dependencies]
tauri-build = "1.5"

// In src/main.rs
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::{Manager, Window};

// Diese Funktion ändert die Transparenz eines Fensters
#[tauri::command]
fn set_window_transparency(window: Window, transparency: f64) {
    window.set_decorations(false).unwrap();
    let _ = window.set_transparent(true);
    if let Some(window) = window.get_window("main") {
        // Hier können weitere Anpassungen zur Transparenz gemacht werden
        println!("Transparenz auf {} gesetzt", transparency);
    }
}

// Diese Funktion wechselt das Farbthema
#[tauri::command]
fn change_theme(window: Window, theme: String) {
    if let Some(window) = window.get_window("main") {
        // Hier würde die Logik zum Wechseln des Themas stehen
        println!("Thema zu {} gewechselt", theme);
    }
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let main_window = app.get_window("main").unwrap();
            main_window.set_decorations(false).unwrap();
            main_window.set_transparent(true).unwrap();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            set_window_transparency,
            change_theme
        ])
        .run(tauri::generate_context!())
        .expect("Fehler beim Ausführen der Anwendung");
}
