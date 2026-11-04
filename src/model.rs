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
    MouseMove {
        position: [i32; 2],
    },
    MouseMoveSmooth {
        position: [i32; 2],
        duration_ms: u64,
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
    /// Optional user-arranged position in the visual flow canvas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canvas_position: Option<[f32; 2]>,
}

impl Default for SequenceStep {
    fn default() -> Self {
        Self {
            enabled: true,
            action: Action::default(),
            delay_after_ms: 0,
            canvas_position: None,
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

pub const MAX_SEQUENCE_STEPS: usize = 256;
const MAX_ACTION_DURATION_MS: u64 = 86_400_000;
pub const MAX_SMOOTH_MOVE_DURATION_MS: u64 = 120_000;

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
        if self.interval_ms > 3_600_000 {
            return Err("The cycle interval cannot exceed one hour.".into());
        }
        if self.repeat_count > 1_000_000_000 {
            return Err("The repeat count cannot exceed 1,000,000,000.".into());
        }
        if self.sequence.len() > MAX_SEQUENCE_STEPS {
            return Err(format!(
                "A sequence can contain at most {MAX_SEQUENCE_STEPS} actions."
            ));
        }
        if self.sequence.is_empty() {
            return Err("Add at least one action to the sequence.".into());
        }
        if self.sequence.iter().all(|step| !step.enabled) {
            return Err("Enable at least one sequence action.".into());
        }
        for (index, step) in self.sequence.iter().enumerate() {
            if let Some([x, y]) = step.canvas_position {
                if !x.is_finite()
                    || !y.is_finite()
                    || !(0.0..=20_000.0).contains(&x)
                    || !(0.0..=20_000.0).contains(&y)
                {
                    return Err(format!(
                        "Step {} has an invalid diagram position.",
                        index + 1
                    ));
                }
            }
            if step.delay_after_ms > MAX_ACTION_DURATION_MS {
                return Err(format!("Step {} delay cannot exceed 24 hours.", index + 1));
            }
            match &step.action {
                Action::KeyPress { key, .. } if key.trim().is_empty() => {
                    return Err(format!("Step {} needs a key name.", index + 1));
                }
                Action::KeyPress { hold_ms, .. } if *hold_ms > MAX_ACTION_DURATION_MS => {
                    return Err(format!(
                        "Step {} key hold cannot exceed 24 hours.",
                        index + 1
                    ));
                }
                Action::MouseClick { hold_ms, .. } if *hold_ms > MAX_ACTION_DURATION_MS => {
                    return Err(format!(
                        "Step {} mouse hold cannot exceed 24 hours.",
                        index + 1
                    ));
                }
                Action::MouseMoveSmooth { duration_ms, .. }
                    if *duration_ms > MAX_SMOOTH_MOVE_DURATION_MS =>
                {
                    return Err(format!(
                        "Step {} smooth mouse movement cannot exceed 2 minutes.",
                        index + 1
                    ));
                }
                Action::TypeText { text } if text.is_empty() => {
                    return Err(format!("Step {} needs text to type.", index + 1));
                }
                Action::Wait { duration_ms } if *duration_ms > MAX_ACTION_DURATION_MS => {
                    return Err(format!("Step {} wait cannot exceed 24 hours.", index + 1));
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
    fn mouse_move_action_round_trips_through_json() {
        let action = Action::MouseMove {
            position: [420, 280],
        };
        let encoded = serde_json::to_string(&action).unwrap();
        assert_eq!(serde_json::from_str::<Action>(&encoded).unwrap(), action);
    }

    #[test]
    fn canvas_positions_round_trip_and_older_settings_still_load() {
        let mut settings = AppSettings::default();
        settings.sequence[0].canvas_position = Some([125.5, 240.0]);
        let encoded = serde_json::to_value(&settings).unwrap();
        assert_eq!(
            serde_json::from_value::<AppSettings>(encoded.clone()).unwrap(),
            settings
        );

        let mut legacy = encoded;
        legacy["sequence"][0]
            .as_object_mut()
            .unwrap()
            .remove("canvas_position");
        let loaded: AppSettings = serde_json::from_value(legacy).unwrap();
        assert_eq!(loaded.sequence[0].canvas_position, None);
    }

    #[test]
    fn rejects_invalid_canvas_positions() {
        for position in [
            [f32::NAN, 10.0],
            [10.0, f32::INFINITY],
            [-1.0, 10.0],
            [20_001.0, 10.0],
        ] {
            let settings = AppSettings {
                sequence: vec![SequenceStep {
                    canvas_position: Some(position),
                    ..Default::default()
                }],
                ..Default::default()
            };
            assert!(settings.validate().is_err());
        }
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

    #[test]
    fn rejects_imported_sequences_with_unsafe_bounds() {
        let too_many = AppSettings {
            sequence: vec![SequenceStep::default(); MAX_SEQUENCE_STEPS + 1],
            ..Default::default()
        };
        assert!(too_many.validate().is_err());

        let excessive_wait = AppSettings {
            sequence: vec![SequenceStep {
                action: Action::Wait {
                    duration_ms: MAX_ACTION_DURATION_MS + 1,
                },
                ..Default::default()
            }],
            ..Default::default()
        };
        assert!(excessive_wait.validate().is_err());
    }
}
