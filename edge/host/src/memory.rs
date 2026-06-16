//! Embedded memory / learnings / templates store (Phase G).
//!
//! A pure-Rust embedded SurrealDB (SurrealKV backend) lives in-process in the Tauri
//! host as the durable store for learnings, run history, and reusable workflow
//! templates. The schema carries `user_id` / `project_id` from day one so the same
//! schema points at a central server later (Phase H) by *filtering* rather than a
//! rewrite — the engineer chose SurrealDB for that multi-model + central-path fit
//! (see `docs/research/agent-memory-persistence-research.md`).
//!
//! Curated learnings are *also* projected to git-diffable Markdown under
//! `.wagner/memory/<slug>.md` (Serena-style) so they travel with the repo and stay
//! human-inspectable; SurrealDB is the queryable index, the files are the source of
//! truth for curated memory. Central/cloud sync is deliberately out of scope here.

use serde::{Deserialize, Serialize};
use std::path::Path;
use surrealdb::engine::local::{Db, SurrealKv};
use surrealdb::Surreal;

/// A new learning to persist (the caller supplies the project + text + tags).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInput {
    pub project_id: String,
    pub text: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub source_type: Option<String>,
    #[serde(default)]
    pub source_ref: Option<String>,
}

/// A stored learning. `uid` is our own ULID (not the Surreal RecordId) so records
/// round-trip as plain strings without RecordId (de)serialization friction. Every
/// field is a plain scalar/array — SurrealDB 2.x's content serializer rejects Rust
/// enums (`Option`, `serde_json::Value`), so absent sources are stored as `""`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub uid: String,
    pub user_id: String,
    pub project_id: String,
    pub text: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub created_at: String,
    pub curation_state: String,
    #[serde(default)]
    pub source_type: String,
    #[serde(default)]
    pub source_ref: String,
}

/// A persisted workflow template (decision #4 — workflows are reusable). `content`
/// is the serialized `Workflow` graph as a JSON **string** (not a nested value —
/// see the enum note on `MemoryRecord`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredTemplate {
    pub uid: String,
    pub author_id: String,
    pub name: String,
    pub description: String,
    pub content: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// The embedded store. Holds one `Surreal<Db>` connection (SurrealKV file-backed).
pub struct MemoryStore {
    db: Surreal<Db>,
    /// Single-tenant default until a central server lands; carried on every row.
    user_id: String,
}

/// Errors the store surfaces to callers. User-input validation is expressed as
/// `InvalidInput` rather than borrowing a SurrealDB-internal error variant, so the
/// module is not coupled to the DB engine's error taxonomy for its own rules.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error(transparent)]
    Db(#[from] surrealdb::Error),
}

/// Stable id from a name (template ids, memory slugs).
pub fn slug(name: &str) -> String {
    let s: String = name
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    s.trim_matches('-').split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-")
}

/// Project a learning to git-diffable Markdown (YAML frontmatter + body). Mirrors
/// Serena's `.serena/memories` and this repo's own `memory/` convention.
pub fn memory_markdown(rec: &MemoryRecord) -> String {
    // Quote + newline-strip every frontmatter scalar so a stray `:`/newline/`---`
    // in a project path or tag can't malform the YAML block (the body is free text).
    let q = |s: &str| format!("\"{}\"", s.replace(['\n', '\r'], " ").replace('"', "'"));
    let tags = rec.tags.iter().map(|t| q(t)).collect::<Vec<_>>().join(", ");
    format!(
        "---\nuid: {uid}\nproject: {project}\ntags: [{tags}]\ncreated: {created}\ncuration: {curation}\n---\n\n{text}\n",
        uid = q(&rec.uid),
        project = q(&rec.project_id),
        tags = tags,
        created = q(&rec.created_at),
        curation = q(&rec.curation_state),
        text = rec.text,
    )
}

