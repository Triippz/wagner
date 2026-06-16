//! Composable workflow model (Phase A).
//!
//! A workflow is an engineer-authored directed graph of *stages*, each bound to a
//! hired operative (a skill/agent). The current hardcoded goal loop becomes just
//! the built-in **Standard** template. This module is the pure data model +
//! validation; `workflow_exec` walks it over the `AgentPool`.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// What a node does. Phase A executes every kind as "run the operative once and
/// pass the artifact on"; later phases deepen Gate (block), Research (fan-out),
/// and Test (pass/fail) semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageKind {
    Research,
    Aggregate,
    Scope,
    Gate,
    Plan,
    Interrogate,
    Tdd,
    Execute,
    Review,
    Test,
    Done,
}

impl StageKind {
    /// Stages that decide pass/fail (and thus may have `OnFail` out-edges).
    pub fn is_gate_or_check(self) -> bool {
        matches!(self, StageKind::Gate | StageKind::Review | StageKind::Test)
    }
}

/// How a `Gate` node resolves. `Human` blocks for an engineer approve/reject;
/// `Auto` passes straight through (the "full-autonomous" toggle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GateMode {
    #[default]
    Human,
    Auto,
}

/// One stage in the graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowNode {
    pub id: String,
    pub kind: StageKind,
    /// The hired operative (roster id) this stage runs as. `None` → the lead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operative_id: Option<String>,
    /// Extra per-stage instruction appended to the operative's prompt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instruction: Option<String>,
    /// For `Gate` nodes only: human-approval vs auto-pass. Ignored elsewhere.
    /// Absent → `Human` (a gate blocks by default; auto-pass is opt-in).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate_mode: Option<GateMode>,
    /// Optional fan-out (decision #1): roster ids run *in parallel* before this
    /// node's primary operative consolidates their findings. Empty → no fan-out.
    /// Classic use is `Research`, but any stage may fan out.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sub_operatives: Vec<String>,
}

impl WorkflowNode {
    /// A node bound to an operative (or the lead when `operative_id` is `None`).
    pub fn new(id: impl Into<String>, kind: StageKind, operative_id: Option<String>) -> Self {
        Self {
            id: id.into(),
            kind,
            operative_id,
            instruction: None,
            gate_mode: None,
            sub_operatives: Vec::new(),
        }
    }

    /// The gate mode for this node, defaulting to `Human` (only meaningful for `Gate`).
    pub fn gate_mode(&self) -> GateMode {
        self.gate_mode.unwrap_or_default()
    }
}

/// When a transition fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EdgeWhen {
    #[default]
    Always,
    OnPass,
    OnFail,
}

/// A directed transition between stages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowEdge {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub when: EdgeWhen,
    /// Optional per-edge loop cap (decision #3): the most times this edge may be
    /// traversed in one run. Mainly for `OnFail` fix-loops (e.g. ≤3 review cycles).
    /// `None` → bounded only by the run-wide `max_steps`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_traversals: Option<usize>,
}

impl WorkflowEdge {
    /// An uncapped edge (bounded only by the run-wide step ceiling).
    pub fn new(from: impl Into<String>, to: impl Into<String>, when: EdgeWhen) -> Self {
        Self { from: from.into(), to: to.into(), when, max_traversals: None }
    }

    /// An edge that may be traversed at most `cap` times in one run.
    pub fn capped(
        from: impl Into<String>,
        to: impl Into<String>,
        when: EdgeWhen,
        cap: usize,
    ) -> Self {
        Self { from: from.into(), to: to.into(), when, max_traversals: Some(cap) }
    }
}

/// An engineer-authored workflow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Workflow {
    pub schema: String,
    pub root_goal: String,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
}

pub const WORKFLOW_SCHEMA: &str = "workflow.v1";

