//! Tauri command surface (T035) — what the frontend invokes, and the event
//! channels it listens on (`wagner://event|run|transmission`).

use wagner_edge_host::cli::CliStatus;
use wagner_edge_host::orchestrator::roster::{Agent, Roster};
use wagner_edge_host::orchestrator::run_loop::{run_goal, LoopDeps};
use wagner_edge_host::orchestrator::{
    builtin_templates, run_workflow, ExecConfig, GateDecision, NamedTemplate, StepRecord,
    TestOutcome, Workflow,
};
use wagner_edge_host::memory::{MemoryInput, MemoryRecord, MemoryStore};
use wagner_edge_host::bus::{Event, RunEvent, UiEvent, VoiceEvent};
use wagner_edge_host::voice::stt::Stt;
use crate::bus_gateway::UiGateway;
use crate::pool::CliAgentPool;
use crate::voice_lifecycle::SidecarState;
use wagner_edge_host::state::{ConsoleInput, Guardrails, Run};
use wagner_edge_host::transmissions::TransmissionRegistry;
use serde::Deserialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, State};

use wagner_edge_host::schema::WORKFLOW_STEP_EVENT_SCHEMA;

/// Default node-execution ceiling for a composed workflow when the engineer sets
/// no iteration cap (fix-loops are additionally bounded by their per-edge caps).
const DEFAULT_MAX_WORKFLOW_STEPS: usize = 200;

// ── Vault graph DTOs ────────────────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultNodeDto {
    pub uid: String,
    pub title: String,
    pub tier: String,
    pub lifecycle: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultEdgeDto {
    pub source_uid: String,
    pub target_uid: String,
    pub rel_type: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct VaultGraphDto {
    pub nodes: Vec<VaultNodeDto>,
    pub edges: Vec<VaultEdgeDto>,
}

/// Return the vault knowledge graph for the selected project.
#[tauri::command]
pub async fn vault_graph(
    store: State<'_, MemoryStore>,
    project_dir: String,
) -> Result<VaultGraphDto, String> {
    let graph = store.vault_graph(&project_dir).await.map_err(|e| e.to_string())?;
    Ok(VaultGraphDto {
        nodes: graph
            .nodes
            .into_iter()
            .map(|n| VaultNodeDto { uid: n.uid, title: n.title, tier: n.tier, lifecycle: n.lifecycle })
            .collect(),
        edges: graph
            .edges
            .into_iter()
            .map(|e| VaultEdgeDto { source_uid: e.source_uid, target_uid: e.target_uid, rel_type: e.rel_type })
            .collect(),
    })
}

/// Which run ids to abort given an optional target: `Some(id)` → just that id (if
/// live); `None` → every live run (the single-run UI sends no id today).
fn abort_targets(live: &[String], target: Option<&str>) -> Vec<String> {
    match target {
        Some(id) => live.iter().filter(|r| r.as_str() == id).cloned().collect(),
        None => live.to_vec(),
    }
}

/// Reset a reloaded run to a live state before re-entering the loop: a resumed
/// session runs again, and any prior guardrail halt is cleared.
fn prepare_resumed(mut run: Run) -> Run {
    run.status = wagner_edge_host::state::RunStatus::Running;
    run.halt_reason = None;
    run
}

/// Append a goal to a session's goal thread and make it the current primary goal
/// (the planner reads `goal`; `goals` keeps the history). ponytail: latest goal
/// wins; per-goal independent completion would need a status per goal entry.
fn append_goal(mut run: Run, text: String, now: String) -> Run {
    run.goals.push(text.clone());
    run.goal = text;
    run.updated_at = now;
    run
}

/// Everything `spawn_run_loop` needs to drive one session's goal loop. Grouped
/// into a struct so both `start_run` (fresh) and `resume_run` (reloaded) build it
/// the same way (and to keep the spawn signature off the clippy too-many-args path).
struct SpawnLoop {
    gateway: UiGateway,
    run: Run,
    roster: Roster,
    cwd: std::path::PathBuf,
    runs_root: std::path::PathBuf,
    gate_config: wagner_edge_host::cli::GateConfig,
    suite_command: Option<String>,
    blocked_halt: Arc<AtomicBool>,
    console: Arc<Mutex<Vec<ConsoleInput>>>,
}

/// Build the goal-loop future for one session (014 US1). The registry spawns it
/// and hands it the cancel receiver so `run_goal` can interrupt the in-flight turn
/// on abort (FR-013). The permission-gate server is torn down when the loop ends.
fn build_run_future(
    s: SpawnLoop,
    gate_server: tauri::async_runtime::JoinHandle<()>,
    cancel_rx: tokio::sync::watch::Receiver<bool>,
) -> impl std::future::Future<Output = ()> + Send + 'static {
    let SpawnLoop {
        gateway,
        run,
        roster,
        cwd,
        runs_root,
        gate_config,
        suite_command,
        blocked_halt,
        console,
    } = s;
    let run_id = run.run_id.clone();
    let gateway_for_loop = gateway;
    let console_for_loop = console;
    let suite_for_loop = suite_command;
    let blocked_for_loop = blocked_halt;
    let suite_cwd = cwd.clone();
    async move {
        // Build one CLI runner per hired agent: Claude agents get the US2 gate
        // and their skill prompt; Codex agents get theirs. The pool routes the
        // loop's plan/judge to the lead and each subtask to its assigned agent.
        let pool = CliAgentPool::build(&roster, &cwd, &gate_config);
        // Emit sides now publish typed events to the bus; the UiGateway re-emits
        // the legacy wagner://* Tauri channels (011 P2).
        let g_emit = gateway_for_loop.clone();
        let rid_emit = run_id.clone();
        let emit = move |ev: wagner_edge_host::events::WagnerEvent| {
            g_emit.publish_run(&rid_emit, Event::Run(RunEvent::Activity(Box::new(ev))));
        };
        // The suite shell-out is blocking and arbitrarily long; offload it to a
        // blocking thread so it never starves the async runtime (M4).
        let suite = move || -> futures::future::BoxFuture<'static, wagner_edge_host::orchestrator::judge::SuiteResult> {
            let cmd = suite_for_loop.clone();
            let cwd = suite_cwd.clone();
            Box::pin(async move {
                match tokio::task::spawn_blocking(move || {
                    crate::suite::run_suite(cmd.as_deref(), &cwd)
                })
                .await
                {
                    Ok(result) => result,
                    Err(e) => {
                        // A JoinError means the suite thread PANICKED — distinct from
                        // a normal red suite. Log it so a broken runner environment
                        // isn't silently treated as "tests failed" and re-iterated.
                        eprintln!("[wagner] suite runner panicked: {e}");
                        wagner_edge_host::orchestrator::judge::SuiteResult { passed: false }
                    }
                }
            })
        };
        // Drain steering instructions submitted since the last iteration (US3).
        let steer = move || std::mem::take(&mut *console_for_loop.lock().unwrap());
        // Promote a gate blocked-timeout to a whole-run halt (T042).
        let external_halt = move || {
            blocked_for_loop
                .load(Ordering::SeqCst)
                .then_some(wagner_edge_host::state::HaltReason::BlockedTimeout)
        };
        // Live snapshot each phase/iteration → mission bar updates in real time.
        let g_progress = gateway_for_loop.clone();
        let rid_progress = run_id.clone();
        let progress = move |r: &wagner_edge_host::state::Run| {
            g_progress.publish_run(&rid_progress, Event::Run(RunEvent::Snapshot(Box::new(r.clone()))));
        };
        // An agent-authored UI-spec panel → the inspector's "AGENT VIEW" (P5).
        let g_panel = gateway_for_loop.clone();
        let rid_panel = run_id.clone();
        let emit_panel = move |operative_id: &str, spec: serde_json::Value| {
            g_panel.publish_run(
                &rid_panel,
                Event::Ui(UiEvent::Panel { operative_id: operative_id.to_string(), spec }),
            );
        };

        let final_run = run_goal(
            run,
            LoopDeps {
                pool: &pool,
                run_suite: &suite,
                runs_root: &runs_root,
                emit: &emit,
                steer: &steer,
                external_halt: &external_halt,
                progress: &progress,
                emit_panel: &emit_panel,
                cancel: Some(cancel_rx),
            },
        )
        .await;
        // On cancel `run_goal` returns a MINIMAL Aborted run (blank goal/iteration);
        // the registry and the shell's `abort` already publish the authoritative
        // terminal snapshot, so publishing this one would race and could overwrite
        // the full state with blanks. Only the loop's own terminal verdicts
        // (Met/Halted) are published here.
        if final_run.status != wagner_edge_host::state::RunStatus::Aborted {
            gateway_for_loop.publish_run(&run_id, Event::Run(RunEvent::Snapshot(Box::new(final_run))));
        }
        // Tear down the permission-gate server with the run (replaces RunControl).
        gate_server.abort();
    }
}

