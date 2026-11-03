use crate::model::{Action, AppSettings, ClickType, MouseButton};
use enigo::{Enigo, Key, KeyboardControllable, MouseButton as EnigoMouseButton, MouseControllable};
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct EngineState {
    running: Arc<AtomicBool>,
    completed_cycles: Arc<AtomicU64>,
}

impl EngineState {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            completed_cycles: Arc::new(AtomicU64::new(0)),
        }
    }
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }
    pub fn completed_cycles(&self) -> u64 {
        self.completed_cycles.load(Ordering::Relaxed)
    }
    pub fn stop(&self) {
        self.running.store(false, Ordering::Release);
    }
}

impl Default for EngineState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AutomationEngine {
    state: EngineState,
    worker: Option<JoinHandle<()>>,
}

impl AutomationEngine {
    pub fn new() -> Self {
        Self {
            state: EngineState::new(),
            worker: None,
        }
    }
    pub fn state(&self) -> EngineState {
        self.state.clone()
    }

    pub fn start(&mut self, settings: AppSettings) -> Result<(), String> {
        settings.validate()?;
        if self.state.is_running() {
            return Ok(());
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        self.state.completed_cycles.store(0, Ordering::Relaxed);
        self.state.running.store(true, Ordering::Release);
        let state = self.state.clone();
        self.worker = Some(
            thread::Builder::new()
                .name("keyboard-clicker-engine".into())
                .spawn(move || {
                    let panic_state = state.clone();
                    if catch_unwind(AssertUnwindSafe(|| run(settings, state))).is_err() {
                        panic_state.stop();
                    }
                })
                .map_err(|error| {
                    self.state.stop();
                    format!("Could not start automation worker: {error}")
                })?,
        );
        Ok(())
    }

    pub fn stop(&mut self) {
        self.state.stop();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Default for AutomationEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AutomationEngine {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run(settings: AppSettings, state: EngineState) {
    let mut enigo = Enigo::new();
    let mut cycle = 0_u64;
    while state.is_running() && (settings.repeat_count == 0 || cycle < settings.repeat_count) {
        let cycle_started = Instant::now();
        for step in settings.sequence.iter().filter(|step| step.enabled) {
            if !state.is_running() {
                break;
            }
            execute(&mut enigo, &step.action, &state);
            if step.delay_after_ms > 0
                && !sleep_interruptible(Duration::from_millis(step.delay_after_ms), &state)
            {
                break;
            }
        }
        if !state.is_running() {
            break;
        }
        cycle += 1;
        state.completed_cycles.store(cycle, Ordering::Relaxed);
        if settings.repeat_count > 0 && cycle >= settings.repeat_count {
            break;
        }
        let target = cycle_started + Duration::from_millis(settings.interval_ms);
        if let Some(wait) = target.checked_duration_since(Instant::now()) {
            sleep_interruptible(wait, &state);
        }
    }
    state.stop();
}

fn execute(enigo: &mut Enigo, action: &Action, state: &EngineState) {
    match action {
        Action::MouseClick {
            button,
            click_type,
            hold_ms,
            position,
        } => {
            if let Some([x, y]) = position {
                enigo.mouse_move_to(*x, *y);
            }
            let button = to_enigo_button(*button);
            match click_type {
                ClickType::Single => enigo.mouse_click(button),
                ClickType::Double => {
                    enigo.mouse_click(button);
                    if sleep_interruptible(Duration::from_millis(30), state) {
                        enigo.mouse_click(button);
                    }
                }
                ClickType::Hold => {
                    enigo.mouse_down(button);
                    sleep_interruptible(Duration::from_millis((*hold_ms).max(1)), state);
                    // Always release even when stop is requested during the hold.
                    enigo.mouse_up(button);
                }
            }
        }
        Action::MouseMove { position } => {
            enigo.mouse_move_to(position[0], position[1]);
        }
        Action::KeyPress { key, hold_ms } => {
            if let Some(keys) = to_enigo_combo(key) {
                for key_code in &keys {
                    enigo.key_down(*key_code);
                }
                sleep_interruptible(Duration::from_millis((*hold_ms).max(1)), state);
                for key_code in keys.into_iter().rev() {
                    enigo.key_up(key_code);
                }
            } else if let Some(key_code) = to_enigo_key(key) {
                enigo.key_down(key_code);
                sleep_interruptible(Duration::from_millis((*hold_ms).max(1)), state);
                enigo.key_up(key_code);
            } else {
                enigo.key_sequence(key);
            }
        }
        Action::TypeText { text } => enigo.key_sequence(text),
        Action::Wait { duration_ms } => {
            sleep_interruptible(Duration::from_millis(*duration_ms), state);
        }
    }
}

fn sleep_interruptible(duration: Duration, state: &EngineState) -> bool {
    let deadline = Instant::now() + duration;
    while state.is_running() {
        let now = Instant::now();
        if now >= deadline {
            return true;
        }
        thread::sleep((deadline - now).min(Duration::from_millis(5)));
    }
    false
}

fn to_enigo_button(button: MouseButton) -> EnigoMouseButton {
    match button {
        MouseButton::Left => EnigoMouseButton::Left,
        MouseButton::Right => EnigoMouseButton::Right,
        MouseButton::Middle => EnigoMouseButton::Middle,
    }
}

fn to_enigo_combo(name: &str) -> Option<Vec<Key>> {
    let parts: Vec<_> = name.split('+').map(str::trim).collect();
    if parts.len() < 2 || parts.iter().any(|part| part.is_empty()) {
        return None;
    }
    parts.into_iter().map(to_enigo_key).collect()
}

fn to_enigo_key(name: &str) -> Option<Key> {
    let key = name.trim().to_ascii_lowercase();
    match key.as_str() {
        "space" => Some(Key::Space),
        "enter" | "return" => Some(Key::Return),
        "tab" => Some(Key::Tab),
        "escape" | "esc" => Some(Key::Escape),
        "backspace" => Some(Key::Backspace),
        "delete" | "del" => Some(Key::Delete),
        "up" | "arrowup" => Some(Key::UpArrow),
        "down" | "arrowdown" => Some(Key::DownArrow),
        "left" | "arrowleft" => Some(Key::LeftArrow),
        "right" | "arrowright" => Some(Key::RightArrow),
        "home" => Some(Key::Home),
        "end" => Some(Key::End),
        "pageup" => Some(Key::PageUp),
        "pagedown" => Some(Key::PageDown),
        "shift" => Some(Key::Shift),
        "ctrl" | "control" => Some(Key::Control),
        "alt" => Some(Key::Alt),
        "super" | "meta" | "win" => Some(Key::Meta),
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
        _ => {
            let mut chars = name.chars();
            let first = chars.next()?;
            if chars.next().is_none() {
                Some(Key::Layout(first))
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_special_and_printable_keys() {
        assert_eq!(to_enigo_key("F6"), Some(Key::F6));
        assert_eq!(to_enigo_key("space"), Some(Key::Space));
        assert_eq!(to_enigo_key("a"), Some(Key::Layout('a')));
        assert_eq!(to_enigo_key("unknown long key"), None);
    }

    #[test]
    fn recognizes_modifier_combinations_as_ordered_keys() {
        assert_eq!(
            to_enigo_combo("Ctrl+A"),
            Some(vec![Key::Control, Key::Layout('A')])
        );
        assert!(to_enigo_combo("Ctrl+").is_none());
    }

    #[test]
    fn wait_and_mouse_actions_are_serializable() {
        let step = Action::MouseClick {
            button: MouseButton::Right,
            click_type: ClickType::Hold,
            hold_ms: 500,
            position: Some([10, 20]),
        };
        let json = serde_json::to_string(&step).unwrap();
        assert_eq!(serde_json::from_str::<Action>(&json).unwrap(), step);
    }

    #[test]
    fn long_wait_is_interruptible() {
        let state = EngineState::new();
        state.running.store(true, Ordering::Release);
        let stop_state = state.clone();
        let stopper = thread::spawn(move || {
            thread::sleep(Duration::from_millis(20));
            stop_state.stop();
        });
        let started = Instant::now();
        assert!(!sleep_interruptible(Duration::from_secs(5), &state));
        assert!(started.elapsed() < Duration::from_secs(1));
        stopper.join().unwrap();
    }
}
