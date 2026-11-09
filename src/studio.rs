use crate::{
    engine::FlowRunEvent,
    flow::{
        import_autoflow_json, Comparison, FlowEdge, FlowNode, FlowNodeKind, FlowPort, FlowProgram,
        FlowRunOptions, FlowValue, Operand, VariableOperation, FLOW_SCHEMA_VERSION, MAX_FLOW_EDGES,
        MAX_FLOW_NODES, MAX_FLOW_VARIABLES,
    },
    model::{Action, ClickType, MouseButton, SequenceStep},
    theme::{CanvasBackground, StudioAppearance, ThemeColors},
};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, RichText, Stroke, Vec2};
use std::{collections::HashSet, path::Path};

const NODE_SIZE: Vec2 = Vec2::new(200.0, 88.0);
const NODE_GAP_X: f32 = 60.0;
const NODE_GAP_Y: f32 = 56.0;
const CANVAS_LIMIT: f32 = 20_000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudioCommand {
    None,
    Start,
    Stop,
    Import,
    Export,
}

pub struct FlowStudio {
    pub program: FlowProgram,
    pub selected_node_id: Option<String>,
    selected_edge: Option<(String, FlowPort)>,
    connecting: Option<(String, FlowPort)>,
    confirm_load_example: bool,
    drag_origin: Option<(String, [f32; 2], Pos2)>,
    message: Option<(String, bool)>,
    animation_time: f64,
    last_frame_time: Option<f64>,
}

impl FlowStudio {
    pub fn new(saved_json: Option<&str>, sequence: &[SequenceStep]) -> (Self, Option<String>) {
        if let Some(json) = saved_json {
            if json.len() <= 2 * 1024 * 1024 {
                if let Ok(mut program) = serde_json::from_str::<FlowProgram>(json) {
                    if program.nodes.len() <= MAX_FLOW_NODES
                        && program.edges.len() <= MAX_FLOW_EDGES
                        && program.variables.len() <= MAX_FLOW_VARIABLES
                        && program.schema_version == FLOW_SCHEMA_VERSION
                    {
                        for node in &mut program.nodes {
                            if node.position.is_some_and(|[x, y]| {
                                !x.is_finite()
                                    || !y.is_finite()
                                    || !(0.0..=CANVAS_LIMIT).contains(&x)
                                    || !(0.0..=CANVAS_LIMIT).contains(&y)
                            }) {
                                node.position = None;
                            }
                            if node.label.len() > 128 {
                                node.label = truncate(&node.label, 128);
                            }
                        }
                        let warning = program.validate().err();
                        let selected_node_id = program
                            .nodes
                            .iter()
                            .any(|node| node.id == program.entry_node_id)
                            .then(|| program.entry_node_id.clone());
                        return (
                            Self {
                                program,
                                selected_node_id,
                                selected_edge: None,
                                connecting: None,
                                confirm_load_example: false,
                                drag_origin: None,
                                message: warning.map(|error| (error, true)),
                                animation_time: 0.0,
                                last_frame_time: None,
                            },
                            None,
                        );
                    }
                }
            }
            return (
                Self::from_sequence(sequence),
                Some(
                    "The saved flow was invalid; restored a flow from your saved sequence.".into(),
                ),
            );
        }
        let warning = (!sequence.is_empty() && sequence.iter().all(|step| !step.enabled))
            .then(|| "The saved Classic sequence has no enabled steps; Studio created a safe inactive placeholder.".to_owned());
        (Self::from_sequence(sequence), warning)
    }

    fn from_sequence(sequence: &[SequenceStep]) -> Self {
        let enabled_steps: Vec<&SequenceStep> =
            sequence.iter().filter(|step| step.enabled).collect();
        let count = enabled_steps.len().clamp(1, MAX_FLOW_NODES - 1);
        let mut nodes = Vec::with_capacity(count + 1);
        let mut edges = Vec::with_capacity(count);
        for (index, step) in enabled_steps.iter().take(count).enumerate() {
            let id = format!("legacy-{index}");
            nodes.push(FlowNode {
                id: id.clone(),
                label: format!("Action {}", index + 1),
                position: Some(sequence_position(index)),
                kind: FlowNodeKind::Action {
                    action: step.action.clone(),
                    delay_after_ms: step.delay_after_ms,
                },
            });
            let next = if index + 1 < count {
                format!("legacy-{}", index + 1)
            } else {
                "flow-end".to_owned()
            };
            edges.push(FlowEdge {
                from_node_id: id,
                port: FlowPort::Next,
                to_node_id: next,
            });
        }
        if enabled_steps.is_empty() {
            let action = if sequence.is_empty() {
                Action::default()
            } else {
                Action::Wait { duration_ms: 0 }
            };
            nodes.push(FlowNode {
                id: "legacy-0".into(),
                label: if sequence.is_empty() {
                    "Mouse Click"
                } else {
                    "No enabled steps"
                }
                .into(),
                position: Some([72.0, 150.0]),
                kind: FlowNodeKind::Action {
                    action,
                    delay_after_ms: 0,
                },
            });
            edges.push(FlowEdge {
                from_node_id: "legacy-0".into(),
                port: FlowPort::Next,
                to_node_id: "flow-end".into(),
            });
        }
        nodes.push(FlowNode {
            id: "flow-end".into(),
            label: "End".into(),
            position: Some(sequence_position(count)),
            kind: FlowNodeKind::End,
        });
        Self {
            program: FlowProgram {
                schema_version: FLOW_SCHEMA_VERSION,
                name: "My workflow".into(),
                entry_node_id: "legacy-0".into(),
                variables: Default::default(),
                nodes,
                edges,
            },
            selected_node_id: Some("legacy-0".into()),
            selected_edge: None,
            connecting: None,
            confirm_load_example: false,
            drag_origin: None,
            message: None,
            animation_time: 0.0,
            last_frame_time: None,
        }
    }

    pub fn draft_json(&self) -> Option<String> {
        serde_json::to_string(&self.program).ok()
    }

    pub fn run_options(&self, interval_ms: u64, repeat_count: u64) -> FlowRunOptions {
        FlowRunOptions {
            interval_ms,
            repeat_count,
        }
    }

    pub fn import_dialog(&mut self) -> Result<(), String> {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Workflow JSON", &["json"])
            .pick_file()
        else {
            return Ok(());
        };
        let metadata = std::fs::metadata(&path).map_err(|error| error.to_string())?;
        if metadata.len() > 2 * 1024 * 1024 {
            return Err("Workflow files cannot be larger than 2 MB.".into());
        }
        let contents = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
        let program = FlowProgram::from_json(&contents)
            .or_else(|_| import_autoflow_json(&contents))
            .map_err(|error| format!("Could not import workflow: {error}"))?;
        self.confirm_load_example = false;
        self.drag_origin = None;
        self.selected_node_id = Some(program.entry_node_id.clone());
        self.program = program;
        self.connecting = None;
        self.message = Some((format!("Imported {}", path.display()), false));
        Ok(())
    }