/// Register a run on the AgentRegistry (014 US1) — the single authority that
/// replaces the shell's RunManager. start / resume / add_goal all funnel through
/// here. The steer callback pushes live instructions into the shared console the
/// loop drains each iteration (US3); abort routes via `registry.cancel` (FR-003).
fn register_run(
    registry: &Arc<wagner_edge_host::bus::AgentRegistry>,
    run_id: String,
    spawn_loop: SpawnLoop,
    gate_server: tauri::async_runtime::JoinHandle<()>,
) -> Result<(), String> {
    let console = spawn_loop.console.clone();
    registry
        .spawn_run(
            run_id,
            move |cancel_rx| build_run_future(spawn_loop, gate_server, cancel_rx),
            move |text: String| {
                console.lock().unwrap().push(ConsoleInput {
                    ts: chrono::Utc::now().to_rfc3339(),
                    text,
                });
            },
        )
        .map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
pub struct GuardrailConfig {
    /// `None` = no iteration cap (run until goal-met).
    #[serde(default)]
    pub max_iterations: Option<u32>,
    pub blocked_timeout_secs: u32,
    pub cost_budget: Option<f64>,
    #[serde(default)]
    pub suite_command: Option<String>,
}

impl Default for GuardrailConfig {
    /// Mirrors `Guardrails::defaults()` (R-GUARDRAILS): unbounded iterations, a
    /// 30-minute blocked timeout, no cost cap, no suite (the repo's CLAUDE.md /
    /// AGENTS.md declares how it tests). Used when the UI omits guardrails.
    fn default() -> Self {
        Self {
            max_iterations: None,
            blocked_timeout_secs: 30 * 60,
            cost_budget: None,
            suite_command: None,
        }
    }
}

/// Detect CLI availability + API-key env (EC-004, SC-002).
#[tauri::command]
pub fn preflight() -> CliStatus {
    wagner_edge_host::cli::detect_system()
}

/// Ping a local-model endpoint (vLLM / Ollama / any OpenAI-compatible server):
/// is it reachable, and what model(s) does it advertise? Used by the roster
/// editor to confirm a local harness before a run. Never errors — an unreachable
/// endpoint comes back `reachable: false` with the error string.
#[tauri::command]
pub async fn ping_endpoint(base_url: String) -> wagner_edge_host::cli::EndpointStatus {
    wagner_edge_host::cli::ping_endpoint(&base_url).await
}

/// The catalog of operative identities the engineer can hire — parsed from the
/// selected project's `.claude/agents/*.md` and `agents/*.md` (FR-007), with a
/// built-in fallback. A blank dir resolves to the app's cwd. Never errors on a
/// missing dir; it just returns the fallback catalog.
#[tauri::command]
pub fn agent_catalog(project_dir: String) -> Vec<wagner_edge_host::orchestrator::AgentIdentity> {
    let dir = resolve_project_dir(
        &project_dir,
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
    )
    .unwrap_or_else(|_| std::path::PathBuf::from("."));
    wagner_edge_host::orchestrator::scan_catalog(&dir)
}

/// The catalog of discoverable skills — parsed from the selected project's skill
/// directories (`.claude/skills`, `skills`, `.agents/skills`), plus the user's
/// global dirs (`~/.claude/skills`, `~/.codex/skills`), and installed plugins
/// (`~/.claude/plugins/**/skills`). De-duped by id; repo wins. Never errors.
#[tauri::command]
pub fn skill_catalog(project_dir: String) -> Vec<wagner_edge_host::orchestrator::SkillRef> {
    let dir = resolve_project_dir(
        &project_dir,
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
    )
    .unwrap_or_else(|_| std::path::PathBuf::from("."));
    wagner_edge_host::orchestrator::scan_skills(&dir)
}

/// Does `project_dir` resolve to an existing directory? Lets the Composer block
/// LAUNCH on a bad path up front instead of failing the run after launch. A blank
/// path is `false` (a dir must be chosen). `~/` is expanded like the run path.
#[tauri::command]
pub fn validate_project_dir(project_dir: String) -> bool {
    !project_dir.trim().is_empty()
        && resolve_project_dir(&project_dir, std::path::PathBuf::from("/")).is_ok()
}

/// Start an autonomous run. Spawns the goal loop on a background task that emits
/// `wagner://event` + `wagner://run` as it progresses. Returns the run id.
// Tauri commands receive each request field as a parameter; grouping them into a
// struct would only obscure the invoke contract the frontend depends on.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn start_run(
    app: AppHandle,
    registry: State<'_, Arc<wagner_edge_host::bus::AgentRegistry>>,
    reg: State<'_, Arc<TransmissionRegistry>>,
    store: State<'_, MemoryStore>,
    gateway: State<'_, UiGateway>,
    goal: String,
    docs: Vec<String>,
    guardrails: Option<GuardrailConfig>,
    project_dir: String,
    roster: Option<Vec<Agent>>,
) -> Result<String, String> {
    if goal.trim().is_empty() {
        return Err("goal must not be empty".into());
    }
    // The redesigned entry screen sends only a folder + goal; guardrails default
    // to R-GUARDRAILS when omitted (acceptance E9).
    let guardrails = guardrails.unwrap_or_default();
    // The hired-agent roster the run deploys. A blank/absent roster falls back to
    // the default two-agent org (Cipher/Vex). Validated before the run starts.
    let roster = match roster {
        Some(agents) if !agents.is_empty() => Roster { agents },
        _ => Roster::default_roster(),
    };
    roster.validate().map_err(|e| e.to_string())?;
    launch_run_core(
        &app,
        registry.inner(),
        reg.inner(),
        store.inner(),
        gateway.inner(),
        goal,
        docs,
        guardrails,
        project_dir,
        roster,
    )
    .await
}

