//! Workflow executor (Phase A).
//!
//! Walks an engineer-authored [`Workflow`] graph over the [`AgentPool`], running
//! one operative per node and routing the next edge by the node's outcome. This
//! is the composable generalization of the hardcoded `run_goal` loop: the built-in
//! [`Workflow::standard_template`] reproduces today's Plan → Execute → Review path.
//!
//! Phase A semantics are deliberately shallow — every node "runs its operative once
//! and passes the artifact on". Gate/Review/Test nodes additionally decide pass/fail
//! (from the engine outcome's `success` flag) so `OnPass`/`OnFail` edges route. Later
//! phases deepen Gate (human block), Research (fan-out), and Test (real suite) — see
//! `plan-workflow-builder.md`. The walk is bounded by `max_steps` so a fix-loop can
//! never spin forever.

use super::engine::Role;
use super::run_loop::AgentPool;
use super::workflow::{EdgeWhen, GateMode, StageKind, Workflow, WorkflowEdge, WorkflowError};
use futures::future::BoxFuture;
use std::collections::HashMap;

/// One executed node, in walk order.
#[derive(Debug, Clone, PartialEq)]
pub struct StepRecord {
    pub node_id: String,
    pub kind: StageKind,
    /// The operative that ran (resolved: a node's `operative_id`, or the lead).
    pub operative_id: String,
    /// The operative's final text — threaded forward as the next node's context.
    pub final_text: String,
    pub success: bool,
    /// For gate/check nodes: did it pass? `None` for ordinary stages.
    pub passed: Option<bool>,
    /// How many sub-operatives ran in parallel before the primary consolidated
    /// (decision #1 fan-out). `0` for an ordinary single-operative stage.
    pub fanout: usize,
}

/// How a workflow walk ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowEnd {
    /// Reached a `Done` node.
    Completed,
    /// A node had no edge matching its outcome (dead end mid-walk).
    Stuck(String),
    /// Hit the `max_steps` ceiling without reaching `Done`.
    StepsExhausted,
    /// An `OnFail` fix-loop hit its per-edge `max_traversals` cap (decision #3):
    /// the engineer must intervene rather than loop further. Holds `from->to`.
    LoopCapReached(String),
}

/// The engineer's verdict on a human `Gate`. `Reject` routes the `OnFail` edge with
/// `reason` threaded forward as fix context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateDecision {
    Approve,
    Reject { reason: String },
}

/// Result of a `Test` stage's deterministic harness run (cargo/vitest/playwright/
/// script). On failure, `summary` is threaded back as fix context along `OnFail`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestOutcome {
    pub passed: bool,
    pub summary: String,
}

/// Everything the executor needs from the outside world, injected for testability.
pub struct ExecConfig<'a> {
    /// The hired roster the stages run as.
    pub pool: &'a dyn AgentPool,
    /// Total node-execution ceiling (the run-wide loop bound).
    pub max_steps: usize,
    /// Resolves a *human* `Gate` given `(node_id, upstream_artifact)`. `Auto` gates
    /// never call this. Async because the real impl opens a transmission and awaits
    /// the engineer's answer (arriving via a separate command); tests return ready.
    pub resolve_gate: &'a (dyn Fn(&str, &str) -> BoxFuture<'static, GateDecision> + Send + Sync),
    /// Runs a `Test` stage's harness, given the node's `instruction` (harness name
    /// or script path; empty if unset). Deterministic — no LLM. The real impl shells
    /// out to cargo/vitest/playwright; tests script it.
    pub run_test: &'a (dyn Fn(&str) -> TestOutcome + Send + Sync),
    /// Observability sink, called once per completed stage so the live builder can
    /// highlight the active node and stream artifacts. No-op in tests. This is the
    /// *single* event sink — collapse new observables into a typed event here rather
    /// than growing this struct (same ceiling rule as `run_loop::LoopDeps`).
    pub on_step: &'a (dyn Fn(&StepRecord) + Send + Sync),
}

/// The result of walking a workflow to its end.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowRun {
    pub steps: Vec<StepRecord>,
    pub end: WorkflowEnd,
    pub cost: f64,
}

