//! The live agent pool — the composition seam that wires the concrete CLI/HTTP
//! runners (`wagner_edge_host::cli`) to the loop's `AgentPool` port
//! (`wagner_edge_host::orchestrator`).
//!
//! It lives in the shell crate, not in the host, on purpose: it depends on
//! `wagner_edge_host::cli`, and `cli` already depends on `orchestrator` (it
//! implements `EngineRunner`). Placing it in `orchestrator` would create an
//! `orchestrator → cli → orchestrator` cycle. As a composition module above both
//! layers, it keeps the dependency DAG acyclic.

use wagner_edge_host::cli::{CliEngineRunner, GateConfig};
use wagner_edge_host::events::Faction;
use wagner_edge_host::orchestrator::roster::{Engine, Roster};
use wagner_edge_host::orchestrator::run_loop::AgentPool;
use wagner_edge_host::orchestrator::EngineRunner;
use std::collections::HashMap;

/// The live agent pool: one [`CliEngineRunner`] per hired agent, plus the roster
/// metadata the loop needs (lead, faction, name, brief). Built per run.
pub struct CliAgentPool {
    roster: Roster,
    runners: HashMap<String, Box<dyn EngineRunner>>,
    lead_id: String,
}

impl CliAgentPool {
    /// Build a runner for each agent: Claude agents get the US2 gate; local
    /// `Endpoint` agents get an HTTP runner; every agent gets its skill prompt
    /// prepended. The lead is the roster's first Claude.
    pub fn build(roster: &Roster, cwd: &std::path::Path, gate: &GateConfig) -> Self {
        let mut runners: HashMap<String, Box<dyn EngineRunner>> = HashMap::new();
        for a in &roster.agents {
            let runner: Box<dyn EngineRunner> = match &a.engine {
                Engine::Claude => {
                    let mut r = CliEngineRunner::claude(cwd).with_gate(gate.clone());
                    if let Some(sp) = &a.skill_prompt {
                        r = r.with_role_prompt(sp.clone());
                    }
                    Box::new(r)
                }
                Engine::Codex => {
                    let mut r = CliEngineRunner::codex(cwd);
                    if let Some(sp) = &a.skill_prompt {
                        r = r.with_role_prompt(sp.clone());
                    }
                    Box::new(r)
                }
                Engine::Endpoint { base_url, model } => {
                    let mut r = wagner_edge_host::cli::EndpointRunner::new(base_url.clone(), model.clone());
                    if let Some(sp) = &a.skill_prompt {
                        r = r.with_role_prompt(sp.clone());
                    }
                    Box::new(r)
                }
            };
            runners.insert(a.id.clone(), runner);
        }
        // `start_run` calls `roster.validate()` before building the pool, which
        // guarantees a lead-capable (Claude) agent exists.
        let lead_id = roster
            .lead()
            .expect("roster validated before build: must have a lead")
            .id
            .clone();
        CliAgentPool {
            roster: roster.clone(),
            runners,
            lead_id,
        }
    }
}

impl AgentPool for CliAgentPool {
    fn lead_id(&self) -> String {
        self.lead_id.clone()
    }
    fn runner(&self, agent_id: &str) -> Option<&dyn EngineRunner> {
        self.runners.get(agent_id).map(|r| r.as_ref())
    }
    fn ids(&self) -> Vec<String> {
        self.roster.ids()
    }
    fn faction(&self, agent_id: &str) -> Faction {
        self.roster
            .get(agent_id)
            .map(|a| a.engine.faction())
            .unwrap_or(Faction::Architects)
    }
    fn name(&self, agent_id: &str) -> String {
        self.roster
            .get(agent_id)
            .map(|a| a.name.clone())
            .unwrap_or_else(|| agent_id.to_string())
    }
    fn brief(&self) -> String {
        self.roster
            .agents
            .iter()
            .map(|a| {
                let engine = match &a.engine {
                    Engine::Claude => "claude".to_string(),
                    Engine::Codex => "codex".to_string(),
                    Engine::Endpoint { model, .. } => format!("local:{model}"),
                };
                format!("- {} ({}) — {} [{}]", a.id, a.name, a.role, engine)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}
