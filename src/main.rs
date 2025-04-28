use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::{Duration, Instant};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

use eframe::{egui, epi};
use egui::{Vec2, Color32, TextEdit, ComboBox, DragValue, Slider};
use enigo::{Enigo, MouseControllable, MouseButton, KeyboardControllable, Key};
use global_hotkey::{GlobalHotKeyManager, HotKeyState, hotkey::{HotKey, Modifiers, Code}};
use device_query::{DeviceQuery, DeviceState, Keycode};

struct AutoClicker {
    running: Arc<AtomicBool>,
    mouse_button: MouseButtonSelection,
    click_type: ClickTypeSelection,
    cursor_position: CursorPositionSelection,
    x_pos: i32,
    y_pos: i32,
    repeat_mode: RepeatModeSelection,
    repeat_count: i32,
    
    // Intervals
    interval_hours: i32,
    interval_minutes: i32,
    interval_seconds: i32,
    interval_milliseconds: i32,
    
    // Holding time
    holding_time_hours: i32,
    holding_time_minutes: i32,
    holding_time_seconds: i32,
    holding_time_milliseconds: i32,
    
    // Key sections
    key_sections: Vec<KeySection>,
    expanded_section: Option<usize>,
    
    // Hotkeys
    start_hotkey: String,
    stop_hotkey: String,
    picking_key: bool,
    picking_location: bool,
    show_help: bool,
    show_hotkey_settings: bool,
    
    // Thread handles
    click_thread: Option<thread::JoinHandle<()>>,
    key_threads: Vec<thread::JoinHandle<()>>,
}

struct KeySection {
    key: String,
    expanded: bool,
    interval_hours: i32,
    interval_minutes: i32,
    interval_seconds: i32,
    interval_milliseconds: i32,
}

#[derive(PartialEq, Clone, Copy)]
enum MouseButtonSelection {
    Left,
    Right,
    Middle,
}

impl MouseButtonSelection {
    fn to_mouse_button(&self) -> MouseButton {
        match self {
            MouseButtonSelection::Left => MouseButton::Left,
            MouseButtonSelection::Right => MouseButton::Right,
            MouseButtonSelection::Middle => MouseButton::Middle,
        }
    }
    