    pub fn export_dialog(&mut self) -> Result<(), String> {
        self.program.validate()?;
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Keyboard Clicker workflow", &["json"])
            .set_file_name("keyboard-clicker-workflow.json")
            .save_file()
        else {
            return Ok(());
        };
        write_workflow_to(&path, &self.program)?;
        self.message = Some((format!("Exported {}", path.display()), false));
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        tr: &impl Fn(&str) -> String,
        appearance: StudioAppearance,
        locked: bool,
        completed_cycles: u64,
        flow_events: &[FlowRunEvent],
        interval_ms: &mut u64,
        repeat_count: &mut u64,
    ) -> (StudioCommand, bool) {
        let mut command = StudioCommand::None;
        let mut changed = false;
        if locked {
            self.confirm_load_example = false;
            self.drag_origin = None;
        }
        let accent = appearance.colors.accent;
        egui::Frame::none()
            .fill(appearance.colors.app_background)
            .inner_margin(egui::Margin::symmetric(10.0, 8.0))
            .show(ui, |ui| {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(tr("studio_title"))
                                    .small()
                                    .color(accent)
                                    .strong(),
                            );
                            changed |= ui
                                .add(
                                    egui::TextEdit::singleline(&mut self.program.name)
                                        .desired_width(250.0)
                                        .font(egui::TextStyle::Heading),
                                )
                                .changed();
                        });
                        ui.separator();
                        ui.label(
                            RichText::new(format!(
                                "{} {}",
                                self.program.nodes.len(),
                                tr("studio_nodes")
                            ))
                            .color(ui.visuals().weak_text_color()),
                        );
                        ui.label(
                            RichText::new(format!(
                                "{} {}",
                                self.program.edges.len(),
                                tr("studio_links")
                            ))
                            .color(ui.visuals().weak_text_color()),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if locked {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new(tr("stop")).color(Color32::WHITE),
                                        )
                                        .fill(Color32::from_rgb(190, 61, 74)),
                                    )
                                    .clicked()
                                {
                                    command = StudioCommand::Stop;
                                }
                            } else if ui
                                .add_enabled(
                                    self.program.validate().is_ok(),
                                    egui::Button::new(
                                        RichText::new(tr("start"))
                                            .color(contrast_text_color(accent)),
                                    )
                                    .fill(accent),
                                )
                                .clicked()
                            {
                                command = StudioCommand::Start;
                            }
                            if ui.button(tr("studio_export")).clicked() {
                                command = StudioCommand::Export;
                            }
                            if ui.button(tr("studio_import")).clicked() {
                                command = StudioCommand::Import;
                            }
                            ui.separator();
                            ui.label(format!("{}  {completed_cycles}", tr("status_cycles")));
                        });
                    });
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.label(tr("cycle_interval"));
                        changed |= ui
                            .add(
                                egui::DragValue::new(interval_ms)
                                    .clamp_range(1..=3_600_000)
                                    .suffix(" ms"),
                            )
                            .changed();
                        ui.separator();
                        ui.label(tr("repeat_count"));
                        changed |= ui
                            .add(
                                egui::DragValue::new(repeat_count)
                                    .clamp_range(0..=1_000_000_000_u64),
                            )
                            .changed();
                        ui.label(RichText::new("0 = until stopped").weak());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if let Some((text, is_error)) = &self.message {
                                if *is_error {
                                    ui.colored_label(Color32::LIGHT_RED, text);
                                } else {
                                    ui.weak(text);
                                }
                            }
                        });
                    });
                });
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                let available = ui.available_size();
                let compact = available.x < 1_100.0;
                let left_width = if compact { 184.0 } else { 224.0 };
                let right_width = if compact { 232.0 } else { 278.0 };
                let center_width = (available.x - left_width - right_width - 20.0).max(280.0);
                let height = available.y.max(280.0);
                let active_node_id = locked
                    .then(|| flow_events.last().map(|event| event.node_id.as_str()))
                    .flatten();
                ui.horizontal_top(|ui| {
                    ui.allocate_ui_with_layout(
                        Vec2::new(left_width, height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            egui::ScrollArea::vertical()
                                .id_source("studio_palette_scroll")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.group(|ui| {
                                        ui.label(RichText::new(tr("studio_palette")).strong());
                                        ui.label(
                                            RichText::new(tr("studio_actions"))
                                                .small()
                                                .color(accent)
                                                .strong(),
                                        );
                                        ui.add_space(5.0);
                                        for (index, (label, action)) in
                                            palette_actions(tr).into_iter().enumerate()
                                        {
                                            let color = match index {
                                                0 => appearance.colors.accent,
                                                1 => appearance.colors.move_node,
                                                2 | 3 => appearance.colors.keyboard_node,
                                                _ => appearance.colors.wait_node,
                                            };
                                            if palette_button(
                                                ui,
                                                &label,
                                                index + 1,
                                                color,
                                                self.program.nodes.len() < MAX_FLOW_NODES,
                                            ) {
                                                self.add_action(action);
                                                changed = true;
                                            }
                                        }
                                        ui.add_space(14.0);
                                        ui.label(
                                            RichText::new(tr("studio_control"))
                                                .small()
                                                .color(accent)
                                                .strong(),
                                        );
                                        if palette_button(
                                            ui,
                                            &tr("studio_condition"),
                                            6,
                                            appearance.colors.condition_node,
                                            self.program.nodes.len() < MAX_FLOW_NODES,
                                        ) {
                                            self.add_condition();
                                            changed = true;
                                        }
                                        if palette_button(
                                            ui,
                                            &tr("studio_repeat"),
                                            7,
                                            appearance.colors.loop_node,
                                            self.program.nodes.len() <= MAX_FLOW_NODES - 2,
                                        ) {
                                            self.add_repeat();
                                            changed = true;
                                        }
                                        if palette_button(
                                            ui,
                                            &tr("studio_set_variable"),
                                            8,
                                            appearance.colors.variable_node,
                                            self.program.nodes.len() < MAX_FLOW_NODES,
                                        ) {
                                            self.add_variable();
                                            changed = true;
                                        }
                                        ui.add_space(8.0);
                                        ui.label(
                                            RichText::new(tr("studio_palette_help")).small().weak(),
                                        );
                                        if self.confirm_load_example {
                                            ui.colored_label(
                                                appearance.colors.warning,
                                                tr("studio_example_replace_warning"),
                                            );
                                            ui.horizontal(|ui| {
                                                if ui.button(tr("studio_example_confirm")).clicked()
                                                {
                                                    self.load_example(tr);
                                                    changed = true;
                                                }
                                                if ui.button(tr("studio_example_cancel")).clicked()
                                                {
                                                    self.confirm_load_example = false;
                                                }
                                            });
                                        } else if ui
                                            .add_enabled(
                                                !locked,
                                                egui::Button::new(tr("studio_load_example")),
                                            )
                                            .on_hover_text(tr("studio_load_example_hint"))
                                            .clicked()
                                        {
                                            self.confirm_load_example = true;
                                        }
                                        if self.connecting.is_some()
                                            && ui.button(tr("studio_cancel_connection")).clicked()
                                        {
                                            self.connecting = None;
                                        }
                                        ui.add_space(12.0);
                                        ui.label(
                                            RichText::new(format!(
                                                "0 = {}",
                                                tr("studio_until_stopped")
                                            ))
                                            .small()
                                            .weak(),
                                        );
                                    });
                                });
                        },
                    );
                    ui.separator();
                    ui.allocate_ui_with_layout(
                        Vec2::new(center_width, height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(tr("studio_canvas")).strong());
                                    if !compact {
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if self.connecting.is_some() {
                                                    ui.colored_label(
                                                        accent,
                                                        tr("studio_connecting"),
                                                    );
                                                } else {
                                                    ui.weak(tr("studio_canvas_hint"));
                                                }
                                            },
                                        );
                                    }
                                });
                                if compact {
                                    if self.connecting.is_some() {
                                        ui.colored_label(accent, tr("studio_connecting"));
                                    } else {
                                        ui.weak(tr("studio_canvas_hint"));
                                    }
                                }
                                ui.add_space(6.0);
                                let available_height = ui.available_height();
                                let monitor_height = (available_height * 0.26).clamp(88.0, 176.0);
                                let canvas_height =
                                    (available_height - monitor_height - 44.0).max(120.0);
                                let center_width = ui.available_width();
                                ui.allocate_ui_with_layout(
                                    Vec2::new(center_width, canvas_height),
                                    egui::Layout::top_down(egui::Align::Min),
                                    |ui| {
                                        changed |= self.draw_canvas(
                                            ui,
                                            tr,
                                            appearance,
                                            locked,
                                            active_node_id,
                                        );
                                    },
                                );
                                match self.program.validate() {
                                    Ok(()) => {
                                        ui.colored_label(
                                            appearance.colors.success,
                                            format!("OK  {}", tr("studio_workflow_ready")),
                                        );
                                    }
                                    Err(error) => {
                                        ui.colored_label(
                                            appearance.colors.warning,
                                            format!(
                                                "!  {}: {error}",
                                                tr("studio_workflow_attention")
                                            ),
                                        );
                                    }
                                }
                                ui.add_space(4.0);
                                ui.allocate_ui_with_layout(
                                    Vec2::new(ui.available_width(), monitor_height),
                                    egui::Layout::top_down(egui::Align::Min),
                                    |ui| {
                                        self.draw_run_monitor(
                                            ui,
                                            tr,
                                            appearance,
                                            locked,
                                            completed_cycles,
                                            flow_events,
                                        );
                                    },
                                );
                            });
                        },
                    );
                    ui.separator();
                    ui.allocate_ui_with_layout(
                        Vec2::new(right_width, height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            egui::ScrollArea::vertical()
                                .id_source("studio_inspector_scroll")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.group(|ui| {
                                        ui.label(RichText::new(tr("studio_inspector")).strong());
                                        ui.add_space(8.0);
                                        changed |= self.draw_inspector(ui, tr, appearance, locked);
                                    });
                                });
                        },
                    );
                });
            });
        (command, changed)
    }

    fn draw_run_monitor(
        &mut self,
        ui: &mut egui::Ui,
        tr: &impl Fn(&str) -> String,
        appearance: StudioAppearance,
        locked: bool,
        completed_cycles: u64,
        flow_events: &[FlowRunEvent],
    ) {
        let palette = appearance.colors;
        let compact = ui.available_height() < 145.0;
        egui::Frame::none()
            .fill(palette.panel_alt)
            .stroke(Stroke::new(1.0_f32, palette.border))
            .inner_margin(if compact {
                egui::Margin::symmetric(7.0, 4.0)
            } else {
                egui::Margin::symmetric(10.0, 7.0)
            })
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(tr("studio_monitor"))
                            .strong()
                            .color(palette.text),
                    );
                    let (status, color) = if locked {
                        (tr("studio_monitor_live"), palette.success)
                    } else if flow_events.is_empty() {
                        (tr("studio_monitor_idle"), palette.muted)
                    } else {
                        (tr("status_stopped"), palette.muted)
                    };
                    ui.colored_label(color, status);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!(
                                "{}  {}  |  {}",
                                tr("status_cycles"),
                                completed_cycles,
                                flow_events.len()
                            ))
                            .small()
                            .color(palette.muted),
                        );
                    });
                });
                ui.add_space(if compact { 2.0 } else { 5.0 });
                ui.horizontal(|ui| {
                    if !compact {
                        self.draw_target_preview(ui, tr, appearance, locked, flow_events);
                        ui.separator();
                    }
                    ui.vertical(|ui| {
                        if flow_events.is_empty() {
                            ui.label(
                                RichText::new(tr("studio_monitor_empty"))
                                    .small()
                                    .color(palette.text),
                            );
                        } else {
                            egui::ScrollArea::horizontal()
                                .id_source("studio_run_trace")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        for (index, event) in flow_events.iter().enumerate() {
                                            if index > 0 {
                                                ui.weak("-->");
                                            }
                                            let Some(node) = self
                                                .program
                                                .nodes
                                                .iter()
                                                .find(|node| node.id == event.node_id)
                                            else {
                                                let label = format!(
                                                    "{}  +{:.2}s",
                                                    event.node_id,
                                                    event.elapsed_ms as f32 / 1_000.0
                                                );
                                                ui.add_sized(
                                                    Vec2::new(
                                                        if compact { 104.0 } else { 112.0 },
                                                        if compact { 32.0 } else { 48.0 },
                                                    ),
                                                    egui::Button::new(RichText::new(label).small())
                                                        .fill(palette.panel)
                                                        .stroke(Stroke::new(
                                                            1.0_f32,
                                                            palette.border,
                                                        )),
                                                );
                                                continue;
                                            };
                                            let (title, detail) = node_text(node, tr);
                                            let color = node_color(&node.kind, &palette);
                                            let label = format!(
                                                "{}  +{:.2}s",
                                                truncate(&title, 13),
                                                event.elapsed_ms as f32 / 1_000.0
                                            );
                                            let is_current =
                                                locked && index + 1 == flow_events.len();
                                            let response = ui
                                                .add_sized(
                                                    Vec2::new(
                                                        if compact { 106.0 } else { 118.0 },
                                                        if compact { 32.0 } else { 48.0 },
                                                    ),
                                                    egui::Button::new(
                                                        RichText::new(label).small().strong(),
                                                    )
                                                    .fill(with_alpha(
                                                        if is_current {
                                                            palette.success
                                                        } else {
                                                            color
                                                        },
                                                        if is_current { 0.22 } else { 0.12 },
                                                    ))
                                                    .stroke(Stroke::new(
                                                        if is_current { 1.6_f32 } else { 1.0_f32 },
                                                        if is_current {
                                                            palette.success
                                                        } else {
                                                            color
                                                        },
                                                    )),
                                                )
                                                .on_hover_text(format!("{}\n{}", title, detail));
                                            if response.clicked() {
                                                self.selected_node_id = Some(event.node_id.clone());
                                                self.selected_edge = None;
                                            }
                                        }
                                    });
                                });
                        }
                    });
                });
                if !compact {
                    ui.add_space(3.0);
                    ui.label(RichText::new(tr("studio_monitor_hint")).small().weak());
                }
            });
    }

    fn draw_target_preview(
        &self,
        ui: &mut egui::Ui,
        tr: &impl Fn(&str) -> String,
        appearance: StudioAppearance,
        locked: bool,
        flow_events: &[FlowRunEvent],
    ) {
        let palette = appearance.colors;
        ui.vertical(|ui| {
            ui.set_min_width(150.0);
            ui.label(
                RichText::new(tr("studio_monitor_preview"))
                    .small()
                    .strong()
                    .color(palette.muted),
            );
            let (rect, _) = ui.allocate_exact_size(Vec2::new(148.0, 64.0), egui::Sense::hover());
            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, 5.0, palette.canvas);
            painter.rect_stroke(rect, 5.0, Stroke::new(1.0_f32, palette.border));
            let inner = rect.shrink(5.0);
            for fraction in [0.25_f32, 0.5, 0.75] {
                let x = egui::lerp(inner.left()..=inner.right(), fraction);
                let y = egui::lerp(inner.top()..=inner.bottom(), fraction);
                painter.line_segment(
                    [Pos2::new(x, inner.top()), Pos2::new(x, inner.bottom())],
                    Stroke::new(0.5_f32, with_alpha(palette.grid, 0.55)),
                );
                painter.line_segment(
                    [Pos2::new(inner.left(), y), Pos2::new(inner.right(), y)],
                    Stroke::new(0.5_f32, with_alpha(palette.grid, 0.55)),
                );
            }

            let selected_id = if locked {
                flow_events.last().map(|event| event.node_id.as_str())
            } else {
                self.selected_node_id.as_deref()
            };
            let target_node =
                selected_id.and_then(|id| self.program.nodes.iter().find(|node| node.id == id));
            let mut target = None;
            let mut caption = tr("studio_monitor_no_target");
            if let Some(node) = target_node {
                if let FlowNodeKind::Action { action, .. } = &node.kind {
                    match action {
                        Action::MouseMove { position }
                        | Action::MouseMoveSmooth { position, .. } => {
                            target = Some(*position);
                            caption = format!("X {}   Y {}", position[0], position[1]);
                        }
                        Action::MouseClick { position, .. } => {
                            if let Some(position) = position {
                                target = Some(*position);
                                caption = format!("X {}   Y {}", position[0], position[1]);
                            } else {
                                target = Some([960, 540]);
                                caption = tr("cursor");
                            }
                        }
                        _ => {}
                    }
                }
            }
            if let Some([x, y]) = target {
                let point = Pos2::new(
                    egui::lerp(
                        inner.left()..=inner.right(),
                        (x as f32 / 1_920.0).clamp(0.0, 1.0),
                    ),
                    egui::lerp(
                        inner.top()..=inner.bottom(),
                        (y as f32 / 1_080.0).clamp(0.0, 1.0),
                    ),
                );
                painter.circle_filled(point, 4.5, palette.accent);
                painter.circle_stroke(point, 7.0, Stroke::new(1.0_f32, palette.text));
            }
            ui.label(RichText::new(caption).small().color(if target.is_some() {
                palette.text
            } else {
                palette.muted
            }));
        });
    }

    fn load_example(&mut self, tr: &impl Fn(&str) -> String) {
        let action_node = |id: &str, label_key: &str, position: [f32; 2], action| FlowNode {
            id: id.to_owned(),
            label: tr(label_key),
            position: Some(position),
            kind: FlowNodeKind::Action {
                action,
                delay_after_ms: 0,
            },
        };
        let ids = [
            "sample-move-a",
            "sample-wait",
            "sample-move-b",
            "sample-end",
        ];
        let nodes = vec![
            action_node(
                ids[0],
                "studio_example_move_a",
                [8.0, 120.0],
                Action::MouseMoveSmooth {
                    position: [320, 240],
                    duration_ms: 450,
                },
            ),
            action_node(
                ids[1],
                "studio_example_pause",
                [220.0, 120.0],
                Action::Wait { duration_ms: 500 },
            ),
            action_node(
                ids[2],
                "studio_example_move_b",
                [432.0, 120.0],
                Action::MouseMoveSmooth {
                    position: [600, 360],
                    duration_ms: 450,
                },
            ),
            FlowNode {
                id: ids[3].to_owned(),
                label: tr("studio_end_node"),
                position: Some([644.0, 120.0]),
                kind: FlowNodeKind::End,
            },
        ];
        let edges = ids
            .windows(2)
            .map(|pair| FlowEdge {
                from_node_id: pair[0].to_owned(),
                port: FlowPort::Next,
                to_node_id: pair[1].to_owned(),
            })
            .collect();
        self.program = FlowProgram {
            schema_version: FLOW_SCHEMA_VERSION,
            name: tr("studio_example_name"),
            entry_node_id: ids[0].to_owned(),
            variables: Default::default(),
            nodes,
            edges,
        };
        self.selected_node_id = Some(ids[0].to_owned());
        self.selected_edge = None;
        self.connecting = None;
        self.confirm_load_example = false;
        self.drag_origin = None;
        self.message = None;
    }

    fn add_action(&mut self, action: Action) {
        if self.program.nodes.len() >= MAX_FLOW_NODES {
            return;
        }
        let ordinal = self.program.nodes.len();
        let used: HashSet<&str> = self
            .program
            .nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect();
        let mut suffix = ordinal;
        let id = loop {
            let candidate = format!("action-{suffix}");
            if !used.contains(candidate.as_str()) {
                break candidate;
            }
            suffix += 1;
        };
        let max_x = self
            .program
            .nodes
            .iter()
            .filter_map(|node| node.position.map(|p| p[0]))
            .fold(32.0_f32, f32::max);
        let max_y = self
            .program
            .nodes
            .iter()
            .filter_map(|node| node.position.map(|p| p[1]))
            .fold(64.0_f32, f32::max);
        let (mut x, mut y) = if max_x + NODE_SIZE.x + NODE_GAP_X < CANVAS_LIMIT - NODE_SIZE.x {
            (max_x + NODE_SIZE.x + NODE_GAP_X, max_y)
        } else {
            (40.0, max_y + 160.0)
        };
        let label = action_label(&action);
        self.program.nodes.push(FlowNode {
            id: id.clone(),
            label,
            position: Some([x, y]),
            kind: FlowNodeKind::Action {
                action,
                delay_after_ms: 0,
            },
        });
        let end_id = self
            .program
            .nodes
            .iter()
            .find(|node| matches!(node.kind, FlowNodeKind::End))
            .map(|node| node.id.clone());
        if let Some(end_id) = end_id {
            let incoming: Vec<usize> = self
                .program
                .edges
                .iter()
                .enumerate()
                .filter(|(_, edge)| edge.to_node_id == end_id)
                .map(|(index, _)| index)
                .collect();
            if incoming.len() == 1 {
                if let Some(end) = self.program.nodes.iter().find(|node| node.id == end_id) {
                    if let Some([end_x, end_y]) = end.position {
                        x = end_x;
                        y = end_y;
                    }
                }
                self.program.edges[incoming[0]].to_node_id = id.clone();
                self.program.edges.push(FlowEdge {
                    from_node_id: id.clone(),
                    port: FlowPort::Next,
                    to_node_id: end_id.clone(),
                });
                if let Some(end) = self.program.nodes.iter_mut().find(|node| node.id == end_id) {
                    end.position = Some([
                        (x + NODE_SIZE.x + NODE_GAP_X).min(CANVAS_LIMIT - NODE_SIZE.x),
                        y,
                    ]);
                }
            } else if incoming.is_empty() && self.program.nodes.len() == 2 {
                self.program.edges.push(FlowEdge {
                    from_node_id: id.clone(),
                    port: FlowPort::Next,
                    to_node_id: end_id,
                });
                self.program.entry_node_id = id.clone();
            }
        }
        self.selected_node_id = Some(id);
    }

    fn unique_id(&self, prefix: &str) -> String {
        let existing: HashSet<&str> = self
            .program
            .nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect();
        (0..MAX_FLOW_NODES)
            .map(|index| format!("{prefix}-{index}"))
            .find(|candidate| !existing.contains(candidate.as_str()))
            .unwrap_or_else(|| format!("{prefix}-new"))
    }

    fn end_insertion(&self) -> Option<(String, [f32; 2], Option<usize>)> {
        let end = self
            .program
            .nodes
            .iter()
            .find(|node| matches!(&node.kind, FlowNodeKind::End))?;
        let position = end.position.unwrap_or([72.0, 150.0]);
        let incoming: Vec<usize> = self
            .program
            .edges
            .iter()
            .enumerate()
            .filter(|(_, edge)| edge.to_node_id == end.id)
            .map(|(index, _)| index)
            .collect();
        let edge_index = if incoming.len() == 1 {
            Some(incoming[0])
        } else {
            None
        };
        Some((end.id.clone(), position, edge_index))
    }

    fn add_condition(&mut self) {
        if self.program.nodes.len() + 1 > MAX_FLOW_NODES {
            return;
        }
        let id = self.unique_id("condition");
        let (end_id, position, incoming) = self
            .end_insertion()
            .unwrap_or_else(|| (String::new(), [320.0, 150.0], None));
        let [x, y] = position;
        let node = FlowNode {
            id: id.clone(),
            label: "Condition".into(),
            position: Some([x, y]),
            kind: FlowNodeKind::Condition {
                left: Operand::Literal(FlowValue::Number(0.0)),
                comparison: Comparison::Equal,
                right: Operand::Literal(FlowValue::Number(0.0)),
            },
        };
        if let Some(edge_index) = incoming {
            self.program.edges[edge_index].to_node_id = id.clone();
            self.program.edges.push(FlowEdge {
                from_node_id: id.clone(),
                port: FlowPort::True,
                to_node_id: end_id.clone(),
            });
            self.program.edges.push(FlowEdge {
                from_node_id: id.clone(),
                port: FlowPort::False,
                to_node_id: end_id.clone(),
            });
            if let Some(end) = self.program.nodes.iter_mut().find(|node| node.id == end_id) {
                end.position = Some([
                    (x + NODE_SIZE.x + NODE_GAP_X).min(CANVAS_LIMIT - NODE_SIZE.x),
                    y,
                ]);
            }
        }
        self.program.nodes.push(node);
        self.selected_node_id = Some(id);
    }

    fn add_repeat(&mut self) {
        if self.program.nodes.len() + 2 > MAX_FLOW_NODES {
            return;
        }
        let id = self.unique_id("repeat");
        let loop_id = self.unique_id("loop-end");
        let (end_id, position, incoming) = self
            .end_insertion()
            .unwrap_or_else(|| (String::new(), [320.0, 150.0], None));
        let [x, y] = position;
        if let Some(edge_index) = incoming {
            self.program.edges[edge_index].to_node_id = id.clone();
        }
        self.program.nodes.push(FlowNode {
            id: id.clone(),
            label: "Repeat".into(),
            position: Some([x, y]),
            kind: FlowNodeKind::Repeat {
                iterations: 3,
                counter_variable: None,
            },
        });
        self.program.nodes.push(FlowNode {
            id: loop_id.clone(),
            label: "Loop end".into(),
            position: Some([x + NODE_SIZE.x + NODE_GAP_X, y + NODE_SIZE.y + NODE_GAP_Y]),
            kind: FlowNodeKind::LoopEnd {
                repeat_node_id: id.clone(),
            },
        });
        if !end_id.is_empty() {
            self.program.edges.push(FlowEdge {
                from_node_id: id.clone(),
                port: FlowPort::Body,
                to_node_id: loop_id.clone(),
            });
            self.program.edges.push(FlowEdge {
                from_node_id: id.clone(),
                port: FlowPort::Done,
                to_node_id: end_id.clone(),
            });
            if let Some(end) = self.program.nodes.iter_mut().find(|node| node.id == end_id) {
                end.position = Some([
                    (x + 2.0 * (NODE_SIZE.x + NODE_GAP_X)).min(CANVAS_LIMIT - NODE_SIZE.x),
                    y,
                ]);
            }
        }
        self.selected_node_id = Some(id);
    }

    fn add_variable(&mut self) {
        if self.program.nodes.len() >= MAX_FLOW_NODES {
            return;
        }
        let id = self.unique_id("variable");
        let end = self.end_insertion();
        let [x, y] = end
            .as_ref()
            .map(|(_, position, _)| *position)
            .unwrap_or([320.0, 150.0]);
        self.program.nodes.push(FlowNode {
            id: id.clone(),
            label: "Set variable".into(),
            position: Some([x, y]),
            kind: FlowNodeKind::SetVariable {
                name: format!("value{}", self.program.nodes.len()),
                operation: VariableOperation::Set {
                    value: FlowValue::Number(0.0),
                },
            },
        });
        if let Some((end_id, position, Some(edge_index))) = end {
            self.program.edges[edge_index].to_node_id = id.clone();
            self.program.edges.push(FlowEdge {
                from_node_id: id.clone(),
                port: FlowPort::Next,
                to_node_id: end_id.clone(),
            });
            if let Some(end_node) = self.program.nodes.iter_mut().find(|node| node.id == end_id) {
                end_node.position = Some([
                    (position[0] + NODE_SIZE.x + NODE_GAP_X).min(CANVAS_LIMIT - NODE_SIZE.x),
                    position[1],
                ]);
            }
        }
        self.selected_node_id = Some(id);
    }

    fn delete_selected_node(&mut self) -> bool {
        let Some(id) = self.selected_node_id.clone() else {
            return false;
        };
        let Some(index) = self.program.nodes.iter().position(|node| node.id == id) else {
            self.selected_node_id = None;
            return false;
        };
        if matches!(self.program.nodes[index].kind, FlowNodeKind::End)
            || self.program.nodes.len() <= 1
        {
            return false;
        }

        self.program.nodes.remove(index);
        self.program
            .edges
            .retain(|edge| edge.from_node_id != id && edge.to_node_id != id);
        if self.program.entry_node_id == id {
            if let Some(next_entry) = self
                .program
                .nodes
                .iter()
                .find(|node| !matches!(node.kind, FlowNodeKind::End))
                .or_else(|| self.program.nodes.first())
            {
                self.program.entry_node_id = next_entry.id.clone();
            }
        }
        self.selected_node_id = None;
        self.selected_edge = None;
        true
    }

    fn delete_selected_edge(&mut self) -> bool {
        let Some((from_node_id, port)) = self.selected_edge.take() else {
            return false;
        };
        let old_len = self.program.edges.len();
        self.program
            .edges
            .retain(|edge| !(edge.from_node_id == from_node_id && edge.port == port));
        old_len != self.program.edges.len()
    }

    fn advance_animation_clock(&mut self, frame_time: f64, enabled: bool) {
        if let Some(previous) = self.last_frame_time {
            if enabled {
                self.animation_time += (frame_time - previous).clamp(0.0, 0.1);
            }
        }
        self.last_frame_time = Some(frame_time);
    }

    fn draw_canvas(
        &mut self,
        ui: &mut egui::Ui,
        tr: &impl Fn(&str) -> String,
        appearance: StudioAppearance,
        locked: bool,
        active_node_id: Option<&str>,
    ) -> bool {
        let mut changed = false;
        let accent = appearance.colors.accent;
        let frame_time = ui.input(|input| input.time);
        self.advance_animation_clock(frame_time, appearance.background_animations);
        if !ui.ctx().wants_keyboard_input() {
            if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                changed |= self.connecting.take().is_some();
                changed |= self.selected_node_id.take().is_some();
                changed |= self.selected_edge.take().is_some();
            }
            if !locked {
                let shortcut = ui.input(|input| {
                    [
                        egui::Key::Num1,
                        egui::Key::Num2,
                        egui::Key::Num3,
                        egui::Key::Num4,
                        egui::Key::Num5,
                        egui::Key::Num6,
                        egui::Key::Num7,
                        egui::Key::Num8,
                    ]
                    .iter()
                    .position(|key| input.key_pressed(*key))
                    .map(|index| index + 1)
                });
                if let Some(shortcut) = shortcut {
                    match shortcut {
                        1..=5 => {
                            if let Some((_, action)) = palette_actions(tr).get(shortcut - 1) {
                                self.add_action(action.clone());
                                changed = true;
                            }
                        }
                        6 => {
                            self.add_condition();
                            changed = true;
                        }
                        7 => {
                            self.add_repeat();
                            changed = true;
                        }
                        8 => {
                            self.add_variable();
                            changed = true;
                        }
                        _ => {}
                    }
                }
            }
            if !locked
                && (ui.input(|input| input.key_pressed(egui::Key::Delete))
                    || ui.input(|input| input.key_pressed(egui::Key::Backspace)))
            {
                changed |= self.delete_selected_edge() || self.delete_selected_node();
            }
        }
        let available_canvas = ui.available_size();
        let mut canvas_width = available_canvas.x.max(640.0);
        let mut canvas_height = available_canvas.y.max(360.0);
        for (index, node) in self.program.nodes.iter_mut().enumerate() {
            if node.position.is_none() {
                node.position = Some([
                    40.0 + (index % 4) as f32 * (NODE_SIZE.x + NODE_GAP_X),
                    64.0 + (index / 4) as f32 * (NODE_SIZE.y + NODE_GAP_Y),
                ]);
                changed = true;
            }
            if let Some([x, y]) = node.position {
                canvas_width = canvas_width.max(x + NODE_SIZE.x + 100.0);
                canvas_height = canvas_height.max(y + NODE_SIZE.y + 120.0);
            }
        }
        egui::ScrollArea::both()
            .id_source("studio_canvas_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let size = Vec2::new(canvas_width, canvas_height);
                let (canvas, canvas_response) = ui.allocate_exact_size(size, egui::Sense::click());
                let painter = ui.painter_at(canvas);
                let palette = appearance.colors;
                painter.rect_filled(canvas, 8.0, palette.canvas);
                painter.rect_stroke(canvas, 8.0, Stroke::new(1.0_f32, palette.border));
                let clip = painter.clip_rect().intersect(canvas);
                paint_canvas_background(
                    &painter,
                    canvas,
                    clip,
                    appearance,
                    self.animation_time as f32,
                );
                let time = ui.input(|input| input.time) as f32;
                for edge in &self.program.edges {
                    if let (Some(source), Some(target)) = (
                        self.program
                            .nodes
                            .iter()
                            .find(|node| node.id == edge.from_node_id),
                        self.program
                            .nodes
                            .iter()
                            .find(|node| node.id == edge.to_node_id),
                    ) {
                        let from = output_position(source, edge.port, canvas.min);
                        let to = input_position(target, canvas.min);
                        let selected = self.selected_edge.as_ref()
                            == Some(&(edge.from_node_id.clone(), edge.port));
                        let color = if selected { palette.warning } else { accent };
                        paint_connection(
                            &painter,
                            from,
                            to,
                            color,
                            if selected { 2.8 } else { 1.8 },
                            appearance.glow_strength,
                        );
                        if appearance.animated_edges && !selected {
                            let phase = (time * 0.22).fract();
                            painter.circle_filled(
                                bezier_point(from, to, phase),
                                3.0,
                                with_alpha(color, 0.9),
                            );
                        }
                        if edge.port != FlowPort::Next {
                            painter.text(
                                from.lerp(to, 0.5) + Vec2::new(0.0, -10.0),
                                Align2::CENTER_CENTER,
                                port_label(edge.port, tr),
                                FontId::proportional(10.0),
                                palette.text,
                            );
                        }
                    }
                }
                let mut new_connection = None;
                let mut node_clicked = false;
                for index in 0..self.program.nodes.len() {
                    let id = self.program.nodes[index].id.clone();
                    let position = self.program.nodes[index].position.unwrap_or([40.0, 40.0]);
                    let top_left = canvas.min + Vec2::new(position[0], position[1]);
                    let rect = egui::Rect::from_min_size(top_left, NODE_SIZE);
                    let is_selected = self.selected_node_id.as_deref() == Some(id.as_str());
                    let sense = if locked {
                        egui::Sense::click()
                    } else {
                        egui::Sense::click_and_drag()
                    };
                    let response =
                        ui.interact(rect, ui.id().with(("studio-node", id.clone())), sense);
                    if response.drag_started() && !locked && self.connecting.is_none() {
                        let pointer_origin = ui
                            .input(|input| input.pointer.press_origin())
                            .or_else(|| response.interact_pointer_pos())
                            .unwrap_or(canvas.min);
                        self.drag_origin = Some((id.clone(), position, pointer_origin));
                    }
                    if response.clicked() {
                        node_clicked = true;
                        self.selected_edge = None;
                        if let Some((from_id, port)) = self.connecting.take() {
                            if from_id != id {
                                self.program.edges.retain(|edge| {
                                    !(edge.from_node_id == from_id && edge.port == port)
                                });
                                self.program.edges.push(FlowEdge {
                                    from_node_id: from_id,
                                    port,
                                    to_node_id: id.clone(),
                                });
                                changed = true;
                            } else {
                                self.selected_node_id = Some(id.clone());
                            }
                        } else {
                            self.selected_node_id = Some(id.clone());
                        }
                    }
                    if (response.dragged() || response.drag_released())
                        && !locked
                        && self.connecting.is_none()
                    {
                        let (start_position, start_pointer) = self
                            .drag_origin
                            .as_ref()
                            .filter(|(dragged_id, _, _)| dragged_id == &id)
                            .map(|(_, start_position, start_pointer)| {
                                (*start_position, *start_pointer)
                            })
                            .unwrap_or((
                                position,
                                response.interact_pointer_pos().unwrap_or(canvas.min),
                            ));
                        let current_pointer =
                            response.interact_pointer_pos().unwrap_or(start_pointer);
                        self.program.nodes[index].position = Some(drag_position(
                            start_position,
                            start_pointer,
                            current_pointer,
                            appearance.snap_to_grid,
                        ));
                        changed = true;
                    }
                    if response.drag_released() {
                        self.drag_origin = None;
                    }
                    let kind = self.program.nodes[index].kind.clone();
                    let category = node_color(&kind, &palette);
                    let is_playing = active_node_id == Some(id.as_str());
                    let node_rounding = if matches!(kind, FlowNodeKind::End) {
                        44.0
                    } else {
                        9.0
                    };
                    let highlight = if is_playing {
                        palette.success
                    } else {
                        category
                    };
                    if appearance.glow_strength > 0.01 {
                        painter.rect_filled(
                            rect.expand(if is_selected || is_playing { 5.0 } else { 2.0 }),
                            node_rounding,
                            with_alpha(
                                highlight,
                                appearance.glow_strength
                                    * if is_selected || is_playing {
                                        0.25
                                    } else {
                                        0.07
                                    },
                            ),
                        );
                    }
                    let node_fill = if is_playing {
                        with_alpha(palette.success, if palette.is_light { 0.16 } else { 0.12 })
                    } else if is_selected {
                        with_alpha(category, if palette.is_light { 0.13 } else { 0.10 })
                    } else {
                        palette.panel_alt
                    };
                    painter.rect_filled(rect, node_rounding, node_fill);
                    painter.rect_stroke(
                        rect,
                        node_rounding,
                        Stroke::new(
                            if is_selected || is_playing {
                                1.8_f32
                            } else {
                                1.0_f32
                            },
                            if is_playing {
                                palette.success
                            } else if is_selected {
                                category
                            } else {
                                palette.border
                            },
                        ),
                    );
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            rect.left_top(),
                            Pos2::new(rect.left() + 5.0, rect.bottom()),
                        ),
                        egui::Rounding::same(node_rounding.min(8.0)),
                        category,
                    );
                    let (title, detail) = node_text(&self.program.nodes[index], tr);
                    let icon_rect = egui::Rect::from_min_size(
                        rect.left_top() + Vec2::new(14.0, 14.0),
                        Vec2::splat(28.0),
                    );
                    painter.rect_filled(icon_rect, 6.0, with_alpha(category, 0.18));
                    painter.text(
                        icon_rect.center(),
                        Align2::CENTER_CENTER,
                        node_glyph(&kind),
                        FontId::proportional(16.0),
                        category,
                    );
                    painter.text(
                        rect.left_top() + Vec2::new(52.0, 14.0),
                        Align2::LEFT_TOP,
                        truncate(&title, 18),
                        FontId::proportional(14.0),
                        palette.text,
                    );
                    painter.text(
                        rect.right_top() - Vec2::new(28.0, -17.0),
                        Align2::CENTER_CENTER,
                        if id == self.program.entry_node_id {
                            tr("studio_entry")
                        } else {
                            format!("#{}", index + 1)
                        },
                        FontId::monospace(9.0),
                        if is_playing {
                            palette.success
                        } else {
                            palette.muted
                        },
                    );
                    painter.text(
                        rect.left_top() + Vec2::new(16.0, 56.0),
                        Align2::LEFT_TOP,
                        truncate(&detail, 30),
                        FontId::proportional(11.0),
                        palette.muted,
                    );
                    if let Some(outputs) = output_ports(&kind) {
                        for (port_index, port) in outputs.iter().copied().enumerate() {
                            let point =
                                output_position(&self.program.nodes[index], port, canvas.min);
                            let port_rect = egui::Rect::from_center_size(point, Vec2::splat(22.0));
                            let port_response = ui.interact(
                                port_rect,
                                ui.id().with(("studio-port", id.clone(), port)),
                                egui::Sense::click(),
                            );
                            let is_connected = self
                                .program
                                .edges
                                .iter()
                                .any(|edge| edge.from_node_id == id && edge.port == port);
                            painter.circle_filled(
                                point,
                                if is_connected { 5.5 } else { 6.5 },
                                category,
                            );
                            painter.circle_stroke(point, 5.5, Stroke::new(1.2_f32, palette.canvas));
                            let label = port_label(port, tr);
                            painter.text(
                                point + Vec2::new(0.0, 16.0 + port_index as f32 * 0.0),
                                Align2::CENTER_TOP,
                                &label,
                                FontId::proportional(10.0),
                                ui.visuals().weak_text_color(),
                            );
                            if port_response.clicked() && !locked {
                                node_clicked = true;
                                self.selected_edge = None;
                                self.selected_node_id = Some(id.clone());
                                new_connection = Some((id.clone(), port));
                            }
                        }
                    }
                    let input = input_position(&self.program.nodes[index], canvas.min);
                    painter.circle_filled(input, 4.0, palette.muted);
                    response.on_hover_text(if self.connecting.is_some() {
                        tr("studio_click_target")
                    } else {
                        tr("studio_drag_node")
                    });
                }
                if canvas_response.clicked() && !node_clicked {
                    if let Some(pointer) = ui.input(|input| input.pointer.interact_pos()) {
                        if let Some((from_node_id, port, _)) =
                            nearest_edge(&self.program, canvas.min, pointer)
                        {
                            self.selected_node_id = None;
                            self.connecting = None;
                            self.selected_edge = Some((from_node_id, port));
                        } else {
                            self.selected_node_id = None;
                            self.selected_edge = None;
                            self.connecting = None;
                        }
                        changed = true;
                    }
                }
                if let Some(connection) = new_connection {
                    self.connecting = Some(connection);
                }
                if let Some((source_id, port)) = self.connecting.as_ref() {
                    if let Some(source) =
                        self.program.nodes.iter().find(|node| &node.id == source_id)
                    {
                        let from = output_position(source, *port, canvas.min);
                        if let Some(pointer) = ui.input(|input| input.pointer.hover_pos()) {
                            paint_connection(
                                &painter,
                                from,
                                pointer,
                                accent,
                                1.4,
                                appearance.glow_strength,
                            );
                        }
                    }
                }
                if appearance.crt_overlay {
                    let mut y = clip.top();
                    while y < clip.bottom() {
                        painter.line_segment(
                            [Pos2::new(clip.left(), y), Pos2::new(clip.right(), y)],
                            Stroke::new(1.0_f32, with_alpha(palette.app_background, 0.14)),
                        );
                        y += 4.0;
                    }
                }
            });
        if (appearance.animated_edges && !self.program.edges.is_empty())
            || (appearance.background_animations && appearance.background_intensity > 0.0)
        {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(50));
        }
        if let Some((text, _)) = self.message.as_ref() {
            ui.label(RichText::new(text).small());
        }
        changed
    }

    fn draw_edge_inspector(
        &mut self,
        ui: &mut egui::Ui,
        tr: &impl Fn(&str) -> String,
        locked: bool,
        source_id: &str,
        port: FlowPort,
    ) -> bool {
        let target_id = self
            .program
            .edges
            .iter()
            .find(|edge| edge.from_node_id == source_id && edge.port == port)
            .map(|edge| edge.to_node_id.clone());
        ui.label(RichText::new(tr("studio_connection_properties")).strong());
        ui.add_space(10.0);
        ui.label(format!("{} · {}", tr("studio_node_label"), source_id));
        ui.label(format!("{} · {}", tr("port_label"), port_label(port, tr)));
        if let Some(target_id) = target_id {
            ui.label(format!("{} · {}", tr("studio_connected_to"), target_id));
        }
        ui.add_space(12.0);
        ui.add_enabled_ui(!locked, |ui| {
            if ui
                .button(format!("X  {}", tr("studio_remove_connection")))
                .clicked()
            {
                return self.delete_selected_edge();
            }
            false
        })
        .inner
    }

    fn draw_inspector(
        &mut self,
        ui: &mut egui::Ui,
        tr: &impl Fn(&str) -> String,
        appearance: StudioAppearance,
        locked: bool,
    ) -> bool {
        if self.selected_node_id.is_none() {
            if let Some((source_id, port)) = self.selected_edge.clone() {
                return self.draw_edge_inspector(ui, tr, locked, &source_id, port);
            }
        }
        let accent = appearance.colors.accent;
        let Some(id) = self.selected_node_id.clone() else {
            ui.add_space(12.0);
            ui.label(RichText::new(tr("studio_select_hint")).weak());
            return false;
        };
        let Some(index) = self.program.nodes.iter().position(|node| node.id == id) else {
            self.selected_node_id = None;
            return false;
        };
        let mut changed = false;
        let node = &mut self.program.nodes[index];
        let (title, _) = node_text(node, tr);
        ui.label(RichText::new(title).color(accent).strong());
        ui.label(RichText::new(format!("ID · {}", node.id)).small().weak());
        ui.add_space(10.0);
        ui.add_enabled_ui(!locked, |ui| {
            ui.label(tr("studio_node_label"));
            changed |= ui.text_edit_singleline(&mut node.label).changed();
            ui.add_space(8.0);
            match &mut node.kind {
                FlowNodeKind::Action {
                    action,
                    delay_after_ms,
                } => {
                    let current = action_kind_index(action);
                    egui::ComboBox::from_id_source(("studio-action-kind", id.as_str()))
                        .selected_text(action_kind_label(action, tr))
                        .show_ui(ui, |ui| {
                            for (i, key) in [
                                "add_click",
                                "add_move",
                                "add_move_smooth",
                                "add_key",
                                "add_text",
                                "add_wait",
                            ]
                            .iter()
                            .enumerate()
                            {
                                if ui.selectable_label(current == i, tr(key)).clicked() {
                                    *action = default_action(i);
                                    changed = true;
                                }
                            }
                        });
                    match action {
                        Action::MouseClick {
                            button,
                            click_type,
                            hold_ms,
                            position,
                        } => {
                            ui.label(tr("button"));
                            egui::ComboBox::from_id_source(("studio-button", id.as_str()))
                                .selected_text(tr(match button {
                                    MouseButton::Left => "button_left",
                                    MouseButton::Right => "button_right",
                                    MouseButton::Middle => "button_middle",
                                }))
                                .show_ui(ui, |ui| {
                                    for (value, key) in [
                                        (MouseButton::Left, "button_left"),
                                        (MouseButton::Right, "button_right"),
                                        (MouseButton::Middle, "button_middle"),
                                    ] {
                                        if ui.selectable_value(button, value, tr(key)).changed() {
                                            changed = true;
                                        }
                                    }
                                });
                            ui.label(tr("click_type"));
                            egui::ComboBox::from_id_source(("studio-click", id.as_str()))
                                .selected_text(tr(match click_type {
                                    ClickType::Single => "single",
                                    ClickType::Double => "double",
                                    ClickType::Hold => "hold",
                                }))
                                .show_ui(ui, |ui| {
                                    for (value, key) in [
                                        (ClickType::Single, "single"),
                                        (ClickType::Double, "double"),
                                        (ClickType::Hold, "hold"),
                                    ] {
                                        if ui.selectable_value(click_type, value, tr(key)).changed()
                                        {
                                            changed = true;
                                        }
                                    }
                                });
                            let mut fixed = position.is_some();
                            if ui.checkbox(&mut fixed, tr("fixed_position")).changed() {
                                *position = if fixed { Some([0, 0]) } else { None };
                                changed = true;
                            }
                            if let Some([x, y]) = position {
                                changed |= ui.add(egui::DragValue::new(x).prefix("X ")).changed();
                                changed |= ui.add(egui::DragValue::new(y).prefix("Y ")).changed();
                            }
                            if *click_type == ClickType::Hold {
                                changed |= ui
                                    .add(
                                        egui::DragValue::new(hold_ms)
                                            .clamp_range(1..=86_400_000)
                                            .suffix(" ms"),
                                    )
                                    .changed();
                            }
                        }
                        Action::MouseMove { position } => {
                            changed |= ui
                                .add(egui::DragValue::new(&mut position[0]).prefix("X "))
                                .changed();
                            changed |= ui
                                .add(egui::DragValue::new(&mut position[1]).prefix("Y "))
                                .changed();
                        }
                        Action::MouseMoveSmooth {
                            position,
                            duration_ms,
                        } => {
                            changed |= ui
                                .add(egui::DragValue::new(&mut position[0]).prefix("X "))
                                .changed();
                            changed |= ui
                                .add(egui::DragValue::new(&mut position[1]).prefix("Y "))
                                .changed();
                            changed |= ui
                                .add(
                                    egui::DragValue::new(duration_ms)
                                        .clamp_range(0..=120_000)
                                        .suffix(" ms"),
                                )
                                .changed();
                        }
                        Action::KeyPress { key, hold_ms } => {
                            ui.label(tr("key"));
                            changed |= ui.text_edit_singleline(key).changed();
                            changed |= ui
                                .add(
                                    egui::DragValue::new(hold_ms)
                                        .clamp_range(1..=60_000)
                                        .suffix(" ms"),
                                )
                                .changed();
                        }
                        Action::TypeText { text } => {
                            ui.label(tr("text"));
                            changed |= ui
                                .add(egui::TextEdit::multiline(text).desired_rows(3))
                                .changed();
                        }
                        Action::Wait { duration_ms } => {
                            changed |= ui
                                .add(
                                    egui::DragValue::new(duration_ms)
                                        .clamp_range(0..=86_400_000)
                                        .suffix(" ms"),
                                )
                                .changed();
                            ui.label(RichText::new(tr("wait_presets")).small().weak());
                            ui.horizontal_wrapped(|ui| {
                                for (label, value) in [
                                    ("250 ms", 250),
                                    ("0.5 s", 500),
                                    ("1 s", 1_000),
                                    ("5 s", 5_000),
                                ] {
                                    if ui.small_button(label).clicked() {
                                        *duration_ms = value;
                                        changed = true;
                                    }
                                }
                            });
                        }
                    }
                    ui.add_space(6.0);
                    ui.label(tr("delay_after"));
                    changed |= ui
                        .add(
                            egui::DragValue::new(delay_after_ms)
                                .clamp_range(0..=86_400_000)
                                .suffix(" ms"),
                        )
                        .changed();
                }
                FlowNodeKind::Repeat {
                    iterations,
                    counter_variable,
                } => {
                    ui.label(tr("studio_iterations"));
                    changed |= ui
                        .add(egui::DragValue::new(iterations).clamp_range(1..=100_000))
                        .changed();
                    ui.label(tr("studio_counter_optional"));
                    let mut value = counter_variable.clone().unwrap_or_default();
                    if ui.text_edit_singleline(&mut value).changed() {
                        *counter_variable = if value.trim().is_empty() {
                            None
                        } else {
                            Some(value)
                        };
                        changed = true;
                    }
                }
                FlowNodeKind::Condition {
                    left,
                    comparison,
                    right,
                } => {
                    ui.label(tr("studio_condition_note"));
                    changed |= edit_operand(ui, left, "studio_left", tr);
                    egui::ComboBox::from_id_source(("studio-compare", id.as_str()))
                        .selected_text(comparison_label(*comparison, tr))
                        .show_ui(ui, |ui| {
                            for value in [
                                Comparison::Equal,
                                Comparison::NotEqual,
                                Comparison::LessThan,
                                Comparison::LessOrEqual,
                                Comparison::GreaterThan,
                                Comparison::GreaterOrEqual,
                                Comparison::Contains,
                            ] {
                                if ui
                                    .selectable_value(
                                        comparison,
                                        value,
                                        comparison_label(value, tr),
                                    )
                                    .changed()
                                {
                                    changed = true;
                                }
                            }
                        });
                    changed |= edit_operand(ui, right, "studio_right", tr);
                }
                FlowNodeKind::SetVariable { name, operation } => {
                    ui.label(tr("studio_variable_name"));
                    changed |= ui.text_edit_singleline(name).changed();
                    let operation_index = variable_operation_index(operation);
                    egui::ComboBox::from_id_source(("studio-variable-operation", id.as_str()))
                        .selected_text(variable_operation_label(operation_index, tr))
                        .show_ui(ui, |ui| {
                            for index in 0..4 {
                                if ui
                                    .selectable_label(
                                        operation_index == index,
                                        variable_operation_label(index, tr),
                                    )
                                    .clicked()
                                {
                                    *operation = default_variable_operation(index);
                                    changed = true;
                                }
                            }
                        });
                    match operation {
                        VariableOperation::Set { value } => {
                            ui.label(tr("studio_value"));
                            changed |= edit_value(ui, value, tr);
                        }
                        VariableOperation::Add { amount } => {
                            changed |= ui.add(egui::DragValue::new(amount).speed(0.1)).changed();
                        }
                        VariableOperation::Toggle => {
                            ui.label(tr("studio_toggle_bool"));
                        }
                        VariableOperation::Append { text } => {
                            changed |= ui.text_edit_singleline(text).changed();
                        }
                    }
                }
                FlowNodeKind::LoopEnd { repeat_node_id } => {
                    ui.label(tr("studio_loop_reference"));
                    changed |= ui.text_edit_singleline(repeat_node_id).changed();
                }
                FlowNodeKind::End => {
                    ui.label(RichText::new(tr("studio_terminal_node")).weak());
                }
            }
        });
        ui.add_space(14.0);
        if !matches!(node.kind, FlowNodeKind::End)
            && ui
                .add_enabled(
                    !locked,
                    egui::Button::new(
                        RichText::new(format!("X  {}", tr("studio_delete_node")))
                            .color(Color32::from_rgb(239, 106, 116)),
                    ),
                )
                .clicked()
        {
            self.program.nodes.remove(index);
            self.program
                .edges
                .retain(|edge| edge.from_node_id != id && edge.to_node_id != id);
            if self.program.entry_node_id == id {
                if let Some(node) = self
                    .program
                    .nodes
                    .iter()
                    .find(|node| !matches!(node.kind, FlowNodeKind::End))
                {
                    self.program.entry_node_id = node.id.clone();
                }
            }
            self.selected_node_id = None;
            changed = true;
        }
        if let Some(edge) = self
            .program
            .edges
            .iter()
            .find(|edge| edge.from_node_id == id)
        {
            ui.add_space(8.0);
            ui.label(
                RichText::new(format!(
                    "{} -> {}",
                    tr("studio_connected_to"),
                    edge.to_node_id
                ))
                .small()
                .weak(),
            );
        }
        changed
    }
}

