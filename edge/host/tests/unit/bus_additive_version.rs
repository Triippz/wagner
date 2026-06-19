//! T008 [US1] — additive-versioning regression (FR-017): a `stable` type's schema
//! may gain an OPTIONAL field + a `version` bump, but a payload written against
//! the prior schema MUST still validate, and no required field is ever added.
//! Plus: a real `stable` contract type (`Envelope`) closes its objects
//! (`additionalProperties:false`) so evolution can only be additive.
//! Covers SC-005, US2-AS-3, FR-017.

use serde::{Deserialize, Serialize};
use wagner_edge_host::bus::export_schemas;
use wagner_edge_host::schema::validate;

/// A `stable` payload type at v1.
#[derive(Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct ThingV1 {
    a: String,
}

/// The same type, additively evolved: one NEW OPTIONAL field (`version` bumped).
#[derive(Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct ThingV2 {
    a: String,
    b: Option<String>,
}

fn schema_string<T: schemars::JsonSchema>() -> String {
    serde_json::to_string(&schemars::schema_for!(T)).unwrap()
}

#[test]
fn old_payload_still_validates_against_additively_evolved_schema() {
    let v2_schema = schema_string::<ThingV2>();
    let v1_payload = serde_json::to_value(ThingV1 { a: "x".into() }).unwrap();
    validate(&v2_schema, &v1_payload)
        .expect("a payload written against v1 must still validate against the v2 schema");
}

#[test]
fn evolution_adds_no_required_field() {
    let v2: serde_json::Value = serde_json::from_str(&schema_string::<ThingV2>()).unwrap();
    let required = v2["required"].as_array().cloned().unwrap_or_default();
    assert!(required.iter().any(|r| r == "a"), "pre-existing required field stays required");
    assert!(
        !required.iter().any(|r| r == "b"),
        "FR-017: an additively-added field must be optional, never required"
    );
}

#[test]
fn stable_envelope_schema_closes_unknown_fields() {
    let (_, envelope_schema) = export_schemas()
        .into_iter()
        .find(|(name, _)| name == "envelope")
        .expect("envelope schema is exported");
    assert_eq!(
        envelope_schema["additionalProperties"],
        serde_json::json!(false),
        "a stable type must close unknown fields so evolution stays additive"
    );
}
