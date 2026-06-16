//! The hired-agent roster (paperclip-style orchestration, lightweight).
//!
//! Instead of two hardcoded factions, a run deploys a configurable roster of
//! named agents. Each agent is bound to an execution engine (`claude`/`codex`),
//! an optional skill prompt (prepended to everything it's asked to do), and an
//! optional per-agent cost budget. The Oracle assigns each subtask to an agent
//! by id; the loop dispatches to that agent's engine.
//!
//! This is the orchestration *data model* only — pure, no I/O. The CLI runners
//! are built per agent in `ipc::commands`; the loop consumes them via `AgentPool`.

use crate::events::Faction;
use serde::{Deserialize, Serialize};

/// The execution backend an agent runs on.
///
/// `Claude`/`Codex` spawn the engineer's subscription CLIs (never metered APIs,
/// SC-002). `Endpoint` calls an OpenAI-compatible HTTP chat endpoint — a local
/// or self-hosted model (vLLM / Ollama / any): the engineer supplies the URL and
/// the model name. Local models are not metered, so SC-002 is untouched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Engine {
    Claude,
    Codex,
    /// An OpenAI-compatible chat endpoint: `POST {base_url}/chat/completions`.
    Endpoint {
        base_url: String,
        model: String,
    },
}

impl Engine {
    /// The floor faction (visual grouping/colour) an engine maps to. Claude
    /// agents are Architects; Codex + local-endpoint agents are Forgers.
    pub fn faction(&self) -> Faction {
        match self {
            Engine::Claude => Faction::Architects,
            Engine::Codex | Engine::Endpoint { .. } => Faction::Forgers,
        }
    }

    /// Whether this engine can plan/judge. Planning + judging are reasoning
    /// passes — Claude only (FR-003/013). Local endpoints execute, never lead.
    pub fn can_lead(&self) -> bool {
        matches!(self, Engine::Claude)
    }
}

/// One hired agent in the roster.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Agent {
    /// Stable, unique handle the Oracle assigns work to (e.g. `cipher`).
    pub id: String,
    /// Display name shown on the floor.
    pub name: String,
    /// Free-text role (e.g. "test author", "implementer").
    pub role: String,
    pub engine: Engine,
    /// Prepended to every prompt this agent is given — its standing instructions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill_prompt: Option<String>,
    /// Optional per-agent cost ceiling (same unit as the run cost budget).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<f64>,
    /// The catalog identity this agent was seeded from (an `AgentIdentity.id`),
    /// when hired from the repo's agent catalog rather than typed by hand.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_ref: Option<String>,
}

/// The deployed roster for a run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Roster {
    pub agents: Vec<Agent>,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum RosterError {
    #[error("roster must have at least one agent")]
    Empty,
    #[error("duplicate agent id: {0}")]
    DuplicateId(String),
    #[error("roster needs at least one Claude agent to plan and judge")]
    NoLead,
}

