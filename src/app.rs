use crate::{
    config,
    engine::{AutomationEngine, EngineState},
    i18n::Catalog,
    model::{Action, AppSettings, ClickType, MouseButton, SequenceStep},
    theme,
};
use eframe::egui::{self, Color32, RichText};
use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager};
use std::time::Duration;

#[derive(PartialEq, Eq, Clone, Copy)]
enum Page {
    Automation,
    Appearance,
    About,
}

pub struct ClickerApp {
    settings: AppSettings,
    catalog: Catalog,
    engine: AutomationEngine,
    state: EngineState,
    page: Page,
    message: Option<(String, bool)>,
    hotkey_manager: Option<GlobalHotKeyManager>,
    registered_hotkeys: Option<(HotKey, HotKey)>,
    start_hotkey_id: u32,
    stop_hotkey_id: u32,
    dirty: bool,
}

impl ClickerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let settings = config::load();
        let catalog = Catalog::load();
        theme::apply(&cc.egui_ctx, &settings.theme, &settings.accent_hex);
        let engine = AutomationEngine::new();
        let state = engine.state();
        let hotkey_manager = GlobalHotKeyManager::new().ok();
        let mut registered_hotkeys = None;
        let mut start_hotkey_id = 0;
        let mut stop_hotkey_id = 0;
        if let Some(manager) = &hotkey_manager {
            if let Ok((start, stop)) = parse_hotkey_pair(&settings) {
                if manager.register(start).is_ok() {
                    if manager.register(stop).is_ok() {
                        start_hotkey_id = start.id();
                        stop_hotkey_id = stop.id();
                        registered_hotkeys = Some((start, stop));
                    } else {
                        let _ = manager.unregister(start);
                    }
                }
            }
        }
        Self {
            settings,
            catalog,
            engine,
            state,
            page: Page::Automation,
            message: None,
            hotkey_manager,
            registered_hotkeys,
            start_hotkey_id,
            stop_hotkey_id,
            dirty: false,
        }
    }

    fn tr(&self, key: &str) -> String {
        self.catalog.text(&self.settings.locale, key)
    }
    fn start(&mut self) {
        match self.engine.start(self.settings.clone()) {
            Ok(()) => self.message = None,
            Err(error) => self.message = Some((error, true)),
        }
    }
    fn stop(&mut self) {
        self.engine.stop();
    }

    fn configure_hotkeys(&mut self) -> Result<(), String> {
        let (start, stop) = parse_hotkey_pair(&self.settings)?;
        let Some(manager) = self.hotkey_manager.as_ref() else {
            return Ok(());
        };
        if self
            .registered_hotkeys
            .map(|(old_start, old_stop)| (old_start.id(), old_stop.id()))
            == Some((start.id(), stop.id()))
        {
            return Ok(());
        }
        let previous = self.registered_hotkeys;
        if let Some((old_start, old_stop)) = previous {
            let _ = manager.unregister(old_start);
            let _ = manager.unregister(old_stop);
        }
        if let Err(error) = manager.register(start) {
            if let Some((old_start, old_stop)) = previous {
                let _ = manager.register(old_start);
                let _ = manager.register(old_stop);
            }
            return Err(format!("Could not register start hotkey: {error}"));
        }
        if let Err(error) = manager.register(stop) {
            let _ = manager.unregister(start);
            if let Some((old_start, old_stop)) = previous {
                let _ = manager.register(old_start);
                let _ = manager.register(old_stop);
            }
            return Err(format!("Could not register stop hotkey: {error}"));
        }
        self.start_hotkey_id = start.id();
        self.stop_hotkey_id = stop.id();
        self.registered_hotkeys = Some((start, stop));
        Ok(())
    }

    fn draw_automation(&mut self, ui: &mut egui::Ui) {
        let catalog = &self.catalog;
        let locale = self.settings.locale.clone();
        let tr = |key: &str| catalog.text(&locale, key);
        ui.horizontal(|ui| {
            ui.heading(tr("nav_automation"));
            let (label, color) = if self.state.is_running() {
                (tr("status_running"), Color32::from_rgb(71, 201, 135))
            } else {
                (tr("status_ready"), Color32::from_rgb(150, 160, 180))
            };
            ui.label(RichText::new(format!("● {label}")).color(color).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.state.is_running() {
                    if ui.button(tr("stop")).clicked() {
                        self.engine.stop();
                    }
                } else if ui.button(tr("start")).clicked() {
                    if let Err(error) = self.engine.start(self.settings.clone()) {
                        self.message = Some((error, true));
                    }
                }
            });
        });
        ui.add_space(4.0);
        ui.label(format!(
            "{}: {}",
            tr("status_cycles"),
            self.state.completed_cycles()
        ));
        ui.add_space(14.0);
        let locked = self.state.is_running();
        ui.add_enabled_ui(!locked, |ui| {
            egui::Grid::new("cycle_settings")
                .num_columns(2)
                .spacing([14.0, 10.0])
                .show(ui, |ui| {
                    ui.label(tr("cycle_interval"));
                    if ui
                        .add(
                            egui::DragValue::new(&mut self.settings.interval_ms)
                                .clamp_range(1..=3_600_000)
                                .suffix(" ms"),
                        )
                        .changed()
                    {
                        self.dirty = true;
                    }
                    ui.end_row();
                    ui.label(tr("repeat_count"));
                    if ui
                        .add(
                            egui::DragValue::new(&mut self.settings.repeat_count)
                                .clamp_range(0..=1_000_000_000_u64),
                        )
                        .changed()
                    {
                        self.dirty = true;
                    }
                    ui.end_row();
                });
        });
        ui.add_space(14.0);
        ui.separator();
        ui.horizontal(|ui| {
            ui.heading(tr("sequence_title"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_enabled_ui(!locked, |ui| {
                    if ui.button(tr("add_wait")).clicked() {
                        self.settings.sequence.push(SequenceStep {
                            action: Action::Wait { duration_ms: 250 },
                            ..Default::default()
                        });
                        self.dirty = true;
                    }
                    if ui.button(tr("add_text")).clicked() {
                        self.settings.sequence.push(SequenceStep {
                            action: Action::TypeText {
                                text: String::new(),
                            },
                            ..Default::default()
                        });
                        self.dirty = true;
                    }
                    if ui.button(tr("add_key")).clicked() {
                        self.settings.sequence.push(SequenceStep {
                            action: Action::KeyPress {
                                key: "Space".into(),
                                hold_ms: 35,
                            },
                            ..Default::default()
                        });
                        self.dirty = true;
                    }
                    if ui.button(tr("add_click")).clicked() {
                        self.settings.sequence.push(SequenceStep::default());
                        self.dirty = true;
                    }
                });
            });
        });
        ui.label(RichText::new(tr("sequence_hint")).weak());
        ui.add_space(8.0);
        let mut remove_index = None;
        let count = self.settings.sequence.len();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for index in 0..count {
                    let mut move_up = false;
                    let mut move_down = false;
                    let mut remove = false;
                    let step = &mut self.settings.sequence[index];
                    egui::CollapsingHeader::new(format!("{} {:02}", tr("step"), index + 1))
                        .id_source(("step", index))
                        .default_open(true)
                        .show(ui, |ui| {
                            ui.add_enabled_ui(!locked, |ui| {
                                ui.horizontal(|ui| {
                                    if ui.checkbox(&mut step.enabled, tr("enabled")).changed() {
                                        self.dirty = true;
                                    }
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui.button(tr("remove")).clicked() {
                                                remove = true;
                                            }
                                            if index + 1 < count
                                                && ui
                                                    .button("↓")
                                                    .on_hover_text(tr("move_down"))
                                                    .clicked()
                                            {
                                                move_down = true;
                                            }
                                            if index > 0
                                                && ui
                                                    .button("↑")
                                                    .on_hover_text(tr("move_up"))
                                                    .clicked()
                                            {
                                                move_up = true;
                                            }
                                        },
                                    );
                                });
                                let current = match &step.action {
                                    Action::MouseClick { .. } => 0,
                                    Action::KeyPress { .. } => 1,
                                    Action::TypeText { .. } => 2,
                                    Action::Wait { .. } => 3,
                                };
                                let mut selected = current;
                                ui.horizontal(|ui| {
                                    egui::ComboBox::from_id_source(("action", index))
                                        .selected_text(match current {
                                            0 => tr("add_click"),
                                            1 => tr("add_key"),
                                            2 => tr("add_text"),
                                            _ => tr("add_wait"),
                                        })
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(&mut selected, 0, tr("add_click"));
                                            ui.selectable_value(&mut selected, 1, tr("add_key"));
                                            ui.selectable_value(&mut selected, 2, tr("add_text"));
                                            ui.selectable_value(&mut selected, 3, tr("add_wait"));
                                        });
                                });
                                if selected != current {
                                    step.action = match selected {
                                        0 => Action::default(),
                                        1 => Action::KeyPress {
                                            key: "Space".into(),
                                            hold_ms: 35,
                                        },
                                        2 => Action::TypeText {
                                            text: String::new(),
                                        },
                                        _ => Action::Wait { duration_ms: 250 },
                                    };
                                    self.dirty = true;
                                }
                                match &mut step.action {
                                    Action::MouseClick {
                                        button,
                                        click_type,
                                        hold_ms,
                                        position,
                                    } => {
                                        ui.horizontal(|ui| {
                                            ui.label(tr("button"));
                                            egui::ComboBox::from_id_source(("button", index))
                                                .selected_text(match button {
                                                    MouseButton::Left => "Left",
                                                    MouseButton::Right => "Right",
                                                    MouseButton::Middle => "Middle",
                                                })
                                                .show_ui(ui, |ui| {
                                                    if ui
                                                        .selectable_value(
                                                            button,
                                                            MouseButton::Left,
                                                            "Left",
                                                        )
                                                        .changed()
                                                    {
                                                        self.dirty = true;
                                                    }
                                                    if ui
                                                        .selectable_value(
                                                            button,
                                                            MouseButton::Right,
                                                            "Right",
                                                        )
                                                        .changed()
                                                    {
                                                        self.dirty = true;
                                                    }
                                                    if ui
                                                        .selectable_value(
                                                            button,
                                                            MouseButton::Middle,
                                                            "Middle",
                                                        )
                                                        .changed()
                                                    {
                                                        self.dirty = true;
                                                    }
                                                });
                                            ui.label(tr("click_type"));
                                            egui::ComboBox::from_id_source(("click_type", index))
                                                .selected_text(match click_type {
                                                    ClickType::Single => tr("single"),
                                                    ClickType::Double => tr("double"),
                                                    ClickType::Hold => tr("hold"),
                                                })
                                                .show_ui(ui, |ui| {
                                                    if ui
                                                        .selectable_value(
                                                            click_type,
                                                            ClickType::Single,
                                                            tr("single"),
                                                        )
                                                        .changed()
                                                    {
                                                        self.dirty = true;
                                                    }
                                                    if ui
                                                        .selectable_value(
                                                            click_type,
                                                            ClickType::Double,
                                                            tr("double"),
                                                        )
                                                        .changed()
                                                    {
                                                        self.dirty = true;
                                                    }
                                                    if ui
                                                        .selectable_value(
                                                            click_type,
                                                            ClickType::Hold,
                                                            tr("hold"),
                                                        )
                                                        .changed()
                                                    {
                                                        self.dirty = true;
                                                    }
                                                });
                                        });
                                        if *click_type == ClickType::Hold
                                            && ui
                                                .add(
                                                    egui::DragValue::new(hold_ms)
                                                        .clamp_range(1..=86_400_000)
                                                        .prefix(format!("{}: ", tr("hold_ms"))),
                                                )
                                                .changed()
                                        {
                                            self.dirty = true;
                                        }
                                        let mut fixed = position.is_some();
                                        if ui.checkbox(&mut fixed, tr("fixed_position")).changed() {
                                            *position = if fixed { Some([0, 0]) } else { None };
                                            self.dirty = true;
                                        }
                                        if let Some(coords) = position {
                                            ui.horizontal(|ui| {
                                                ui.label(tr("x"));
                                                if ui
                                                    .add(egui::DragValue::new(&mut coords[0]))
                                                    .changed()
                                                {
                                                    self.dirty = true;
                                                }
                                                ui.label(tr("y"));
                                                if ui
                                                    .add(egui::DragValue::new(&mut coords[1]))
                                                    .changed()
                                                {
                                                    self.dirty = true;
                                                }
                                            });
                                        } else {
                                            ui.label(tr("cursor"));
                                        }
                                    }
                                    Action::KeyPress { key, hold_ms } => {
                                        ui.horizontal(|ui| {
                                            ui.label(tr("key"));
                                            if ui.text_edit_singleline(key).changed() {
                                                self.dirty = true;
                                            }
                                            if ui
                                                .add(
                                                    egui::DragValue::new(hold_ms)
                                                        .clamp_range(1..=60_000)
                                                        .prefix(format!("{}: ", tr("hold_ms"))),
                                                )
                                                .changed()
                                            {
                                                self.dirty = true;
                                            }
                                        });
                                        ui.label(RichText::new(tr("key_hint")).weak());
                                    }
                                    Action::TypeText { text } => {
                                        ui.label(tr("text"));
                                        if ui.text_edit_multiline(text).changed() {
                                            self.dirty = true;
                                        }
                                    }
                                    Action::Wait { duration_ms } => {
                                        if ui
                                            .add(
                                                egui::DragValue::new(duration_ms)
                                                    .clamp_range(0..=86_400_000)
                                                    .prefix(format!("{}: ", tr("wait_ms"))),
                                            )
                                            .changed()
                                        {
                                            self.dirty = true;
                                        }
                                    }
                                }
                                ui.horizontal(|ui| {
                                    ui.label(tr("delay_after"));
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut step.delay_after_ms)
                                                .clamp_range(0..=86_400_000)
                                                .suffix(" ms"),
                                        )
                                        .changed()
                                    {
                                        self.dirty = true;
                                    }
                                });
                            });
                        });
                    if remove {
                        remove_index = Some(index);
                    }
                    if move_up {
                        self.settings.sequence.swap(index, index - 1);
                        self.dirty = true;
                    }
                    if move_down {
                        self.settings.sequence.swap(index, index + 1);
                        self.dirty = true;
                    }
                }
            });
        if let Some(index) = remove_index {
            self.settings.sequence.remove(index);
            self.dirty = true;
        }
    }

    fn draw_appearance(&mut self, ui: &mut egui::Ui) {
        let catalog = &self.catalog;
        let locale = self.settings.locale.clone();
        let tr = |key: &str| catalog.text(&locale, key);
        ui.heading(tr("appearance_title"));
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.label(tr("language"));
            egui::ComboBox::from_id_source("locale")
                .selected_text(self.catalog.text(&self.settings.locale, "language_name"))
                .show_ui(ui, |ui| {
                    for (code, name) in self.catalog.languages() {
                        if ui
                            .selectable_value(&mut self.settings.locale, code, name)
                            .changed()
                        {
                            self.dirty = true;
                        }
                    }
                });
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(tr("theme"));
            if ui
                .selectable_value(&mut self.settings.theme, "dark".into(), tr("dark"))
                .changed()
            {
                self.dirty = true;
            }
            if ui
                .selectable_value(&mut self.settings.theme, "light".into(), tr("light"))
                .changed()
            {
                self.dirty = true;
            }
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(tr("accent"));
            if ui
                .text_edit_singleline(&mut self.settings.accent_hex)
                .changed()
            {
                self.dirty = true;
            }
            if let Some(color) = theme::parse_color(&self.settings.accent_hex) {
                ui.colored_label(color, "●");
            }
        });
        ui.add_space(12.0);
        ui.label(RichText::new(tr("appearance_hint")).weak());
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.label(tr("start_hotkey"));
            if ui
                .text_edit_singleline(&mut self.settings.start_hotkey)
                .changed()
            {
                self.dirty = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label(tr("stop_hotkey"));
            if ui
                .text_edit_singleline(&mut self.settings.stop_hotkey)
                .changed()
            {
                self.dirty = true;
            }
        });
        ui.label(RichText::new(tr("hotkey_hint")).weak());
        ui.add_space(8.0);
        ui.label(tr("updating"));
    }

    fn draw_about(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.tr("about_title"));
        ui.add_space(12.0);
        ui.label(self.tr("about_text"));
        ui.add_space(12.0);
        ui.label(format!(
            "{}: {} / {}",
            self.tr("hotkeys"),
            self.settings.start_hotkey,
            self.settings.stop_hotkey
        ));
        ui.label("Version 2.0.0 · Rust · Windows 10/11 and Linux");
        ui.hyperlink_to(
            "GitHub project",
            "https://github.com/Mr1Err0r1/Keyboard-Clicker",
        );
    }
}