impl WorkflowRun {
    pub fn completed(&self) -> bool {
        self.end == WorkflowEnd::Completed
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum WorkflowRunError {
    #[error("invalid workflow: {0}")]
    Invalid(#[from] WorkflowError),
    #[error("node '{node}' names operative '{operative}', which is not in the roster")]
    UnknownOperative { node: String, operative: String },
}

/// The CLI role a stage runs as. Planning-shaped stages think; checks judge;
/// everything else does work.
fn role_for(kind: StageKind) -> Role {
    match kind {
        StageKind::Plan
        | StageKind::Interrogate
        | StageKind::Scope
        | StageKind::Aggregate
        | StageKind::Research => Role::Plan,
        StageKind::Gate | StageKind::Review | StageKind::Test => Role::Judge,
        StageKind::Tdd | StageKind::Execute | StageKind::Done => Role::Execute,
    }
}

/// The context handed downstream: the prior node's output, flagged as either a
/// normal artifact or a *fix context* (carried along an `OnFail` edge — a review
/// fix-list or a gate rejection reason that the next stage must address).
#[derive(Default)]
struct FlowContext {
    text: String,
    is_fix: bool,
}

/// Build the prompt a node's operative sees: the root goal, any per-stage
/// instruction, and the upstream context (labelled FIX CONTEXT after an `OnFail`).
fn node_prompt(wf: &Workflow, kind: StageKind, instruction: Option<&str>, ctx: &FlowContext) -> String {
    let mut p = format!("GOAL:\n{}\n\nSTAGE: {:?}", wf.root_goal, kind);
    if let Some(extra) = instruction {
        p.push_str("\n\nINSTRUCTION:\n");
        p.push_str(extra);
    }
    if !ctx.text.is_empty() {
        p.push_str(if ctx.is_fix {
            "\n\nFIX CONTEXT (address this — the prior gate/check did not pass):\n"
        } else {
            "\n\nUPSTREAM ARTIFACT:\n"
        });
        p.push_str(&ctx.text);
    }
    p
}

/// Choose the next edge given the current node's outcome.
///
/// Contract: a check prefers the edge matching its pass/fail verdict, then an
/// `Always` edge; an ordinary stage takes its `Always` edge. Returns `None` when
/// nothing matches — the caller turns that into a `Stuck` end. We deliberately do
/// NOT fall back to "any first edge": routing a verdict to an arbitrary edge would
/// misdirect the walk with no signal, so an unrouted outcome surfaces as `Stuck`.
fn next_edge<'a>(wf: &'a Workflow, from: &'a str, passed: Option<bool>) -> Option<&'a WorkflowEdge> {
    let edges: Vec<&WorkflowEdge> = wf.out_edges(from).collect();
    let preferred = match passed {
        Some(true) => Some(EdgeWhen::OnPass),
        Some(false) => Some(EdgeWhen::OnFail),
        None => None,
    };
    if let Some(want) = preferred {
        if let Some(e) = edges.iter().find(|e| e.when == want) {
            return Some(e);
        }
    }
    edges.iter().find(|e| e.when == EdgeWhen::Always).copied()
}

/// What executing ONE node produced, as a named bundle. Two of its fields are
/// adjacent booleans (`success`, `passed`); a positional tuple here would let a
/// swap compile silently, so the executor builds this struct by field name.
struct NodeOutcome {
    /// Display label for the operative/gate/test that ran.
    operative_label: String,
    /// The node's output, threaded forward as the next node's context.
    final_text: String,
    success: bool,
    /// For gate/check/test nodes: pass/fail. `None` for ordinary stages.
    passed: Option<bool>,
    fanout: usize,
}

/// What running a non-gate stage produced.
struct StageOutcome {
    operative_id: String,
    final_text: String,
    success: bool,
    /// Sub-operatives that ran in parallel before the primary consolidated.
    fanout: usize,
    cost: f64,
}

/// Run a non-gate stage: fan out to `sub_operatives` in parallel (decision #1),
/// then the primary operative consolidates their findings into the stage artifact.
/// With no fan-out this is just one operative call.
async fn run_stage(
    wf: &Workflow,
    node: &super::workflow::WorkflowNode,
    ctx: &FlowContext,
    cfg: &ExecConfig<'_>,
) -> Result<StageOutcome, WorkflowRunError> {
    let role = role_for(node.kind);
    let mut cost = 0.0;

    // 1. Fan-out: identical prompt to N distinct operatives — each brings its own
    //    skill identity, so the *operatives* differ, not the instruction (ICM way).
    let mut findings = String::new();
    let fanout = node.sub_operatives.len();
    if fanout > 0 {
        let sub_prompt = node_prompt(wf, node.kind, node.instruction.as_deref(), ctx);
        let mut futs = Vec::with_capacity(fanout);
        for sub_id in &node.sub_operatives {
            let runner = cfg.pool.runner(sub_id).ok_or_else(|| {
                WorkflowRunError::UnknownOperative {
                    node: node.id.clone(),
                    operative: sub_id.clone(),
                }
            })?;
            let id = sub_id.clone();
            let p = sub_prompt.clone();
            futs.push(async move { (id, runner.run(role, &p).await) });
        }
        for (id, out) in futures::future::join_all(futs).await {
            cost += out.cost;
            findings.push_str(&format!("\n### {id} findings\n{}\n", out.final_text));
        }
    }

    // 2. Primary consolidates (or simply runs, when there was no fan-out).
    let operative_id = node.operative_id.clone().unwrap_or_else(|| cfg.pool.lead_id());
    let runner = cfg.pool.runner(&operative_id).ok_or_else(|| {
        WorkflowRunError::UnknownOperative {
            node: node.id.clone(),
            operative: operative_id.clone(),
        }
    })?;
    let primary_ctx = if fanout > 0 {
        FlowContext {
            text: format!("{}\n\nPARALLEL FINDINGS TO CONSOLIDATE:{findings}", ctx.text),
            is_fix: ctx.is_fix,
        }
    } else {
        FlowContext { text: ctx.text.clone(), is_fix: ctx.is_fix }
    };
    let prompt = node_prompt(wf, node.kind, node.instruction.as_deref(), &primary_ctx);
    let out = runner.run(role, &prompt).await;
    cost += out.cost;

    Ok(StageOutcome { operative_id, final_text: out.final_text, success: out.success, fanout, cost })
}

/// Walk `wf` to completion (or until bounded out), running one operative per node.
///
/// Bounded three ways: the run-wide `cfg.max_steps`, an optional per-`OnFail`-edge
/// `max_traversals` (decision #3), and structural validation up front. `Gate` nodes
/// don't burn a CLI call — an `Auto` gate passes through; a `Human` gate blocks on
/// `cfg.resolve_gate`, and a rejection routes `OnFail` with the reason as fix context.
pub async fn run_workflow(
    wf: &Workflow,
    cfg: &ExecConfig<'_>,
) -> Result<WorkflowRun, WorkflowRunError> {
    wf.validate()?;
    // validate() guarantees exactly one start node.
    let mut current = wf.start().expect("validated workflow has a start").id.clone();
    let mut steps: Vec<StepRecord> = Vec::new();
    let mut ctx = FlowContext::default();
    let mut cost = 0.0;
    // per-edge traversal counts, keyed by (from, to, when), for the loop cap.
    let mut traversals: HashMap<(String, String, EdgeWhen), usize> = HashMap::new();

    for _ in 0..cfg.max_steps {
        let node = wf.node(&current).expect("walk stays on validated nodes");
        if node.kind == StageKind::Done {
            return Ok(WorkflowRun { steps, end: WorkflowEnd::Completed, cost });
        }

        // Produce this node's outcome (built by field name so the two adjacent
        // booleans can't be transposed silently).
        let outcome = if node.kind == StageKind::Gate {
            match node.gate_mode() {
                GateMode::Auto => NodeOutcome {
                    operative_label: "(gate:auto)".to_string(),
                    final_text: "auto-approved".to_string(),
                    success: true,
                    passed: Some(true),
                    fanout: 0,
                },
                GateMode::Human => match (cfg.resolve_gate)(&node.id, &ctx.text).await {
                    GateDecision::Approve => NodeOutcome {
                        operative_label: "(gate:human)".to_string(),
                        final_text: "approved".to_string(),
                        success: true,
                        passed: Some(true),
                        fanout: 0,
                    },
                    GateDecision::Reject { reason } => NodeOutcome {
                        operative_label: "(gate:human)".to_string(),
                        final_text: reason,
                        success: false,
                        passed: Some(false),
                        fanout: 0,
                    },
                },
            }
        } else if node.kind == StageKind::Test {
            // Deterministic harness — not an LLM. Pass/fail routes OnPass/OnFail.
            let out = (cfg.run_test)(node.instruction.as_deref().unwrap_or(""));
            NodeOutcome {
                operative_label: "(test)".to_string(),
                final_text: out.summary,
                success: out.passed,
                passed: Some(out.passed),
                fanout: 0,
            }
        } else {
            let stage = run_stage(wf, node, &ctx, cfg).await?;
            cost += stage.cost;
            let passed = node.kind.is_gate_or_check().then_some(stage.success);
            NodeOutcome {
                operative_label: stage.operative_id,
                final_text: stage.final_text,
                success: stage.success,
                passed,
                fanout: stage.fanout,
            }
        };

        steps.push(StepRecord {
            node_id: node.id.clone(),
            kind: node.kind,
            operative_id: outcome.operative_label,
            final_text: outcome.final_text.clone(),
            success: outcome.success,
            passed: outcome.passed,
            fanout: outcome.fanout,
        });
        (cfg.on_step)(steps.last().expect("just pushed"));

        let Some(edge) = next_edge(wf, &current, outcome.passed) else {
            return Ok(WorkflowRun { steps, end: WorkflowEnd::Stuck(current), cost });
        };

        // Per-edge loop cap (decision #3).
        let key = (edge.from.clone(), edge.to.clone(), edge.when);
        let count = traversals.entry(key).or_insert(0);
        *count += 1;
        if let Some(cap) = edge.max_traversals {
            if *count > cap {
                let label = format!("{}->{}", edge.from, edge.to);
                return Ok(WorkflowRun { steps, end: WorkflowEnd::LoopCapReached(label), cost });
            }
        }

        // An OnFail transition carries fix context (the fix-list / rejection reason).
        ctx = FlowContext { text: outcome.final_text, is_fix: edge.when == EdgeWhen::OnFail };
        current = edge.to.clone();
    }

    Ok(WorkflowRun { steps, end: WorkflowEnd::StepsExhausted, cost })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::Faction;
    use crate::orchestrator::engine::{EngineOutcome, EngineRunner};
    use crate::orchestrator::workflow::{WorkflowNode, WORKFLOW_SCHEMA};
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A runner whose success flag follows a scripted schedule of pass/fail per call,
    /// returning its label as the artifact. Used to drive check routing.
    struct Scripted {
        label: String,
        verdicts: Vec<bool>,
        calls: AtomicUsize,
    }
    impl Scripted {
        fn always(label: &str, ok: bool) -> Self {
            Self { label: label.into(), verdicts: vec![ok], calls: AtomicUsize::new(0) }
        }
        fn schedule(label: &str, verdicts: Vec<bool>) -> Self {
            Self { label: label.into(), verdicts, calls: AtomicUsize::new(0) }
        }
    }
    #[async_trait]
    impl EngineRunner for Scripted {
        async fn run(&self, _role: Role, _prompt: &str) -> EngineOutcome {
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            let ok = *self.verdicts.get(n).or_else(|| self.verdicts.last()).unwrap_or(&true);
            EngineOutcome {
                signals: vec![],
                success: ok,
                cost: 1.0,
                final_text: format!("{}#{}", self.label, n),
            }
        }
    }

    /// A pool wiring `cipher` (lead) + `vex` to two scripted runners.
    struct Pool {
        cipher: Scripted,
        vex: Scripted,
    }
    impl AgentPool for Pool {
        fn lead_id(&self) -> String {
            "cipher".into()
        }
        fn runner(&self, id: &str) -> Option<&dyn EngineRunner> {
            match id {
                "cipher" => Some(&self.cipher),
                "vex" => Some(&self.vex),
                _ => None,
            }
        }
        fn ids(&self) -> Vec<String> {
            vec!["cipher".into(), "vex".into()]
        }
        fn faction(&self, _id: &str) -> Faction {
            Faction::Architects
        }
        fn name(&self, id: &str) -> String {
            id.into()
        }
        fn brief(&self) -> String {
            "cipher, vex".into()
        }
    }

    fn standard() -> Workflow {
        Workflow::standard_template("build the thing", "cipher", "vex")
    }

    /// Default gate resolver for tests with no gate (or that want approve).
    fn always_approve(_node: &str, _ctx: &str) -> BoxFuture<'static, GateDecision> {
        Box::pin(async { GateDecision::Approve })
    }

    /// Default test harness for tests with no `Test` stage (or that want green).
    fn green_tests(_harness: &str) -> TestOutcome {
        TestOutcome { passed: true, summary: "all green".into() }
    }

    /// No-op step observer for tests.
    fn ignore_step(_s: &StepRecord) {}

    /// Build an `ExecConfig` with approving gate + green test harness.
    fn cfg<'a>(pool: &'a Pool, max_steps: usize) -> ExecConfig<'a> {
        ExecConfig {
            pool,
            max_steps,
            resolve_gate: &always_approve,
            run_test: &green_tests,
            on_step: &ignore_step,
        }
    }