/// The shared run-launch path (ADR-0004): resolve the workspace, fold recall into
/// the goal, build the `Run`, start the permission gate, and register the goal loop
/// on the `AgentRegistry`. Both the `start_run` command and the engine's
/// `RunLaunch` adapter ([`ShellRunLaunch`]) funnel through here so the UI path and
/// the bus/voice path never drift. `guardrails`/`roster` are already defaulted +
/// validated by the caller.
#[allow(clippy::too_many_arguments)]
async fn launch_run_core(
    app: &AppHandle,
    registry: &Arc<wagner_edge_host::bus::AgentRegistry>,
    reg: &Arc<TransmissionRegistry>,
    store: &MemoryStore,
    gateway: &UiGateway,
    goal: String,
    docs: Vec<String>,
    guardrails: GuardrailConfig,
    project_dir: String,
    roster: Roster,
) -> Result<String, String> {
    // Resolve the project directory the operatives run in — this is what makes
    // their per-project `claude`/`codex` settings (`.claude/`, AGENTS.md, MCP
    // servers) apply. Falls back to the app's cwd when left blank.
    let project_cwd = resolve_project_dir(
        &project_dir,
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
    )?;
    let run_id = ulid::Ulid::new().to_string();
    // Normalized timestamp (always `…Z`, no sub-seconds) so `created_at` sorts
    // lexicographically, matching save_memory/save_workflow_template.
    let created_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    // Recall loop (read side): fold prior learnings for this project into the goal
    // so the run's operatives see them — symmetric with `start_workflow`.
    let goal = match store.recall_block(&project_dir, 8).await {
        Some(block) => format!("{goal}\n\n{block}"),
        None => goal,
    };

    let mut run = Run::new(run_id.clone(), goal, docs, created_at);
    run.guardrails = Guardrails {
        max_iterations: guardrails.max_iterations,
        iterations_used: 0,
        blocked_timeout_secs: guardrails.blocked_timeout_secs,
        cost: wagner_edge_host::state::CostBudget {
            mode: wagner_edge_host::state::CostMode::CliUsage,
            budget: guardrails.cost_budget,
            used: 0.0,
        },
    };
    // Session fields: persist the resolved project dir so a closed session can be
    // resumed (the pool is rebuilt against this cwd); derive a rail label from the
    // folder name; updated_at starts at created_at and advances on each save.
    run.project_dir = project_cwd.to_string_lossy().into_owned();
    run.name = project_cwd
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| run_id.clone());
    run.updated_at = run.created_at.clone();

    let console = Arc::new(Mutex::new(Vec::<ConsoleInput>::new()));
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let runs_root = app_data.join("runs");
    let suite_command = guardrails.suite_command.clone();

    // US2 permission gate: start the loopback server, write the MCP gate script,
    // and hand Claude a `--permission-prompt-tool` that routes here.
    // A blocked-too-long transmission flips this flag; the loop reads it each
    // iteration and promotes the stall to a whole-run halt (T042/FR-016).
    let blocked_halt = Arc::new(AtomicBool::new(false));
    let gate = crate::gate::start_gate_server(
        gateway.clone(),
        &app_data,
        reg.clone(),
        u64::from(run.guardrails.blocked_timeout_secs),
        blocked_halt.clone(),
    )
    .await?;

    register_run(
        registry,
        run_id.clone(),
        SpawnLoop {
            gateway: gateway.clone(),
            run,
            roster,
            cwd: project_cwd.clone(),
            runs_root,
            gate_config: gate.config.clone(),
            suite_command,
            blocked_halt,
            console,
        },
        gate.server_task,
    )?;
    Ok(run_id)
}

/// Shell adapter for the engine's `RunLaunch` port (ADR-0004). Lets a bus-dispatched
/// `RunCommand` — from voice intake, or any future client/transport — start, steer,
/// or abort a run through the *same* `launch_run_core`/registry path the UI uses.
pub struct ShellRunLaunch {
    app: AppHandle,
}

impl ShellRunLaunch {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

#[async_trait::async_trait]
impl wagner_edge_host::orchestrator::RunLaunch for ShellRunLaunch {
    async fn launch(&self, goal: String) -> Result<String, String> {
        let registry = self.app.state::<Arc<wagner_edge_host::bus::AgentRegistry>>();
        let reg = self.app.state::<Arc<TransmissionRegistry>>();
        let store = self.app.state::<MemoryStore>();
        let gateway = self.app.state::<UiGateway>();
        // ponytail: a voice/bus-launched run has no folder picker — default to the
        // app cwd (resolve_project_dir's blank fallback) + the default roster.
        // Carry an explicit project_dir here once a client provides one (multi-run UX).
        launch_run_core(
            &self.app,
            registry.inner(),
            reg.inner(),
            store.inner(),
            gateway.inner(),
            goal,
            vec![],
            GuardrailConfig::default(),
            String::new(),
            Roster::default_roster(),
        )
        .await
    }

    async fn steer(&self, run_id: String, text: String) -> Result<(), String> {
        self.app
            .state::<Arc<wagner_edge_host::bus::AgentRegistry>>()
            .steer(&run_id, text);
        Ok(())
    }