impl Roster {
    /// Validate the roster: non-empty, unique ids, at least one lead-capable
    /// (Claude) agent to run the plan/judge passes.
    pub fn validate(&self) -> Result<(), RosterError> {
        if self.agents.is_empty() {
            return Err(RosterError::Empty);
        }
        let mut seen = std::collections::HashSet::new();
        for a in &self.agents {
            if !seen.insert(a.id.as_str()) {
                return Err(RosterError::DuplicateId(a.id.clone()));
            }
        }
        if !self.agents.iter().any(|a| a.engine.can_lead()) {
            return Err(RosterError::NoLead);
        }
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&Agent> {
        self.agents.iter().find(|a| a.id == id)
    }

    pub fn ids(&self) -> Vec<String> {
        self.agents.iter().map(|a| a.id.clone()).collect()
    }

    /// The agent that runs the plan + judge passes — the first lead-capable
    /// (Claude) agent. The roster is guaranteed to have one after `validate`.
    pub fn lead(&self) -> Option<&Agent> {
        self.agents.iter().find(|a| a.engine.can_lead())
    }

    /// The default starter roster — a two-agent org preserving the original
    /// Architect/Forger relay, but now named and configurable.
    pub fn default_roster() -> Self {
        Roster {
            agents: vec![
                Agent {
                    id: "cipher".into(),
                    name: "Cipher".into(),
                    role: "Architect — planning, tests, judgement".into(),
                    engine: Engine::Claude,
                    skill_prompt: None,
                    budget: None,
                    identity_ref: None,
                },
                Agent {
                    id: "vex".into(),
                    name: "Vex".into(),
                    role: "Forger — scoped implementation".into(),
                    engine: Engine::Codex,
                    skill_prompt: None,
                    budget: None,
                    identity_ref: None,
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(id: &str, engine: Engine) -> Agent {
        Agent {
            id: id.into(),
            name: id.into(),
            role: "x".into(),
            engine,
            skill_prompt: None,
            budget: None,
            identity_ref: None,
        }
    }

    #[test]
    fn default_roster_is_valid_with_a_lead_and_a_forger() {
        let r = Roster::default_roster();
        r.validate().expect("default roster must validate");
        assert!(r.agents.iter().any(|a| a.engine == Engine::Claude));
        assert!(r.agents.iter().any(|a| a.engine == Engine::Codex));
        assert_eq!(r.lead().unwrap().engine, Engine::Claude);
    }

    #[test]
    fn empty_roster_is_rejected() {
        assert_eq!(
            Roster { agents: vec![] }.validate(),
            Err(RosterError::Empty)
        );
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let r = Roster {
            agents: vec![agent("a", Engine::Claude), agent("a", Engine::Codex)],
        };
        assert_eq!(r.validate(), Err(RosterError::DuplicateId("a".into())));
    }

    #[test]
    fn a_roster_with_no_claude_cannot_lead() {
        let r = Roster {
            agents: vec![agent("only", Engine::Codex)],
        };
        assert_eq!(r.validate(), Err(RosterError::NoLead));
    }

    #[test]
    fn lookup_and_ids() {
        let r = Roster::default_roster();
        assert_eq!(r.get("vex").unwrap().engine, Engine::Codex);
        assert!(r.get("nobody").is_none());
        assert_eq!(r.ids(), vec!["cipher".to_string(), "vex".to_string()]);
    }

    #[test]
    fn engine_maps_to_faction() {
        assert_eq!(Engine::Claude.faction(), Faction::Architects);
        assert_eq!(Engine::Codex.faction(), Faction::Forgers);
        assert!(Engine::Claude.can_lead());
        assert!(!Engine::Codex.can_lead());
    }

    #[test]
    fn endpoint_engine_is_a_forger_that_cannot_lead() {
        let e = Engine::Endpoint {
            base_url: "http://localhost:11434/v1".into(),
            model: "qwen2.5-coder".into(),
        };
        assert_eq!(e.faction(), Faction::Forgers);
        assert!(!e.can_lead(), "local models execute, never plan/judge");
    }

    #[test]
    fn endpoint_engine_round_trips_through_serde() {
        let e = Engine::Endpoint {
            base_url: "http://localhost:8000/v1".into(),
            model: "llama3".into(),
        };
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("endpoint"), "tagged variant: {json}");
        assert_eq!(serde_json::from_str::<Engine>(&json).unwrap(), e);
        // Unit variants stay as bare strings.
        assert_eq!(
            serde_json::to_string(&Engine::Claude).unwrap(),
            "\"claude\""
        );
    }

    #[test]
    fn a_roster_of_only_endpoints_cannot_lead() {
        let r = Roster {
            agents: vec![Agent {
                id: "local".into(),
                name: "Local".into(),
                role: "x".into(),
                engine: Engine::Endpoint {
                    base_url: "http://localhost:11434/v1".into(),
                    model: "m".into(),
                },
                skill_prompt: None,
                budget: None,
                identity_ref: None,
            }],
        };
        assert_eq!(r.validate(), Err(RosterError::NoLead));
    }
}
