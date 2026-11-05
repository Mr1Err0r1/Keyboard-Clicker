use crate::{
    flow::{
        import_autoflow_json, Comparison, FlowEdge, FlowNode, FlowNodeKind, FlowPort, FlowProgram,
        FlowRunOptions, FlowValue, Operand, VariableOperation, FLOW_SCHEMA_VERSION, MAX_FLOW_EDGES,
        MAX_FLOW_NODES, MAX_FLOW_VARIABLES,
    },
    model::{Action, ClickType, MouseButton, SequenceStep},
};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, RichText, Stroke, Vec2};
use std::collections::HashSet;

const NODE_SIZE: Vec2 = Vec2::new(220.0, 106.0);
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
    connecting: Option<(String, FlowPort)>,
    message: Option<(String, bool)>,
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
                                connecting: None,
                                message: warning.map(|error| (error, true)),
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
            connecting: None,
            message: None,
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
        let contents = self.program.to_json()?;
        std::fs::write(&path, contents).map_err(|error| error.to_string())?;
        self.message = Some((format!("Exported {}", path.display()), false));
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        tr: &impl Fn(&str) -> String,
        accent: Color32,
        locked: bool,
        completed_cycles: u64,
        interval_ms: &mut u64,
        repeat_count: &mut u64,
    ) -> (StudioCommand, bool) {
        let mut command = StudioCommand::None;
        let mut changed = false;
        egui::Frame::none()
            .inner_margin(egui::Margin::symmetric(18.0, 14.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("AUTOMATION STUDIO")
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
                        RichText::new(format!("{} nodes", self.program.nodes.len()))
                            .color(ui.visuals().weak_text_color()),
                    );
                    ui.label(
                        RichText::new(format!("{} links", self.program.edges.len()))
                            .color(ui.visuals().weak_text_color()),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if locked {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(format!("■  {}", tr("stop")))
                                            .color(Color32::WHITE),
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
                                    RichText::new(format!("▶  {}", tr("start")))
                                        .color(Color32::WHITE),
                                )
                                .fill(accent),
                            )
                            .clicked()
                        {
                            command = StudioCommand::Start;
                        }
                        if ui.button(format!("↑  {}", tr("studio_export"))).clicked() {
                            command = StudioCommand::Export;
                        }
                        if ui.button(format!("↓  {}", tr("studio_import"))).clicked() {
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
                        .add(egui::DragValue::new(repeat_count).clamp_range(0..=1_000_000_000_u64))
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
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                let available = ui.available_size();
                let left_width = 184.0;
                let right_width = 254.0;
                let center_width = (available.x - left_width - right_width - 28.0).max(300.0);
                let height = available.y.max(420.0);
                ui.horizontal_top(|ui| {
                    ui.allocate_ui(Vec2::new(left_width, height), |ui| {
                        ui.label(RichText::new(tr("studio_palette")).strong());
                        ui.label(
                            RichText::new(tr("studio_actions"))
                                .small()
                                .color(accent)
                                .strong(),
                        );
                        ui.add_space(5.0);
                        for (label, action) in palette_actions(tr) {
                            if ui
                                .add_enabled(
                                    self.program.nodes.len() < MAX_FLOW_NODES,
                                    egui::Button::new(label)
                                        .min_size(Vec2::new(ui.available_width(), 34.0)),
                                )
                                .clicked()
                            {
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
                        if ui
                            .add_enabled(
                                self.program.nodes.len() < MAX_FLOW_NODES,
                                egui::Button::new(tr("studio_condition"))
                                    .min_size(Vec2::new(ui.available_width(), 32.0)),
                            )
                            .clicked()
                        {
                            self.add_condition();
                            changed = true;
                        }
                        if ui
                            .add_enabled(
                                self.program.nodes.len() <= MAX_FLOW_NODES - 2,
                                egui::Button::new(tr("studio_repeat"))
                                    .min_size(Vec2::new(ui.available_width(), 32.0)),
                            )
                            .clicked()
                        {
                            self.add_repeat();
                            changed = true;
                        }
                        if ui
                            .add_enabled(
                                self.program.nodes.len() < MAX_FLOW_NODES,
                                egui::Button::new(tr("studio_set_variable"))
                                    .min_size(Vec2::new(ui.available_width(), 32.0)),
                            )
                            .clicked()
                        {
                            self.add_variable();
                            changed = true;
                        }
                        ui.add_space(8.0);
                        ui.label(RichText::new(tr("studio_palette_help")).small().weak());
                        if self.connecting.is_some()
                            && ui.button(tr("studio_cancel_connection")).clicked()
                        {
                            self.connecting = None;
                        }
                        ui.add_space(12.0);
                        ui.label(
                            RichText::new(format!("0 = {}", tr("studio_until_stopped")))
                                .small()
                                .weak(),
                        );
                        ui.label(RichText::new(format!("F7  {}", tr("stop"))).small().weak());
                    });
                    ui.separator();
                    ui.allocate_ui(Vec2::new(center_width, height), |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(tr("studio_canvas")).strong());
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if self.connecting.is_some() {
                                        ui.colored_label(accent, tr("studio_connecting"));
                                    } else {
                                        ui.weak(tr("studio_canvas_hint"));
                                    }
                                },
                            );
                        });
                        ui.add_space(6.0);
                        changed |= self.draw_canvas(ui, tr, accent, locked);
                        if let Err(error) = self.program.validate() {
                            ui.colored_label(
                                Color32::LIGHT_RED,
                                format!("{}: {error}", tr("studio_validation_error")),
                            );
                        }
                    });
                    ui.separator();
                    ui.allocate_ui(Vec2::new(right_width, height), |ui| {
                        ui.label(RichText::new(tr("studio_inspector")).strong());
                        ui.add_space(8.0);
                        changed |= self.draw_inspector(ui, tr, accent, locked);
                    });
                });
            });
        (command, changed)
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
        let (mut x, mut y) = if max_x + 270.0 < CANVAS_LIMIT - NODE_SIZE.x {
            (max_x + 270.0, max_y)
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
                    end.position = Some([(x + 270.0).min(CANVAS_LIMIT - NODE_SIZE.x), y]);
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
                end.position = Some([(x + 270.0).min(CANVAS_LIMIT - NODE_SIZE.x), y]);
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
            position: Some([x + 270.0, y + 150.0]),
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
                end.position = Some([(x + 540.0).min(CANVAS_LIMIT - NODE_SIZE.x), y]);
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
                    (position[0] + 270.0).min(CANVAS_LIMIT - NODE_SIZE.x),
                    position[1],
                ]);
            }
        }
        self.selected_node_id = Some(id);
    }

    fn draw_canvas(
        &mut self,
        ui: &mut egui::Ui,
        tr: &impl Fn(&str) -> String,
        accent: Color32,
        locked: bool,
    ) -> bool {
        let mut changed = false;
        let mut canvas_width = 1100.0_f32;
        let mut canvas_height = 700.0_f32;
        for (index, node) in self.program.nodes.iter_mut().enumerate() {
            if node.position.is_none() {
                node.position = Some([
                    52.0 + (index % 4) as f32 * 282.0,
                    90.0 + (index / 4) as f32 * 170.0,
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
                let (canvas, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                let painter = ui.painter_at(canvas);
                let background = Color32::from_rgb(13, 18, 29);
                painter.rect_filled(canvas, 8.0, background);
                painter.rect_stroke(
                    canvas,
                    8.0,
                    Stroke::new(1.0_f32, Color32::from_rgb(35, 47, 65)),
                );
                let clip = painter.clip_rect().intersect(canvas);
                let start_x = ((clip.left() - canvas.left()).max(0.0) / 24.0).floor() as i32;
                let end_x = ((clip.right() - canvas.left()).max(0.0) / 24.0).ceil() as i32;
                let start_y = ((clip.top() - canvas.top()).max(0.0) / 24.0).floor() as i32;
                let end_y = ((clip.bottom() - canvas.top()).max(0.0) / 24.0).ceil() as i32;
                for x in start_x..=end_x {
                    for y in start_y..=end_y {
                        painter.circle_filled(
                            canvas.min + Vec2::new(x as f32 * 24.0, y as f32 * 24.0),
                            1.0,
                            Color32::from_rgb(43, 56, 77),
                        );
                    }
                }
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
                        paint_connection(&painter, from, to, accent, 2.2);
                        let label = match edge.port {
                            FlowPort::True => Some("T"),
                            FlowPort::False => Some("F"),
                            FlowPort::Body => Some("loop"),
                            FlowPort::Done => Some("done"),
                            FlowPort::Next => None,
                        };
                        if let Some(label) = label {
                            painter.text(
                                from.lerp(to, 0.5) + Vec2::new(0.0, -10.0),
                                Align2::CENTER_CENTER,
                                label,
                                FontId::proportional(10.0),
                                accent,
                            );
                        }
                    }
                }
                let mut new_connection = None;
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
                    if response.clicked() {
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
                    if response.dragged() && !locked && self.connecting.is_none() {
                        let delta = ui.input(|input| input.pointer.delta());
                        let x = (position[0] + delta.x).clamp(16.0, CANVAS_LIMIT - NODE_SIZE.x);
                        let y = (position[1] + delta.y).clamp(16.0, CANVAS_LIMIT - NODE_SIZE.y);
                        self.program.nodes[index].position = Some([x, y]);
                        changed = true;
                    }
                    let kind = self.program.nodes[index].kind.clone();
                    let fill = if is_selected {
                        Color32::from_rgb(29, 45, 65)
                    } else {
                        Color32::from_rgb(24, 32, 46)
                    };
                    let category = node_color(&kind, accent);
                    painter.rect_filled(rect, 9.0, fill);
                    painter.rect_stroke(
                        rect,
                        9.0,
                        Stroke::new(
                            if is_selected { 2.0_f32 } else { 1.0_f32 },
                            if is_selected {
                                accent
                            } else {
                                Color32::from_rgb(48, 63, 84)
                            },
                        ),
                    );
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            rect.left_top(),
                            Pos2::new(rect.left() + 5.0, rect.bottom()),
                        ),
                        egui::Rounding::same(8.0),
                        category,
                    );
                    let (title, detail) = node_text(&self.program.nodes[index], tr);
                    painter.text(
                        rect.left_top() + Vec2::new(16.0, 16.0),
                        Align2::LEFT_TOP,
                        title,
                        FontId::proportional(14.0),
                        category,
                    );
                    painter.text(
                        rect.left_top() + Vec2::new(16.0, 43.0),
                        Align2::LEFT_TOP,
                        detail,
                        FontId::proportional(11.5),
                        ui.visuals().text_color(),
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
                            painter.circle_filled(point, 5.5, category);
                            painter.circle_stroke(point, 5.5, Stroke::new(1.2_f32, background));
                            let label = match port {
                                FlowPort::Next => "→",
                                FlowPort::True => "✓",
                                FlowPort::False => "×",
                                FlowPort::Body => "↻",
                                FlowPort::Done => "✓",
                            };
                            painter.text(
                                point + Vec2::new(0.0, 16.0 + port_index as f32 * 0.0),
                                Align2::CENTER_TOP,
                                label,
                                FontId::proportional(10.0),
                                ui.visuals().weak_text_color(),
                            );
                            if port_response.clicked() && !locked {
                                self.selected_node_id = Some(id.clone());
                                new_connection = Some((id.clone(), port));
                            }
                        }
                    }
                    let input = input_position(&self.program.nodes[index], canvas.min);
                    painter.circle_filled(input, 4.0, Color32::from_rgb(112, 132, 156));
                    response.on_hover_text(if self.connecting.is_some() {
                        tr("studio_click_target")
                    } else {
                        tr("studio_drag_node")
                    });
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
                            paint_connection(&painter, from, pointer, accent, 1.4);
                        }
                    }
                }
            });
        if let Some((text, _)) = self.message.as_ref() {
            ui.label(RichText::new(text).small());
        }
        changed
    }

    fn draw_inspector(
        &mut self,
        ui: &mut egui::Ui,
        tr: &impl Fn(&str) -> String,
        accent: Color32,
        locked: bool,
    ) -> bool {
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
                            for (i, key) in
                                ["add_click", "add_move", "add_key", "add_text", "add_wait"]
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
                            egui::ComboBox::from_id_source(("studio-button", id.as_str()))
                                .selected_text(format!("{:?}", button))
                                .show_ui(ui, |ui| {
                                    for value in
                                        [MouseButton::Left, MouseButton::Right, MouseButton::Middle]
                                    {
                                        if ui
                                            .selectable_value(button, value, format!("{:?}", value))
                                            .changed()
                                        {
                                            changed = true;
                                        }
                                    }
                                });
                            egui::ComboBox::from_id_source(("studio-click", id.as_str()))
                                .selected_text(format!("{:?}", click_type))
                                .show_ui(ui, |ui| {
                                    for value in
                                        [ClickType::Single, ClickType::Double, ClickType::Hold]
                                    {
                                        if ui
                                            .selectable_value(
                                                click_type,
                                                value,
                                                format!("{:?}", value),
                                            )
                                            .changed()
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
                        RichText::new(format!("✕  {}", tr("studio_delete_node")))
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
                    "{} → {}",
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

fn palette_actions(tr: &impl Fn(&str) -> String) -> Vec<(String, Action)> {
    vec![
        (format!("●  {}", tr("add_click")), Action::default()),
        (
            format!("↗  {}", tr("add_move")),
            Action::MouseMoveSmooth {
                position: [600, 400],
                duration_ms: 350,
            },
        ),
        (
            format!("⌨  {}", tr("add_key")),
            Action::KeyPress {
                key: "Space".into(),
                hold_ms: 35,
            },
        ),
        (
            format!("▤  {}", tr("add_text")),
            Action::TypeText {
                text: "Text".into(),
            },
        ),
        (
            format!("◷  {}", tr("add_wait")),
            Action::Wait { duration_ms: 500 },
        ),
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
    [72.0 + column as f32 * 282.0, 150.0 + row as f32 * 160.0]
}

fn default_action(index: usize) -> Action {
    match index {
        1 => Action::MouseMoveSmooth {
            position: [600, 400],
            duration_ms: 350,
        },
        2 => Action::KeyPress {
            key: "Space".into(),
            hold_ms: 35,
        },
        3 => Action::TypeText {
            text: "Text".into(),
        },
        4 => Action::Wait { duration_ms: 500 },
        _ => Action::default(),
    }
}

fn action_kind_index(action: &Action) -> usize {
    match action {
        Action::MouseClick { .. } => 0,
        Action::MouseMove { .. } | Action::MouseMoveSmooth { .. } => 1,
        Action::KeyPress { .. } => 2,
        Action::TypeText { .. } => 3,
        Action::Wait { .. } => 4,
    }
}

fn action_kind_label(action: &Action, tr: &impl Fn(&str) -> String) -> String {
    match action {
        Action::MouseClick { .. } => tr("add_click"),
        Action::MouseMove { .. } | Action::MouseMoveSmooth { .. } => tr("add_move"),
        Action::KeyPress { .. } => tr("add_key"),
        Action::TypeText { .. } => tr("add_text"),
        Action::Wait { .. } => tr("add_wait"),
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
                    "{:?} {:?} click · {}",
                    button,
                    click_type,
                    position
                        .map(|p| format!("{}, {}", p[0], p[1]))
                        .unwrap_or_else(|| "current cursor".into())
                ),
                Action::MouseMove { position } => {
                    format!("Move to {}, {}", position[0], position[1])
                }
                Action::MouseMoveSmooth {
                    position,
                    duration_ms,
                } => format!(
                    "Glide to {}, {} · {duration_ms} ms",
                    position[0], position[1]
                ),
                Action::KeyPress { key, .. } => format!("Press {key}"),
                Action::TypeText { text } => format!("Type {}", truncate(text, 25)),
                Action::Wait { duration_ms } => format!("Wait {duration_ms} ms"),
            };
            let detail = if *delay_after_ms > 0 {
                format!("{detail} · +{delay_after_ms} ms")
            } else {
                detail
            };
            (node.label.clone(), detail)
        }
        FlowNodeKind::SetVariable { name, .. } => {
            (node.label.clone(), format!("Set variable · {name}"))
        }
        FlowNodeKind::Condition { comparison, .. } => {
            (node.label.clone(), format!("Condition · {:?}", comparison))
        }
        FlowNodeKind::Repeat { iterations, .. } => {
            (node.label.clone(), format!("Repeat {iterations} times"))
        }
        FlowNodeKind::LoopEnd { .. } => (node.label.clone(), "Return to repeat node".into()),
        FlowNodeKind::End => (tr("studio_end_node"), "Workflow complete".into()),
    }
}

fn node_color(kind: &FlowNodeKind, accent: Color32) -> Color32 {
    match kind {
        FlowNodeKind::Action {
            action: Action::MouseClick { .. },
            ..
        } => accent,
        FlowNodeKind::Action {
            action: Action::MouseMove { .. } | Action::MouseMoveSmooth { .. },
            ..
        } => Color32::from_rgb(52, 196, 217),
        FlowNodeKind::Action {
            action: Action::KeyPress { .. } | Action::TypeText { .. },
            ..
        } => Color32::from_rgb(70, 203, 145),
        FlowNodeKind::Action {
            action: Action::Wait { .. },
            ..
        } => Color32::from_rgb(239, 177, 74),
        FlowNodeKind::Condition { .. } => Color32::from_rgb(173, 119, 240),
        FlowNodeKind::Repeat { .. } | FlowNodeKind::LoopEnd { .. } => {
            Color32::from_rgb(240, 128, 82)
        }
        FlowNodeKind::SetVariable { .. } => Color32::from_rgb(76, 160, 226),
        FlowNodeKind::End => Color32::from_rgb(124, 145, 170),
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

fn paint_connection(painter: &egui::Painter, from: Pos2, to: Pos2, color: Color32, width: f32) {
    let bend = ((to.x - from.x).abs() * 0.5).max(42.0);
    let c1 = from + Vec2::new(bend, 0.0);
    let c2 = to - Vec2::new(bend, 0.0);
    let points: Vec<Pos2> = (0..=16)
        .map(|step| {
            let t = step as f32 / 16.0;
            let inv = 1.0 - t;
            let point = from.to_vec2() * (inv * inv * inv)
                + c1.to_vec2() * (3.0 * inv * inv * t)
                + c2.to_vec2() * (3.0 * inv * t * t)
                + to.to_vec2() * (t * t * t);
            Pos2::new(point.x, point.y)
        })
        .collect();
    painter.add(egui::Shape::line(
        points.clone(),
        Stroke::new(width + 3.0, color.linear_multiply(0.10)),
    ));
    painter.add(egui::Shape::line(points, Stroke::new(width, color)));
    let direction = (to - c2).normalized();
    let base = to - direction * 10.0;
    let side = Vec2::new(-direction.y, direction.x) * 4.5;
    painter.line_segment([to, base + side], Stroke::new(width, color));
    painter.line_segment([to, base - side], Stroke::new(width, color));
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
}
