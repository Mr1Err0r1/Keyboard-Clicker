//! Versioned, validated automation-flow format and deterministic interpreter.
//!
//! A flow is intentionally data-only: it cannot invoke shell commands, load code,
//! or evaluate arbitrary scripts. OS input is supplied by a `FlowHost` provided by
//! the application engine. The interpreter applies graph and resource limits before
//! doing any work.

use crate::model::{Action, AppSettings, SequenceStep};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::time::Instant;

pub const FLOW_SCHEMA_VERSION: u32 = 1;
/// Allows 256 executable steps plus one terminal node in the visual editor.
pub const MAX_FLOW_NODES: usize = 257;
pub const MAX_FLOW_EDGES: usize = 512;
pub const MAX_FLOW_VARIABLES: usize = 128;
pub const MAX_FLOW_NAME_BYTES: usize = 128;
pub const MAX_FLOW_TEXT_BYTES: usize = 16_384;
pub const MAX_LOOP_ITERATIONS: u32 = 100_000;
pub const MAX_LOOP_DEPTH: usize = 16;
pub const MAX_INSTRUCTIONS_PER_CYCLE: usize = 100_000;
pub const MAX_INTERVAL_MS: u64 = 3_600_000;
pub const MAX_REPEAT_COUNT: u64 = 1_000_000_000;
pub const MAX_FLOW_DURATION_MS: u64 = 86_400_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowProgram {
    pub schema_version: u32,
    pub name: String,
    pub entry_node_id: String,
    #[serde(default)]
    pub variables: BTreeMap<String, FlowValue>,
    pub nodes: Vec<FlowNode>,
    #[serde(default)]
    pub edges: Vec<FlowEdge>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowNode {
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<[f32; 2]>,
    pub kind: FlowNodeKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FlowNodeKind {
    Action {
        action: Action,
        #[serde(default)]
        delay_after_ms: u64,
    },
    SetVariable {
        name: String,
        operation: VariableOperation,
    },
    Condition {
        left: Operand,
        comparison: Comparison,
        right: Operand,
    },
    Repeat {
        iterations: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        counter_variable: Option<String>,
    },
    LoopEnd {
        repeat_node_id: String,
    },
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowPort {
    Next,
    True,
    False,
    Body,
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowEdge {
    pub from_node_id: String,
    pub port: FlowPort,
    pub to_node_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum FlowValue {
    Number(f64),
    Text(String),
    Boolean(bool),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "source", content = "value", rename_all = "snake_case")]
pub enum Operand {
    Literal(FlowValue),
    Variable(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Comparison {
    Equal,
    NotEqual,
    LessThan,
    LessOrEqual,
    GreaterThan,
    GreaterOrEqual,
    Contains,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum VariableOperation {
    Set { value: FlowValue },
    Add { amount: f64 },
    Toggle,
    Append { text: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlowRunOptions {
    /// Zero repeats the whole graph until the host is cancelled.
    pub repeat_count: u64,
    pub interval_ms: u64,
}

impl Default for FlowRunOptions {
    fn default() -> Self {
        Self {
            repeat_count: 1,
            interval_ms: 100,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlowRunSummary {
    pub completed_cycles: u64,
    pub cancelled: bool,
}

/// Runtime bridge. Implementations must make waits interruptible and avoid
/// leaving held inputs pressed when an action is cancelled or returns an error.
pub trait FlowHost {
    fn is_cancelled(&self) -> bool;
    fn execute_action(&mut self, action: &Action) -> Result<(), String>;
    fn wait(&mut self, duration_ms: u64) -> Result<(), String>;
}

impl FlowProgram {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != FLOW_SCHEMA_VERSION {
            return Err(format!(
                "Unsupported flow schema version {} (supported: {}).",
                self.schema_version, FLOW_SCHEMA_VERSION
            ));
        }
        if self.name.trim().is_empty() || self.name.len() > MAX_FLOW_NAME_BYTES {
            return Err(format!(
                "Flow name must contain 1–{MAX_FLOW_NAME_BYTES} bytes."
            ));
        }
        if self.nodes.is_empty() || self.nodes.len() > MAX_FLOW_NODES {
            return Err(format!("A flow must contain 1–{MAX_FLOW_NODES} nodes."));
        }
        if self.edges.len() > MAX_FLOW_EDGES {
            return Err(format!(
                "A flow may contain at most {MAX_FLOW_EDGES} connections."
            ));
        }
        if self.variables.len() > MAX_FLOW_VARIABLES {
            return Err(format!(
                "A flow may define at most {MAX_FLOW_VARIABLES} variables."
            ));
        }
        for (name, value) in &self.variables {
            validate_variable_name(name)?;
            validate_value(value)?;
        }

        let mut nodes = HashMap::with_capacity(self.nodes.len());
        for node in &self.nodes {
            validate_id(&node.id)?;
            if !node.label.is_empty() && node.label.len() > MAX_FLOW_NAME_BYTES {
                return Err(format!("Node '{}' label is too long.", node.id));
            }
            if let Some([x, y]) = node.position {
                if !x.is_finite()
                    || !y.is_finite()
                    || !(0.0..=20_000.0).contains(&x)
                    || !(0.0..=20_000.0).contains(&y)
                {
                    return Err(format!(
                        "Node '{}' has an invalid canvas position.",
                        node.id
                    ));
                }
            }
            if nodes.insert(node.id.as_str(), node).is_some() {
                return Err(format!("Duplicate node id '{}'.", node.id));
            }
        }
        let Some(entry) = nodes.get(self.entry_node_id.as_str()) else {
            return Err("The flow entry node does not exist.".into());
        };
        if matches!(entry.kind, FlowNodeKind::LoopEnd { .. }) {
            return Err("The flow cannot start at a loop-end node.".into());
        }

        let mut edges: HashMap<(&str, FlowPort), &str> = HashMap::new();
        let mut incoming: HashMap<&str, usize> = nodes.keys().map(|id| (*id, 0)).collect();
        let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();
        for edge in &self.edges {
            let Some(source) = nodes.get(edge.from_node_id.as_str()) else {
                return Err(format!(
                    "Connection starts at unknown node '{}'.",
                    edge.from_node_id
                ));
            };
            if !nodes.contains_key(edge.to_node_id.as_str()) {
                return Err(format!(
                    "Connection points to unknown node '{}'.",
                    edge.to_node_id
                ));
            }
            if !port_is_valid(&source.kind, edge.port) {
                return Err(format!(
                    "Port '{:?}' is not available on node '{}'.",
                    edge.port, edge.from_node_id
                ));
            }
            if edges
                .insert(
                    (edge.from_node_id.as_str(), edge.port),
                    edge.to_node_id.as_str(),
                )
                .is_some()
            {
                return Err(format!(
                    "Node '{}' has more than one connection on port '{:?}'.",
                    edge.from_node_id, edge.port
                ));
            }
            *incoming.entry(edge.to_node_id.as_str()).or_default() += 1;
            adjacency
                .entry(edge.from_node_id.as_str())
                .or_default()
                .push(edge.to_node_id.as_str());
        }
        if incoming
            .get(self.entry_node_id.as_str())
            .copied()
            .unwrap_or(0)
            != 0
        {
            return Err("The flow entry node cannot have incoming connections.".into());
        }
        for node in &self.nodes {
            for port in required_ports(&node.kind) {
                if !edges.contains_key(&(node.id.as_str(), *port)) {
                    return Err(format!(
                        "Node '{}' is missing its '{:?}' connection.",
                        node.id, port
                    ));
                }
            }
            validate_node(node)?;
        }

        let repeat_ids: HashSet<&str> = self
            .nodes
            .iter()
            .filter_map(|node| {
                matches!(node.kind, FlowNodeKind::Repeat { .. }).then_some(node.id.as_str())
            })
            .collect();
        let mut loop_ends = HashSet::new();
        let mut declared_variables: HashSet<&str> =
            self.variables.keys().map(String::as_str).collect();
        for node in &self.nodes {
            match &node.kind {
                FlowNodeKind::LoopEnd { repeat_node_id } => {
                    if !repeat_ids.contains(repeat_node_id.as_str()) {
                        return Err(format!(
                            "Loop-end node '{}' must refer to an existing repeat node.",
                            node.id
                        ));
                    }
                    loop_ends.insert(repeat_node_id.as_str());
                }
                FlowNodeKind::SetVariable { name, .. } => {
                    declared_variables.insert(name.as_str());
                }
                FlowNodeKind::Repeat {
                    counter_variable: Some(name),
                    ..
                } => {
                    validate_variable_name(name)?;
                    declared_variables.insert(name.as_str());
                }
                _ => {}
            }
        }
        for node in &self.nodes {
            if matches!(&node.kind, FlowNodeKind::Repeat { .. })
                && !loop_ends.contains(node.id.as_str())
            {
                return Err(format!(
                    "Repeat node '{}' has no matching loop-end node.",
                    node.id
                ));
            }
            if let FlowNodeKind::Condition { left, right, .. } = &node.kind {
                for operand in [left, right] {
                    if let Operand::Variable(name) = operand {
                        validate_variable_name(name)?;
                        if !declared_variables.contains(name.as_str()) {
                            return Err(format!("Condition uses undeclared variable '{name}'."));
                        }
                    }
                    if let Operand::Literal(value) = operand {
                        validate_value(value)?;
                    }
                }
            }
        }

        // Arbitrary graph cycles are rejected. Repetition is represented only by
        // the explicit LoopEnd -> Repeat runtime transition and has a hard cap.
        let mut indegree: HashMap<&str, usize> = incoming.clone();
        let mut queue: VecDeque<&str> = indegree
            .iter()
            .filter_map(|(id, degree)| (*degree == 0).then_some(*id))
            .collect();
        let mut visited_count = 0;
        while let Some(id) = queue.pop_front() {
            visited_count += 1;
            for target in adjacency.get(id).into_iter().flatten() {
                let degree = indegree.get_mut(target).expect("validated target exists");
                *degree -= 1;
                if *degree == 0 {
                    queue.push_back(target);
                }
            }
        }
        if visited_count != nodes.len() {
            return Err(
                "Connections form an uncontrolled cycle; use Repeat and LoopEnd nodes.".into(),
            );
        }

        // Every node must be reachable from the entry through a normal connection
        // or the explicit back-edge represented by a LoopEnd node.
        let mut reachable = HashSet::new();
        let mut stack = vec![self.entry_node_id.as_str()];
        while let Some(id) = stack.pop() {
            if !reachable.insert(id) {
                continue;
            }
            if let Some(node) = nodes.get(id) {
                for port in all_ports(&node.kind) {
                    if let Some(target) = edges.get(&(id, *port)) {
                        stack.push(target);
                    }
                }
                if let FlowNodeKind::LoopEnd { repeat_node_id } = &node.kind {
                    stack.push(repeat_node_id);
                }
            }
        }
        if reachable.len() != nodes.len() {
            let orphan = self
                .nodes
                .iter()
                .find(|node| !reachable.contains(node.id.as_str()))
                .map(|node| node.id.as_str())
                .unwrap_or("unknown");
            return Err(format!(
                "Node '{orphan}' cannot be reached from the flow entry."
            ));
        }
        Ok(())
    }

    pub fn to_json(&self) -> Result<String, String> {
        self.validate()?;
        serde_json::to_string_pretty(self)
            .map_err(|error| format!("Could not serialize flow: {error}"))
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        let program: Self =
            serde_json::from_str(json).map_err(|error| format!("Invalid flow JSON: {error}"))?;
        program.validate()?;
        Ok(program)
    }
}

fn validate_node(node: &FlowNode) -> Result<(), String> {
    match &node.kind {
        FlowNodeKind::Action {
            action,
            delay_after_ms,
        } => {
            let settings = AppSettings {
                sequence: vec![SequenceStep {
                    enabled: true,
                    action: action.clone(),
                    delay_after_ms: *delay_after_ms,
                    canvas_position: node.position,
                }],
                ..AppSettings::default()
            };
            settings
                .validate()
                .map_err(|error| format!("Node '{}': {error}", node.id))?;
        }
        FlowNodeKind::SetVariable { name, operation } => {
            validate_variable_name(name)?;
            match operation {
                VariableOperation::Set { value } => validate_value(value)?,
                VariableOperation::Add { amount } if !amount.is_finite() => {
                    return Err(format!(
                        "Node '{}' has a non-finite variable increment.",
                        node.id
                    ));
                }
                VariableOperation::Append { text } if text.len() > MAX_FLOW_TEXT_BYTES => {
                    return Err(format!("Node '{}' appends too much text.", node.id));
                }
                _ => {}
            }
        }
        FlowNodeKind::Condition { comparison, .. } => {
            if matches!(comparison, Comparison::Contains) {
                // The operand types are checked at execution because variables are
                // allowed to be assigned on different graph branches.
            }
        }
        FlowNodeKind::Repeat { iterations, .. } => {
            if *iterations == 0 || *iterations > MAX_LOOP_ITERATIONS {
                return Err(format!(
                    "Repeat node '{}' must run between 1 and {MAX_LOOP_ITERATIONS} times.",
                    node.id
                ));
            }
        }
        FlowNodeKind::LoopEnd { repeat_node_id } => validate_id(repeat_node_id)?,
        FlowNodeKind::End => {}
    }
    Ok(())
}

fn validate_value(value: &FlowValue) -> Result<(), String> {
    match value {
        FlowValue::Number(number) if !number.is_finite() => {
            Err("Flow numbers must be finite.".into())
        }
        FlowValue::Text(text) if text.len() > MAX_FLOW_TEXT_BYTES => Err(format!(
            "Flow text cannot exceed {MAX_FLOW_TEXT_BYTES} bytes."
        )),
        _ => Ok(()),
    }
}

fn validate_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > 64 || id.chars().any(char::is_control) {
        return Err("Node ids must contain 1–64 printable bytes.".into());
    }
    Ok(())
}

fn validate_variable_name(name: &str) -> Result<(), String> {
    let mut chars = name.chars();
    let valid_first = chars
        .next()
        .is_some_and(|ch| ch == '_' || ch.is_ascii_alphabetic());
    if !valid_first || name.len() > 64 || !chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric()) {
        return Err(format!("'{name}' is not a valid variable name."));
    }
    Ok(())
}

fn port_is_valid(kind: &FlowNodeKind, port: FlowPort) -> bool {
    required_ports(kind).contains(&port)
}

fn required_ports(kind: &FlowNodeKind) -> &'static [FlowPort] {
    match kind {
        FlowNodeKind::Action { .. } | FlowNodeKind::SetVariable { .. } => &[FlowPort::Next],
        FlowNodeKind::Condition { .. } => &[FlowPort::True, FlowPort::False],
        FlowNodeKind::Repeat { .. } => &[FlowPort::Body, FlowPort::Done],
        FlowNodeKind::LoopEnd { .. } | FlowNodeKind::End => &[],
    }
}

fn all_ports(kind: &FlowNodeKind) -> &'static [FlowPort] {
    required_ports(kind)
}

#[derive(Debug, Clone, Copy)]
struct LoopFrame<'a> {
    node_id: &'a str,
    completed_iterations: u32,
    total_iterations: u32,
}

/// Run a validated graph on the caller's background worker. `repeat_count == 0`
/// means run until the supplied host reports cancellation. The callback receives
/// the number of fully completed graph cycles.
pub fn execute_flow(
    program: &FlowProgram,
    options: FlowRunOptions,
    host: &mut impl FlowHost,
    mut on_cycle: impl FnMut(u64),
) -> Result<FlowRunSummary, String> {
    program.validate()?;
    if options.interval_ms == 0 || options.interval_ms > MAX_INTERVAL_MS {
        return Err(format!(
            "Flow interval must be between 1 and {MAX_INTERVAL_MS} ms."
        ));
    }
    if options.repeat_count > MAX_REPEAT_COUNT {
        return Err(format!(
            "Flow repeat count cannot exceed {MAX_REPEAT_COUNT}."
        ));
    }

    let nodes: HashMap<&str, &FlowNode> = program
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect();
    let edges: HashMap<(&str, FlowPort), &str> = program
        .edges
        .iter()
        .map(|edge| {
            (
                (edge.from_node_id.as_str(), edge.port),
                edge.to_node_id.as_str(),
            )
        })
        .collect();
    let mut variables = program.variables.clone();
    let mut completed_cycles = 0_u64;

    while options.repeat_count == 0 || completed_cycles < options.repeat_count {
        if host.is_cancelled() {
            return Ok(FlowRunSummary {
                completed_cycles,
                cancelled: true,
            });
        }
        let cycle_started = Instant::now();
        let mut current_id = program.entry_node_id.as_str();
        let mut loop_stack: Vec<LoopFrame<'_>> = Vec::new();
        let mut instruction_count = 0_usize;

        loop {
            if host.is_cancelled() {
                return Ok(FlowRunSummary {
                    completed_cycles,
                    cancelled: true,
                });
            }
            instruction_count += 1;
            if instruction_count > MAX_INSTRUCTIONS_PER_CYCLE {
                return Err(format!(
                    "Flow exceeded the {MAX_INSTRUCTIONS_PER_CYCLE}-instruction cycle limit."
                ));
            }
            let node = nodes
                .get(current_id)
                .copied()
                .ok_or_else(|| format!("Flow reached missing node '{current_id}'."))?;
            match &node.kind {
                FlowNodeKind::Action {
                    action,
                    delay_after_ms,
                } => {
                    host.execute_action(action)
                        .map_err(|error| format!("Node '{}': {error}", node.id))?;
                    if host.is_cancelled() {
                        return Ok(FlowRunSummary {
                            completed_cycles,
                            cancelled: true,
                        });
                    }
                    if *delay_after_ms > 0 {
                        host.wait(*delay_after_ms)
                            .map_err(|error| format!("Node '{}': {error}", node.id))?;
                    }
                    current_id = edge_target(&edges, node.id.as_str(), FlowPort::Next)?;
                }
                FlowNodeKind::SetVariable { name, operation } => {
                    apply_variable_operation(&mut variables, name, operation)?;
                    current_id = edge_target(&edges, node.id.as_str(), FlowPort::Next)?;
                }
                FlowNodeKind::Condition {
                    left,
                    comparison,
                    right,
                } => {
                    let left_value = resolve_operand(left, &variables)?;
                    let right_value = resolve_operand(right, &variables)?;
                    let result = compare_values(&left_value, *comparison, &right_value)?;
                    current_id = edge_target(
                        &edges,
                        node.id.as_str(),
                        if result {
                            FlowPort::True
                        } else {
                            FlowPort::False
                        },
                    )?;
                }
                FlowNodeKind::Repeat {
                    iterations,
                    counter_variable,
                } => {
                    let next_iteration = match loop_stack.last_mut() {
                        Some(frame) if frame.node_id == node.id => {
                            if frame.completed_iterations < frame.total_iterations {
                                frame.completed_iterations += 1;
                                Some(frame.completed_iterations)
                            } else {
                                loop_stack.pop();
                                None
                            }
                        }
                        Some(frame) if frame.node_id != node.id => {
                            return Err(format!(
                                "Repeat node '{}' was entered before inner loop '{}' completed.",
                                node.id, frame.node_id
                            ));
                        }
                        _ => {
                            if loop_stack.len() >= MAX_LOOP_DEPTH {
                                return Err(format!(
                                    "Flow exceeded the maximum nesting depth of {MAX_LOOP_DEPTH}."
                                ));
                            }
                            loop_stack.push(LoopFrame {
                                node_id: node.id.as_str(),
                                completed_iterations: 1,
                                total_iterations: *iterations,
                            });
                            Some(1)
                        }
                    };
                    if let Some(iteration) = next_iteration {
                        if let Some(name) = counter_variable {
                            set_variable(
                                &mut variables,
                                name,
                                FlowValue::Number(f64::from(iteration)),
                            )?;
                        }
                        current_id = edge_target(&edges, node.id.as_str(), FlowPort::Body)?;
                    } else {
                        current_id = edge_target(&edges, node.id.as_str(), FlowPort::Done)?;
                    }
                }
                FlowNodeKind::LoopEnd { repeat_node_id } => match loop_stack.last() {
                    Some(frame) if frame.node_id == repeat_node_id => {
                        current_id = repeat_node_id;
                    }
                    Some(frame) => {
                        return Err(format!(
                            "Loop-end '{}' expected '{}', but active loop is '{}'.",
                            node.id, repeat_node_id, frame.node_id
                        ));
                    }
                    None => {
                        return Err(format!(
                            "Loop-end '{}' was reached outside a repeat.",
                            node.id
                        ))
                    }
                },
                FlowNodeKind::End => {
                    if let Some(frame) = loop_stack.last() {
                        return Err(format!(
                            "Flow reached End while repeat '{}' was still active; every repeat body path must reach its matching loop-end.",
                            frame.node_id
                        ));
                    }
                    break;
                }
            }
        }

        completed_cycles += 1;
        on_cycle(completed_cycles);
        if options.repeat_count > 0 && completed_cycles >= options.repeat_count {
            return Ok(FlowRunSummary {
                completed_cycles,
                cancelled: false,
            });
        }
        if host.is_cancelled() {
            return Ok(FlowRunSummary {
                completed_cycles,
                cancelled: true,
            });
        }
        let elapsed = cycle_started.elapsed();
        let interval = std::time::Duration::from_millis(options.interval_ms);
        if elapsed < interval {
            host.wait((interval - elapsed).as_millis().min(u128::from(u64::MAX)) as u64)?;
        }
    }
    Ok(FlowRunSummary {
        completed_cycles,
        cancelled: host.is_cancelled(),
    })
}

fn edge_target<'a>(
    edges: &HashMap<(&'a str, FlowPort), &'a str>,
    node_id: &'a str,
    port: FlowPort,
) -> Result<&'a str, String> {
    edges
        .get(&(node_id, port))
        .copied()
        .ok_or_else(|| format!("Node '{node_id}' has no '{port:?}' connection."))
}

fn set_variable(
    variables: &mut BTreeMap<String, FlowValue>,
    name: &str,
    value: FlowValue,
) -> Result<(), String> {
    validate_value(&value)?;
    if !variables.contains_key(name) && variables.len() >= MAX_FLOW_VARIABLES {
        return Err(format!(
            "Flow exceeded the {MAX_FLOW_VARIABLES}-variable limit."
        ));
    }
    variables.insert(name.to_owned(), value);
    Ok(())
}

fn apply_variable_operation(
    variables: &mut BTreeMap<String, FlowValue>,
    name: &str,
    operation: &VariableOperation,
) -> Result<(), String> {
    match operation {
        VariableOperation::Set { value } => set_variable(variables, name, value.clone()),
        VariableOperation::Add { amount } => {
            if !amount.is_finite() {
                return Err(format!("Variable '{name}' has a non-finite increment."));
            }
            let Some(FlowValue::Number(current)) = variables.get(name) else {
                return Err(format!(
                    "Variable '{name}' must be initialized to a number before adding."
                ));
            };
            let next = current + amount;
            if !next.is_finite() {
                return Err(format!("Variable '{name}' overflowed."));
            }
            set_variable(variables, name, FlowValue::Number(next))
        }
        VariableOperation::Toggle => {
            let Some(FlowValue::Boolean(current)) = variables.get(name) else {
                return Err(format!(
                    "Variable '{name}' must be initialized to a boolean before toggling."
                ));
            };
            set_variable(variables, name, FlowValue::Boolean(!current))
        }
        VariableOperation::Append { text } => {
            let Some(FlowValue::Text(current)) = variables.get(name) else {
                return Err(format!(
                    "Variable '{name}' must be initialized to text before appending."
                ));
            };
            let mut next = current.clone();
            next.push_str(text);
            set_variable(variables, name, FlowValue::Text(next))
        }
    }
}

fn resolve_operand(
    operand: &Operand,
    variables: &BTreeMap<String, FlowValue>,
) -> Result<FlowValue, String> {
    match operand {
        Operand::Literal(value) => Ok(value.clone()),
        Operand::Variable(name) => variables
            .get(name)
            .cloned()
            .ok_or_else(|| format!("Condition reads variable '{name}' before it is initialized.")),
    }
}

fn compare_values(
    left: &FlowValue,
    comparison: Comparison,
    right: &FlowValue,
) -> Result<bool, String> {
    use Comparison as C;
    use FlowValue as V;
    match (left, right) {
        (V::Number(a), V::Number(b)) => Ok(match comparison {
            C::Equal => a == b,
            C::NotEqual => a != b,
            C::LessThan => a < b,
            C::LessOrEqual => a <= b,
            C::GreaterThan => a > b,
            C::GreaterOrEqual => a >= b,
            C::Contains => return Err("Contains comparison only supports text values.".into()),
        }),
        (V::Text(a), V::Text(b)) => Ok(match comparison {
            C::Equal => a == b,
            C::NotEqual => a != b,
            C::LessThan => a < b,
            C::LessOrEqual => a <= b,
            C::GreaterThan => a > b,
            C::GreaterOrEqual => a >= b,
            C::Contains => a.contains(b),
        }),
        (V::Boolean(a), V::Boolean(b)) => match comparison {
            C::Equal => Ok(a == b),
            C::NotEqual => Ok(a != b),
            _ => Err("Boolean values only support equal and not-equal comparisons.".into()),
        },
        _ => Err("Condition operands must use the same value type.".into()),
    }
}

/// Import the user's Autoflow Designer JSON format. The archive describes a
/// visual, single-successor chain; this importer validates it and converts it to
/// the native flow schema. No code from the archive is executed.
pub fn import_autoflow_json(json: &str) -> Result<FlowProgram, String> {
    let document: AutoflowDocument =
        serde_json::from_str(json).map_err(|error| format!("Invalid Autoflow JSON: {error}"))?;
    if let Some(version) = &document.version {
        if version != "1.0" {
            return Err(format!(
                "Unsupported Autoflow document version '{version}'."
            ));
        }
    }
    if document.nodes.is_empty() || document.nodes.len() > MAX_FLOW_NODES {
        return Err(format!("Autoflow must contain 1–{MAX_FLOW_NODES} nodes."));
    }

    let mut source_ids = HashSet::new();
    for node in &document.nodes {
        validate_id(&node.id)?;
        if !source_ids.insert(node.id.as_str()) {
            return Err(format!("Duplicate Autoflow node id '{}'.", node.id));
        }
    }
    let known: HashSet<&str> = document.nodes.iter().map(|node| node.id.as_str()).collect();
    let mut incoming = HashSet::new();
    for node in &document.nodes {
        if let Some(next) = &node.next_node_id {
            if !known.contains(next.as_str()) {
                return Err(format!(
                    "Autoflow node '{}' links to unknown node '{next}'.",
                    node.id
                ));
            }
            if !incoming.insert(next.as_str()) {
                return Err(format!(
                    "Autoflow node '{next}' has multiple incoming connections."
                ));
            }
        }
    }
    let roots: Vec<_> = document
        .nodes
        .iter()
        .filter(|node| !incoming.contains(node.id.as_str()))
        .collect();
    if roots.len() != 1 {
        return Err("Autoflow must have exactly one starting node.".into());
    }

    let mut native_nodes = Vec::new();
    let mut mapped_ids: HashMap<&str, Vec<String>> = HashMap::new();
    // Reserve every input id before generating ids for expanded multi-action
    // nodes, so a later source node cannot collide with an earlier generated id.
    let mut allocated_ids: HashSet<String> = source_ids.iter().map(|id| (*id).to_owned()).collect();
    let mut generated_id_counter = 0_u32;
    for source in &document.nodes {
        let position = source.position.as_ref().map(|p| [p.x as f32, p.y as f32]);
        if position.is_some_and(|[x, y]| {
            !x.is_finite()
                || !y.is_finite()
                || !(0.0..=20_000.0).contains(&x)
                || !(0.0..=20_000.0).contains(&y)
        }) {
            return Err(format!(
                "Autoflow node '{}' has an invalid canvas position.",
                source.id
            ));
        }
        let actions = autoflow_actions(source)?;
        let mut ids = Vec::new();
        for (index, action) in actions.into_iter().enumerate() {
            let id = if index == 0 {
                source.id.clone()
            } else {
                unique_generated_id(&mut allocated_ids, &mut generated_id_counter)
            };
            validate_id(&id)?;
            let node_position = position.map(|[x, y]| [x + index as f32 * 48.0, y]);
            ids.push(id.clone());
            native_nodes.push(FlowNode {
                id,
                label: source.node_type.clone(),
                position: node_position,
                kind: FlowNodeKind::Action {
                    action,
                    delay_after_ms: 0,
                },
            });
        }
        mapped_ids.insert(source.id.as_str(), ids);
    }

    let mut edges = Vec::new();
    for source in &document.nodes {
        let ids = mapped_ids
            .get(source.id.as_str())
            .ok_or_else(|| "Autoflow node conversion failed.".to_string())?;
        for pair in ids.windows(2) {
            edges.push(FlowEdge {
                from_node_id: pair[0].clone(),
                port: FlowPort::Next,
                to_node_id: pair[1].clone(),
            });
        }
        let last = ids.last().expect("each Autoflow node becomes an action");
        if let Some(next) = &source.next_node_id {
            let target_ids = mapped_ids
                .get(next.as_str())
                .ok_or_else(|| format!("Autoflow node '{next}' was not converted."))?;
            edges.push(FlowEdge {
                from_node_id: last.clone(),
                port: FlowPort::Next,
                to_node_id: target_ids[0].clone(),
            });
        }
    }

    let end_id = unique_generated_id(&mut allocated_ids, &mut generated_id_counter);
    let terminal_nodes: Vec<String> = document
        .nodes
        .iter()
        .filter(|node| node.next_node_id.is_none())
        .filter_map(|node| {
            mapped_ids
                .get(node.id.as_str())
                .and_then(|ids| ids.last().cloned())
        })
        .collect();
    if terminal_nodes.is_empty() {
        return Err("Autoflow contains a cycle and has no terminal node.".into());
    }
    native_nodes.push(FlowNode {
        id: end_id.clone(),
        label: "End".into(),
        position: None,
        kind: FlowNodeKind::End,
    });
    for terminal in terminal_nodes {
        edges.push(FlowEdge {
            from_node_id: terminal,
            port: FlowPort::Next,
            to_node_id: end_id.clone(),
        });
    }

    let entry_ids = mapped_ids
        .get(roots[0].id.as_str())
        .ok_or_else(|| "Autoflow entry node was not converted.".to_string())?;
    let program = FlowProgram {
        schema_version: FLOW_SCHEMA_VERSION,
        name: "Imported Autoflow".into(),
        entry_node_id: entry_ids[0].clone(),
        variables: BTreeMap::new(),
        nodes: native_nodes,
        edges,
    };
    program.validate()?;
    Ok(program)
}

fn unique_generated_id(allocated: &mut HashSet<String>, counter: &mut u32) -> String {
    loop {
        *counter = counter.saturating_add(1);
        let candidate = format!("__autoflow_generated_{}", *counter);
        if allocated.insert(candidate.clone()) {
            return candidate;
        }
    }
}

fn autoflow_actions(node: &AutoflowNode) -> Result<Vec<Action>, String> {
    let config = &node.config;
    match node.node_type.as_str() {
        "click" => {
            let button = match config.button.as_deref().unwrap_or("left") {
                "left" => crate::model::MouseButton::Left,
                "right" => crate::model::MouseButton::Right,
                "middle" => crate::model::MouseButton::Middle,
                other => {
                    return Err(format!(
                        "Node '{}' has unsupported mouse button '{other}'.",
                        node.id
                    ))
                }
            };
            let position = optional_position(config.x, config.y, &node.id)?;
            let click_type = match config.click_type.as_deref().unwrap_or("single") {
                "single" => crate::model::ClickType::Single,
                "double" => crate::model::ClickType::Double,
                "hold" => crate::model::ClickType::Hold,
                other => {
                    return Err(format!(
                        "Node '{}' has unsupported click type '{other}'.",
                        node.id
                    ))
                }
            };
            Ok(vec![Action::MouseClick {
                button,
                click_type,
                hold_ms: duration_ms(config.hold_ms, 100, &node.id)?,
                position,
            }])
        }
        "move" => {
            let position = required_position(config.x, config.y, &node.id)?;
            Ok(vec![Action::MouseMoveSmooth {
                position,
                duration_ms: duration_ms(config.duration, 0, &node.id)?,
            }])
        }
        "keyboard" => {
            let text = config.text.as_deref().filter(|value| !value.is_empty());
            let key = config
                .key
                .as_deref()
                .filter(|value| !value.trim().is_empty());
            let mut actions = Vec::new();
            if let Some(key) = key {
                actions.push(Action::KeyPress {
                    key: key.to_owned(),
                    hold_ms: duration_ms(config.hold_ms, 30, &node.id)?,
                });
            }
            if let Some(text) = text {
                actions.push(Action::TypeText {
                    text: text.to_owned(),
                });
            }
            if actions.is_empty() {
                return Err(format!("Keyboard node '{}' needs a key or text.", node.id));
            }
            Ok(actions)
        }
        "delay" => Ok(vec![Action::Wait {
            duration_ms: duration_ms(config.duration, 1_000, &node.id)?,
        }]),
        other => Err(format!(
            "Node '{}' uses unsupported Autoflow type '{other}'.",
            node.id
        )),
    }
}

fn optional_position(
    x: Option<f64>,
    y: Option<f64>,
    node_id: &str,
) -> Result<Option<[i32; 2]>, String> {
    match (x, y) {
        (None, None) => Ok(None),
        (Some(x), Some(y)) => Ok(Some([coordinate(x, node_id)?, coordinate(y, node_id)?])),
        _ => Err(format!(
            "Node '{node_id}' must specify both X and Y coordinates."
        )),
    }
}

fn required_position(x: Option<f64>, y: Option<f64>, node_id: &str) -> Result<[i32; 2], String> {
    optional_position(x, y, node_id)?
        .ok_or_else(|| format!("Node '{node_id}' needs X and Y coordinates."))
}

fn coordinate(value: f64, node_id: &str) -> Result<i32, String> {
    if !value.is_finite() || value < f64::from(i32::MIN) || value > f64::from(i32::MAX) {
        return Err(format!(
            "Node '{node_id}' has an invalid screen coordinate."
        ));
    }
    Ok(value.round() as i32)
}

fn duration_ms(value: Option<f64>, default: u64, node_id: &str) -> Result<u64, String> {
    let Some(value) = value else {
        return Ok(default);
    };
    if !value.is_finite() || value < 0.0 || value > MAX_FLOW_DURATION_MS as f64 {
        return Err(format!("Node '{node_id}' has an invalid duration."));
    }
    Ok(value.round() as u64)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AutoflowDocument {
    version: Option<String>,
    nodes: Vec<AutoflowNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AutoflowNode {
    id: String,
    #[serde(rename = "type")]
    node_type: String,
    #[serde(default)]
    position: Option<AutoflowPosition>,
    #[serde(default)]
    config: AutoflowConfig,
    #[serde(default)]
    next_node_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AutoflowPosition {
    x: f64,
    y: f64,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AutoflowConfig {
    x: Option<f64>,
    y: Option<f64>,
    button: Option<String>,
    click_type: Option<String>,
    hold_ms: Option<f64>,
    key: Option<String>,
    text: Option<String>,
    duration: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    #[derive(Default)]
    struct FakeHost {
        actions: Vec<Action>,
        waits: Vec<u64>,
        cancelled: Arc<AtomicBool>,
    }

    impl FlowHost for FakeHost {
        fn is_cancelled(&self) -> bool {
            self.cancelled.load(Ordering::Acquire)
        }
        fn execute_action(&mut self, action: &Action) -> Result<(), String> {
            self.actions.push(action.clone());
            Ok(())
        }
        fn wait(&mut self, duration_ms: u64) -> Result<(), String> {
            self.waits.push(duration_ms);
            Ok(())
        }
    }

    fn node(id: &str, kind: FlowNodeKind) -> FlowNode {
        FlowNode {
            id: id.into(),
            label: String::new(),
            position: None,
            kind,
        }
    }

    fn edge(from: &str, port: FlowPort, to: &str) -> FlowEdge {
        FlowEdge {
            from_node_id: from.into(),
            port,
            to_node_id: to.into(),
        }
    }

    fn click_node(id: &str) -> FlowNode {
        node(
            id,
            FlowNodeKind::Action {
                action: Action::MouseClick {
                    button: crate::model::MouseButton::Left,
                    click_type: crate::model::ClickType::Single,
                    hold_ms: 100,
                    position: None,
                },
                delay_after_ms: 0,
            },
        )
    }

    #[test]
    fn imports_autoflow_json_and_preserves_key_plus_text() {
        let json = r#"{"version":"1.0","nodes":[
          {"id":"a","type":"keyboard","position":{"x":10,"y":20},"config":{"key":"Ctrl+A","text":"hello"},"nextNodeId":"b"},
          {"id":"b","type":"delay","position":{"x":100,"y":20},"config":{"duration":500}}
        ]}"#;
        let flow = import_autoflow_json(json).unwrap();
        assert_eq!(flow.nodes.len(), 4);
        assert_eq!(flow.entry_node_id, "a");
        assert!(matches!(
            flow.nodes[0].kind,
            FlowNodeKind::Action {
                action: Action::KeyPress { .. },
                ..
            }
        ));
        assert!(matches!(
            flow.nodes[1].kind,
            FlowNodeKind::Action {
                action: Action::TypeText { .. },
                ..
            }
        ));
        assert!(matches!(
            flow.nodes[2].kind,
            FlowNodeKind::Action {
                action: Action::Wait { duration_ms: 500 },
                ..
            }
        ));
        assert!(FlowProgram::from_json(&flow.to_json().unwrap()).is_ok());
    }

    #[test]
    fn generated_import_ids_never_collide_with_source_node_ids() {
        let json = r#"{"version":"1.0","nodes":[
          {"id":"keyboard","type":"keyboard","config":{"key":"A","text":"hello"},"nextNodeId":"keyboard-1"},
          {"id":"keyboard-1","type":"delay","config":{"duration":5}}
        ]}"#;
        let flow = import_autoflow_json(json).unwrap();
        let ids: HashSet<&str> = flow.nodes.iter().map(|node| node.id.as_str()).collect();
        assert_eq!(ids.len(), flow.nodes.len());
        assert!(ids.contains("keyboard-1"));
        assert!(flow.validate().is_ok());
    }

    #[test]
    fn imports_mouse_move_click_and_terminal_connection() {
        let json = r#"{"version":"1.0","nodes":[
          {"id":"move","type":"move","position":{"x":0,"y":0},"config":{"x":640,"y":480,"duration":250},"nextNodeId":"click"},
          {"id":"click","type":"click","position":{"x":300,"y":0},"config":{"button":"right","x":20,"y":30}}
        ]}"#;
        let flow = import_autoflow_json(json).unwrap();
        assert!(matches!(
            flow.nodes[0].kind,
            FlowNodeKind::Action {
                action: Action::MouseMoveSmooth {
                    duration_ms: 250,
                    ..
                },
                ..
            }
        ));
        assert!(matches!(
            flow.nodes[1].kind,
            FlowNodeKind::Action {
                action: Action::MouseClick {
                    button: crate::model::MouseButton::Right,
                    ..
                },
                ..
            }
        ));
        assert!(matches!(flow.nodes.last().unwrap().kind, FlowNodeKind::End));
    }

    #[test]
    fn rejects_duplicate_ids_and_uncontrolled_cycles() {
        let mut flow = FlowProgram {
            schema_version: FLOW_SCHEMA_VERSION,
            name: "test".into(),
            entry_node_id: "a".into(),
            variables: BTreeMap::new(),
            nodes: vec![click_node("a"), click_node("a")],
            edges: vec![],
        };
        assert!(flow.validate().unwrap_err().contains("Duplicate"));
        flow.nodes = vec![click_node("start"), click_node("a"), click_node("b")];
        flow.entry_node_id = "start".into();
        flow.edges = vec![
            edge("start", FlowPort::Next, "a"),
            edge("a", FlowPort::Next, "b"),
            edge("b", FlowPort::Next, "a"),
        ];
        assert!(flow.validate().unwrap_err().contains("cycle"));
    }

    #[test]
    fn runs_condition_variable_and_bounded_loop_with_fake_host() {
        let flow = FlowProgram {
            schema_version: FLOW_SCHEMA_VERSION,
            name: "branch-loop".into(),
            entry_node_id: "set".into(),
            variables: BTreeMap::new(),
            nodes: vec![
                node(
                    "set",
                    FlowNodeKind::SetVariable {
                        name: "enabled".into(),
                        operation: VariableOperation::Set {
                            value: FlowValue::Boolean(true),
                        },
                    },
                ),
                node(
                    "check",
                    FlowNodeKind::Condition {
                        left: Operand::Variable("enabled".into()),
                        comparison: Comparison::Equal,
                        right: Operand::Literal(FlowValue::Boolean(true)),
                    },
                ),
                node(
                    "repeat",
                    FlowNodeKind::Repeat {
                        iterations: 3,
                        counter_variable: Some("iteration".into()),
                    },
                ),
                click_node("click"),
                node(
                    "loop_end",
                    FlowNodeKind::LoopEnd {
                        repeat_node_id: "repeat".into(),
                    },
                ),
                node("end", FlowNodeKind::End),
            ],
            edges: vec![
                edge("set", FlowPort::Next, "check"),
                edge("check", FlowPort::True, "repeat"),
                edge("check", FlowPort::False, "end"),
                edge("repeat", FlowPort::Body, "click"),
                edge("repeat", FlowPort::Done, "end"),
                edge("click", FlowPort::Next, "loop_end"),
            ],
        };
        let mut host = FakeHost::default();
        let mut cycles = Vec::new();
        let summary = execute_flow(
            &flow,
            FlowRunOptions {
                repeat_count: 1,
                interval_ms: 1,
            },
            &mut host,
            |count| cycles.push(count),
        )
        .unwrap();
        assert_eq!(summary.completed_cycles, 1);
        assert!(!summary.cancelled);
        assert_eq!(cycles, vec![1]);
        assert_eq!(host.actions.len(), 3);
    }

    #[test]
    fn rejects_a_branch_that_ends_before_the_active_loop_end() {
        let flow = FlowProgram {
            schema_version: FLOW_SCHEMA_VERSION,
            name: "early-end".into(),
            entry_node_id: "repeat".into(),
            variables: BTreeMap::new(),
            nodes: vec![
                node(
                    "repeat",
                    FlowNodeKind::Repeat {
                        iterations: 2,
                        counter_variable: None,
                    },
                ),
                node(
                    "condition",
                    FlowNodeKind::Condition {
                        left: Operand::Literal(FlowValue::Boolean(false)),
                        comparison: Comparison::Equal,
                        right: Operand::Literal(FlowValue::Boolean(true)),
                    },
                ),
                node(
                    "loop_end",
                    FlowNodeKind::LoopEnd {
                        repeat_node_id: "repeat".into(),
                    },
                ),
                node("end", FlowNodeKind::End),
            ],
            edges: vec![
                edge("repeat", FlowPort::Body, "condition"),
                edge("repeat", FlowPort::Done, "end"),
                edge("condition", FlowPort::True, "loop_end"),
                edge("condition", FlowPort::False, "end"),
            ],
        };
        assert!(flow.validate().is_ok());
        let mut host = FakeHost::default();
        let error = execute_flow(&flow, FlowRunOptions::default(), &mut host, |_| {}).unwrap_err();
        assert!(error.contains("while repeat 'repeat' was still active"));
    }

    #[test]
    fn cancellation_before_start_is_honored() {
        let flow = FlowProgram {
            schema_version: FLOW_SCHEMA_VERSION,
            name: "cancel".into(),
            entry_node_id: "end".into(),
            variables: BTreeMap::new(),
            nodes: vec![node("end", FlowNodeKind::End)],
            edges: vec![],
        };
        let mut host = FakeHost::default();
        host.cancelled.store(true, Ordering::Release);
        let summary = execute_flow(&flow, FlowRunOptions::default(), &mut host, |_| {}).unwrap();
        assert!(summary.cancelled);
        assert_eq!(summary.completed_cycles, 0);
    }

    #[test]
    fn settings_format_round_trip_remains_independent() {
        let settings = AppSettings::default();
        assert_eq!(
            serde_json::from_str::<AppSettings>(&serde_json::to_string(&settings).unwrap())
                .unwrap(),
            settings
        );
    }
}

#[cfg(test)]
mod importer_tests {
    use super::*;

    #[test]
    fn rejects_disconnected_nodes_malformed_coordinates_and_missing_links() {
        let disconnected = r#"{"version":"1.0","nodes":[
          {"id":"a","type":"delay","config":{"duration":1}},
          {"id":"b","type":"delay","config":{"duration":1}}
        ]}"#;
        assert!(import_autoflow_json(disconnected).is_err());
        let malformed = r#"{"version":"1.0","nodes":[
          {"id":"a","type":"move","config":{"x":1}}
        ]}"#;
        assert!(import_autoflow_json(malformed).is_err());
    }
}