/// Default per-edge cap for built-in fix-loops (review/test/gate retries): the
/// most times such an `OnFail` edge may be traversed in one run.
const FIX_LOOP_CAP: usize = 3;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum WorkflowError {
    #[error("workflow must have at least one node")]
    Empty,
    #[error("duplicate node id: {0}")]
    DuplicateId(String),
    #[error("edge references unknown node: {0}")]
    UnknownNode(String),
    #[error("workflow needs exactly one start node (no incoming edges), found {0}")]
    NotOneStart(usize),
    #[error("workflow needs at least one Done node")]
    NoDone,
    #[error("Done node {0} must have no outgoing edges")]
    DoneHasOutgoing(String),
    #[error("non-terminal node {0} has no outgoing edge (dead end)")]
    DeadEnd(String),
    #[error("OnFail edge must originate from a Gate/Review/Test node, not {0}")]
    BadOnFail(String),
    #[error("no Done node is reachable from the start")]
    DoneUnreachable,
}

impl Workflow {
    pub fn node(&self, id: &str) -> Option<&WorkflowNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Outgoing edges of a node, in declared order.
    pub fn out_edges<'a>(&'a self, id: &'a str) -> impl Iterator<Item = &'a WorkflowEdge> {
        self.edges.iter().filter(move |e| e.from == id)
    }

    /// The single start node (no incoming edges). Caller should `validate` first.
    pub fn start(&self) -> Option<&WorkflowNode> {
        let has_incoming: HashSet<&str> = self.edges.iter().map(|e| e.to.as_str()).collect();
        self.nodes.iter().find(|n| !has_incoming.contains(n.id.as_str()))
    }

    /// Validate structural integrity (see `WorkflowError`).
    pub fn validate(&self) -> Result<(), WorkflowError> {
        if self.nodes.is_empty() {
            return Err(WorkflowError::Empty);
        }
        // unique ids
        let mut ids = HashSet::new();
        for n in &self.nodes {
            if !ids.insert(n.id.as_str()) {
                return Err(WorkflowError::DuplicateId(n.id.clone()));
            }
        }
        // edges reference real nodes
        for e in &self.edges {
            if !ids.contains(e.from.as_str()) {
                return Err(WorkflowError::UnknownNode(e.from.clone()));
            }
            if !ids.contains(e.to.as_str()) {
                return Err(WorkflowError::UnknownNode(e.to.clone()));
            }
        }
        let kind: HashMap<&str, StageKind> =
            self.nodes.iter().map(|n| (n.id.as_str(), n.kind)).collect();
        // OnFail only from a gate/check
        for e in &self.edges {
            if e.when == EdgeWhen::OnFail && !kind[e.from.as_str()].is_gate_or_check() {
                return Err(WorkflowError::BadOnFail(e.from.clone()));
            }
        }
        // exactly one start
        let has_incoming: HashSet<&str> = self.edges.iter().map(|e| e.to.as_str()).collect();
        let starts: Vec<&str> = self
            .nodes
            .iter()
            .map(|n| n.id.as_str())
            .filter(|id| !has_incoming.contains(id))
            .collect();
        if starts.len() != 1 {
            return Err(WorkflowError::NotOneStart(starts.len()));
        }
        // ≥1 Done; Done has no out-edges; non-Done has ≥1 out-edge
        let has_outgoing: HashSet<&str> = self.edges.iter().map(|e| e.from.as_str()).collect();
        let mut any_done = false;
        for n in &self.nodes {
            if n.kind == StageKind::Done {
                any_done = true;
                if has_outgoing.contains(n.id.as_str()) {
                    return Err(WorkflowError::DoneHasOutgoing(n.id.clone()));
                }
            } else if !has_outgoing.contains(n.id.as_str()) {
                return Err(WorkflowError::DeadEnd(n.id.clone()));
            }
        }
        if !any_done {
            return Err(WorkflowError::NoDone);
        }
        // a Done node is reachable from the start
        if !self.reaches_done(starts[0]) {
            return Err(WorkflowError::DoneUnreachable);
        }
        Ok(())
    }

    fn reaches_done(&self, start: &str) -> bool {
        let mut stack = vec![start];
        let mut seen = HashSet::new();
        while let Some(id) = stack.pop() {
            if !seen.insert(id) {
                continue;
            }
            if self.node(id).map(|n| n.kind) == Some(StageKind::Done) {
                return true;
            }
            for e in self.out_edges(id) {
                stack.push(e.to.as_str());
            }
        }
        false
    }