impl eframe::App for ClickerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let _keep_hotkeys_alive = &self.hotkey_manager;
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if event.id == self.start_hotkey_id {
                self.start();
            }
            if event.id == self.stop_hotkey_id {
                self.stop();
            }
        }
        theme::apply(ctx, &self.settings.theme, &self.settings.accent_hex);
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(RichText::new(self.tr("app_title")).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.state.is_running() {
                        ui.colored_label(
                            Color32::from_rgb(71, 201, 135),
                            format!("● {}", self.tr("status_running")),
                        );
                    } else {
                        ui.label(format!("● {}", self.tr("status_ready")));
                    }
                });
            });
        });
        egui::SidePanel::left("navigation")
            .resizable(false)
            .default_width(155.0)
            .show(ctx, |ui| {
                ui.add_space(12.0);
                for (page, key) in [
                    (Page::Automation, "nav_automation"),
                    (Page::Appearance, "nav_appearance"),
                    (Page::About, "nav_about"),
                ] {
                    if ui
                        .selectable_label(self.page == page, self.tr(key))
                        .clicked()
                    {
                        self.page = page;
                    }
                }
                ui.add_space(20.0);
                ui.separator();
                ui.label(
                    RichText::new(format!(
                        "{}  {}",
                        self.settings.start_hotkey,
                        self.tr("start")
                    ))
                    .weak(),
                );
                ui.label(
                    RichText::new(format!(
                        "{}  {}",
                        self.settings.stop_hotkey,
                        self.tr("stop")
                    ))
                    .weak(),
                );
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    match self.page {
                        Page::Automation => self.draw_automation(ui),
                        Page::Appearance => self.draw_appearance(ui),
                        Page::About => self.draw_about(ui),
                    }
                    if let Some((message, is_error)) = &self.message {
                        ui.add_space(12.0);
                        if *is_error {
                            ui.colored_label(
                                Color32::LIGHT_RED,
                                format!("{}: {message}", self.tr("error")),
                            );
                        } else {
                            ui.weak(message);
                        }
                    }
                });
        });
        if theme::parse_color(&self.settings.accent_hex).is_none() {
            self.message = Some(("Use a 6-digit HEX color, for example #6C8CFF.".into(), true));
        }
        if self.dirty && theme::parse_color(&self.settings.accent_hex).is_some() {
            let hotkeys_result = self.configure_hotkeys();
            match hotkeys_result
                .and_then(|()| config::save(&self.settings).map_err(|error| error.to_string()))
            {
                Ok(()) => {
                    self.message = None;
                    self.dirty = false;
                }
                Err(error) => {
                    self.message = Some((error, true));
                    self.dirty = false;
                }
            }
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

fn parse_hotkey_pair(settings: &AppSettings) -> Result<(HotKey, HotKey), String> {
    let start = HotKey::try_from(settings.start_hotkey.trim())
        .map_err(|error| format!("Invalid start hotkey: {error}"))?;
    let stop = HotKey::try_from(settings.stop_hotkey.trim())
        .map_err(|error| format!("Invalid stop hotkey: {error}"))?;
    if start.id() == stop.id() {
        return Err("Start and stop hotkeys must be different.".into());
    }
    Ok((start, stop))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_modifier_hotkeys_and_rejects_duplicates() {
        let settings = AppSettings {
            start_hotkey: "Ctrl+F6".into(),
            stop_hotkey: "Alt+F7".into(),
            ..Default::default()
        };
        assert!(parse_hotkey_pair(&settings).is_ok());

        let duplicate = AppSettings {
            start_hotkey: "F6".into(),
            stop_hotkey: "F6".into(),
            ..Default::default()
        };
        assert!(parse_hotkey_pair(&duplicate).is_err());
    }
}