    async fn abort(&self, run_id: Option<String>) -> Result<(), String> {
        let registry = self.app.state::<Arc<wagner_edge_host::bus::AgentRegistry>>();
        match run_id {
            Some(id) => {
                registry.cancel(&id);
            }
            // `None` = the single live session (the UI/voice send no id) → every live run.
            None => {
                for id in registry.running_runs() {
                    registry.cancel(&id);
                }
            }
        }
        Ok(())
    }
}

/// Resume a persisted session (acceptance E6): load its state, rebuild the gate
/// and the agent pool against the persisted `project_dir`, and re-enter the goal
/// loop — sharing `spawn_run_loop` with `start_run` so the two never drift.
/// ponytail: the hired roster and suite command aren't persisted on `Run`, so a
/// resumed session uses the default roster and no suite. Persist them on `Run`
/// to restore custom rosters when that matters.
#[tauri::command]
pub async fn resume_run(
    app: AppHandle,
    registry: State<'_, Arc<wagner_edge_host::bus::AgentRegistry>>,
    reg: State<'_, Arc<TransmissionRegistry>>,
    gateway: State<'_, UiGateway>,
    run_id: String,
) -> Result<(), String> {
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let runs_root = app_data.join("runs");
    let run = prepare_resumed(
        wagner_edge_host::state::load(&runs_root, &run_id).map_err(|e| e.to_string())?,
    );

    // Rebuild the run cwd from the persisted project dir (falls back to the app's
    // cwd for legacy runs that never stored one).
    let project_cwd = resolve_project_dir(
        &run.project_dir,
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
    )?;
    let roster = Roster::default_roster();
    let console = Arc::new(Mutex::new(Vec::<ConsoleInput>::new()));

    let blocked_halt = Arc::new(AtomicBool::new(false));
    let gate = crate::gate::start_gate_server(
        gateway.inner().clone(),
        &app_data,
        reg.inner().clone(),
        u64::from(run.guardrails.blocked_timeout_secs),
        blocked_halt.clone(),
    )
    .await?;

    register_run(
        registry.inner(),
        run_id,
        SpawnLoop {
            gateway: gateway.inner().clone(),
            run,
            roster,
            cwd: project_cwd,
            runs_root,
            gate_config: gate.config.clone(),
            suite_command: None,
            blocked_halt,
            console,
        },
        gate.server_task,
    )?;
    Ok(())
}

/// Add a goal to a session over its lifetime (acceptance E8). For a **live**
/// session, inject the goal into the running loop via its steering console (the
/// planner folds it into the next iteration). For a **closed** (paused/met)
/// session, persist the goal to the goal thread, then resume — reactivating it.
#[tauri::command]
pub async fn add_goal(
    app: AppHandle,
    registry: State<'_, Arc<wagner_edge_host::bus::AgentRegistry>>,
    reg: State<'_, Arc<TransmissionRegistry>>,
    gateway: State<'_, UiGateway>,
    run_id: String,
    text: String,
) -> Result<(), String> {
    if text.trim().is_empty() {
        return Err("goal must not be empty".into());
    }
    // Live session: inject the goal via the registry's steering path (the loop
    // folds it into the next iteration). Closed session: resume to reactivate it.
    if registry.is_running(&run_id) {
        registry.steer(&run_id, format!("New goal: {}", text.trim()));
        return Ok(());
    }
    // Closed session: persist the appended goal, then resume to reactivate it.
    let runs_root = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("runs");
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let run = append_goal(
        wagner_edge_host::state::load(&runs_root, &run_id).map_err(|e| e.to_string())?,
        text.trim().to_string(),
        now,
    );
    wagner_edge_host::state::save(&runs_root, &run).map_err(|e| e.to_string())?;
    resume_run(app, registry, reg, gateway, run_id).await
}

/// The workflow templates for the builder's picker (decision #4): the built-in
/// starters plus any the engineer has saved. A saved template shadows a built-in of
/// the same name (the engineer's version wins).
#[tauri::command]
pub async fn list_workflow_templates(
    store: State<'_, MemoryStore>,
) -> Result<Vec<NamedTemplate>, String> {
    let roster = Roster::default_roster();
    let lead = roster.lead().map(|a| a.id.clone()).unwrap_or_else(|| "cipher".into());
    let forger = roster
        .agents
        .iter()
        .find(|a| a.id != lead)
        .map(|a| a.id.clone())
        .unwrap_or_else(|| "vex".into());

    let mut out = builtin_templates(&lead, &forger);
    // Append saved templates; a saved name replaces the built-in of that name.
    for t in store.list_templates().await.map_err(|e| e.to_string())? {
        if let Ok(workflow) = serde_json::from_str::<Workflow>(&t.content) {
            let named = NamedTemplate { name: t.name, description: t.description, workflow };
            if let Some(slot) = out.iter_mut().find(|b| b.name == named.name) {
                *slot = named;
            } else {
                out.push(named);
            }
        }
    }
    Ok(out)
}

/// Save the current builder graph as a reusable named template (decision #4).
#[tauri::command]
pub async fn save_workflow_template(
    store: State<'_, MemoryStore>,
    name: String,
    description: String,
    workflow: Workflow,
) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("template name must not be empty".into());
    }
    let content = serde_json::to_value(&workflow).map_err(|e| e.to_string())?;
    // Normalized form (always `…Z`, no sub-seconds) so `ORDER BY created_at` sorts
    // chronologically as a plain lexicographic string compare.
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    store
        .save_template(name.trim(), description.trim(), &content, Vec::new(), &now)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Persist a learning and project it to git-diffable Markdown under the project's
/// `.wagner/memory/`. `project_dir` doubles as the multi-tenant `project_id`.
#[tauri::command]
pub async fn save_memory(
    store: State<'_, MemoryStore>,
    project_dir: String,
    text: String,
    tags: Vec<String>,
    source_type: Option<String>,
    source_ref: Option<String>,
) -> Result<MemoryRecord, String> {
    if text.trim().is_empty() {
        return Err("memory text must not be empty".into());
    }
    // Normalized form (always `…Z`, no sub-seconds) so `ORDER BY created_at` sorts
    // chronologically as a plain lexicographic string compare.
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let rec = store
        .save_memory(
            MemoryInput {
                project_id: project_dir.clone(),
                text: text.trim().to_string(),
                tags,
                source_type,
                source_ref,
            },
            &now,
        )
        .await
        .map_err(|e| e.to_string())?;
    // Best-effort Markdown projection — never fail the save on an FS hiccup. The
    // write itself lives on the store, not in this command handler.
    if let Ok(dir) = resolve_project_dir(&project_dir, std::path::PathBuf::from(".")) {
        store.write_markdown_projection(&dir, &rec);
    }
    Ok(rec)
}