fn write_workflow_to(path: &Path, program: &FlowProgram) -> Result<(), String> {
    let contents = program.to_json()?;
    std::fs::write(path, contents)
        .map_err(|error| format!("Could not write workflow export: {error}"))
}

fn palette_actions(tr: &impl Fn(&str) -> String) -> Vec<(String, Action)> {
    vec![
        (tr("add_click"), Action::default()),
        (
            tr("add_move_smooth"),
            Action::MouseMoveSmooth {
                position: [600, 400],
                duration_ms: 350,
            },
        ),
        (
            tr("add_key"),
            Action::KeyPress {
                key: "Space".into(),
                hold_ms: 35,
            },
        ),
        (
            tr("add_text"),
            Action::TypeText {
                text: "Text".into(),
            },
        ),
        (tr("add_wait"), Action::Wait { duration_ms: 500 }),
    ]
}

fn sequence_position(index: usize) -> [f32; 2] {
    const COLUMNS: usize = 4;
    let row = index / COLUMNS;
    let offset = index % COLUMNS;
    let column = if row & 1 == 0 {
        offset
    } else {
        COLUMNS - 1 - offset
    };
    [
        40.0 + column as f32 * (NODE_SIZE.x + NODE_GAP_X),
        64.0 + row as f32 * (NODE_SIZE.y + NODE_GAP_Y),
    ]
}