    fn two(cipher: Scripted, vex: Scripted) -> Pool {
        Pool { cipher, vex }
    }

    #[tokio::test]
    async fn passing_review_reaches_done() {
        // review passes first time → plan, execute, review, done = 3 executed steps
        let pool = two(Scripted::always("cipher", true), Scripted::always("vex", true));
        let run = run_workflow(&standard(), &cfg(&pool, 20)).await.unwrap();
        assert_eq!(run.end, WorkflowEnd::Completed);
        let path: Vec<&str> = run.steps.iter().map(|s| s.node_id.as_str()).collect();
        assert_eq!(path, vec!["plan", "execute", "review"]);
        assert_eq!(run.cost, 3.0);
    }

    #[tokio::test]
    async fn failing_review_loops_back_to_execute_then_passes() {
        // cipher runs plan(ok), review(fail), review(ok); vex executes twice.
        let pool = two(
            Scripted::schedule("cipher", vec![true, false, true]),
            Scripted::always("vex", true),
        );
        let run = run_workflow(&standard(), &cfg(&pool, 20)).await.unwrap();
        assert_eq!(run.end, WorkflowEnd::Completed);
        let path: Vec<&str> = run.steps.iter().map(|s| s.node_id.as_str()).collect();
        // plan → execute → review(fail) → execute → review(pass) → done
        assert_eq!(path, vec!["plan", "execute", "review", "execute", "review"]);
    }