    fn as_str(&self) -> &'static str {
        match self {
            MouseButtonSelection::Left => "Left",
            MouseButtonSelection::Right => "Right",
            MouseButtonSelection::Middle => "Middle",
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
enum ClickTypeSelection {
    Single,
    Double,
    Hold,
}

impl ClickTypeSelection {
    fn as_str(&self) -> &'static str {
        match self {
            ClickTypeSelection::Single => "Single",
            ClickTypeSelection::Double => "Double",
            ClickTypeSelection::Hold => "Hold",
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
enum CursorPositionSelection {
    Current,
    Pick,
}

impl CursorPositionSelection {
    fn as_str(&self) -> &'static str {
        match self {
            CursorPositionSelection::Current => "Current",
            CursorPositionSelection::Pick => "Pick",
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
enum RepeatModeSelection {
    UntilStopped,
    Count,
}

impl RepeatModeSelection {
    fn as_str(&self) -> &'static str {
        match self {
            RepeatModeSelection::UntilStopped => "Until stopped",
            RepeatModeSelection::Count => "Count",
        }
    }
}

impl Default for AutoClicker {
    fn default() -> Self {
        let mut clicker = Self {
            running: Arc::new(AtomicBool::new(false)),
            mouse_button: MouseButtonSelection::Left,
            click_type: ClickTypeSelection::Single,
            cursor_position: CursorPositionSelection::Current,
            x_pos: 0,
            y_pos: 0,
            repeat_mode: RepeatModeSelection::UntilStopped,
            repeat_count: 1,
            
            interval_hours: 0,
            interval_minutes: 0,
            interval_seconds: 0,
            interval_milliseconds: 100,
            
            holding_time_hours: 0,
            holding_time_minutes: 0,
            holding_time_seconds: 10,
            holding_time_milliseconds: 0,
            
            key_sections: vec![KeySection {
                key: String::new(),
                expanded: false,
                interval_hours: 0,
                interval_minutes: 0,
                interval_seconds: 0,
                interval_milliseconds: 100,
            }],
            expanded_section: None,
            
            start_hotkey: "F6".to_string(),
            stop_hotkey: "F7".to_string(),
            picking_key: false,
            picking_location: false,
            show_help: false,
            show_hotkey_settings: false,
            
            click_thread: None,
            key_threads: Vec::new(),
        };
        
        clicker.load_hotkeys();
        clicker
    }
}

impl AutoClicker {
    fn get_interval_ms(&self) -> u64 {
        self.interval_hours as u64 * 3_600_000 +
        self.interval_minutes as u64 * 60_000 +
        self.interval_seconds as u64 * 1_000 +
        self.interval_milliseconds as u64
    }
    
    fn get_holding_time_ms(&self) -> u64 {
        let holding_time = self.holding_time_hours as u64 * 3_600_000 +
                          self.holding_time_minutes as u64 * 60_000 +
                          self.holding_time_seconds as u64 * 1_000 +
                          self.holding_time_milliseconds as u64;
        
        if holding_time == 0 {
            500 // Default to 500ms if not set
        } else {
            holding_time
        }
    }
    
    fn get_section_interval_ms(&self, section_index: usize) -> u64 {
        if let Some(section) = self.key_sections.get(section_index) {
            section.interval_hours as u64 * 3_600_000 +
            section.interval_minutes as u64 * 60_000 +
            section.interval_seconds as u64 * 1_000 +
            section.interval_milliseconds as u64
        } else {
            100 // Default fallback
        }
    }
    
    fn start_clicking(&mut self) {
        if self.running.load(Ordering::SeqCst) {
            return; // Already running
        }
        
        self.running.store(true, Ordering::SeqCst);
        
        // Start mouse click thread
        let running = self.running.clone();
        let mouse_button = self.mouse_button;
        let click_type = self.click_type;
        let cursor_position = self.cursor_position;
        let x_pos = self.x_pos;
        let y_pos = self.y_pos;
        let repeat_mode = self.repeat_mode;
        let repeat_count = self.repeat_count;
        let interval_ms = self.get_interval_ms();
        let holding_time_ms = self.get_holding_time_ms();
        
        self.click_thread = Some(thread::spawn(move || {
            let mut enigo = Enigo::new();
            let mut repeat_counter = 0;
            let max_repeats = match repeat_mode {
                RepeatModeSelection::UntilStopped => i32::MAX,
                RepeatModeSelection::Count => repeat_count,
            };
            
            while running.load(Ordering::SeqCst) && repeat_counter < max_repeats {
                // Position cursor if needed
                if cursor_position == CursorPositionSelection::Pick {
                    enigo.mouse_move_to(x_pos, y_pos);
                }
                
                // Perform click based on click type
                match click_type {
                    ClickTypeSelection::Single => {
                        enigo.mouse_click(mouse_button.to_mouse_button());
                    },
                    ClickTypeSelection::Double => {
                        enigo.mouse_click(mouse_button.to_mouse_button());
                        thread::sleep(Duration::from_millis(10)); // Small delay between clicks
                        enigo.mouse_click(mouse_button.to_mouse_button());
                    },
                    ClickTypeSelection::Hold => {
                        enigo.mouse_down(mouse_button.to_mouse_button());
                        thread::sleep(Duration::from_millis(holding_time_ms));
                        enigo.mouse_up(mouse_button.to_mouse_button());
                    },
                }
                
                // Increment counter
                repeat_counter += 1;
                
                // Break if we've reached max repeats
                if repeat_counter >= max_repeats {
                    running.store(false, Ordering::SeqCst);
                    break;
                }
                
                // Sleep for interval
                if running.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(interval_ms));
                }
            }
        }));
        
        // Start key press threads for each key section
        self.key_threads.clear();
        
        for (i, section) in self.key_sections.iter().enumerate() {
            if !section.key.is_empty() {
                let running = self.running.clone();
                let key = section.key.clone();
                let repeat_mode = self.repeat_mode;
                let repeat_count = self.repeat_count;
                let interval_ms = self.get_section_interval_ms(i);
                
                let key_thread = thread::spawn(move || {
                    let mut enigo = Enigo::new();
                    let mut repeat_counter = 0;
                    let max_repeats = match repeat_mode {
                        RepeatModeSelection::UntilStopped => i32::MAX,
                        RepeatModeSelection::Count => repeat_count,
                    };
                    
                    // Try to parse the key
                    while running.load(Ordering::SeqCst) && repeat_counter < max_repeats {
                        // Press and release the key
                        if let Some(key_code) = Self::string_to_key(&key) {
                            enigo.key_click(key_code);
                        } else {
                            // Try as sequence of characters
                            for c in key.chars() {
                                enigo.key_sequence(&c.to_string());
                            }
                        }
                        
                        // Increment counter
                        repeat_counter += 1;
                        
                        // Break if we've reached max repeats
                        if repeat_counter >= max_repeats {
                            running.store(false, Ordering::SeqCst);
                            break;
                        }
                        
                        // Sleep for interval
                        if running.load(Ordering::SeqCst) {
                            thread::sleep(Duration::from_millis(interval_ms));
                        }
                    }
                });
                
                self.key_threads.push(key_thread);
            }
        }
    }
    
    fn stop_clicking(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        
        // Wait for threads to finish
        if let Some(thread) = self.click_thread.take() {
            let _ = thread.join();
        }
        
        for thread in self.key_threads.drain(..) {
            let _ = thread.join();
        }
    }
    
    fn add_key_section(&mut self) {
        if self.key_sections.len() >= 3 {
            return; // Maximum of 3 key sections
        }
        
        self.key_sections.push(KeySection {
            key: String::new(),
            expanded: false,
            interval_hours: 0,
            interval_minutes: 0,
            interval_seconds: 0,
            interval_milliseconds: 100,
        });
    }
    
    fn pick_location(&mut self) {
        if !self.picking_location {
            self.picking_location = true;
        }
    }
    
    fn update_picked_location(&mut self) {
        if self.picking_location {
            let device_state = DeviceState::new();
            let mouse_pos = device_state.get_mouse().coords;
            
            self.x_pos = mouse_pos.0;
            self.y_pos = mouse_pos.1;
            
            // Check for ESC key to finish picking
            let keys: Vec<Keycode> = device_state.get_keys();
            if keys.contains(&Keycode::Escape) {
                self.picking_location = false;
                self.cursor_position = CursorPositionSelection::Pick;
            }
        }
    }
    
    fn pick_key(&mut self, section_index: usize) {
        self.picking_key = true;
        self.expanded_section = Some(section_index);
    }
    
    fn update_picked_key(&mut self) {
        if self.picking_key && self.expanded_section.is_some() {
            let section_index = self.expanded_section.unwrap();
            let device_state = DeviceState::new();
            let keys: Vec<Keycode> = device_state.get_keys();
            
            if !keys.is_empty() {
                if let Some(section) = self.key_sections.get_mut(section_index) {
                    section.key = format!("{:?}", keys[0]).replace("Key", "");
                }
                self.picking_key = false;
            }
        }
    }
    
    fn load_hotkeys(&mut self) {
        if let Ok(mut file) = File::open("hotkeys.cfg") {
            let mut contents = String::new();
            if file.read_to_string(&mut contents).is_ok() {
                for line in contents.lines() {
                    if let Some((key, value)) = line.split_once('=') {
                        match key {
                            "start" => self.start_hotkey = value.to_string(),
                            "stop" => self.stop_hotkey = value.to_string(),
                            _ => {}
                        }
                    }
                }
            }
        }
    }
    
    fn save_hotkeys(&self) {
        if let Ok(mut file) = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open("hotkeys.cfg") {
            
            let contents = format!(
                "start={}\nstop={}\n",
                self.start_hotkey,
                self.stop_hotkey
            );
            
            let _ = file.write_all(contents.as_bytes());
        }
    }
    
    fn string_to_key(key_str: &str) -> Option<Key> {
        match key_str.to_lowercase().as_str() {
            "space" => Some(Key::Space),
            "up" => Some(Key::UpArrow),
            "down" => Some(Key::DownArrow),
            "left" => Some(Key::LeftArrow),
            "right" => Some(Key::RightArrow),
            "return" | "enter" => Some(Key::Return),
            "tab" => Some(Key::Tab),
            "backspace" => Some(Key::Backspace),
            "escape" | "esc" => Some(Key::Escape),
            "home" => Some(Key::Home),
            "end" => Some(Key::End),
            "pageup" => Some(Key::PageUp),
            "pagedown" => Some(Key::PageDown),
            "delete" | "del" => Some(Key::Delete),
            "f1" => Some(Key::F1),
            "f2" => Some(Key::F2),
            "f3" => Some(Key::F3),
            "f4" => Some(Key::F4),
            "f5" => Some(Key::F5),
            "f6" => Some(Key::F6),
            "f7" => Some(Key::F7),
            "f8" => Some(Key::F8),
            "f9" => Some(Key::F9),
            "f10" => Some(Key::F10),
            "f11" => Some(Key::F11),
            "f12" => Some(Key::F12),
            "shift" => Some(Key::Shift),
            "control" | "ctrl" => Some(Key::Control),
            "alt" => Some(Key::Alt),
            "capslock" => Some(Key::CapsLock),
            _ => None,
        }
    }
}

impl epi::App for AutoClicker {
    fn name(&self) -> &str {
        "ZEROGREAD Auto-Keyboard Clicker"
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &epi::Frame) {
        // Handle location picking
        if self.picking_location {
            self.update_picked_location();
        }
        
        // Handle key picking
        if self.picking_key {
            self.update_picked_key();
        }
        
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("ZEROGREAD Auto-Keyboard Clicker");
            ui.add_space(10.0);
            
            // Interval section
            egui::CollapsingHeader::new("Click interval")
                .default_open(true)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(DragValue::new(&mut self.interval_hours).clamp_range(0..=23).speed(0.1));
                        ui.label("hours");
                        ui.add_space(10.0);
                        
                        ui.add(DragValue::new(&mut self.interval_minutes).clamp_range(0..=59).speed(0.1));
                        ui.label("mins");
                        ui.add_space(10.0);
                        
                        ui.add(DragValue::new(&mut self.interval_seconds).clamp_range(0..=59).speed(0.1));
                        ui.label("secs");
                        ui.add_space(10.0);
                        
                        ui.add(DragValue::new(&mut self.interval_milliseconds).clamp_range(0..=999).speed(1.0));
                        ui.label("ms");
                    });
                });
            