fn default_action(index: usize) -> Action {
    match index {
        1 => Action::MouseMove {
            position: [600, 400],
        },
        2 => Action::MouseMoveSmooth {
            position: [600, 400],
            duration_ms: 350,
        },
        3 => Action::KeyPress {
            key: "Space".into(),
            hold_ms: 35,
        },
        4 => Action::TypeText {
            text: "Text".into(),
        },
        5 => Action::Wait { duration_ms: 500 },
        _ => Action::default(),
    }
}

fn action_kind_index(action: &Action) -> usize {
    match action {
        Action::MouseClick { .. } => 0,
        Action::MouseMove { .. } => 1,
        Action::MouseMoveSmooth { .. } => 2,
        Action::KeyPress { .. } => 3,
        Action::TypeText { .. } => 4,
        Action::Wait { .. } => 5,
    }
}

fn action_kind_label(action: &Action, tr: &impl Fn(&str) -> String) -> String {
    match action {
        Action::MouseClick { .. } => tr("add_click"),
        Action::MouseMove { .. } => tr("add_move"),
        Action::MouseMoveSmooth { .. } => tr("add_move_smooth"),
        Action::KeyPress { .. } => tr("add_key"),
        Action::TypeText { .. } => tr("add_text"),
        Action::Wait { .. } => tr("add_wait"),
    }
}