impl MemoryStore {
    /// Open (or create) the embedded store at `dir`, defining the schema/indexes
    /// idempotently. `user_id` stamps every row (single-tenant for now).
    pub async fn open(dir: &Path, user_id: &str) -> surrealdb::Result<Self> {
        let db = Surreal::new::<SurrealKv>(dir.to_string_lossy().to_string()).await?;
        db.use_ns("wagner").use_db("memory").await?;
        // SCHEMALESS tables + the indexes that matter now. BM25 full-text on memory
        // text for keyword recall; a project index for the multi-tenant filter. The
        // durable DiskANN embedding index (3.1) is deferred until embeddings land.
        db.query(
            r#"
            DEFINE TABLE IF NOT EXISTS memory SCHEMALESS;
            DEFINE TABLE IF NOT EXISTS workflow_template SCHEMALESS;
            DEFINE ANALYZER IF NOT EXISTS wagner_en TOKENIZERS class FILTERS ascii, lowercase, snowball(english);
            DEFINE INDEX IF NOT EXISTS memory_project ON TABLE memory COLUMNS project_id;
            DEFINE INDEX IF NOT EXISTS memory_text ON TABLE memory COLUMNS text SEARCH ANALYZER wagner_en BM25;
            "#,
        )
        .await?
        .check()?;
        Ok(Self { db, user_id: user_id.to_string() })
    }

    /// Persist a learning and return the stored record.
    pub async fn save_memory(&self, input: MemoryInput, now: &str) -> surrealdb::Result<MemoryRecord> {
        let uid = ulid::Ulid::new().to_string();
        let rec = MemoryRecord {
            uid: uid.clone(),
            user_id: self.user_id.clone(),
            project_id: input.project_id,
            text: input.text,
            tags: input.tags,
            created_at: now.to_string(),
            curation_state: "auto".into(),
            source_type: input.source_type.unwrap_or_default(),
            source_ref: input.source_ref.unwrap_or_default(),
        };
        // CREATE with a known id. Deserialize the response into `MemoryRecord` (which
        // omits the Surreal `id`) — a `serde_json::Value` would choke on the RecordId.
        let _: Option<MemoryRecord> =
            self.db.create(("memory", uid.as_str())).content(rec.clone()).await?;
        Ok(rec)
    }

    /// Recall the most recent learnings for a project (newest first). `tag`, when
    /// set, restricts to learnings carrying it. This is the recall loop's read side.
    pub async fn recall(
        &self,
        project_id: &str,
        tag: Option<&str>,
        limit: usize,
    ) -> surrealdb::Result<Vec<MemoryRecord>> {
        // created_at is stored as `to_rfc3339_opts(Secs, true)` (always `…Z`, no
        // sub-seconds) so this lexicographic ORDER BY is a true chronological sort.
        let base = "SELECT * FROM memory WHERE project_id = $pid";
        let mut query = if let Some(t) = tag {
            self.db
                .query(format!("{base} AND $tag IN tags ORDER BY created_at DESC LIMIT $n"))
                .bind(("tag", t.to_string()))
        } else {
            self.db.query(format!("{base} ORDER BY created_at DESC LIMIT $n"))
        };
        query = query.bind(("pid", project_id.to_string())).bind(("n", limit));
        let mut res = query.await?;
        // SELECT * also yields the Surreal `id` (RecordId); serde drops the unknown
        // field when deserializing into `MemoryRecord`.
        let rows: Vec<MemoryRecord> = res.take(0)?;
        Ok(rows)
    }

    /// Upsert a workflow template by name (decision #4 — reusable templates).
    pub async fn save_template(
        &self,
        name: &str,
        description: &str,
        content: &serde_json::Value,
        tags: Vec<String>,
        now: &str,
    ) -> Result<StoredTemplate, StoreError> {
        let uid = slug(name);
        if uid.is_empty() {
            return Err(StoreError::InvalidInput(
                "template name has no usable characters".into(),
            ));
        }
        // Preserve the original `created_at` across re-saves; only `updated_at` moves.
        let existing: Option<StoredTemplate> =
            self.db.select(("workflow_template", uid.as_str())).await?;
        let created_at = existing.map(|t| t.created_at).unwrap_or_else(|| now.to_string());
        let rec = StoredTemplate {
            uid: uid.clone(),
            author_id: self.user_id.clone(),
            name: name.to_string(),
            description: description.to_string(),
            content: content.to_string(),
            tags,
            created_at,
            updated_at: now.to_string(),
        };
        let _: Option<StoredTemplate> =
            self.db.upsert(("workflow_template", uid.as_str())).content(rec.clone()).await?;
        Ok(rec)
    }