/// Recall recent learnings for a project (newest first) — the recall loop's read side.
#[tauri::command]
pub async fn recall_memory(
    store: State<'_, MemoryStore>,
    project_dir: String,
    tag: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<MemoryRecord>, String> {
    store
        .recall(&project_dir, tag.as_deref(), limit.unwrap_or(20))
        .await
        .map_err(|e| e.to_string())
}

/// Validate an engineer-authored workflow graph (the same structural rules the
/// builder enforces client-side). `Ok(())` means it is launchable.
#[tauri::command]
pub fn validate_workflow(workflow: Workflow) -> Result<(), String> {
    workflow.validate().map_err(|e| e.to_string())
}

/// Launch a composed workflow (Phase E). Walks the engineer's graph over the hired
/// roster on a background task, emitting `wagner://workflow` per stage and a final
/// snapshot. Human `Gate` stages open a `wagner://transmission` and block on the
/// engineer's answer (resolved via [`answer_transmission`]). Returns the run id.
// Tauri commands receive each request field as a parameter (+ injected State); a
// param struct would obscure the invoke contract the frontend depends on.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn start_workflow(
    app: AppHandle,
    registry: State<'_, Arc<wagner_edge_host::bus::AgentRegistry>>,
    reg: State<'_, Arc<TransmissionRegistry>>,
    store: State<'_, MemoryStore>,
    gateway: State<'_, UiGateway>,
    mut workflow: Workflow,
    guardrails: GuardrailConfig,
    project_dir: String,
    roster: Option<Vec<Agent>>,
) -> Result<String, String> {
    workflow.validate().map_err(|e| e.to_string())?;
    if workflow.root_goal.trim().is_empty() {
        return Err("workflow root_goal must not be empty".into());
    }
    // Recall loop (read side): fold prior learnings for this project into the root
    // goal so every stage's operative sees them — symmetric with `start_run`.
    if let Some(block) = store.recall_block(&project_dir, 8).await {
        workflow.root_goal = format!("{}\n\n{block}", workflow.root_goal);
    }
    let roster = match roster {
        Some(agents) if !agents.is_empty() => Roster { agents },
        _ => Roster::default_roster(),
    };
    roster.validate().map_err(|e| e.to_string())?;
    let project_cwd = resolve_project_dir(
        &project_dir,
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
    )?;

    let run_id = ulid::Ulid::new().to_string();
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let suite_command = guardrails.suite_command.clone();
    // Each node is one step; reuse the iteration cap as the step ceiling (fix-loops
    // are additionally bounded by their per-edge caps). Generous default.
    let max_steps = guardrails
        .max_iterations
        .map(|n| n as usize)
        .unwrap_or(DEFAULT_MAX_WORKFLOW_STEPS);

    // Claude operatives still need the US2 per-tool permission gate.
    let blocked_halt = Arc::new(AtomicBool::new(false));
    let gate = crate::gate::start_gate_server(
        gateway.inner().clone(),
        &app_data,
        reg.inner().clone(),
        u64::from(guardrails.blocked_timeout_secs),
        blocked_halt.clone(),
    )
    .await?;

    let gateway_for_task = gateway.inner().clone();
    let gate_config = gate.config.clone();
    let reg_for_gate = reg.inner().clone();
    let cwd = project_cwd.clone();
    let suite_cwd = project_cwd.clone();
    let run_id_for_task = run_id.clone();
    let gate_server = gate.server_task;

    registry
        .inner()
        .spawn_run(
            run_id.clone(),
            move |mut cancel_rx| async move {
                let workflow_body = async {
        let pool = CliAgentPool::build(&roster, &cwd, &gate_config);

        // Stage-level human gate: open a transmission, publish it, await the answer.
        let gateway_for_gate = gateway_for_task.clone();
        let resolve_gate = move |node_id: &str, artifact: &str| -> futures::future::BoxFuture<'static, GateDecision> {
            let reg = reg_for_gate.clone();
            let gateway = gateway_for_gate.clone();
            let node = node_id.to_string();
            let art = artifact.to_string();
            Box::pin(async move {
                let id = ulid::Ulid::new().to_string();
                let rx = reg.open(&id);
                // Human gate: deliver synchronously + reliably (emit_now), not via
                // the bus fan-out — the workflow is blocked on the engineer's answer.
                gateway.emit_now(
                    "wagner://transmission",
                    &serde_json::json!({
                        "schema": "transmission.v1",
                        "id": id,
                        "subtask_id": node,
                        "kind": "gate",
                        "prompt": format!("Approve stage '{node}'?"),
                        "artifact": art,
                        "options": [
                            {"id": "allow", "label": "Approve"},
                            {"id": "deny", "label": "Reject"}
                        ],
                        "raised_at": chrono::Utc::now().to_rfc3339(),
                        "state": "open"
                    }),
                );
                match rx.await {
                    Ok(wagner_edge_host::transmissions::Decision::Allow) => GateDecision::Approve,
                    _ => GateDecision::Reject {
                        reason: format!("engineer rejected stage '{node}'"),
                    },
                }
            })
        };

        // Deterministic Test harness — the configured suite command, shelled out.
        // `run_test` is sync (the executor's port); offload the blocking shell-out
        // onto a blocking thread so it doesn't stall the workflow executor (M4).
        // This closure only ever runs on the app's multi-thread runtime.
        let run_test = move |_harness: &str| {
            let cmd = suite_command.clone();
            let cwd = suite_cwd.clone();
            let r = tokio::task::block_in_place(move || {
                crate::suite::run_suite(cmd.as_deref(), &cwd)
            });
            TestOutcome {
                passed: r.passed,
                summary: if r.passed { "suite passed".into() } else { "suite failed".into() },
            }
        };

        // Per-stage event → the builder highlights the active node + shows artifacts.
        let gateway_for_step = gateway_for_task.clone();
        let run_id_for_step = run_id_for_task.clone();
        let on_step = move |s: &StepRecord| {
            let payload = serde_json::json!({
                "run_id": run_id_for_step,
                "node_id": s.node_id,
                "kind": s.kind,
                "operative_id": s.operative_id,
                "success": s.success,
                "passed": s.passed,
                "fanout": s.fanout,
                "final_text": s.final_text,
            });
            // Validate the Rust→TS contract for this channel before emitting, so a
            // shape drift is caught here rather than silently in the frontend.
            if let Err(e) = wagner_edge_host::schema::validate(
                WORKFLOW_STEP_EVENT_SCHEMA,
                &payload,
            ) {
                eprintln!("[wagner-edge] workflow-step event failed schema validation: {e}");
            }
            gateway_for_step.publish_run(&run_id_for_step, Event::Run(RunEvent::WorkflowStep(payload)));
        };

        let result = run_workflow(
            &workflow,
            &ExecConfig {
                pool: &pool,
                max_steps,
                resolve_gate: &resolve_gate,
                run_test: &run_test,
                on_step: &on_step,
            },
        )
        .await;

        // Final snapshot: outcome + the full step log.
        let done = serde_json::json!({
            "run_id": run_id_for_task.clone(),
            "end": format!("{:?}", result.as_ref().map(|r| &r.end)),
            "cost": result.as_ref().map(|r| r.cost).unwrap_or(0.0),
            "error": result.as_ref().err().map(|e| e.to_string()),
            "steps": result.as_ref().ok().map(|r| {
                r.steps.iter().map(|s| serde_json::json!({
                    "node_id": s.node_id, "kind": s.kind, "operative_id": s.operative_id,
                    "success": s.success, "passed": s.passed, "fanout": s.fanout,
                })).collect::<Vec<_>>()
            }),
        });
        gateway_for_task.publish_run(&run_id_for_task, Event::Run(RunEvent::WorkflowDone(done)));
                };
                // Interrupt the workflow on abort (FR-013), then tear down the gate.
                tokio::select! {
                    // `wait_for` resolves when cancel flips to true; an Err (all
                    // senders dropped — the registry tore the run down) is ALSO a
                    // cancel. Either way this arm wins and `workflow_body` is dropped.
                    _ = async { let _ = cancel_rx.wait_for(|c| *c).await; } => {}
                    _ = workflow_body => {}
                }
                gate_server.abort();
            },
            |_text: String| {}, // workflows do not steer
        )
        .map_err(|e| e.to_string())?;
    Ok(run_id)
}

/// Inject a steering instruction into the in-flight run (US3). Recorded now;
/// the loop drains the queue each iteration.
#[tauri::command]
pub fn steer(
    registry: State<'_, Arc<wagner_edge_host::bus::AgentRegistry>>,
    run_id: Option<String>,
    text: String,
) -> Result<(), String> {
    // Target the named session; with no id, target the sole live run (the
    // single-session UI sends none). Refuse to guess when several are live.
    let target = match run_id {
        Some(id) => {
            if !registry.is_running(&id) {
                return Err(format!("no run with id {id}"));
            }
            id
        }
        None => {
            let live = registry.running_runs();
            match live.len() {
                1 => live.into_iter().next().unwrap(),
                0 => return Err("no active run".into()),
                _ => return Err("multiple active runs; specify run_id".into()),
            }
        }
    };
    registry.steer(&target, text);
    Ok(())
}