fn action_label(action: &Action) -> String {
    match action {
        Action::MouseClick { .. } => "Mouse Click".into(),
        Action::MouseMove { .. } => "Mouse Move".into(),
        Action::MouseMoveSmooth { .. } => "Smooth Mouse Move".into(),
        Action::KeyPress { .. } => "Keyboard".into(),
        Action::TypeText { .. } => "Type Text".into(),
        Action::Wait { .. } => "Delay".into(),
    }
}

fn comparison_label(comparison: Comparison, tr: &impl Fn(&str) -> String) -> String {
    tr(match comparison {
        Comparison::Equal => "studio_compare_equal",
        Comparison::NotEqual => "studio_compare_not_equal",
        Comparison::LessThan => "studio_compare_less",
        Comparison::LessOrEqual => "studio_compare_less_equal",
        Comparison::GreaterThan => "studio_compare_greater",
        Comparison::GreaterOrEqual => "studio_compare_greater_equal",
        Comparison::Contains => "studio_compare_contains",
    })
}

fn node_text(node: &FlowNode, tr: &impl Fn(&str) -> String) -> (String, String) {
    match &node.kind {
        FlowNodeKind::Action {
            action,
            delay_after_ms,
        } => {
            let detail = match action {
                Action::MouseClick {
                    button,
                    click_type,
                    position,
                    ..
                } => format!(
                    "{} · {} · {}",
                    tr(match button {
                        MouseButton::Left => "button_left",
                        MouseButton::Right => "button_right",
                        MouseButton::Middle => "button_middle",
                    }),
                    tr(match click_type {
                        ClickType::Single => "single",
                        ClickType::Double => "double",
                        ClickType::Hold => "hold",
                    }),
                    position
                        .map(|p| format!("{}, {}", p[0], p[1]))
                        .unwrap_or_else(|| tr("cursor"))
                ),
                Action::MouseMove { position } => {
                    format!("{} · {}, {}", tr("add_move"), position[0], position[1])
                }
                Action::MouseMoveSmooth {
                    position,
                    duration_ms,
                } => format!(
                    "{} · {}, {} · {duration_ms} ms",
                    tr("add_move_smooth"),
                    position[0],
                    position[1]
                ),
                Action::KeyPress { key, .. } => format!("{} · {key}", tr("add_key")),
                Action::TypeText { text } => {
                    format!("{} · {}", tr("add_text"), truncate(text, 25))
                }
                Action::Wait { duration_ms } => {
                    format!("{} · {duration_ms} ms", tr("add_wait"))
                }
            };
            let detail = if *delay_after_ms > 0 {
                format!("{detail} · +{delay_after_ms} ms")
            } else {
                detail
            };
            (node.label.clone(), detail)
        }
        FlowNodeKind::SetVariable { name, .. } => (
            node.label.clone(),
            format!("{} · {name}", tr("studio_set_variable")),
        ),
        FlowNodeKind::Condition { comparison, .. } => (
            node.label.clone(),
            format!(
                "{} · {}",
                tr("studio_condition"),
                comparison_label(*comparison, tr)
            ),
        ),
        FlowNodeKind::Repeat { iterations, .. } => (
            node.label.clone(),
            format!("{} · {iterations}", tr("studio_repeat")),
        ),
        FlowNodeKind::LoopEnd { .. } => (node.label.clone(), tr("studio_loop_reference")),
        FlowNodeKind::End => (tr("studio_end_node"), tr("studio_terminal_node")),
    }
}

