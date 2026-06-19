//! Uniform `PluginManifest`, the closed v1 `Capability` vocabulary, `Namespace`,
//! `SchemaRef`, and `StabilityTier`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::Contract;

/// The stability tier of a contract type (FR-010). A newly added type defaults
/// to `Experimental`; promotion to `Stable` (which binds the no-break rule
/// FR-017) is a deliberate change; `Internal` is for engine-private types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StabilityTier {
    Stable,
    #[default]
    Experimental,
    Internal,
}

/// The v1 event/command namespaces a plugin may emit or subscribe to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Namespace {
    Run,
    Goal,
    Vault,
    Voice,
    Ui,
    Ext,
}

/// The **closed** v1 capability vocabulary (FR-014): the coarse permission kinds
/// a plugin may request. Honored on trust today (declared, not enforced — the
/// sandbox is deferred, `specs/012` §13.7); the set grows additively. No
/// per-path/per-host scoping in v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Capability {
    #[serde(rename = "network")]
    Network,
    #[serde(rename = "process.spawn")]
    ProcessSpawn,
    #[serde(rename = "vault.read")]
    VaultRead,
    #[serde(rename = "vault.write")]
    VaultWrite,
    #[serde(rename = "fs.read")]
    FsRead,
    #[serde(rename = "fs.write")]
    FsWrite,
    #[serde(rename = "secrets.read")]
    SecretsRead,
}

/// A reference to a JSON Schema a plugin registers (name + version).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SchemaRef {
    pub name: String,
    pub version: u32,
}

/// The uniform plugin manifest (FR-012): what a plugin provides, the namespaces
/// it emits/subscribes, the schemas it registers, the capabilities it requests,
/// and its stability tier. An empty `capabilities` set is valid — a pure
/// subscriber requests nothing (EC-004).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PluginManifest {
    /// Logical names of the participants (each an `Agent`) this plugin provides.
    pub participants_provided: Vec<String>,
    pub emits: Vec<Namespace>,
    pub subscribes: Vec<Namespace>,
    pub registered_schemas: Vec<SchemaRef>,
    pub capabilities: Vec<Capability>,
    #[serde(default)]
    pub stability: StabilityTier,
}

impl Contract for PluginManifest {
    const SCHEMA: &'static str = "plugin_manifest.v1";
    const TIER: StabilityTier = StabilityTier::Stable;
}
