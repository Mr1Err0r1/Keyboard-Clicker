use crate::{
    flow::{execute_flow, FlowHost, FlowProgram, FlowRunOptions},
    model::{Action, AppSettings, ClickType, MouseButton},
};
use enigo::{Enigo, Key, KeyboardControllable, MouseButton as EnigoMouseButton, MouseControllable};
use std::{
    collections::VecDeque,
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const MAX_FLOW_RUN_EVENTS: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlowRunEvent {
    pub node_id: String,
    pub elapsed_ms: u64,
}

#[derive(Clone)]
pub struct EngineState {
    running: Arc<AtomicBool>,
    completed_cycles: Arc<AtomicU64>,
    last_error: Arc<Mutex<Option<String>>>,
    run_started_at: Arc<Mutex<Option<Instant>>>,
    flow_events: Arc<Mutex<VecDeque<FlowRunEvent>>>,
}

impl EngineState {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            completed_cycles: Arc::new(AtomicU64::new(0)),
            last_error: Arc::new(Mutex::new(None)),
            run_started_at: Arc::new(Mutex::new(None)),
            flow_events: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }
    pub fn completed_cycles(&self) -> u64 {
        self.completed_cycles.load(Ordering::Relaxed)
    }
    pub fn flow_events(&self) -> Vec<FlowRunEvent> {
        self.flow_events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .cloned()
            .collect()
    }
    fn reset_flow_trace(&self) {
        *self
            .run_started_at
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Instant::now());
        self.flow_events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
    }
    fn record_flow_node(&self, node_id: &str) {
        let elapsed_ms = self
            .run_started_at
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .map(|started| started.elapsed().as_millis() as u64)
            .unwrap_or(0);
        let mut events = self
            .flow_events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if events.len() == MAX_FLOW_RUN_EVENTS {
            events.pop_front();
        }
        events.push_back(FlowRunEvent {
            node_id: node_id.to_owned(),
            elapsed_ms,
        });
    }
    pub fn take_error(&self) -> Option<String> {
        self.last_error
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
    }
    fn set_error(&self, error: String) {
        *self
            .last_error
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(error);
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
        self.spawn_worker("keyboard-clicker-engine", move |state| run(settings, state))
    }

    /// Validate and execute a graph program on the same cancellable background
    /// worker used by the sequence editor.
    pub fn start_flow(
        &mut self,
        program: FlowProgram,
        options: FlowRunOptions,
    ) -> Result<(), String> {
        program.validate()?;
        if options.interval_ms == 0 || options.interval_ms > crate::flow::MAX_INTERVAL_MS {
            return Err(format!(
                "Flow interval must be between 1 and {} ms.",
                crate::flow::MAX_INTERVAL_MS
            ));
        }
        if options.repeat_count > crate::flow::MAX_REPEAT_COUNT {
            return Err(format!(
                "Flow repeat count cannot exceed {}.",
                crate::flow::MAX_REPEAT_COUNT
            ));
        }
        self.spawn_worker("keyboard-clicker-flow", move |state| {
            let mut host = EnigoFlowHost {
                enigo: SafeEnigo::new(),
                state: state.clone(),
            };
            let result = execute_flow(&program, options, &mut host, |cycles| {
                state.completed_cycles.store(cycles, Ordering::Relaxed);
            });
            if let Err(error) = result {
                state.set_error(error);
            }
            state.stop();
        })
    }

    fn spawn_worker(
        &mut self,
        name: &'static str,
        work: impl FnOnce(EngineState) + Send + 'static,
    ) -> Result<(), String> {
        if self.state.is_running() {
            return Ok(());
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        self.state.completed_cycles.store(0, Ordering::Relaxed);
        self.state.reset_flow_trace();
        *self
            .state
            .last_error
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
        self.state.running.store(true, Ordering::Release);
        let state = self.state.clone();
        self.worker = Some(
            thread::Builder::new()
                .name(name.into())
                .spawn(move || {
                    let panic_state = state.clone();
                    if catch_unwind(AssertUnwindSafe(|| work(state))).is_err() {
                        panic_state.set_error("The automation worker stopped unexpectedly.".into());
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

/// Tracks held inputs and releases them on every worker exit, including panic
/// unwinding. Each release is isolated so one backend panic cannot prevent the
/// remaining keys/buttons from being released.
struct SafeEnigo {
    inner: Enigo,
    held_keys: Vec<Key>,
    held_buttons: Vec<EnigoMouseButton>,
}

impl SafeEnigo {
    fn new() -> Self {
        Self {
            inner: Enigo::new(),
            held_keys: Vec::new(),
            held_buttons: Vec::new(),
        }
    }

    fn key_down(&mut self, key: Key) {
        if !self.held_keys.contains(&key) {
            self.held_keys.push(key);
        }
        self.inner.key_down(key);
    }

    fn key_up(&mut self, key: Key) {
        self.inner.key_up(key);
        self.held_keys.retain(|held| *held != key);
    }

    fn mouse_down(&mut self, button: EnigoMouseButton) {
        if !self.held_buttons.contains(&button) {
            self.held_buttons.push(button);
        }
        self.inner.mouse_down(button);
    }

    fn mouse_up(&mut self, button: EnigoMouseButton) {
        self.inner.mouse_up(button);
        self.held_buttons.retain(|held| *held != button);
    }
}

impl Drop for SafeEnigo {
    fn drop(&mut self) {
        for button in std::mem::take(&mut self.held_buttons).into_iter().rev() {
            let _ = catch_unwind(AssertUnwindSafe(|| self.inner.mouse_up(button)));
        }
        for key in std::mem::take(&mut self.held_keys).into_iter().rev() {
            let _ = catch_unwind(AssertUnwindSafe(|| self.inner.key_up(key)));
        }
    }
}

struct EnigoFlowHost {
    enigo: SafeEnigo,
    state: EngineState,
}

impl FlowHost for EnigoFlowHost {
    fn is_cancelled(&self) -> bool {
        !self.state.is_running()
    }

    fn node_started(&mut self, node: &crate::flow::FlowNode) {
        self.state.record_flow_node(&node.id);
    }

    fn execute_action(&mut self, action: &Action) -> Result<(), String> {
        execute(&mut self.enigo, action, &self.state);
        Ok(())
    }

    fn wait(&mut self, duration_ms: u64) -> Result<(), String> {
        sleep_interruptible(Duration::from_millis(duration_ms), &self.state);
        Ok(())
    }
}

fn run(settings: AppSettings, state: EngineState) {
    let mut enigo = SafeEnigo::new();
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

fn execute(enigo: &mut SafeEnigo, action: &Action, state: &EngineState) {
    match action {
        Action::MouseClick {
            button,
            click_type,
            hold_ms,
            position,
        } => {
            if let Some([x, y]) = position {
                enigo.inner.mouse_move_to(*x, *y);
            }
            let button = to_enigo_button(*button);
            match click_type {
                ClickType::Single => {
                    enigo.mouse_down(button);
                    enigo.mouse_up(button);
                }
                ClickType::Double => {
                    enigo.mouse_down(button);
                    enigo.mouse_up(button);
                    if sleep_interruptible(Duration::from_millis(30), state) {
                        enigo.mouse_down(button);
                        enigo.mouse_up(button);
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
            enigo.inner.mouse_move_to(position[0], position[1]);
        }
        Action::MouseMoveSmooth {
            position,
            duration_ms,
        } => move_smoothly(enigo, *position, *duration_ms, state),
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
                enigo.inner.key_sequence(key);
            }
        }
        Action::TypeText { text } => enigo.inner.key_sequence(text),
        Action::Wait { duration_ms } => {
            sleep_interruptible(Duration::from_millis(*duration_ms), state);
        }
    }
}

fn move_smoothly(enigo: &mut SafeEnigo, target: [i32; 2], duration_ms: u64, state: &EngineState) {
    if duration_ms == 0 {
        enigo.inner.mouse_move_to(target[0], target[1]);
        return;
    }
    let (start_x, start_y) = enigo.inner.mouse_location();
    let steps = duration_ms.div_ceil(16).max(1);
    let step_wait = Duration::from_millis((duration_ms / steps).max(1));
    let dx = i64::from(target[0]) - i64::from(start_x);
    let dy = i64::from(target[1]) - i64::from(start_y);
    for step in 1..=steps {
        if !sleep_interruptible(step_wait, state) {
            return;
        }
        let progress = step as f64 / steps as f64;
        let x = f64::from(start_x) + dx as f64 * progress;
        let y = f64::from(start_y) + dy as f64 * progress;
        enigo
            .inner
            .mouse_move_to(x.round() as i32, y.round() as i32);
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

    #[test]
    fn flow_run_trace_is_bounded_ordered_and_reset_between_runs() {
        let state = EngineState::new();
        state.reset_flow_trace();
        for index in 0..(MAX_FLOW_RUN_EVENTS + 7) {
            state.record_flow_node(&format!("node-{index}"));
        }

        let events = state.flow_events();
        assert_eq!(events.len(), MAX_FLOW_RUN_EVENTS);
        assert_eq!(events.first().unwrap().node_id, "node-7");
        assert_eq!(events.last().unwrap().node_id, "node-70");
        assert!(events
            .windows(2)
            .all(|pair| pair[0].elapsed_ms <= pair[1].elapsed_ms));

        state.reset_flow_trace();
        assert!(state.flow_events().is_empty());
    }
}