fn node_color(kind: &FlowNodeKind, palette: &ThemeColors) -> Color32 {
    match kind {
        FlowNodeKind::Action {
            action: Action::MouseClick { .. },
            ..
        } => palette.accent,
        FlowNodeKind::Action {
            action: Action::MouseMove { .. } | Action::MouseMoveSmooth { .. },
            ..
        } => palette.move_node,
        FlowNodeKind::Action {
            action: Action::KeyPress { .. } | Action::TypeText { .. },
            ..
        } => palette.keyboard_node,
        FlowNodeKind::Action {
            action: Action::Wait { .. },
            ..
        } => palette.wait_node,
        FlowNodeKind::Condition { .. } => palette.condition_node,
        FlowNodeKind::Repeat { .. } | FlowNodeKind::LoopEnd { .. } => palette.loop_node,
        FlowNodeKind::SetVariable { .. } => palette.variable_node,
        FlowNodeKind::End => palette.end_node,
    }
}

fn output_ports(kind: &FlowNodeKind) -> Option<&'static [FlowPort]> {
    match kind {
        FlowNodeKind::Action { .. } | FlowNodeKind::SetVariable { .. } => Some(&[FlowPort::Next]),
        FlowNodeKind::Condition { .. } => Some(&[FlowPort::True, FlowPort::False]),
        FlowNodeKind::Repeat { .. } => Some(&[FlowPort::Body, FlowPort::Done]),
        FlowNodeKind::LoopEnd { .. } | FlowNodeKind::End => None,
    }
}

fn input_position(node: &FlowNode, canvas_origin: Pos2) -> Pos2 {
    canvas_origin
        + Vec2::new(
            node.position.map(|p| p[0]).unwrap_or(0.0),
            node.position.map(|p| p[1]).unwrap_or(0.0) + NODE_SIZE.y * 0.5,
        )
}

fn output_position(node: &FlowNode, port: FlowPort, canvas_origin: Pos2) -> Pos2 {
    let x = node.position.map(|p| p[0]).unwrap_or(0.0) + NODE_SIZE.x;
    let base_y = node.position.map(|p| p[1]).unwrap_or(0.0);
    let y = match port {
        FlowPort::True | FlowPort::Body => base_y + NODE_SIZE.y * 0.35,
        FlowPort::False | FlowPort::Done => base_y + NODE_SIZE.y * 0.72,
        FlowPort::Next => base_y + NODE_SIZE.y * 0.5,
    };
    canvas_origin + Vec2::new(x, y)
}

fn palette_button(
    ui: &mut egui::Ui,
    label: &str,
    shortcut: usize,
    color: Color32,
    enabled: bool,
) -> bool {
    ui.add_enabled_ui(enabled, |ui| {
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 42.0), egui::Sense::click());
        let painter = ui.painter_at(rect);
        let fill = with_alpha(color, if response.hovered() { 0.20 } else { 0.10 });
        painter.rect_filled(rect, 8.0, fill);
        painter.rect_stroke(
            rect,
            8.0,
            Stroke::new(
                1.0_f32,
                with_alpha(color, if response.hovered() { 0.8 } else { 0.42 }),
            ),
        );
        let badge = egui::Rect::from_min_size(rect.min + Vec2::new(8.0, 8.0), Vec2::splat(26.0));
        painter.rect_filled(badge, 6.0, with_alpha(color, 0.24));
        painter.text(
            badge.center(),
            Align2::CENTER_CENTER,
            shortcut.to_string(),
            FontId::monospace(11.0),
            color,
        );
        painter.text(
            rect.min + Vec2::new(44.0, 13.0),
            Align2::LEFT_TOP,
            label,
            FontId::proportional(12.0),
            ui.visuals().text_color(),
        );
        response.clicked()
    })
    .inner
}

fn contrast_text_color(color: Color32) -> Color32 {
    let luminance = 0.2126 * f32::from(color.r())
        + 0.7152 * f32::from(color.g())
        + 0.0722 * f32::from(color.b());
    if luminance > 145.0 {
        Color32::from_rgb(12, 20, 31)
    } else {
        Color32::WHITE
    }
}

fn with_alpha(color: Color32, alpha: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        color.r(),
        color.g(),
        color.b(),
        (255.0 * alpha.clamp(0.0, 1.0)) as u8,
    )
}

fn node_glyph(kind: &FlowNodeKind) -> &'static str {
    match kind {
        FlowNodeKind::Action {
            action: Action::MouseClick { .. },
            ..
        } => "C",
        FlowNodeKind::Action {
            action: Action::MouseMove { .. } | Action::MouseMoveSmooth { .. },
            ..
        } => "M",
        FlowNodeKind::Action {
            action: Action::KeyPress { .. } | Action::TypeText { .. },
            ..
        } => "K",
        FlowNodeKind::Action {
            action: Action::Wait { .. },
            ..
        } => "W",
        FlowNodeKind::Condition { .. } => "?",
        FlowNodeKind::Repeat { .. } => "R",
        FlowNodeKind::LoopEnd { .. } => "B",
        FlowNodeKind::SetVariable { .. } => "V",
        FlowNodeKind::End => "E",
    }
}

fn port_label(port: FlowPort, tr: &impl Fn(&str) -> String) -> String {
    tr(match port {
        FlowPort::Next => "port_next",
        FlowPort::True => "port_true",
        FlowPort::False => "port_false",
        FlowPort::Body => "port_body",
        FlowPort::Done => "port_done",
    })
}

