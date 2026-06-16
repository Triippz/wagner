//! JSON Schema validation (Constitution Article VII).
//!
//! Every structured payload is validated against its declared schema before
//! it is written to disk or emitted to the frontend. Schemas live in
//! `platform/edge/host/schemas/` (ported with the engine, T000a) and are
//! compiled in at build time so the validator needs no filesystem access at
//! runtime.

use serde_json::Value;

/// The five schema sources, embedded at compile time.
pub const CONSTRUCT_EVENT_SCHEMA: &str = include_str!("../schemas/wagner-event.schema.json");
pub const RUN_STATE_SCHEMA: &str = include_str!("../schemas/run-state.schema.json");
pub const ORACLE_PLAN_SCHEMA: &str = include_str!("../schemas/oracle-plan.schema.json");
pub const TRANSMISSION_SCHEMA: &str = include_str!("../schemas/transmission.schema.json");
pub const WORKFLOW_STEP_EVENT_SCHEMA: &str =
    include_str!("../schemas/workflow-step-event.schema.json");

#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    #[error("schema is not valid JSON: {0}")]
    InvalidSchema(String),
    #[error("payload failed validation: {0}")]
    ValidationFailed(String),
}

/// Validate a serde_json `Value` against a schema source string.
/// Returns `Ok(())` when valid, or the first validation error.
pub fn validate(schema_src: &str, instance: &Value) -> Result<(), SchemaError> {
    let schema_json: Value =
        serde_json::from_str(schema_src).map_err(|e| SchemaError::InvalidSchema(e.to_string()))?;
    let compiled = jsonschema::JSONSchema::compile(&schema_json)
        .map_err(|e| SchemaError::InvalidSchema(e.to_string()))?;
    if let Err(errors) = compiled.validate(instance) {
        let msg = errors.map(|e| e.to_string()).collect::<Vec<_>>().join("; ");
        return Err(SchemaError::ValidationFailed(msg));
    }
    Ok(())
}

/// Serialize a value and validate it against the schema in one step.
pub fn validate_serialized<T: serde::Serialize>(
    schema_src: &str,
    value: &T,
) -> Result<Value, SchemaError> {
    let json =
        serde_json::to_value(value).map_err(|e| SchemaError::ValidationFailed(e.to_string()))?;
    validate(schema_src, &json)?;
    Ok(json)
}