/// Answer an open transmission (US2): resolve the pending permission request so
/// the waiting MCP permission tool returns allow/deny to Claude. Returns an error
/// if the transmission id is unknown (already answered / timed out).
#[tauri::command]
pub fn answer_transmission(
    reg: State<'_, Arc<TransmissionRegistry>>,
    id: String,
    response: String,
) -> Result<(), String> {
    let decision = wagner_edge_host::transmissions::Decision::from_answer(&response);
    if reg.answer(&id, decision) {
        Ok(())
    } else {
        Err(format!("no open transmission with id {id}"))
    }
}

/// Flip a run to its terminal `Aborted` state. The killed loop task never reaches
/// its own terminal emit, so without this the console + rail would stay "running"
/// forever after Abort (B3 — operator saw "nothing happens").
fn aborted(mut run: Run) -> Run {
    run.status = wagner_edge_host::state::RunStatus::Aborted;
    run.phase = wagner_edge_host::state::RunPhase::Halted;
    run
}

/// Abort a run (FR-017): terminate its loop task; CLI children are killed on
/// drop (`kill_on_drop`). `run_id = Some(id)` aborts that session only (others
/// keep running); `None` aborts every live run (the single-session UI default).
/// After killing the task we persist + emit the `Aborted` run so the UI leaves
/// the running state (the loop task's own terminal emit never fires once aborted).
#[tauri::command]
pub fn abort(
    app: AppHandle,
    registry: State<'_, Arc<wagner_edge_host::bus::AgentRegistry>>,
    gateway: State<'_, UiGateway>,
    run_id: Option<String>,
) -> Result<(), String> {
    // 011 P4 (ADR-0004): bus-dispatched aborts (e.g. voice spoken-cancel) now route
    // through the `RunCommandRouter` → `registry.cancel`. The UI abort acts inline
    // below directly (no bus dispatch) so a run is never cancelled twice. A run must
    // always be stoppable, so this path never depends on the bounded intake.
    let runs_root = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("runs");
    let live = registry.running_runs();
    for id in abort_targets(&live, run_id.as_deref()) {
        // Deliver the abort directly to the registry — bypasses the bounded intake
        // so a saturated queue can never leave a run un-abortable (challenge C2).
        // cancel interrupts the loop's in-flight turn (FR-013) and publishes the
        // terminal Aborted snapshot (FR-006); the run future tears down its gate
        // server on cancel. Only persist the aborted terminal when the run was
        // actually live — a run that finished naturally between the snapshot above
        // and now must not be mislabeled Aborted on disk.
        if registry.cancel(&id) {
            match wagner_edge_host::state::load(&runs_root, &id) {
                Ok(run) => {
                    let run = aborted(run);
                    if let Err(e) = wagner_edge_host::state::save(&runs_root, &run) {
                        eprintln!("[wagner] abort: state save failed for run {id}: {e}");
                    }
                    // The full, persisted aborted run (goal + iteration) is the last
                    // word so the rail leaves "running" with correct detail.
                    gateway.publish_run(&id, Event::Run(RunEvent::Snapshot(Box::new(run))));
                }
                Err(e) => {
                    eprintln!("[wagner] abort: no persisted state for run {id} ({e}) — bus snapshot only");
                }
            }
        }
    }
    Ok(())
}

/// List persisted sessions newest-first for the session rail — reads
/// `{app_data}/runs/*/state.json`. Corrupt/legacy run dirs are skipped; an
/// absent runs dir (no sessions yet) yields an empty list.
#[tauri::command]
pub fn list_runs(app: AppHandle) -> Result<Vec<wagner_edge_host::state::RunSummary>, String> {
    let runs_root = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("runs");
    wagner_edge_host::state::list_summaries(&runs_root).map_err(|e| e.to_string())
}

/// A tiered-retrieval hit, flattened for the frontend (Plan 004 step 8).
#[derive(serde::Serialize)]
pub struct TieredResultDto {
    pub uid: String,
    pub summary: String,
    pub snippet: String,
    pub tier_kind: String,
}

fn tiered_dto(hit: &wagner_edge_host::memory::TieredResult) -> TieredResultDto {
    let rec = hit.record();
    TieredResultDto {
        uid: rec.uid.clone(),
        summary: rec.summary.clone(),
        snippet: rec.text.chars().take(160).collect(),
        tier_kind: hit.tier_kind().to_string(),
    }
}

/// Tiered vault retrieval for a project — summary → section → full → related.
#[tauri::command]
pub async fn vault_summary(
    store: State<'_, MemoryStore>,
    project_dir: String,
    query: String,
) -> Result<Vec<TieredResultDto>, String> {
    let hits = store
        .tiered_query(wagner_edge_host::memory::TieredQuery {
            project_id: &project_dir,
            terms: &query,
            limit: 20,
        })
        .await
        .map_err(|e| e.to_string())?;
    Ok(hits.iter().map(tiered_dto).collect())
}

/// Approve a staged vault note (move it from `_staging/` to the curated dir).
#[tauri::command]
pub async fn approve_staging(
    store: State<'_, MemoryStore>,
    project_dir: String,
    uid: String,
) -> Result<(), String> {
    let dir = resolve_project_dir(&project_dir, std::path::PathBuf::from("."))?;
    store
        .approve_staging_note(&uid, &dir)
        .await
        .map_err(|e| e.to_string())
}

/// The uids of vault notes awaiting approval in `_staging/`.
#[tauri::command]
pub fn list_staging(store: State<'_, MemoryStore>, project_dir: String) -> Result<Vec<String>, String> {
    let dir = resolve_project_dir(&project_dir, std::path::PathBuf::from("."))?;
    Ok(store.list_staging(&dir))
}

/// Load one persisted run's full state (reopening a session from the rail).
#[tauri::command]
pub fn get_run(app: AppHandle, run_id: String) -> Result<Run, String> {
    let runs_root = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("runs");
    wagner_edge_host::state::load(&runs_root, &run_id).map_err(|e| e.to_string())
}

// ── Voice IPC ───────────────────────────────────────────────────────────────

/// Wire shape the UI lane depends on: `{ enabled, ready }`.
/// Mirrors `VoiceStatus` from the host but lives here so the shell can add
/// serde attributes without touching the Tauri-free host.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VoiceStatusDto {
    pub enabled: bool,
    pub ready: bool,
}

impl From<wagner_edge_host::voice::VoiceStatus> for VoiceStatusDto {
    fn from(s: wagner_edge_host::voice::VoiceStatus) -> Self {
        Self {
            enabled: s.enabled,
            ready: s.ready,
        }
    }
}

/// Per-model group state returned by `voice_models_status`.
///
/// Each field is the lowercase state string: `"absent"` | `"ready"`.
/// Matches the `state` values emitted on `wagner://voice-download`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VoiceModelsDto {
    pub stt: String,
    pub tts: String,
}

/// Resolve the models directory under the app-data dir.
fn models_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|d| d.join("models"))
        .map_err(|e| format!("could not resolve app-data dir: {e}"))
}

/// Return the current on-disk state of the voice model files.
///
/// IPC contract: `voice_models_status() -> { stt: string, tts: string }`
/// where each string is one of: `"absent"` | `"ready"`.
#[tauri::command]
pub fn voice_models_status(app: AppHandle) -> Result<VoiceModelsDto, String> {
    let dir = models_dir(&app)?;
    let s = wagner_edge_host::voice::models_status(&dir);
    Ok(VoiceModelsDto { stt: s.stt, tts: s.tts })
}