fn paint_canvas_background(
    painter: &egui::Painter,
    canvas: egui::Rect,
    clip: egui::Rect,
    appearance: StudioAppearance,
    animation_time: f32,
) {
    let palette = appearance.colors;
    let intensity = appearance.background_intensity;
    if intensity <= 0.0 {
        return;
    }
    let grid = with_alpha(palette.grid, intensity * 0.58);
    let accent = with_alpha(palette.accent, intensity * 0.42);
    match appearance.background {
        CanvasBackground::Solid => {
            let pulse = 0.5 + (animation_time * 0.4).sin() * 0.5;
            let radius = canvas.width().max(canvas.height()) * 0.35;
            let first = canvas.left_top()
                + Vec2::new(
                    canvas.width() * (0.28 + (animation_time * 0.16).sin() * 0.04),
                    canvas.height() * 0.3,
                );
            let second = canvas.left_top()
                + Vec2::new(
                    canvas.width() * 0.72,
                    canvas.height() * (0.7 + (animation_time * 0.13).cos() * 0.04),
                );
            painter.circle_filled(
                first,
                radius,
                with_alpha(palette.accent, intensity * (0.025 + pulse * 0.025)),
            );
            painter.circle_filled(
                second,
                radius * 0.82,
                with_alpha(
                    palette.move_node,
                    intensity * (0.02 + (1.0 - pulse) * 0.025),
                ),
            );
        }
        CanvasBackground::Dots => {
            let step = 24.0;
            let phase = (animation_time * 2.0).rem_euclid(step);
            let start_x = ((clip.left() - canvas.left() - phase) / step).floor() as i32 - 1;
            let end_x = ((clip.right() - canvas.left() - phase) / step).ceil() as i32 + 1;
            let start_y = ((clip.top() - canvas.top() - phase) / step).floor() as i32 - 1;
            let end_y = ((clip.bottom() - canvas.top() - phase) / step).ceil() as i32 + 1;
            for x in start_x..=end_x {
                for y in start_y..=end_y {
                    painter.circle_filled(
                        canvas.min + Vec2::new(x as f32 * step + phase, y as f32 * step + phase),
                        1.0,
                        grid,
                    );
                }
            }
        }
        CanvasBackground::Grid | CanvasBackground::Blueprint => {
            let step = if appearance.background == CanvasBackground::Blueprint {
                24.0
            } else {
                32.0
            };
            let phase = (animation_time * 1.5).rem_euclid(step);
            let mut x = canvas.left()
                + ((clip.left() - canvas.left() - phase) / step).floor() * step
                + phase;
            while x <= clip.right() {
                let major = appearance.background == CanvasBackground::Blueprint
                    && ((x - canvas.left() - phase) / step).round() as i32 % 4 == 0;
                painter.line_segment(
                    [Pos2::new(x, clip.top()), Pos2::new(x, clip.bottom())],
                    Stroke::new(
                        if major { 1.1 } else { 0.65_f32 },
                        if major { accent } else { grid },
                    ),
                );
                x += step;
            }
            let mut y =
                canvas.top() + ((clip.top() - canvas.top() - phase) / step).floor() * step + phase;
            while y <= clip.bottom() {
                let major = appearance.background == CanvasBackground::Blueprint
                    && ((y - canvas.top() - phase) / step).round() as i32 % 4 == 0;
                painter.line_segment(
                    [Pos2::new(clip.left(), y), Pos2::new(clip.right(), y)],
                    Stroke::new(
                        if major { 1.1 } else { 0.65_f32 },
                        if major { accent } else { grid },
                    ),
                );
                y += step;
            }
        }
        CanvasBackground::Scanlines => {
            let phase = (animation_time * 3.0).rem_euclid(4.0);
            let mut y =
                canvas.top() + ((clip.top() - canvas.top() - phase) / 4.0).floor() * 4.0 + phase;
            while y <= clip.bottom() {
                painter.line_segment(
                    [Pos2::new(clip.left(), y), Pos2::new(clip.right(), y)],
                    Stroke::new(1.0_f32, with_alpha(palette.accent, intensity * 0.16)),
                );
                y += 4.0;
            }
        }
        CanvasBackground::Hex => {
            let radius = 13.0;
            let step_x = radius * 2.0;
            let step_y = radius * 1.75;
            let phase_x = (animation_time * 1.2).rem_euclid(step_x);
            let phase_y = animation_time.rem_euclid(step_y);
            let start_x = ((clip.left() - canvas.left() - phase_x) / step_x).floor() as i32 - 1;
            let end_x = ((clip.right() - canvas.left() - phase_x) / step_x).ceil() as i32 + 1;
            let start_y = ((clip.top() - canvas.top() - phase_y) / step_y).floor() as i32 - 1;
            let end_y = ((clip.bottom() - canvas.top() - phase_y) / step_y).ceil() as i32 + 1;
            for row in start_y..=end_y {
                for column in start_x..=end_x {
                    let center = canvas.min
                        + Vec2::new(
                            column as f32 * step_x
                                + if row.rem_euclid(2) == 0 { 0.0 } else { radius }
                                + phase_x,
                            row as f32 * step_y + phase_y,
                        );
                    let points: Vec<Pos2> = (0..=6)
                        .map(|index| {
                            let angle = std::f32::consts::PI / 3.0 * index as f32;
                            center + Vec2::new(radius * angle.cos(), radius * angle.sin())
                        })
                        .collect();
                    painter.add(egui::Shape::line(points, Stroke::new(0.7_f32, grid)));
                }
            }
        }
        CanvasBackground::Circuit => {
            let step = 64.0;
            let phase = (animation_time * 1.3).rem_euclid(step);
            let start_x = ((clip.left() - canvas.left() - phase) / step).floor() as i32 - 1;
            let end_x = ((clip.right() - canvas.left() - phase) / step).ceil() as i32 + 1;
            let start_y = ((clip.top() - canvas.top() - phase) / step).floor() as i32 - 1;
            let end_y = ((clip.bottom() - canvas.top() - phase) / step).ceil() as i32 + 1;
            for column in start_x..=end_x {
                for row in start_y..=end_y {
                    let origin = canvas.min
                        + Vec2::new(column as f32 * step + phase, row as f32 * step + phase);
                    let elbow = origin + Vec2::new(32.0, 32.0);
                    let end = origin + Vec2::new(62.0, 32.0);
                    painter.line_segment([origin, elbow], Stroke::new(0.8_f32, grid));
                    painter.line_segment([elbow, end], Stroke::new(0.8_f32, grid));
                    painter.circle_filled(elbow, 2.0, accent);
                }
            }
        }
        CanvasBackground::Aurora => {
            let radius = canvas.width().max(canvas.height()) * 0.35;
            let drift = animation_time * 0.22;
            painter.circle_filled(
                canvas.left_top()
                    + Vec2::new(
                        canvas.width() * (0.24 + drift.sin() * 0.04),
                        canvas.height() * (0.35 + (drift * 0.7).cos() * 0.04),
                    ),
                radius,
                with_alpha(palette.condition_node, intensity * 0.08),
            );
            painter.circle_filled(
                canvas.left_top()
                    + Vec2::new(
                        canvas.width() * (0.72 + (drift * 0.8).cos() * 0.04),
                        canvas.height() * (0.68 + (drift * 0.6).sin() * 0.04),
                    ),
                radius * 0.82,
                with_alpha(palette.move_node, intensity * 0.07),
            );
        }
        CanvasBackground::Horizon => {
            let center = canvas.left_top() + Vec2::new(canvas.width() * 0.5, 220.0);
            painter.circle_filled(center, 92.0, with_alpha(palette.warning, intensity * 0.12));
            let horizon_y = canvas.top() + 330.0;
            let phase = (animation_time * 15.0).rem_euclid(24.0);
            for index in 0..12 {
                let y = horizon_y + index as f32 * 24.0 + phase;
                if y >= clip.top() && y <= clip.bottom() {
                    painter.line_segment(
                        [Pos2::new(clip.left(), y), Pos2::new(clip.right(), y)],
                        Stroke::new(0.8_f32, grid),
                    );
                }
            }
            for index in -16..=16 {
                let bottom_x = canvas.center().x + index as f32 * 34.0;
                painter.line_segment(
                    [
                        Pos2::new(canvas.center().x, horizon_y),
                        Pos2::new(bottom_x, canvas.bottom()),
                    ],
                    Stroke::new(0.8_f32, grid),
                );
            }
        }
        CanvasBackground::Stars => {
            let step = 120.0;
            let start_x = ((clip.left() - canvas.left()) / step).floor() as i32;
            let end_x = ((clip.right() - canvas.left()) / step).ceil() as i32;
            let start_y = ((clip.top() - canvas.top()) / step).floor() as i32 - 1;
            let end_y = ((clip.bottom() - canvas.top()) / step).ceil() as i32 + 1;
            let drift = (animation_time * 10.0).rem_euclid(canvas.height() + step);
            for column in start_x..=end_x {
                for row in start_y..=end_y {
                    let seed = (column * 37 + row * 61).rem_euclid(97) as f32;
                    let y = (row as f32 * step + 20.0 + (seed * 1.7).rem_euclid(83.0) - drift)
                        .rem_euclid(canvas.height() + step);
                    let point = canvas.min + Vec2::new(column as f32 * step + 12.0 + seed, y);
                    painter.circle_filled(
                        point,
                        1.1 + (seed % 2.0),
                        with_alpha(palette.text, intensity * 0.72),
                    );
                }
            }
        }
        CanvasBackground::Matrix => {
            let step_x = 22.0;
            let step_y = 18.0;
            let stream_height = canvas.height() + step_y * 9.0;
            let start_column = ((clip.left() - canvas.left()) / step_x).floor() as i32 - 1;
            let end_column = ((clip.right() - canvas.left()) / step_x).ceil() as i32 + 1;
            let frame = (animation_time * 3.0) as i32;
            for column in start_column..=end_column {
                let seed = (column * 37).rem_euclid(97) as f32;
                let head =
                    (animation_time * 38.0 + seed / 97.0 * stream_height).rem_euclid(stream_height);
                for row in 0..8 {
                    let y = canvas.top() + (head - row as f32 * step_y).rem_euclid(stream_height);
                    let bit = (column * 31 + row * 17 + frame).rem_euclid(2);
                    let strength = intensity * (0.58 - row as f32 * 0.045).max(0.18);
                    painter.text(
                        Pos2::new(canvas.left() + column as f32 * step_x, y),
                        Align2::CENTER_CENTER,
                        if bit == 0 { "0" } else { "1" },
                        FontId::monospace(11.0),
                        with_alpha(palette.success, strength),
                    );
                }
            }
        }
        CanvasBackground::Hearts => {
            let step = 124.0;
            let start_column = ((clip.left() - canvas.left()) / step).floor() as i32 - 1;
            let end_column = ((clip.right() - canvas.left()) / step).ceil() as i32 + 1;
            let start_row = ((clip.top() - canvas.top()) / step).floor() as i32 - 1;
            let end_row = ((clip.bottom() - canvas.top()) / step).ceil() as i32 + 1;
            let drift = (animation_time * 12.0).rem_euclid(canvas.height() + step);
            for column in start_column..=end_column {
                for row in start_row..=end_row {
                    let seed = (column * 37 + row * 61).rem_euclid(97) as f32;
                    let x = canvas.left() + column as f32 * step + 24.0 + seed;
                    let y = canvas.top()
                        + (row as f32 * step + 24.0 + (seed * 1.7).rem_euclid(68.0) - drift)
                            .rem_euclid(canvas.height() + step);
                    let color = match (column + row).rem_euclid(3) {
                        0 => palette.accent,
                        1 => palette.condition_node,
                        _ => palette.move_node,
                    };
                    let heart_color = with_alpha(color, intensity * 0.3);
                    let radius = 5.0 + seed.rem_euclid(4.0);
                    paint_heart(painter, Pos2::new(x, y), radius, heart_color);
                    let sparkle = Pos2::new(x + 28.0, y - 14.0);
                    let sparkle_color = with_alpha(palette.warning, intensity * 0.5);
                    painter.line_segment(
                        [sparkle - Vec2::new(3.5, 0.0), sparkle + Vec2::new(3.5, 0.0)],
                        Stroke::new(1.0_f32, sparkle_color),
                    );
                    painter.line_segment(
                        [sparkle - Vec2::new(0.0, 3.5), sparkle + Vec2::new(0.0, 3.5)],
                        Stroke::new(1.0_f32, sparkle_color),
                    );
                }
            }
        }
        CanvasBackground::Radar => {
            let center = canvas.center();
            for index in 1..=8 {
                painter.circle_stroke(center, index as f32 * 48.0, Stroke::new(0.7_f32, grid));
            }
            painter.line_segment(
                [
                    Pos2::new(clip.left(), center.y),
                    Pos2::new(clip.right(), center.y),
                ],
                Stroke::new(0.8_f32, grid),
            );
            painter.line_segment(
                [
                    Pos2::new(center.x, clip.top()),
                    Pos2::new(center.x, clip.bottom()),
                ],
                Stroke::new(0.8_f32, grid),
            );
            let angle = (animation_time * 0.65).rem_euclid(std::f32::consts::TAU);
            let radius = canvas.width().max(canvas.height()) * 0.55;
            painter.line_segment(
                [
                    center,
                    center + Vec2::new(angle.cos() * radius, angle.sin() * radius),
                ],
                Stroke::new(1.4_f32, with_alpha(palette.success, intensity * 0.68)),
            );
        }
    }
}

fn paint_heart(painter: &egui::Painter, center: Pos2, radius: f32, color: Color32) {
    painter.circle_filled(
        center + Vec2::new(-radius * 0.48, -radius * 0.2),
        radius * 0.58,
        color,
    );
    painter.circle_filled(
        center + Vec2::new(radius * 0.48, -radius * 0.2),
        radius * 0.58,
        color,
    );
    painter.add(egui::Shape::convex_polygon(
        vec![
            center + Vec2::new(-radius, -radius * 0.05),
            center + Vec2::new(radius, -radius * 0.05),
            center + Vec2::new(0.0, radius * 1.25),
        ],
        color,
        Stroke::NONE,
    ));
}

fn drag_position(
    start: [f32; 2],
    pointer_start: Pos2,
    pointer_current: Pos2,
    snap_to_grid: bool,
) -> [f32; 2] {
    let delta = pointer_current - pointer_start;
    let max_x = CANVAS_LIMIT - NODE_SIZE.x;
    let max_y = CANVAS_LIMIT - NODE_SIZE.y;
    let mut x = (start[0] + delta.x).clamp(16.0, max_x);
    let mut y = (start[1] + delta.y).clamp(16.0, max_y);
    if snap_to_grid {
        x = ((x / 24.0).round() * 24.0).clamp(16.0, max_x);
        y = ((y / 24.0).round() * 24.0).clamp(16.0, max_y);
    }
    [x, y]
}

fn bezier_point(from: Pos2, to: Pos2, t: f32) -> Pos2 {
    let bend = ((to.x - from.x).abs() * 0.5).max(42.0);
    let c1 = from + Vec2::new(bend, 0.0);
    let c2 = to - Vec2::new(bend, 0.0);
    let t = t.clamp(0.0, 1.0);
    let inv = 1.0 - t;
    let point = from.to_vec2() * (inv * inv * inv)
        + c1.to_vec2() * (3.0 * inv * inv * t)
        + c2.to_vec2() * (3.0 * inv * t * t)
        + to.to_vec2() * (t * t * t);
    Pos2::new(point.x, point.y)
}

fn paint_connection(
    painter: &egui::Painter,
    from: Pos2,
    to: Pos2,
    color: Color32,
    width: f32,
    glow_strength: f32,
) {
    let bend = ((to.x - from.x).abs() * 0.5).max(42.0);
    let control_end = to - Vec2::new(bend, 0.0);
    let points: Vec<Pos2> = (0..=24)
        .map(|step| bezier_point(from, to, step as f32 / 24.0))
        .collect();
    painter.add(egui::Shape::line(
        points.clone(),
        Stroke::new(width + 4.0, with_alpha(color, glow_strength * 0.18)),
    ));
    painter.add(egui::Shape::line(points, Stroke::new(width, color)));
    let direction = Vec2::new(to.x - control_end.x, to.y - control_end.y);
    let direction = direction / direction.length().max(f32::EPSILON);
    let base = to - direction * 10.0;
    let side = Vec2::new(-direction.y, direction.x) * 4.5;
    painter.line_segment([to, base + side], Stroke::new(width, color));
    painter.line_segment([to, base - side], Stroke::new(width, color));
}

fn nearest_edge(
    program: &FlowProgram,
    canvas_origin: Pos2,
    pointer: Pos2,
) -> Option<(String, FlowPort, f32)> {
    program
        .edges
        .iter()
        .filter_map(|edge| {
            let source = program
                .nodes
                .iter()
                .find(|node| node.id == edge.from_node_id)?;
            let target = program
                .nodes
                .iter()
                .find(|node| node.id == edge.to_node_id)?;
            let from = output_position(source, edge.port, canvas_origin);
            let to = input_position(target, canvas_origin);
            let mut previous = from;
            let mut distance = f32::MAX;
            for step in 1..=24 {
                let current = bezier_point(from, to, step as f32 / 24.0);
                distance = distance.min(distance_to_segment(pointer, previous, current));
                previous = current;
            }
            Some((edge.from_node_id.clone(), edge.port, distance))
        })
        .filter(|(_, _, distance)| *distance <= 12.0)
        .min_by(|left, right| left.2.total_cmp(&right.2))
}

fn distance_to_segment(point: Pos2, start: Pos2, end: Pos2) -> f32 {
    let segment = end - start;
    let length_squared = segment.length_sq();
    if length_squared <= f32::EPSILON {
        return point.distance(start);
    }
    let projection = ((point - start).dot(segment) / length_squared).clamp(0.0, 1.0);
    point.distance(start + segment * projection)
}

