use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MouseButton {
    #[default]
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClickType {
    #[default]
    Single,
    Double,
    Hold,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    MouseClick {
        button: MouseButton,
        click_type: ClickType,
        hold_ms: u64,
        position: Option<[i32; 2]>,
    },
    KeyPress {
        key: String,
        hold_ms: u64,
    },
    TypeText {
        text: String,
    },
    Wait {
        duration_ms: u64,
    },
}

impl Default for Action {
    fn default() -> Self {
        Self::MouseClick {
            button: MouseButton::Left,
            click_type: ClickType::Single,
            hold_ms: 100,
            position: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SequenceStep {
    pub enabled: bool,
    pub action: Action,
    /// Delay after this action, in milliseconds.
    pub delay_after_ms: u64,
}

impl Default for SequenceStep {
    fn default() -> Self {
        Self {
            enabled: true,
            action: Action::default(),
            delay_after_ms: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppSettings {
    pub locale: String,
    pub theme: String,
    pub accent_hex: String,
    /// Milliseconds between completed sequence cycles.
    pub interval_ms: u64,
    /// Zero means repeat until stopped.
    pub repeat_count: u64,
    pub start_hotkey: String,
    pub stop_hotkey: String,
    pub sequence: Vec<SequenceStep>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            locale: "en".into(),
            theme: "dark".into(),
            accent_hex: "#6C8CFF".into(),
            interval_ms: 100,
            repeat_count: 0,
            start_hotkey: "F6".into(),
            stop_hotkey: "F7".into(),
            sequence: vec![SequenceStep::default()],
        }
    }
}

impl AppSettings {
    pub fn validate(&self) -> Result<(), String> {
        if self.interval_ms == 0 {
            return Err("The cycle interval must be at least 1 ms.".into());
        }
        if self.sequence.is_empty() {
            return Err("Add at least one action to the sequence.".into());
        }
        if self.sequence.iter().all(|step| !step.enabled) {
            return Err("Enable at least one sequence action.".into());
        }
        for (index, step) in self.sequence.iter().enumerate() {
            match &step.action {
                Action::KeyPress { key, .. } if key.trim().is_empty() => {
                    return Err(format!("Step {} needs a key name.", index + 1));
                }
                Action::TypeText { text } if text.is_empty() => {
                    return Err(format!("Step {} needs text to type.", index + 1));
                }
                _ => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_an_infinite_mouse_click_sequence() {
        let settings = AppSettings::default();
        assert_eq!(settings.locale, "en");
        assert_eq!(settings.repeat_count, 0);
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn supports_a_mixed_keyboard_and_mouse_sequence() {
        let settings = AppSettings {
            sequence: vec![
                SequenceStep {
                    action: Action::KeyPress {
                        key: "Space".into(),
                        hold_ms: 30,
                    },
                    ..Default::default()
                },
                SequenceStep {
                    action: Action::MouseClick {
                        button: MouseButton::Left,
                        click_type: ClickType::Single,
                        hold_ms: 100,
                        position: None,
                    },
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let encoded = serde_json::to_string(&settings).unwrap();
        let decoded: AppSettings = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, settings);
    }

    #[test]
    fn rejects_empty_or_disabled_sequences() {
        let empty = AppSettings {
            sequence: vec![],
            ..Default::default()
        };
        assert!(empty.validate().is_err());
        let disabled = AppSettings {
            sequence: vec![SequenceStep {
                enabled: false,
                ..Default::default()
            }],
            ..Default::default()
        };
        assert!(disabled.validate().is_err());
    }
}