/// Download all voice model files into the app-data models dir, emitting
/// progress on the `wagner://voice-download` Tauri event channel.
///
/// IPC contract: `voice_download_models() -> void`
///
/// Event payload shape (matches the UI lane contract):
/// ```json
/// { "model": "stt"|"tts_model"|"tts_voices",
///   "state": "downloading"|"verifying"|"ready"|"failed",
///   "received": u64,
///   "total": u64 }
/// ```
#[tauri::command]
pub async fn voice_download_models(
    app: AppHandle,
    gateway: State<'_, UiGateway>,
) -> Result<(), String> {
    let dir = models_dir(&app)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("could not create models dir: {e}"))?;

    // B: limit redirects to 3 and reject non-HTTPS URLs so a compromised CDN
    // cannot redirect the download to an unencrypted or attacker-controlled URL.
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(3))
        .https_only(true)
        .build()
        .map_err(|e| format!("could not build HTTP client: {e}"))?;

    let gateway_for_emit = gateway.inner().clone();
    wagner_edge_host::voice::download_models(&dir, &client, move |progress| {
        gateway_for_emit.publish_workspace(
            "voice",
            Event::Voice(VoiceEvent::DownloadProgress(Box::new(progress))),
        );
    })
    .await
    .map_err(|e| e.to_string())
}

/// Return the current voice-feature state.
///
/// IPC contract: `voice_status() -> { enabled: bool, ready: bool }`
#[tauri::command]
pub fn voice_status(
    vm: State<'_, Arc<wagner_edge_host::voice::VoiceManager>>,
) -> VoiceStatusDto {
    vm.status().into()
}

/// Push-to-talk capture state (015 US1, M2a). Holds the in-flight mic capture
/// between the `voice_ptt_start`/`voice_ptt_stop` IPC calls, plus the STT adapter
/// that transcribes the held utterance via the whisper sidecar (:8771).
pub struct PttState {
    capture: Mutex<Option<wagner_edge_host::voice::capture::MicCapture>>,
    stt: wagner_edge_host::voice::HttpStt,
}

impl PttState {
    pub fn new() -> Self {
        Self {
            capture: Mutex::new(None),
            stt: wagner_edge_host::voice::HttpStt::new("http://127.0.0.1:8771"),
        }
    }
}

impl Default for PttState {
    fn default() -> Self {
        Self::new()
    }
}

/// Begin a push-to-talk capture (button/key down): open the mic and accumulate
/// until `voice_ptt_stop`. Errors if voice is disabled or the mic is unavailable
/// (the typed mic error is also surfaced via `VoiceStatus`).
#[tauri::command]
pub async fn voice_ptt_start(
    ptt: State<'_, PttState>,
    vm: State<'_, Arc<wagner_edge_host::voice::VoiceManager>>,
) -> Result<(), String> {
    if !vm.enabled() {
        return Err("voice is disabled — enable it in Voice settings first".into());
    }
    let mic = wagner_edge_host::voice::capture::MicCapture::start().map_err(|e| {
        vm.report_error(&e);
        e.to_string()
    })?;
    *ptt.capture.lock().unwrap() = Some(mic);
    Ok(())
}

/// End a push-to-talk capture (button/key up): stop the mic, transcribe the held
/// utterance via the whisper sidecar, and return the recognised text. M2a proves
/// capture + STT on-device; M2b will publish the transcript onto the bus so it
/// starts/steers a run.
#[tauri::command]
pub async fn voice_ptt_stop(
    ptt: State<'_, PttState>,
    vm: State<'_, Arc<wagner_edge_host::voice::VoiceManager>>,
    registry: State<'_, Arc<wagner_edge_host::bus::AgentRegistry>>,
) -> Result<String, String> {
    // Take the capture out from under the lock before the await (no guard held
    // across the STT round-trip).
    let mic = ptt
        .capture
        .lock()
        .unwrap()
        .take()
        .ok_or("no capture in progress")?;
    let audio = mic.stop();
    let transcript = ptt.stt.transcribe(audio).await.map_err(|e| {
        vm.report_error(&e);
        e.to_string()
    })?;
    let text = transcript.text.trim().to_string();
    // Publish the utterance onto the bus so VoiceIntake routes it (start / steer /
    // spoken-cancel) → RunCommandRouter → run. Also returned for the UI to display.
    if !text.is_empty() {
        use wagner_edge_host::bus::{NodeId, ParticipantId, ParticipantKind, StreamId};
        let ctx = registry.context(ParticipantId {
            node: NodeId("local".into()),
            kind: ParticipantKind::Agent,
            name: "voice-capture".into(),
            instance: ulid::Ulid::new(),
        });
        ctx.publish(
            StreamId::Workspace("voice".into()),
            Event::Voice(VoiceEvent::UtteranceTranscribed { text: text.clone() }),
        );
    }
    Ok(text)
}

/// Enable or disable the voice feature.
///
/// IPC contract: `voice_set_enabled(on: bool) -> { enabled: bool, ready: bool }`
///
/// When `on = true` the shell spawns the STT and TTS sidecars (passing the
/// app-data model paths as CLI arguments), waits for each `/health` endpoint,
/// and marks the manager `ready`. Returns an error containing
/// `"models not ready"` when any model file is absent — the UI should call
/// `voice_download_models` first.
///
/// When `on = false` the sidecars are killed and `ready` is cleared.
///
/// Idempotent: enabling when already up returns the current status without
/// re-spawning. Spawn / health-wait failure returns a typed `Err(String)` and
/// leaves `enabled = false` (Article III).
///
/// R3: `sc.op_lock` is held for the full body so that two concurrent
/// `voice_set_enabled(true)` calls cannot both pass the idempotency guard and
/// double-spawn. The re-check after lock acquisition is intentional.
#[tauri::command]
pub async fn voice_set_enabled(
    on: bool,
    app: AppHandle,
    vm: State<'_, Arc<wagner_edge_host::voice::VoiceManager>>,
    sc: State<'_, SidecarState>,
) -> Result<VoiceStatusDto, String> {
    // Serialise enable/disable so concurrent calls can't both spawn (R3).
    let _op = sc.op_lock.lock().await;

    if on {
        // Re-check idempotency under the lock (the status may have changed
        // while we were waiting for op_lock).
        if vm.status().enabled && vm.status().ready {
            return Ok(vm.status().into());
        }

        // Adopt sidecars already serving on :8771/:8772 (started out-of-band by
        // `make voice-up` / `make run`) instead of spawning our own. This makes
        // voice work on the dev path — where the Tauri-bundled binaries are absent
        // — and avoids a double-spawn port clash (B1). We did not start them, so
        // toggling voice off won't kill them; the dev script owns their lifecycle.
        if crate::voice_lifecycle::sidecars_healthy().await {
            vm.set_enabled(true);
            vm.set_ready(true);
            return Ok(vm.status().into());
        }

        // Gate: all model files must be present before spawning sidecars.
        let dir = models_dir(&app)?;
        if !wagner_edge_host::voice::all_models_ready(&dir) {
            return Err("models not ready — open Voice settings to download them".into());
        }

        // Mark enabled now so the UI reflects "starting" even before ready.
        vm.set_enabled(true);

        // Resolve model paths from the app-data models dir.
        let paths = crate::voice_lifecycle::ModelPaths::from_dir(&dir);

        // Spawn sidecars and wait for health.
        match crate::voice_lifecycle::spawn_sidecars(&app, &sc, &paths).await {
            Ok(()) => {
                vm.set_ready(true);
            }
            Err(e) => {
                // R2: children were pushed into sc before health-wait; kill them
                // now so no orphaned processes keep 8771/8772 bound.
                crate::voice_lifecycle::kill_sidecars(&sc);
                // Hard-fail: revert enabled, surface the error.
                vm.set_enabled(false);
                return Err(e);
            }
        }
    } else {
        // Kill sidecars; VoiceManager::set_enabled(false) also clears ready.
        crate::voice_lifecycle::kill_sidecars(&sc);
        vm.set_enabled(false);
    }
    Ok(vm.status().into())
}