fn edit_operand(
    ui: &mut egui::Ui,
    operand: &mut Operand,
    label_key: &str,
    tr: &impl Fn(&str) -> String,
) -> bool {
    let mut changed = false;
    ui.push_id(label_key, |ui| {
        let is_variable = matches!(operand, Operand::Variable(_));
        egui::ComboBox::from_id_source("operand-source")
            .selected_text(if is_variable {
                tr("studio_variable")
            } else {
                tr("studio_literal")
            })
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(!is_variable, tr("studio_literal"))
                    .clicked()
                    && is_variable
                {
                    *operand = Operand::Literal(FlowValue::Number(0.0));
                    changed = true;
                }
                if ui
                    .selectable_label(is_variable, tr("studio_variable"))
                    .clicked()
                    && !is_variable
                {
                    *operand = Operand::Variable("value".into());
                    changed = true;
                }
            });
        match operand {
            Operand::Literal(value) => changed |= edit_value(ui, value, tr),
            Operand::Variable(name) => changed |= ui.text_edit_singleline(name).changed(),
        }
    });
    changed
}

fn edit_value(ui: &mut egui::Ui, value: &mut FlowValue, tr: &impl Fn(&str) -> String) -> bool {
    let mut changed = false;
    let value_index = match value {
        FlowValue::Number(_) => 0,
        FlowValue::Text(_) => 1,
        FlowValue::Boolean(_) => 2,
    };
    egui::ComboBox::from_id_source("flow-value-type")
        .selected_text(value_type_label(value_index, tr))
        .show_ui(ui, |ui| {
            for index in 0..3 {
                if ui
                    .selectable_label(value_index == index, value_type_label(index, tr))
                    .clicked()
                    && value_index != index
                {
                    *value = default_flow_value(index);
                    changed = true;
                }
            }
        });
    changed |= match value {
        FlowValue::Number(number) => ui.add(egui::DragValue::new(number).speed(0.1)).changed(),
        FlowValue::Text(text) => ui.text_edit_singleline(text).changed(),
        FlowValue::Boolean(boolean) => ui.checkbox(boolean, tr("studio_true")).changed(),
    };
    changed
}

fn variable_operation_index(operation: &VariableOperation) -> usize {
    match operation {
        VariableOperation::Set { .. } => 0,
        VariableOperation::Add { .. } => 1,
        VariableOperation::Toggle => 2,
        VariableOperation::Append { .. } => 3,
    }
}

fn default_variable_operation(index: usize) -> VariableOperation {
    match index {
        1 => VariableOperation::Add { amount: 1.0 },
        2 => VariableOperation::Toggle,
        3 => VariableOperation::Append {
            text: String::new(),
        },
        _ => VariableOperation::Set {
            value: FlowValue::Number(0.0),
        },
    }
}

fn variable_operation_label(index: usize, tr: &impl Fn(&str) -> String) -> String {
    tr(match index {
        1 => "studio_op_add",
        2 => "studio_op_toggle",
        3 => "studio_op_append",
        _ => "studio_op_set",
    })
}

fn value_type_label(index: usize, tr: &impl Fn(&str) -> String) -> String {
    tr(match index {
        1 => "studio_type_text",
        2 => "studio_type_boolean",
        _ => "studio_type_number",
    })
}

fn default_flow_value(index: usize) -> FlowValue {
    match index {
        1 => FlowValue::Text(String::new()),
        2 => FlowValue::Boolean(false),
        _ => FlowValue::Number(0.0),
    }
}

fn truncate(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let head: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{head}…")
    } else {
        head
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_sequence_becomes_a_valid_editable_graph() {
        let sequence = vec![
            SequenceStep::default(),
            SequenceStep {
                action: Action::KeyPress {
                    key: "Space".into(),
                    hold_ms: 30,
                },
                ..Default::default()
            },
        ];
        let studio = FlowStudio::from_sequence(&sequence);
        assert_eq!(studio.program.nodes.len(), 3);
        assert!(studio.program.validate().is_ok());
    }

    #[test]
    fn background_animation_clock_pauses_and_resumes_without_a_time_jump() {
        let mut studio = FlowStudio::from_sequence(&[SequenceStep::default()]);
        studio.advance_animation_clock(1.0, true);
        studio.advance_animation_clock(1.08, true);
        let before_pause = studio.animation_time;

        studio.advance_animation_clock(10.0, false);
        studio.advance_animation_clock(20.0, false);
        assert_eq!(studio.animation_time, before_pause);

        studio.advance_animation_clock(20.04, true);
        assert!((studio.animation_time - before_pause - 0.04).abs() < 0.001);
    }

    #[test]
    fn loaded_example_is_valid_and_uses_only_safe_pointer_moves_and_waits() {
        let translate = |key: &str| key.to_owned();
        let mut studio = FlowStudio::from_sequence(&[]);
        studio.load_example(&translate);

        assert!(studio.program.validate().is_ok());
        assert_eq!(studio.program.nodes.len(), 4);
        assert_eq!(studio.program.edges.len(), 3);
        assert_eq!(
            studio.program.nodes.last().unwrap().label,
            "studio_end_node"
        );
        assert!(studio.program.nodes.iter().all(|node| match &node.kind {
            FlowNodeKind::Action { action, .. } =>
                matches!(action, Action::MouseMoveSmooth { .. } | Action::Wait { .. }),
            FlowNodeKind::End => true,
            _ => false,
        }));
    }

    #[test]
    fn slow_pointer_deltas_accumulate_before_grid_snapping() {
        let pointer_start = Pos2::new(100.0, 100.0);
        let start = [48.0, 48.0];
        let mut snapped = start;
        for step in 1..=48 {
            let offset = step as f32 * 0.5;
            snapped = drag_position(
                start,
                pointer_start,
                pointer_start + Vec2::splat(offset),
                true,
            );
        }
        assert_eq!(snapped, [72.0, 72.0]);

        let unsnapped = drag_position(
            start,
            pointer_start,
            pointer_start + Vec2::splat(0.5),
            false,
        );
        assert_eq!(unsnapped, [48.5, 48.5]);
    }

    #[test]
    fn action_insertions_extend_the_visual_end_of_a_linear_flow() {
        let mut studio = FlowStudio::from_sequence(&[SequenceStep::default()]);
        studio.add_action(Action::Wait { duration_ms: 250 });
        studio.add_action(Action::TypeText {
            text: "done".into(),
        });
        assert_eq!(studio.program.nodes.len(), 4);
        assert!(studio.program.validate().is_ok());
        assert_eq!(studio.program.edges.len(), 3);
    }

    #[test]
    fn flow_control_palette_nodes_create_valid_graphs() {
        let sequence = [SequenceStep::default()];
        let mut condition = FlowStudio::from_sequence(&sequence);
        condition.add_condition();
        assert!(condition.program.validate().is_ok());

        let mut repeat = FlowStudio::from_sequence(&sequence);
        repeat.add_repeat();
        assert!(repeat.program.validate().is_ok());

        let mut variable = FlowStudio::from_sequence(&sequence);
        variable.add_variable();
        assert!(variable.program.validate().is_ok());
    }

    #[test]
    fn an_incomplete_saved_draft_is_restored_but_not_executable() {
        let mut studio = FlowStudio::from_sequence(&[SequenceStep::default()]);
        studio.program.edges.clear();
        let encoded = studio.draft_json().unwrap();
        let (restored, warning) = FlowStudio::new(Some(&encoded), &[]);
        assert_eq!(restored.program, studio.program);
        assert!(warning.is_none());
        assert!(restored.program.validate().is_err());
    }

    #[test]
    fn all_256_legacy_steps_fit_alongside_the_terminal_node() {
        let sequence = vec![SequenceStep::default(); 256];
        let studio = FlowStudio::from_sequence(&sequence);
        assert_eq!(studio.program.nodes.len(), 257);
        assert!(studio.program.validate().is_ok());
    }

    #[test]
    fn disabled_legacy_steps_are_never_migrated_as_clicks() {
        let disabled = [SequenceStep {
            enabled: false,
            ..Default::default()
        }];
        let studio = FlowStudio::from_sequence(&disabled);
        assert!(matches!(
            studio.program.nodes[0].kind,
            FlowNodeKind::Action {
                action: Action::Wait { duration_ms: 0 },
                ..
            }
        ));
        assert!(studio.program.validate().is_ok());
    }

    #[test]
    fn palette_allows_manual_wiring_after_a_branch() {
        let mut studio = FlowStudio::from_sequence(&[SequenceStep::default()]);
        studio.add_condition();
        let node_count = studio.program.nodes.len();
        studio.add_action(Action::Wait { duration_ms: 10 });
        studio.add_variable();
        assert_eq!(studio.program.nodes.len(), node_count + 2);
        assert!(studio.program.validate().is_err());
        let selected = studio
            .program
            .nodes
            .iter()
            .find(|node| Some(node.id.as_str()) == studio.selected_node_id.as_deref())
            .unwrap();
        assert!(matches!(selected.kind, FlowNodeKind::SetVariable { .. }));
    }

    #[test]
    fn workflow_export_round_trips_through_native_json() {
        let studio = FlowStudio::from_sequence(&[SequenceStep::default()]);
        let path = std::env::temp_dir().join(format!(
            "keyboard-clicker-flow-export-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);

        write_workflow_to(&path, &studio.program).unwrap();
        let contents = std::fs::read_to_string(&path).unwrap();
        let imported = FlowProgram::from_json(&contents).unwrap();
        let _ = std::fs::remove_file(path);

        assert_eq!(imported, studio.program);
    }

    #[test]
    fn workflow_export_reports_invalid_graph_without_creating_a_file() {
        let mut studio = FlowStudio::from_sequence(&[SequenceStep::default()]);
        studio.program.edges.clear();
        let path = std::env::temp_dir().join(format!(
            "keyboard-clicker-invalid-export-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);

        assert!(write_workflow_to(&path, &studio.program).is_err());
        assert!(!path.exists());
    }

    #[test]
    fn delete_selected_node_removes_links_and_replaces_the_entry() {
        let mut studio = FlowStudio::from_sequence(&[SequenceStep::default()]);

        assert!(studio.delete_selected_node());
        assert_eq!(studio.program.nodes.len(), 1);
        assert_eq!(studio.program.entry_node_id, "flow-end");
        assert!(studio.program.edges.is_empty());
        assert!(studio.selected_node_id.is_none());
        assert!(studio.program.validate().is_ok());
    }

    #[test]
    fn deleting_a_selected_connection_keeps_its_nodes_intact() {
        let mut studio = FlowStudio::from_sequence(&[SequenceStep::default()]);
        let edge = &studio.program.edges[0];
        studio.selected_edge = Some((edge.from_node_id.clone(), edge.port));
        let node_count = studio.program.nodes.len();

        assert!(studio.delete_selected_edge());
        assert!(studio.program.edges.is_empty());
        assert_eq!(studio.program.nodes.len(), node_count);
        assert!(studio.selected_edge.is_none());
    }

    #[test]
    fn edge_hit_testing_uses_the_same_bezier_curve_as_canvas_rendering() {
        let studio = FlowStudio::from_sequence(&[SequenceStep::default()]);
        let origin = Pos2::new(40.0, 35.0);
        let edge = &studio.program.edges[0];
        let source = studio
            .program
            .nodes
            .iter()
            .find(|node| node.id == edge.from_node_id)
            .unwrap();
        let target = studio
            .program
            .nodes
            .iter()
            .find(|node| node.id == edge.to_node_id)
            .unwrap();
        let from = output_position(source, edge.port, origin);
        let to = input_position(target, origin);
        let midpoint = bezier_point(from, to, 0.5);

        let hit = nearest_edge(&studio.program, origin, midpoint).unwrap();
        assert_eq!(hit.0, edge.from_node_id);
        assert_eq!(hit.1, edge.port);
        assert!(hit.2 <= 1.0);
        assert!(nearest_edge(&studio.program, origin, Pos2::new(9_000.0, 9_000.0)).is_none());
    }

    #[test]
    fn instant_and_smooth_mouse_moves_remain_distinct_in_the_inspector() {
        assert_eq!(
            action_kind_index(&default_action(1)),
            1,
            "index 1 is the immediate pointer move"
        );
        assert!(matches!(default_action(1), Action::MouseMove { .. }));
        assert!(matches!(default_action(2), Action::MouseMoveSmooth { .. }));
        assert_eq!(action_kind_index(&default_action(2)), 2);
    }

    #[test]
    fn numbered_palette_entries_match_their_inserted_actions() {
        let translate = |key: &str| key.to_owned();
        let palette = palette_actions(&translate);
        assert_eq!(palette.len(), 5);
        assert!(matches!(&palette[0].1, Action::MouseClick { .. }));
        assert!(matches!(&palette[1].1, Action::MouseMoveSmooth { .. }));
        assert_eq!(palette[1].0, "add_move_smooth");
        assert!(matches!(&palette[2].1, Action::KeyPress { .. }));
        assert!(matches!(&palette[3].1, Action::TypeText { .. }));
        assert!(matches!(&palette[4].1, Action::Wait { .. }));
    }
}