            // Click options section
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    egui::CollapsingHeader::new("Click options")
                        .default_open(true)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Mouse button:");
                                ComboBox::from_id_source("mouse_button")
                                    .selected_text(self.mouse_button.as_str())
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.mouse_button, MouseButtonSelection::Left, "Left");
                                        ui.selectable_value(&mut self.mouse_button, MouseButtonSelection::Right, "Right");
                                        ui.selectable_value(&mut self.mouse_button, MouseButtonSelection::Middle, "Middle");
                                    });
                            });
                            
                            ui.horizontal(|ui| {
                                ui.label("Click type:");
                                ComboBox::from_id_source("click_type")
                                    .selected_text(self.click_type.as_str())
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.click_type, ClickTypeSelection::Single, "Single");
                                        ui.selectable_value(&mut self.click_type, ClickTypeSelection::Double, "Double");
                                        ui.selectable_value(&mut self.click_type, ClickTypeSelection::Hold, "Hold");
                                    });
                            });
                        });
                });
                
                ui.add_space(10.0);
                
                ui.vertical(|ui| {
                    egui::CollapsingHeader::new("Click repeat")
                        .default_open(true)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.radio_value(&mut self.repeat_mode, RepeatModeSelection::Count, "Repeat");
                                if self.repeat_mode == RepeatModeSelection::Count {
                                    ui.add(DragValue::new(&mut self.repeat_count).clamp_range(1..=9999));
                                    ui.label("times");
                                }
                            });
                            
                            ui.radio_value(&mut self.repeat_mode, RepeatModeSelection::UntilStopped, "Repeat until stopped");
                        });
                });
            });
            
            // Holding time section (when "Hold" is selected)
            if self.click_type == ClickTypeSelection::Hold {
                ui.add_space(10.0);
                egui::CollapsingHeader::new("Holding Time")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.add(DragValue::new(&mut self.holding_time_hours).clamp_range(0..=23).speed(0.1));
                            ui.label("hours");
                            ui.add_space(10.0);
                            
                            ui.add(DragValue::new(&mut self.holding_time_minutes).clamp_range(0..=59).speed(0.1));
                            ui.label("mins");
                            ui.add_space(10.0);
                            
                            ui.add(DragValue::new(&mut self.holding_time_seconds).clamp_range(0..=59).speed(0.1));
                            ui.label("secs");
                            ui.add_space(10.0);
                            
                            ui.add(DragValue::new(&mut self.holding_time_milliseconds).clamp_range(0..=999).speed(1.0));
                            ui.label("ms");
                        });
                    });
            }
            
            // Cursor position section
            ui.add_space(10.0);
            egui::CollapsingHeader::new("Cursor position")
                .default_open(true)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut self.cursor_position, CursorPositionSelection::Current, "Current location");
                        ui.radio_value(&mut self.cursor_position, CursorPositionSelection::Pick, "");
                        
                        if ui.button("Pick location").clicked() {
                            self.pick_location();
                        }
                        
                        ui.label("X");
                        ui.add(DragValue::new(&mut self.x_pos));
                        ui.label("Y");
                        ui.add(DragValue::new(&mut self.y_pos));
                    });
                });
            
            // Key section
            ui.add_space(10.0);
            egui::CollapsingHeader::new("Key Sequence")
                .default_open(true)
                .show(ui, |ui| {
                    for i in 0..self.key_sections.len() {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                if i < 2 { // Allow expansion for first two sections only
                                    if ui.button(if self.key_sections[i].expanded { "-" } else { "+" }).clicked() {
                                        self.key_sections[i].expanded = !self.key_sections[i].expanded;
                                        if self.key_sections[i].expanded && i == self.key_sections.len() - 1 && i < 2 {
                                            self.add_key_section();
                                        }
                                    }
                                } else {
                                    ui.add_space(20.0); // Placeholder for alignment
                                }
                                
                                ui.label("Key:");
                                let mut key = self.key_sections[i].key.clone();
                                if ui.text_edit_singleline(&mut key).changed() {
                                    self.key_sections[i].key = key;
                                }
                                
                                if ui.button("Pick Key").clicked() {
                                    self.pick_key(i);
                                }
                            });
                            
                            if self.key_sections[i].expanded {
                                ui.separator();
                                
                                ui.horizontal(|ui| {
                                    ui.label("Interval:");
                                    ui.add(DragValue::new(&mut self.key_sections[i].interval_hours).clamp_range(0..=23).speed(0.1));
                                    ui.label("h");
                                    ui.add_space(5.0);
                                    ui.add(DragValue::new(&mut self.key_sections[i].interval_minutes).clamp_range(0..=59).speed(0.1));
                                    ui.label("m");
                                    ui.add_space(5.0);
                                    ui.add(DragValue::new(&mut self.key_sections[i].interval_seconds).clamp_range(0..=59).speed(0.1));
                                    ui.label("s");
                                    ui.add_space(5.0);
                                    ui.add(DragValue::new(&mut self.key_sections[i].interval_milliseconds).clamp_range(0..=999).speed(1.0));
                                    ui.label("ms");
                                });
                            }
                        });
                    }
                });
            
            // Bottom buttons
            ui.add_space(20.0);
            ui.horizontal(|ui| {
                if ui.button("Start (F6)").clicked() {
                    self.start_clicking();
                }
                
                if ui.button("Stop (F7)").clicked() {
                    self.stop_clicking();
                }
            });
            
            ui.horizontal(|ui| {
                if ui.button("Hotkey setting").clicked() {
                    self.show_hotkey_settings = true;
                }
                
                if ui.button("Help? >>").clicked() {
                    self.show_help = true;
                }
            });
            
            // Status indicator
            if self.running.load(Ordering::SeqCst) {
                ui.colored_label(Color32::GREEN, "● Running");
            }
            
            // Display help dialog
            if self.show_help {
                egui::Window::new("Help")
                    .open(&mut self.show_help)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label("OP Auto Clicker Help\n\n\
                                - Auto Clicker: Configure mouse click parameters\n\
                                - Keyboard Clicker: Set keyboard key to press\n\
                                - Key Sequence: Configure additional keys in sequence\n\n\
                                Click interval: Time between clicks/key presses\n\
                                Mouse button: Left, Right, or Middle mouse button\n\
                                Click type: Single, Double, or Hold\n\
                                Repeat: Until stopped or specified number of times\n\
                                Cursor position: Current mouse position or pick specific coordinates\n\n\
                                Hotkeys:\n\
                                F6 - Start\n\
                                F7 - Stop\n\n\
                                The program continues running when minimized. Use F7 to stop clicking.");
                    });
            }
            
            // Hotkey settings dialog
            if self.show_hotkey_settings {
                egui::Window::new("Hotkey Settings")
                    .open(&mut self.show_hotkey_settings)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Start:");
                            let mut start_hotkey = self.start_hotkey.clone();
                            if ui.text_edit_singleline(&mut start_hotkey).changed() {
                                self.start_hotkey = start_hotkey;
                            }
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Stop:");
                            let mut stop_hotkey = self.stop_hotkey.clone();
                            if ui.text_edit_singleline(&mut stop_hotkey).changed() {
                                self.stop_hotkey = stop_hotkey;
                            }
                        });
                        
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            if ui.button("OK").clicked() {
                                self.save_hotkeys();
                                self.show_hotkey_settings = false;
                            }
                            
                            if ui.button("Cancel").clicked() {
                                self.load_hotkeys();
                                self.show_hotkey_settings = false;
                            }
                        });
                    });
            }
            
            // Coordinate tooltip during location picking
            if self.picking_location {
                let mouse_pos = ui.input().pointer.hover_pos().unwrap_or(Vec2::new(0.0, 0.0));
                egui::Window::new("Coordinates")
                    .fixed_pos(mouse_pos + Vec2::new(15.0, 15.0))
                    .title_bar(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label(format!("X: {}, Y: {}", self.x_pos, self.y_pos));
                        ui.label("Press ESC to select");
                    });
            }
        });
    }
}