    /// List all saved workflow templates (newest update first).
    pub async fn list_templates(&self) -> surrealdb::Result<Vec<StoredTemplate>> {
        let mut res = self
            .db
            .query("SELECT * FROM workflow_template ORDER BY updated_at DESC")
            .await?;
        let rows: Vec<StoredTemplate> = res.take(0)?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_is_kebab_and_trimmed() {
        assert_eq!(slug("  Full Pipeline! "), "full-pipeline");
        assert_eq!(slug("Standard"), "standard");
        assert_eq!(slug("a__b  c"), "a-b-c");
    }

    #[test]
    fn markdown_projection_has_frontmatter_and_body() {
        let rec = MemoryRecord {
            uid: "01ULID".into(),
            user_id: "me".into(),
            project_id: "proj".into(),
            text: "prefer struct params".into(),
            tags: vec!["style".into(), "rust".into()],
            created_at: "2026-06-13T00:00:00Z".into(),
            curation_state: "auto".into(),
            source_type: String::new(),
            source_ref: String::new(),
        };
        let md = memory_markdown(&rec);
        assert!(md.starts_with("---\n"));
        assert!(md.contains("tags: [\"style\", \"rust\"]"));
        assert!(md.contains("project: \"proj\""));
        assert!(md.trim_end().ends_with("prefer struct params"));
    }

    #[test]
    fn markdown_frontmatter_is_injection_safe() {
        let rec = MemoryRecord {
            uid: "u".into(),
            user_id: "me".into(),
            project_id: "/tmp/p: evil\n---\ninjected: true".into(),
            text: "body".into(),
            tags: vec!["a\nb".into()],
            created_at: "2026-06-13T00:00:00Z".into(),
            curation_state: "auto".into(),
            source_type: String::new(),
            source_ref: String::new(),
        };
        let md = memory_markdown(&rec);
        // exactly one frontmatter close before the body — no injected second doc.
        assert_eq!(md.matches("\n---\n").count(), 1);
        // the payload survives only as a quoted single-line scalar, never as its own
        // newline-prefixed YAML key.
        assert!(!md.contains("\ninjected:"));
        assert!(md.contains("project: \"/tmp/p: evil --- injected: true\""));
    }

    async fn temp_store() -> (MemoryStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let store = MemoryStore::open(dir.path(), "tester").await.unwrap();
        (store, dir)
    }

    #[tokio::test]
    async fn save_then_recall_roundtrips_newest_first() {
        let (store, _d) = temp_store().await;
        store
            .save_memory(
                MemoryInput { project_id: "p".into(), text: "first".into(), tags: vec!["t".into()], source_type: None, source_ref: None },
                "2026-06-13T00:00:01Z",
            )
            .await
            .unwrap();
        store
            .save_memory(
                MemoryInput { project_id: "p".into(), text: "second".into(), tags: vec![], source_type: None, source_ref: None },
                "2026-06-13T00:00:02Z",
            )
            .await
            .unwrap();
        // a learning in a different project must not leak into p's recall.
        store
            .save_memory(
                MemoryInput { project_id: "other".into(), text: "elsewhere".into(), tags: vec![], source_type: None, source_ref: None },
                "2026-06-13T00:00:03Z",
            )
            .await
            .unwrap();

        let all = store.recall("p", None, 10).await.unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].text, "second"); // newest first
        assert_eq!(all[1].text, "first");

        let tagged = store.recall("p", Some("t"), 10).await.unwrap();
        assert_eq!(tagged.len(), 1);
        assert_eq!(tagged[0].text, "first");
    }

    #[tokio::test]
    async fn template_upsert_is_idempotent_by_name() {
        let (store, _d) = temp_store().await;
        let wf = serde_json::json!({"schema": "workflow.v1", "nodes": [], "edges": []});
        store.save_template("My Flow", "v1", &wf, vec![], "2026-06-13T00:00:01Z").await.unwrap();
        store.save_template("My Flow", "v2 desc", &wf, vec!["x".into()], "2026-06-13T00:00:02Z").await.unwrap();
        let all = store.list_templates().await.unwrap();
        assert_eq!(all.len(), 1, "same name upserts, not duplicates");
        assert_eq!(all[0].description, "v2 desc");
        assert_eq!(all[0].created_at, "2026-06-13T00:00:01Z", "created_at preserved across re-save");
        assert_eq!(all[0].updated_at, "2026-06-13T00:00:02Z", "updated_at advances");
    }

    #[tokio::test]
    async fn template_with_unusable_name_is_rejected() {
        let (store, _d) = temp_store().await;
        let wf = serde_json::json!({});
        assert!(store.save_template("!!!", "x", &wf, vec![], "2026-06-13T00:00:00Z").await.is_err());
    }
}