// ── Resolve project directory ────────────────────────────────────────────────

/// Resolve the engineer-selected project directory into an existing directory
/// the operatives run in. A blank selection falls back to `fallback` (the app's
/// cwd). A leading `~` expands to `$HOME`. Errors if the path is not an existing
/// directory — the run must not start against a path that doesn't exist.
fn resolve_project_dir(
    input: &str,
    fallback: std::path::PathBuf,
) -> Result<std::path::PathBuf, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(fallback);
    }
    let expanded = if let Some(rest) = trimmed.strip_prefix("~/") {
        match std::env::var_os("HOME") {
            Some(home) => std::path::PathBuf::from(home).join(rest),
            None => std::path::PathBuf::from(trimmed),
        }
    } else {
        std::path::PathBuf::from(trimmed)
    };
    if !expanded.is_dir() {
        return Err(format!(
            "project directory does not exist or is not a directory: {}",
            expanded.display()
        ));
    }
    // Canonicalize so the CLIs get a stable absolute cwd.
    expanded
        .canonicalize()
        .map_err(|e| format!("could not resolve project directory: {e}"))
}

#[cfg(test)]
mod tests {
    use super::{
        abort_targets, append_goal, prepare_resumed, resolve_project_dir, validate_project_dir,
    };
    use std::path::PathBuf;

    #[test]
    fn append_goal_adds_to_thread_and_becomes_primary() {
        use wagner_edge_host::state::Run;
        let run = Run::new(
            "01J0GOAL00000000000000001".into(),
            "first goal".into(),
            vec![],
            "2026-06-17T00:00:00Z".into(),
        );
        let updated = append_goal(run, "second goal".into(), "2026-06-17T05:00:00Z".into());
        assert_eq!(
            updated.goals,
            vec!["first goal".to_string(), "second goal".to_string()]
        );
        // The planner reads `goal` (the current primary) — it's the new one.
        assert_eq!(updated.goal, "second goal");
        assert_eq!(updated.updated_at, "2026-06-17T05:00:00Z");
    }

    #[test]
    fn prepare_resumed_sets_running_and_clears_halt() {
        use wagner_edge_host::state::{HaltReason, Run, RunStatus};
        let mut run = Run::new(
            "01J0RESUME0000000000000001".into(),
            "resume me".into(),
            vec![],
            "2026-06-17T00:00:00Z".into(),
        );
        run.status = RunStatus::Paused;
        run.halt_reason = Some(HaltReason::BlockedTimeout);
        let resumed = prepare_resumed(run);
        assert_eq!(resumed.status, RunStatus::Running);
        assert_eq!(resumed.halt_reason, None);
    }

    #[test]
    fn tiered_dto_carries_kind_and_fields() {
        use super::tiered_dto;
        use wagner_edge_host::memory::{MemoryRecord, TieredResult};
        let rec = MemoryRecord {
            uid: "01X".into(),
            summary: "short".into(),
            text: "a longer body of text".into(),
            ..Default::default()
        };
        let dto = tiered_dto(&TieredResult::Related(rec.clone()));
        assert_eq!(dto.uid, "01X");
        assert_eq!(dto.tier_kind, "related");
        assert_eq!(dto.summary, "short");
        let dto2 = tiered_dto(&TieredResult::Summary(rec));
        assert_eq!(dto2.tier_kind, "summary");
    }

    #[test]
    fn guardrail_config_default_matches_r_guardrails() {
        // Acceptance E9: omitted guardrails fall back to unbounded iterations,
        // a 30-min blocked timeout, and no cost cap / suite.
        let g = super::GuardrailConfig::default();
        assert_eq!(g.max_iterations, None);
        assert_eq!(g.blocked_timeout_secs, 30 * 60);
        assert_eq!(g.cost_budget, None);
        assert_eq!(g.suite_command, None);
    }

    #[test]
    fn abort_targets_selects_one_or_all() {
        let live = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        // Some(id) → just that run (acceptance E4: aborting one leaves others).
        assert_eq!(abort_targets(&live, Some("b")), vec!["b".to_string()]);
        // An unknown id aborts nothing.
        assert_eq!(abort_targets(&live, Some("zzz")), Vec::<String>::new());
        // None → every live run (single-session UI default).
        assert_eq!(abort_targets(&live, None), live);
    }

    #[test]
    fn aborted_marks_run_terminal() {
        use wagner_edge_host::state::{Run, RunPhase, RunStatus};
        let mut run = Run::new("r1".into(), "goal".into(), vec![], "2026-06-17T00:00:00Z".into());
        run.status = RunStatus::Running;
        run.phase = RunPhase::Dispatching;
        let out = super::aborted(run);
        assert_eq!(out.status, RunStatus::Aborted);
        assert_eq!(out.phase, RunPhase::Halted);
    }

    #[test]
    fn validate_project_dir_gates_blank_and_missing() {
        assert!(!validate_project_dir("   ".into()), "blank is not a valid dir");
        assert!(
            !validate_project_dir("/no/such/wagner/dir/xyz".into()),
            "missing dir is invalid"
        );
        assert!(
            validate_project_dir(std::env::temp_dir().to_string_lossy().into()),
            "an existing dir is valid"
        );
    }

    #[test]
    fn blank_input_falls_back_verbatim() {
        let fallback = std::env::temp_dir();
        assert_eq!(
            resolve_project_dir("   ", fallback.clone()).unwrap(),
            fallback
        );
    }

    #[test]
    fn existing_dir_is_resolved() {
        let dir = std::env::temp_dir();
        let resolved = resolve_project_dir(&dir.to_string_lossy(), PathBuf::from(".")).unwrap();
        assert!(resolved.is_dir());
    }

    #[test]
    fn nonexistent_dir_errors() {
        let err = resolve_project_dir("/no/such/wagner/dir/xyz", PathBuf::from("."))
            .expect_err("a missing directory must error, not silently fall back");
        assert!(err.contains("does not exist"));
    }

    #[test]
    fn a_file_is_not_a_valid_project_dir() {
        let file = std::env::temp_dir().join(format!("wagner-pd-{}", std::process::id()));
        std::fs::write(&file, "x").unwrap();
        assert!(resolve_project_dir(&file.to_string_lossy(), PathBuf::from(".")).is_err());
        let _ = std::fs::remove_file(&file);
    }
}