// fn main() -> Result<(), eframe::Error> {
//     let options = eframe::NativeOptions {
//         initial_window_size: Some(Vec2::new(495.0, 530.0)),
//         resizable: false,
//         always_on_top: true,
//         ..Default::default()
//     };
    
//     // Setup hotkey manager
//     let manager = GlobalHotKeyManager::new().unwrap();
    
//     // Register default F6/F7 hotkeys
//     let start_hotkey = HotKey::new(None, Code::F6);
//     let stop_hotkey = HotKey::new(None, Code::F7);
    
//     manager.register(start_hotkey).unwrap();
//     manager.register(stop_hotkey).unwrap();
    
//     // Create shared state for hotkeys
//     let running = Arc::new(AtomicBool::new(false));
//     let running_clone = running.clone();
    
//     // Start hotkey listener
//     let hotkey_running = Arc::new(AtomicBool::new(false));
    
//     let mut hotkey_listener = manager.listen(move |event| {
//         match event {
//             HotKeyState::Pressed(_) => {
//                 if event.hotkey == start_hotkey {
//                     running_clone.store(true, Ordering::SeqCst);
//                 } else if event.hotkey == stop_hotkey {
//                     running_clone.store(false, Ordering::SeqCst);
//                 }
//             }
//             _ => {}
//         }
//     }).unwrap();
    
//     // Run GUI event loop
//     eframe::run_native(
//         "OP Auto Clicker",
//         options,
//         Box::new(|cc| Box::new(AutoClicker::new(cc, running_clone))),
//     )