    /// The built-in **Standard** template — today's loop as a workflow:
    /// Plan → Execute → Review → Done. `lead`/`forger` are roster ids.
    pub fn standard_template(goal: &str, lead: &str, forger: &str) -> Workflow {
        let node = |id: &str, kind, op: &str| WorkflowNode::new(id, kind, Some(op.into()));
        Workflow {
            schema: WORKFLOW_SCHEMA.into(),
            root_goal: goal.into(),
            nodes: vec![
                node("plan", StageKind::Plan, lead),
                node("execute", StageKind::Execute, forger),
                node("review", StageKind::Review, lead),
                WorkflowNode::new("done", StageKind::Done, None),
            ],
            edges: vec![
                WorkflowEdge::new("plan", "execute", EdgeWhen::Always),
                WorkflowEdge::new("execute", "review", EdgeWhen::Always),
                WorkflowEdge::new("review", "done", EdgeWhen::OnPass),
                // review failure loops back to execute (fix-list threaded as fix context)
                WorkflowEdge::new("review", "execute", EdgeWhen::OnFail),
            ],
        }
    }

    /// The **Full pipeline** template from the brief:
    /// Research → Aggregate → Scope → Gate(human) → Plan → Interrogate → TDD →
    /// Execute → Review⟲ → Test⟲ → Done. Fix-loops are capped at 3 cycles each.
    /// `Research` ships with no sub-operatives — the engineer adds fan-out workers
    /// in the builder. `lead`/`forger` are roster ids.
    pub fn full_pipeline_template(goal: &str, lead: &str, forger: &str) -> Workflow {
        let n = |id: &str, kind, op: &str| WorkflowNode::new(id, kind, Some(op.into()));
        Workflow {
            schema: WORKFLOW_SCHEMA.into(),
            root_goal: goal.into(),
            nodes: vec![
                n("research", StageKind::Research, lead),
                n("aggregate", StageKind::Aggregate, lead),
                n("scope", StageKind::Scope, lead),
                WorkflowNode::new("gate", StageKind::Gate, None),
                n("plan", StageKind::Plan, lead),
                n("interrogate", StageKind::Interrogate, lead),
                n("tdd", StageKind::Tdd, forger),
                n("execute", StageKind::Execute, forger),
                n("review", StageKind::Review, lead),
                WorkflowNode::new("test", StageKind::Test, None),
                WorkflowNode::new("done", StageKind::Done, None),
            ],
            edges: vec![
                WorkflowEdge::new("research", "aggregate", EdgeWhen::Always),
                WorkflowEdge::new("aggregate", "scope", EdgeWhen::Always),
                WorkflowEdge::new("scope", "gate", EdgeWhen::Always),
                WorkflowEdge::new("gate", "plan", EdgeWhen::OnPass),
                WorkflowEdge::capped("gate", "scope", EdgeWhen::OnFail, FIX_LOOP_CAP), // reject → rescope
                WorkflowEdge::new("plan", "interrogate", EdgeWhen::Always),
                WorkflowEdge::new("interrogate", "tdd", EdgeWhen::Always),
                WorkflowEdge::new("tdd", "execute", EdgeWhen::Always),
                WorkflowEdge::new("execute", "review", EdgeWhen::Always),
                WorkflowEdge::new("review", "test", EdgeWhen::OnPass),
                WorkflowEdge::capped("review", "execute", EdgeWhen::OnFail, FIX_LOOP_CAP),
                WorkflowEdge::new("test", "done", EdgeWhen::OnPass),
                WorkflowEdge::capped("test", "execute", EdgeWhen::OnFail, FIX_LOOP_CAP),
            ],
        }
    }
}

/// A reusable, named starter workflow shown in the builder's template picker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedTemplate {
    pub name: String,
    pub description: String,
    pub workflow: Workflow,
}