    #[tokio::test]
    async fn fix_context_is_threaded_into_the_retry() {
        // The re-entered execute must see the review's fix-list as FIX CONTEXT.
        struct PromptSpy {
            saw_fix: std::sync::Mutex<bool>,
        }
        #[async_trait]
        impl EngineRunner for PromptSpy {
            async fn run(&self, _role: Role, prompt: &str) -> EngineOutcome {
                if prompt.contains("FIX CONTEXT") {
                    *self.saw_fix.lock().unwrap() = true;
                }
                EngineOutcome { signals: vec![], success: true, cost: 1.0, final_text: "ok".into() }
            }
        }
        let spy = PromptSpy { saw_fix: std::sync::Mutex::new(false) };
        struct P<'a> {
            cipher: Scripted,
            vex: &'a PromptSpy,
        }
        impl AgentPool for P<'_> {
            fn lead_id(&self) -> String { "cipher".into() }
            fn runner(&self, id: &str) -> Option<&dyn EngineRunner> {
                match id { "cipher" => Some(&self.cipher), "vex" => Some(self.vex), _ => None }
            }
            fn ids(&self) -> Vec<String> { vec!["cipher".into(), "vex".into()] }
            fn faction(&self, _: &str) -> Faction { Faction::Architects }
            fn name(&self, id: &str) -> String { id.into() }
            fn brief(&self) -> String { String::new() }
        }
        let pool = P { cipher: Scripted::schedule("cipher", vec![true, false, true]), vex: &spy };
        let exec = ExecConfig { pool: &pool, max_steps: 20, resolve_gate: &always_approve, run_test: &green_tests, on_step: &ignore_step };
        run_workflow(&standard(), &exec).await.unwrap();
        assert!(*spy.saw_fix.lock().unwrap(), "retry execute should receive FIX CONTEXT");
    }

    #[tokio::test]
    async fn perpetually_failing_review_is_bounded_by_max_steps() {
        let pool = two(Scripted::always("cipher", false), Scripted::always("vex", true));
        let run = run_workflow(&standard(), &cfg(&pool, 6)).await.unwrap();
        assert_eq!(run.end, WorkflowEnd::StepsExhausted);
        assert_eq!(run.steps.len(), 6);
    }

    #[tokio::test]
    async fn per_edge_loop_cap_stops_a_runaway_fix_loop() {
        // cap the review--OnFail-->execute edge at 2 traversals.
        let mut wf = standard();
        for e in &mut wf.edges {
            if e.from == "review" && e.when == EdgeWhen::OnFail {
                e.max_traversals = Some(2);
            }
        }
        let pool = two(Scripted::always("cipher", false), Scripted::always("vex", true));
        let run = run_workflow(&wf, &cfg(&pool, 100)).await.unwrap();
        assert_eq!(run.end, WorkflowEnd::LoopCapReached("review->execute".into()));
    }

    #[tokio::test]
    async fn loop_cap_of_one_allows_exactly_one_traversal() {
        // cap review--OnFail-->execute at 1: the edge may be taken once, then blocks.
        let mut wf = standard();
        for e in &mut wf.edges {
            if e.from == "review" && e.when == EdgeWhen::OnFail {
                e.max_traversals = Some(1);
            }
        }
        let pool = two(Scripted::always("cipher", false), Scripted::always("vex", true));
        let run = run_workflow(&wf, &cfg(&pool, 100)).await.unwrap();
        assert_eq!(run.end, WorkflowEnd::LoopCapReached("review->execute".into()));
        // plan → execute → review(fail) → [edge#1] → execute → review(fail) → [edge#2 → cap]
        let path: Vec<&str> = run.steps.iter().map(|s| s.node_id.as_str()).collect();
        assert_eq!(path, vec!["plan", "execute", "review", "execute", "review"]);
    }

    #[tokio::test]
    async fn auto_gate_passes_without_burning_a_call() {
        // plan → gate(auto) → done. The gate must not invoke an operative.
        let wf = Workflow {
            schema: WORKFLOW_SCHEMA.into(),
            root_goal: "g".into(),
            nodes: vec![
                WorkflowNode::new("plan", StageKind::Plan, Some("cipher".into())),
                WorkflowNode { gate_mode: Some(GateMode::Auto), ..WorkflowNode::new("gate", StageKind::Gate, None) },
                WorkflowNode::new("done", StageKind::Done, None),
            ],
            edges: vec![
                WorkflowEdge::new("plan", "gate", EdgeWhen::Always),
                WorkflowEdge::new("gate", "done", EdgeWhen::OnPass),
            ],
        };
        let pool = two(Scripted::always("cipher", true), Scripted::always("vex", true));
        let run = run_workflow(&wf, &cfg(&pool, 20)).await.unwrap();
        assert_eq!(run.end, WorkflowEnd::Completed);
        // cost is 1.0 — only the plan operative ran; the gate is free.
        assert_eq!(run.cost, 1.0);
        let gate = run.steps.iter().find(|s| s.node_id == "gate").unwrap();
        assert_eq!(gate.operative_id, "(gate:auto)");
    }

    #[tokio::test]
    async fn human_gate_reject_routes_onfail_with_reason() {
        // plan → gate(human) → done(OnPass) / execute(OnFail); reject once, approve next.
        let wf = Workflow {
            schema: WORKFLOW_SCHEMA.into(),
            root_goal: "g".into(),
            nodes: vec![
                WorkflowNode::new("plan", StageKind::Plan, Some("cipher".into())),
                WorkflowNode::new("gate", StageKind::Gate, None),
                WorkflowNode::new("execute", StageKind::Execute, Some("vex".into())),
                WorkflowNode::new("done", StageKind::Done, None),
            ],
            edges: vec![
                WorkflowEdge::new("plan", "gate", EdgeWhen::Always),
                WorkflowEdge::new("gate", "done", EdgeWhen::OnPass),
                WorkflowEdge::new("gate", "execute", EdgeWhen::OnFail),
                WorkflowEdge::new("execute", "gate", EdgeWhen::Always),
            ],
        };
        let calls = AtomicUsize::new(0);
        let resolve = |_node: &str, _ctx: &str| -> BoxFuture<'static, GateDecision> {
            // decide synchronously, then hand back a ready future.
            let decision = if calls.fetch_add(1, Ordering::SeqCst) == 0 {
                GateDecision::Reject { reason: "scope too broad".into() }
            } else {
                GateDecision::Approve
            };
            Box::pin(async move { decision })
        };
        let pool = two(Scripted::always("cipher", true), Scripted::always("vex", true));
        let exec = ExecConfig { pool: &pool, max_steps: 20, resolve_gate: &resolve, run_test: &green_tests, on_step: &ignore_step };
        let run = run_workflow(&wf, &exec).await.unwrap();
        assert_eq!(run.end, WorkflowEnd::Completed);
        let path: Vec<&str> = run.steps.iter().map(|s| s.node_id.as_str()).collect();
        // plan → gate(reject) → execute → gate(approve) → done
        assert_eq!(path, vec!["plan", "gate", "execute", "gate"]);
        // the first gate step recorded the rejection reason
        let first_gate = &run.steps[1];
        assert_eq!(first_gate.passed, Some(false));
        assert_eq!(first_gate.final_text, "scope too broad");
    }

    #[tokio::test]
    async fn research_fanout_runs_subs_then_primary_consolidates() {
        // research(primary=cipher, subs=[vex,vex]) → done.
        let wf = Workflow {
            schema: WORKFLOW_SCHEMA.into(),
            root_goal: "survey the options".into(),
            nodes: vec![
                WorkflowNode {
                    sub_operatives: vec!["vex".into(), "vex".into()],
                    ..WorkflowNode::new("research", StageKind::Research, Some("cipher".into()))
                },
                WorkflowNode::new("done", StageKind::Done, None),
            ],
            edges: vec![WorkflowEdge::new("research", "done", EdgeWhen::Always)],
        };
        let pool = two(Scripted::always("cipher", true), Scripted::always("vex", true));
        let run = run_workflow(&wf, &cfg(&pool, 10)).await.unwrap();
        assert_eq!(run.end, WorkflowEnd::Completed);
        let step = &run.steps[0];
        assert_eq!(step.fanout, 2);
        // 2 sub-operatives (vex×2) + 1 primary (cipher) = 3 calls.
        assert_eq!(run.cost, 3.0);
    }

    #[tokio::test]
    async fn fanout_consolidation_prompt_carries_parallel_findings() {
        struct Spy {
            saw_findings: std::sync::Mutex<bool>,
        }
        #[async_trait]
        impl EngineRunner for Spy {
            async fn run(&self, _role: Role, prompt: &str) -> EngineOutcome {
                if prompt.contains("PARALLEL FINDINGS TO CONSOLIDATE") {
                    *self.saw_findings.lock().unwrap() = true;
                }
                EngineOutcome { signals: vec![], success: true, cost: 1.0, final_text: "note".into() }
            }
        }
        let spy = Spy { saw_findings: std::sync::Mutex::new(false) };
        struct P<'a> {
            primary: &'a Spy,
            worker: Scripted,
        }
        impl AgentPool for P<'_> {
            fn lead_id(&self) -> String { "primary".into() }
            fn runner(&self, id: &str) -> Option<&dyn EngineRunner> {
                match id { "primary" => Some(self.primary), "worker" => Some(&self.worker), _ => None }
            }
            fn ids(&self) -> Vec<String> { vec!["primary".into(), "worker".into()] }
            fn faction(&self, _: &str) -> Faction { Faction::Architects }
            fn name(&self, id: &str) -> String { id.into() }
            fn brief(&self) -> String { String::new() }
        }
        let pool = P { primary: &spy, worker: Scripted::always("worker", true) };
        let wf = Workflow {
            schema: WORKFLOW_SCHEMA.into(),
            root_goal: "g".into(),
            nodes: vec![
                WorkflowNode {
                    sub_operatives: vec!["worker".into()],
                    ..WorkflowNode::new("research", StageKind::Research, Some("primary".into()))
                },
                WorkflowNode::new("done", StageKind::Done, None),
            ],
            edges: vec![WorkflowEdge::new("research", "done", EdgeWhen::Always)],
        };
        let exec = ExecConfig { pool: &pool, max_steps: 10, resolve_gate: &always_approve, run_test: &green_tests, on_step: &ignore_step };
        run_workflow(&wf, &exec).await.unwrap();
        assert!(*spy.saw_findings.lock().unwrap(), "primary should see PARALLEL FINDINGS");
    }

    #[tokio::test]
    async fn fanout_unknown_sub_operative_is_an_error() {
        let wf = Workflow {
            schema: WORKFLOW_SCHEMA.into(),
            root_goal: "g".into(),
            nodes: vec![
                WorkflowNode {
                    sub_operatives: vec!["ghost".into()],
                    ..WorkflowNode::new("research", StageKind::Research, Some("cipher".into()))
                },
                WorkflowNode::new("done", StageKind::Done, None),
            ],
            edges: vec![WorkflowEdge::new("research", "done", EdgeWhen::Always)],
        };
        let pool = two(Scripted::always("cipher", true), Scripted::always("vex", true));
        let err = run_workflow(&wf, &cfg(&pool, 10)).await.unwrap_err();
        assert_eq!(
            err,
            WorkflowRunError::UnknownOperative { node: "research".into(), operative: "ghost".into() }
        );
    }

    #[tokio::test]
    async fn test_stage_runs_deterministic_harness_no_operative() {
        // plan → test(green) → done. The test stage must not invoke an operative.
        let wf = Workflow {
            schema: WORKFLOW_SCHEMA.into(),
            root_goal: "g".into(),
            nodes: vec![
                WorkflowNode::new("plan", StageKind::Plan, Some("cipher".into())),
                WorkflowNode::new("test", StageKind::Test, None),
                WorkflowNode::new("done", StageKind::Done, None),
            ],
            edges: vec![
                WorkflowEdge::new("plan", "test", EdgeWhen::Always),
                WorkflowEdge::new("test", "done", EdgeWhen::OnPass),
            ],
        };
        let pool = two(Scripted::always("cipher", true), Scripted::always("vex", true));
        let run = run_workflow(&wf, &cfg(&pool, 20)).await.unwrap();
        assert_eq!(run.end, WorkflowEnd::Completed);
        // cost is 1.0 — only plan ran; the deterministic harness is free of CLI cost.
        assert_eq!(run.cost, 1.0);
        let test = run.steps.iter().find(|s| s.node_id == "test").unwrap();
        assert_eq!(test.operative_id, "(test)");
        assert_eq!(test.passed, Some(true));
    }

    #[tokio::test]
    async fn failing_test_loops_to_execute_with_failures_as_fix_context() {
        // plan → execute → test → done(OnPass)/execute(OnFail). Test reds once, greens next.
        let wf = Workflow {
            schema: WORKFLOW_SCHEMA.into(),
            root_goal: "g".into(),
            nodes: vec![
                WorkflowNode::new("plan", StageKind::Plan, Some("cipher".into())),
                WorkflowNode::new("execute", StageKind::Execute, Some("vex".into())),
                WorkflowNode::new("test", StageKind::Test, None),
                WorkflowNode::new("done", StageKind::Done, None),
            ],
            edges: vec![
                WorkflowEdge::new("plan", "execute", EdgeWhen::Always),
                WorkflowEdge::new("execute", "test", EdgeWhen::Always),
                WorkflowEdge::new("test", "done", EdgeWhen::OnPass),
                WorkflowEdge::new("test", "execute", EdgeWhen::OnFail),
            ],
        };
        let runs = AtomicUsize::new(0);
        let harness = |_h: &str| {
            if runs.fetch_add(1, Ordering::SeqCst) == 0 {
                TestOutcome { passed: false, summary: "2 tests failed: foo, bar".into() }
            } else {
                TestOutcome { passed: true, summary: "all green".into() }
            }
        };
        // a forger that records whether it ever saw the test failures as fix context.
        let saw_fix = std::sync::Arc::new(std::sync::Mutex::new(false));
        struct Forger {
            saw_fix: std::sync::Arc<std::sync::Mutex<bool>>,
        }
        #[async_trait]
        impl EngineRunner for Forger {
            async fn run(&self, _role: Role, prompt: &str) -> EngineOutcome {
                if prompt.contains("FIX CONTEXT") && prompt.contains("2 tests failed") {
                    *self.saw_fix.lock().unwrap() = true;
                }
                EngineOutcome { signals: vec![], success: true, cost: 1.0, final_text: "impl".into() }
            }
        }
        struct P {
            cipher: Scripted,
            vex: Forger,
        }
        impl AgentPool for P {
            fn lead_id(&self) -> String { "cipher".into() }
            fn runner(&self, id: &str) -> Option<&dyn EngineRunner> {
                match id { "cipher" => Some(&self.cipher), "vex" => Some(&self.vex), _ => None }
            }
            fn ids(&self) -> Vec<String> { vec!["cipher".into(), "vex".into()] }
            fn faction(&self, _: &str) -> Faction { Faction::Architects }
            fn name(&self, id: &str) -> String { id.into() }
            fn brief(&self) -> String { String::new() }
        }
        let pool = P { cipher: Scripted::always("cipher", true), vex: Forger { saw_fix: saw_fix.clone() } };
        let exec = ExecConfig { pool: &pool, max_steps: 20, resolve_gate: &always_approve, run_test: &harness, on_step: &ignore_step };
        let run = run_workflow(&wf, &exec).await.unwrap();
        assert_eq!(run.end, WorkflowEnd::Completed);
        let path: Vec<&str> = run.steps.iter().map(|s| s.node_id.as_str()).collect();
        // plan → execute → test(red) → execute → test(green) → done
        assert_eq!(path, vec!["plan", "execute", "test", "execute", "test"]);
        assert!(*saw_fix.lock().unwrap(), "retried execute should see the test failures as fix context");
    }

    #[tokio::test]
    async fn unknown_operative_is_an_error() {
        let mut wf = standard();
        wf.nodes[0].operative_id = Some("ghost".into());
        let pool = two(Scripted::always("cipher", true), Scripted::always("vex", true));
        let err = run_workflow(&wf, &cfg(&pool, 10)).await.unwrap_err();
        assert_eq!(
            err,
            WorkflowRunError::UnknownOperative { node: "plan".into(), operative: "ghost".into() }
        );
    }

    #[tokio::test]
    async fn invalid_workflow_is_rejected_before_running() {
        let wf = Workflow {
            schema: "workflow.v1".into(),
            root_goal: "g".into(),
            nodes: vec![],
            edges: vec![],
        };
        let pool = two(Scripted::always("cipher", true), Scripted::always("vex", true));
        let err = run_workflow(&wf, &cfg(&pool, 10)).await.unwrap_err();
        assert!(matches!(err, WorkflowRunError::Invalid(_)));
    }

    #[test]
    fn role_mapping_is_stable() {
        assert_eq!(role_for(StageKind::Plan), Role::Plan);
        assert_eq!(role_for(StageKind::Review), Role::Judge);
        assert_eq!(role_for(StageKind::Execute), Role::Execute);
        assert_eq!(role_for(StageKind::Research), Role::Plan);
    }
}
