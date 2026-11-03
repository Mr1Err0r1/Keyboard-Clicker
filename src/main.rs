#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use eframe::egui::Vec2;
use keyboard_clicker::app::ClickerApp;

fn main() -> eframe::Result<()> {
    let _ = env_logger::try_init();
    let options = eframe::NativeOptions {
        initial_window_size: Some(Vec2::new(940.0, 720.0)),
        min_window_size: Some(Vec2::new(760.0, 560.0)),
        ..Default::default()
    };
    eframe::run_native(
        "Keyboard Clicker",
        options,
        Box::new(|creation_context| Box::new(ClickerApp::new(creation_context))),
    )
}