/// The built-in starter templates (decision #4: workflows are reusable templates).
/// `lead`/`forger` seed each stage's default operative; the engineer rebinds them
/// in the builder. `goal` is a placeholder the engineer overwrites at launch.
pub fn builtin_templates(lead: &str, forger: &str) -> Vec<NamedTemplate> {
    vec![
        NamedTemplate {
            name: "Standard".into(),
            description: "Today's loop: Plan → Execute → Review (fix-loop) → Done.".into(),
            workflow: Workflow::standard_template("", lead, forger),
        },
        NamedTemplate {
            name: "Full pipeline".into(),
            description:
                "Research → Aggregate → Scope → Gate → Plan → Interrogate → TDD → Execute → \
                 Review⟲ → Test⟲ → Done."
                    .into(),
            workflow: Workflow::full_pipeline_template("", lead, forger),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn standard() -> Workflow {
        Workflow::standard_template("build the thing", "cipher", "vex")
    }

    #[test]
    fn standard_template_validates() {
        standard().validate().expect("standard template must validate");
    }

    #[test]
    fn full_pipeline_template_validates() {
        Workflow::full_pipeline_template("g", "cipher", "vex")
            .validate()
            .expect("full pipeline template must validate");
    }

    #[test]
    fn full_pipeline_starts_at_research() {
        let wf = Workflow::full_pipeline_template("g", "cipher", "vex");
        assert_eq!(wf.start().unwrap().id, "research");
    }

    #[test]
    fn builtin_templates_all_validate() {
        for t in builtin_templates("cipher", "vex") {
            t.workflow
                .validate()
                .unwrap_or_else(|e| panic!("template {} must validate: {e}", t.name));
        }
    }

    #[test]
    fn workflow_json_roundtrips() {
        let wf = Workflow::full_pipeline_template("ship it", "cipher", "vex");
        let json = serde_json::to_string(&wf).unwrap();
        let back: Workflow = serde_json::from_str(&json).unwrap();
        assert_eq!(wf, back);
    }

    #[test]
    fn start_is_the_plan_node() {
        assert_eq!(standard().start().unwrap().id, "plan");
    }

    #[test]
    fn empty_is_rejected() {
        let wf = Workflow {
            schema: WORKFLOW_SCHEMA.into(),
            root_goal: "g".into(),
            nodes: vec![],
            edges: vec![],
        };
        assert_eq!(wf.validate(), Err(WorkflowError::Empty));
    }

    #[test]
    fn missing_done_is_rejected() {
        let wf = Workflow {
            schema: WORKFLOW_SCHEMA.into(),
            root_goal: "g".into(),
            nodes: vec![
                WorkflowNode::new("a", StageKind::Plan, None),
                WorkflowNode::new("b", StageKind::Execute, None),
            ],
            edges: vec![WorkflowEdge::new("a", "b", EdgeWhen::Always)],
        };
        // b is a non-Done dead end → DeadEnd fires before NoDone, both are valid rejections
        assert!(wf.validate().is_err());
    }

    #[test]
    fn done_with_outgoing_edge_is_rejected() {
        let mut wf = standard();
        // point at a node that already has an incoming edge so the start set is unchanged
        wf.edges.push(WorkflowEdge::new("done", "execute", EdgeWhen::Always));
        assert_eq!(wf.validate(), Err(WorkflowError::DoneHasOutgoing("done".into())));
    }

    #[test]
    fn onfail_from_a_non_check_node_is_rejected() {
        let mut wf = standard();
        // plan is not a gate/check — an OnFail from it is illegal
        wf.edges.push(WorkflowEdge::new("plan", "done", EdgeWhen::OnFail));
        assert_eq!(wf.validate(), Err(WorkflowError::BadOnFail("plan".into())));
    }

    #[test]
    fn two_start_nodes_are_rejected() {
        let mut wf = standard();
        // add an orphan node with no incoming edge → two starts
        wf.nodes.push(WorkflowNode::new("orphan", StageKind::Scope, None));
        wf.edges.push(WorkflowEdge::new("orphan", "done", EdgeWhen::Always));
        assert_eq!(wf.validate(), Err(WorkflowError::NotOneStart(2)));
    }

    #[test]
    fn unknown_edge_endpoint_is_rejected() {
        let mut wf = standard();
        wf.edges.push(WorkflowEdge::new("plan", "ghost", EdgeWhen::Always));
        assert_eq!(wf.validate(), Err(WorkflowError::UnknownNode("ghost".into())));
    }
}
