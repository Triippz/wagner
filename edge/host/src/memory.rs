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

use crate::vault::WikiLink;
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

/// The vault knowledge-model metadata for a note (Plan 004). Separate from the
/// persisted scalars so callers (and the future semantic-extraction step) build
/// it explicitly. `relationships` is `(target, rel_type)` — stored in the
/// relationship table, not on the record (Surreal rejects nested objects).
#[derive(Debug, Clone, Default)]
pub struct NoteMeta {
    /// Display name for the link index; derived from the body's first line if empty.
    pub title: String,
    pub summary: String,
    pub tier: String,
    pub lifecycle: String,
    pub provenance: String,
    pub relationships: Vec<(String, String)>,
}

/// A tiered-retrieval query (Plan 004 step 5): cheapest-match-first lookup that
/// keeps cost ~constant as the vault grows.
pub struct TieredQuery<'a> {
    pub project_id: &'a str,
    pub terms: &'a str,
    pub limit: usize,
}

/// One tiered-retrieval hit, tagged by the tier it matched at (summary → section
/// → full body → reached via a relationship hop).
#[derive(Debug, Clone, PartialEq)]
pub enum TieredResult {
    Summary(MemoryRecord),
    Section(MemoryRecord),
    Full(MemoryRecord),
    Related(MemoryRecord),
}

impl TieredResult {
    pub fn record(&self) -> &MemoryRecord {
        match self {
            TieredResult::Summary(r)
            | TieredResult::Section(r)
            | TieredResult::Full(r)
            | TieredResult::Related(r) => r,
        }
    }
}

/// A stored learning. `uid` is our own ULID (not the Surreal RecordId) so records
/// round-trip as plain strings without RecordId (de)serialization friction. Every
/// field is a plain scalar/array — SurrealDB 2.x's content serializer rejects Rust
/// enums (`Option`, `serde_json::Value`), so absent sources are stored as `""`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
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
    // --- Vault knowledge-model fields (Plan 004). Plain scalars only: SurrealDB
    // 2.x's content serializer rejects enums/Option/nested objects, so these are
    // Strings ("" = absent). Typed relationships live in a separate table, not
    // here. summary powers cheap tiered retrieval (≤200ch by convention).
    #[serde(default)]
    pub summary: String,
    /// core | supporting | peripheral (or "").
    #[serde(default)]
    pub tier: String,
    /// draft | reviewed | verified | disputed | archived (or "").
    #[serde(default)]
    pub lifecycle: String,
    /// extracted | inferred | ambiguous (or "").
    #[serde(default)]
    pub provenance: String,
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

/// A note's display title for the link index: the first non-empty line (stripped
/// of a leading `#`), capped to 80 chars. Empty body → empty title.
pub fn derive_title(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(|l| l.trim_start_matches('#').trim().chars().take(80).collect())
        .unwrap_or_default()
}

/// Path to a note's Markdown projection. `staging = true` targets the
/// `.wagner/memory/_staging/` approval gate; false targets the curated dir.
fn note_md_path(project_dir: &std::path::Path, uid: &str, staging: bool) -> std::path::PathBuf {
    let mut p = project_dir.join(".wagner").join("memory");
    if staging {
        p = p.join("_staging");
    }
    p.join(format!("{uid}.md"))
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
    // Vault fields are emitted only when set, so legacy notes keep their exact
    // shape and the frontmatter stays minimal until semantic extraction fills it.
    let mut extra = String::new();
    for (key, val) in [
        ("summary", &rec.summary),
        ("tier", &rec.tier),
        ("lifecycle", &rec.lifecycle),
        ("provenance", &rec.provenance),
    ] {
        if !val.is_empty() {
            extra.push_str(&format!("{key}: {}\n", q(val)));
        }
    }
    format!(
        "---\nuid: {uid}\nproject: {project}\ntags: [{tags}]\ncreated: {created}\ncuration: {curation}\n{extra}---\n\n{text}\n",
        uid = q(&rec.uid),
        project = q(&rec.project_id),
        tags = tags,
        created = q(&rec.created_at),
        curation = q(&rec.curation_state),
        extra = extra,
        text = rec.text,
    )
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
            DEFINE TABLE IF NOT EXISTS vault_name_index SCHEMALESS;
            DEFINE INDEX IF NOT EXISTS name_idx_uid ON TABLE vault_name_index COLUMNS uid UNIQUE;
            DEFINE INDEX IF NOT EXISTS name_idx_name ON TABLE vault_name_index COLUMNS display_name;
            DEFINE TABLE IF NOT EXISTS vault_wikilink SCHEMALESS;
            DEFINE INDEX IF NOT EXISTS wikilink_source ON TABLE vault_wikilink COLUMNS source_uid;
            DEFINE TABLE IF NOT EXISTS vault_backlink SCHEMALESS;
            DEFINE INDEX IF NOT EXISTS backlink_target ON TABLE vault_backlink COLUMNS target_uid;
            DEFINE TABLE IF NOT EXISTS vault_relationship SCHEMALESS;
            DEFINE INDEX IF NOT EXISTS rel_source ON TABLE vault_relationship COLUMNS source_uid;
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
            ..Default::default()
        };
        // CREATE with a known id. Deserialize the response into `MemoryRecord` (which
        // omits the Surreal `id`) — a `serde_json::Value` would choke on the RecordId.
        let _: Option<MemoryRecord> =
            self.db.create(("memory", uid.as_str())).content(rec.clone()).await?;
        Ok(rec)
    }

    /// Save a vault note (Plan 004 step 4): the unified write path. Persists the
    /// record (with vault scalars), indexes its title, parses `[[wikilinks]]`
    /// deterministically, writes the wikilink rows (with resolved target uids),
    /// the inbound backlinks for resolved targets, the typed relationships, and
    /// the Markdown projection. A body with no links saves cleanly; an unresolved
    /// link is recorded with an empty target, never an error.
    /// ponytail: relationship frontmatter projection is deferred — the
    /// relationship table is the source of truth; add it to the projection when
    /// the graph view needs links visible in the `.md` itself.
    pub async fn save_note(
        &self,
        input: MemoryInput,
        meta: NoteMeta,
        now: &str,
        project_dir: &Path,
    ) -> surrealdb::Result<MemoryRecord> {
        let uid = ulid::Ulid::new().to_string();
        let title = if meta.title.trim().is_empty() {
            derive_title(&input.text)
        } else {
            meta.title.trim().to_string()
        };
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
            summary: meta.summary,
            tier: meta.tier,
            lifecycle: meta.lifecycle,
            provenance: meta.provenance,
        };
        let _: Option<MemoryRecord> =
            self.db.create(("memory", uid.as_str())).content(rec.clone()).await?;

        if !title.is_empty() {
            self.upsert_name_index(&uid, &title).await?;
        }
        let links = crate::vault::parse_wikilinks(&rec.text);
        self.write_wikilinks(&uid, &links).await?;
        let mut targets = Vec::new();
        for l in &links {
            if let Some(t) = self.resolve_name(&l.display_name).await? {
                targets.push(t);
            }
        }
        self.write_backlinks(&uid, &targets).await?;
        self.write_relationships(&uid, &meta.relationships).await?;
        // Agent-authored notes land in the _staging/ gate until a human approves
        // them (Plan 004 step 6) — keeps the curated vault trustworthy.
        let staged = note_md_path(project_dir, &uid, true);
        if let Some(parent) = staged.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&staged, memory_markdown(&rec));
        Ok(rec)
    }

    /// Approve a staged note: move its Markdown from `_staging/` to the curated
    /// dir and flip its curation state to `captured`. Errors if the uid isn't
    /// staged. ponytail: single-note approval only; batch approve/reject later.
    pub async fn approve_staging_note(
        &self,
        uid: &str,
        project_dir: &Path,
    ) -> Result<(), StoreError> {
        let from = note_md_path(project_dir, uid, true);
        if !from.exists() {
            return Err(StoreError::InvalidInput(format!("no staged note {uid}")));
        }
        let to = note_md_path(project_dir, uid, false);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(|e| StoreError::InvalidInput(e.to_string()))?;
        }
        std::fs::rename(&from, &to).map_err(|e| StoreError::InvalidInput(e.to_string()))?;
        self.db
            .query("UPDATE type::thing('memory', $u) SET curation_state = 'captured'")
            .bind(("u", uid.to_string()))
            .await?
            .check()?;
        Ok(())
    }

    /// The uids of notes currently awaiting approval in `_staging/`.
    pub fn list_staging(&self, project_dir: &Path) -> Vec<String> {
        let dir = project_dir.join(".wagner").join("memory").join("_staging");
        let mut out = Vec::new();
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                if e.path().extension().and_then(|x| x.to_str()) == Some("md") {
                    if let Some(stem) = e.path().file_stem().and_then(|s| s.to_str()) {
                        out.push(stem.to_string());
                    }
                }
            }
        }
        out
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

    /// The "PRIOR LEARNINGS" block to fold into a run/workflow goal: the most
    /// recent learnings for a project as a bullet list, or `None` if there are
    /// none (or recall failed). Shared read side of the recall loop used by both
    /// `start_run` and `start_workflow`.
    pub async fn recall_block(&self, project_id: &str, limit: usize) -> Option<String> {
        let learnings = self.recall(project_id, None, limit).await.ok()?;
        if learnings.is_empty() {
            return None;
        }
        let block = learnings
            .iter()
            .map(|m| format!("- {}", m.text))
            .collect::<Vec<_>>()
            .join("\n");
        Some(format!("PRIOR LEARNINGS (from earlier runs):\n{block}"))
    }

    // ---- Vault link graph (Plan 004 step 3) — deterministic links/backlinks. ----

    /// Record (or refresh) a note's canonical display name for link resolution.
    /// Idempotent per uid (UPSERT on a known record id).
    pub async fn upsert_name_index(
        &self,
        uid: &str,
        display_name: &str,
    ) -> surrealdb::Result<()> {
        self.db
            .query("UPSERT type::thing('vault_name_index', $uid) SET uid = $uid, display_name = $name")
            .bind(("uid", uid.to_string()))
            .bind(("name", display_name.to_string()))
            .await?
            .check()?;
        Ok(())
    }

    /// Resolve a `[[display name]]` to a note uid, if one is indexed.
    pub async fn resolve_name(&self, display_name: &str) -> surrealdb::Result<Option<String>> {
        #[derive(Deserialize)]
        struct UidProj {
            uid: String,
        }
        let mut res = self
            .db
            .query("SELECT uid FROM vault_name_index WHERE display_name = $name LIMIT 1")
            .bind(("name", display_name.to_string()))
            .await?;
        let rows: Vec<UidProj> = res.take(0)?;
        Ok(rows.into_iter().next().map(|r| r.uid))
    }

    /// Replace the raw wikilinks recorded for a source note. Each row carries the
    /// resolved target uid ("" when the target isn't indexed yet).
    pub async fn write_wikilinks(
        &self,
        source_uid: &str,
        links: &[WikiLink],
    ) -> surrealdb::Result<()> {
        self.db
            .query("DELETE vault_wikilink WHERE source_uid = $s")
            .bind(("s", source_uid.to_string()))
            .await?
            .check()?;
        for l in links {
            let resolved = self.resolve_name(&l.display_name).await?.unwrap_or_default();
            self.db
                .query("CREATE vault_wikilink SET source_uid = $s, display_name = $d, resolved_uid = $r")
                .bind(("s", source_uid.to_string()))
                .bind(("d", l.display_name.clone()))
                .bind(("r", resolved))
                .await?
                .check()?;
        }
        Ok(())
    }

    /// Replace the backlinks for a source note: one inbound edge per resolved
    /// target (target ← source).
    pub async fn write_backlinks(
        &self,
        source_uid: &str,
        target_uids: &[String],
    ) -> surrealdb::Result<()> {
        self.db
            .query("DELETE vault_backlink WHERE source_uid = $s")
            .bind(("s", source_uid.to_string()))
            .await?
            .check()?;
        for t in target_uids {
            self.db
                .query("CREATE vault_backlink SET target_uid = $t, source_uid = $s")
                .bind(("t", t.clone()))
                .bind(("s", source_uid.to_string()))
                .await?
                .check()?;
        }
        Ok(())
    }

    /// The note uids that link TO `target_uid` (its inbound backlinks).
    pub async fn backlinks_for(&self, target_uid: &str) -> surrealdb::Result<Vec<String>> {
        #[derive(Deserialize)]
        struct SrcProj {
            source_uid: String,
        }
        let mut res = self
            .db
            .query("SELECT source_uid FROM vault_backlink WHERE target_uid = $t")
            .bind(("t", target_uid.to_string()))
            .await?;
        let rows: Vec<SrcProj> = res.take(0)?;
        Ok(rows.into_iter().map(|r| r.source_uid).collect())
    }

    /// Replace a source note's typed relationships (`(target_uid, rel_type)`).
    pub async fn write_relationships(
        &self,
        source_uid: &str,
        rels: &[(String, String)],
    ) -> surrealdb::Result<()> {
        self.db
            .query("DELETE vault_relationship WHERE source_uid = $s")
            .bind(("s", source_uid.to_string()))
            .await?
            .check()?;
        for (target, rel_type) in rels {
            self.db
                .query("CREATE vault_relationship SET source_uid = $s, target_uid = $t, rel_type = $rt")
                .bind(("s", source_uid.to_string()))
                .bind(("t", target.clone()))
                .bind(("rt", rel_type.clone()))
                .await?
                .check()?;
        }
        Ok(())
    }

    /// Notes reached by walking `vault_relationship` edges up to `max_hops` from
    /// the seeds (Plan 004 step 7). Excludes the seeds; cycle-safe (each uid
    /// visited once); `max_hops = 0` returns empty. Used by `tiered_query` (1 hop)
    /// and by the graph view (Plan 005) with larger hop counts.
    pub async fn related_by_bfs(
        &self,
        seeds: &[String],
        max_hops: usize,
    ) -> surrealdb::Result<Vec<MemoryRecord>> {
        use std::collections::HashSet;
        let mut visited: HashSet<String> = seeds.iter().cloned().collect();
        let mut frontier: Vec<String> = seeds.to_vec();
        let mut reached: Vec<String> = Vec::new();
        #[derive(Deserialize)]
        struct T {
            target_uid: String,
        }
        for _ in 0..max_hops {
            if frontier.is_empty() {
                break;
            }
            let mut res = self
                .db
                .query("SELECT target_uid FROM vault_relationship WHERE source_uid IN $f")
                .bind(("f", frontier.clone()))
                .await?;
            let rows: Vec<T> = res.take(0)?;
            let mut next = Vec::new();
            for r in rows {
                if visited.insert(r.target_uid.clone()) {
                    reached.push(r.target_uid.clone());
                    next.push(r.target_uid);
                }
            }
            frontier = next;
        }
        self.records_in_order(&reached).await
    }

    /// Fetch records for `uids`, preserving the given order (missing uids dropped).
    async fn records_in_order(&self, uids: &[String]) -> surrealdb::Result<Vec<MemoryRecord>> {
        if uids.is_empty() {
            return Ok(vec![]);
        }
        let mut res = self
            .db
            .query("SELECT * FROM memory WHERE uid IN $u")
            .bind(("u", uids.to_vec()))
            .await?;
        let recs: Vec<MemoryRecord> = res.take(0)?;
        let by_uid: std::collections::HashMap<String, MemoryRecord> =
            recs.into_iter().map(|r| (r.uid.clone(), r)).collect();
        Ok(uids.iter().filter_map(|u| by_uid.get(u).cloned()).collect())
    }

    /// Tiered retrieval (Plan 004 step 5): summary-match → section (first ~300ch)
    /// → full-body → 1-hop relationship neighbours. Dedup across tiers, returned
    /// in tier order, capped at `limit` direct hits (Related neighbours append).
    /// ponytail: substring matching + an in-memory project scan (≤500 notes) —
    /// push to SQL / BM25 ranking when a vault outgrows that.
    pub async fn tiered_query(
        &self,
        q: TieredQuery<'_>,
    ) -> surrealdb::Result<Vec<TieredResult>> {
        let term = q.terms.to_lowercase();
        let mut res = self
            .db
            .query("SELECT * FROM memory WHERE project_id = $pid LIMIT 500")
            .bind(("pid", q.project_id.to_string()))
            .await?;
        let notes: Vec<MemoryRecord> = res.take(0)?;

        let mut out: Vec<TieredResult> = Vec::new();
        let mut matched: Vec<String> = Vec::new();
        // Pass in tier order so earlier tiers win the dedup.
        for tier in 0..3 {
            for n in &notes {
                if out.len() >= q.limit {
                    break;
                }
                if matched.contains(&n.uid) {
                    continue;
                }
                let hit = match tier {
                    0 => n.summary.to_lowercase().contains(&term) && !term.is_empty(),
                    1 => {
                        let head: String = n.text.chars().take(300).collect();
                        head.to_lowercase().contains(&term) && !term.is_empty()
                    }
                    _ => n.text.to_lowercase().contains(&term) && !term.is_empty(),
                };
                if hit {
                    matched.push(n.uid.clone());
                    out.push(match tier {
                        0 => TieredResult::Summary(n.clone()),
                        1 => TieredResult::Section(n.clone()),
                        _ => TieredResult::Full(n.clone()),
                    });
                }
            }
        }
        // Tier 4: one relationship hop out from the direct hits.
        for r in self.related_by_bfs(&matched, 1).await? {
            if !matched.contains(&r.uid) {
                out.push(TieredResult::Related(r));
            }
        }
        Ok(out)
    }

    /// Project a saved learning to git-diffable Markdown under the project's
    /// `.wagner/memory/`. Best-effort — never fails on an FS hiccup. Owned by the
    /// store rather than the command handler.
    pub fn write_markdown_projection(&self, project_dir: &Path, rec: &MemoryRecord) {
        let mem_dir = project_dir.join(".wagner").join("memory");
        if std::fs::create_dir_all(&mem_dir).is_ok() {
            let _ = std::fs::write(mem_dir.join(format!("{}.md", rec.uid)), memory_markdown(rec));
        }
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
            ..Default::default()
        };
        let md = memory_markdown(&rec);
        assert!(md.starts_with("---\n"));
        assert!(md.contains("tags: [\"style\", \"rust\"]"));
        assert!(md.contains("project: \"proj\""));
        assert!(md.trim_end().ends_with("prefer struct params"));
    }

    #[test]
    fn markdown_emits_vault_fields_when_set_and_omits_when_empty() {
        // Vault frontmatter (Plan 004): present only when populated, so legacy
        // notes keep their minimal shape.
        let mut rec = MemoryRecord {
            uid: "u".into(),
            user_id: "me".into(),
            project_id: "proj".into(),
            text: "body".into(),
            created_at: "2026-06-17T00:00:00Z".into(),
            curation_state: "captured".into(),
            ..Default::default()
        };
        // Empty by default → no vault keys in the block.
        let bare = memory_markdown(&rec);
        assert!(!bare.contains("summary:"));
        assert!(!bare.contains("tier:"));

        rec.summary = "prefers struct params over many args".into();
        rec.tier = "core".into();
        rec.lifecycle = "reviewed".into();
        rec.provenance = "extracted".into();
        let md = memory_markdown(&rec);
        assert!(md.contains("summary: \"prefers struct params over many args\""));
        assert!(md.contains("tier: \"core\""));
        assert!(md.contains("lifecycle: \"reviewed\""));
        assert!(md.contains("provenance: \"extracted\""));
        // Still exactly one frontmatter close.
        assert_eq!(md.matches("\n---\n").count(), 1);
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
            ..Default::default()
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

    fn link(name: &str) -> crate::vault::WikiLink {
        crate::vault::WikiLink { display_name: name.into(), alias: None }
    }

    #[tokio::test]
    async fn name_index_upsert_and_resolve() {
        let (store, _d) = temp_store().await;
        store.upsert_name_index("01ABC", "Auth Flow").await.unwrap();
        assert_eq!(store.resolve_name("Auth Flow").await.unwrap().as_deref(), Some("01ABC"));
        // Idempotent: re-upserting the same uid doesn't duplicate or error.
        store.upsert_name_index("01ABC", "Auth Flow").await.unwrap();
        assert_eq!(store.resolve_name("Auth Flow").await.unwrap().as_deref(), Some("01ABC"));
        assert!(store.resolve_name("Nonexistent").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn wikilinks_written_and_queryable() {
        let (store, _d) = temp_store().await;
        store.write_wikilinks("01A", &[link("X"), link("Y")]).await.unwrap();
        #[derive(serde::Deserialize)]
        struct Row {
            #[allow(dead_code)]
            source_uid: String,
        }
        let mut res = store
            .db
            .query("SELECT source_uid FROM vault_wikilink WHERE source_uid = $s")
            .bind(("s", "01A".to_string()))
            .await
            .unwrap();
        let rows: Vec<Row> = res.take(0).unwrap();
        assert_eq!(rows.len(), 2);
        // Re-writing replaces (not appends).
        store.write_wikilinks("01A", &[link("Z")]).await.unwrap();
        let mut res2 = store
            .db
            .query("SELECT source_uid FROM vault_wikilink WHERE source_uid = $s")
            .bind(("s", "01A".to_string()))
            .await
            .unwrap();
        let rows2: Vec<Row> = res2.take(0).unwrap();
        assert_eq!(rows2.len(), 1);
    }

    #[tokio::test]
    async fn backlinks_for_resolves_inbound() {
        let (store, _d) = temp_store().await;
        store.write_backlinks("01A", &["01B".to_string()]).await.unwrap();
        assert_eq!(store.backlinks_for("01B").await.unwrap(), vec!["01A".to_string()]);
        assert!(store.backlinks_for("01ZZZ").await.unwrap().is_empty());
    }

    fn note_input(project: &str, text: &str) -> MemoryInput {
        MemoryInput {
            project_id: project.into(),
            text: text.into(),
            tags: vec![],
            source_type: None,
            source_ref: None,
        }
    }

    #[tokio::test]
    async fn save_note_writes_record_links_and_projection() {
        let (store, dir) = temp_store().await;
        let rec = store
            .save_note(
                note_input("proj", "# My Note\nSee [[Target Note]]."),
                NoteMeta { summary: "a short note".into(), tier: "core".into(), ..Default::default() },
                "2026-06-17T00:00:00Z",
                dir.path(),
            )
            .await
            .unwrap();
        // Title indexed (derived from the first heading line).
        assert_eq!(store.resolve_name("My Note").await.unwrap().as_deref(), Some(rec.uid.as_str()));
        // Wikilink row written.
        #[derive(serde::Deserialize)]
        struct Row {
            #[allow(dead_code)]
            display_name: String,
        }
        let mut res = store
            .db
            .query("SELECT display_name FROM vault_wikilink WHERE source_uid = $s")
            .bind(("s", rec.uid.clone()))
            .await
            .unwrap();
        let rows: Vec<Row> = res.take(0).unwrap();
        assert_eq!(rows.len(), 1);
        // Markdown projected (to the _staging gate) with vault scalars.
        let md_path = dir
            .path()
            .join(".wagner")
            .join("memory")
            .join("_staging")
            .join(format!("{}.md", rec.uid));
        assert!(md_path.exists());
        let md = std::fs::read_to_string(&md_path).unwrap();
        assert!(md.contains("summary: \"a short note\""));
        assert!(md.contains("tier: \"core\""));
    }

    #[tokio::test]
    async fn staging_gate_holds_then_approve_promotes() {
        let (store, dir) = temp_store().await;
        let rec = store
            .save_note(note_input("p", "# Note\nbody"), NoteMeta::default(), "2026-06-17T00:00:00Z", dir.path())
            .await
            .unwrap();
        let staged = dir.path().join(".wagner/memory/_staging").join(format!("{}.md", rec.uid));
        let curated = dir.path().join(".wagner/memory").join(format!("{}.md", rec.uid));
        // Lands in staging, not curated.
        assert!(staged.exists());
        assert!(!curated.exists());
        assert_eq!(store.list_staging(dir.path()), vec![rec.uid.clone()]);
        // Approve → moves to curated, leaves staging.
        store.approve_staging_note(&rec.uid, dir.path()).await.unwrap();
        assert!(!staged.exists());
        assert!(curated.exists());
        assert!(store.list_staging(dir.path()).is_empty());
    }

    #[tokio::test]
    async fn approve_unknown_staged_note_errors() {
        let (store, dir) = temp_store().await;
        assert!(store.approve_staging_note("01NOPE", dir.path()).await.is_err());
    }

    #[tokio::test]
    async fn save_note_resolves_existing_link_into_backlink() {
        let (store, dir) = temp_store().await;
        let a = store
            .save_note(
                note_input("p", "Auth flow internals"),
                NoteMeta { title: "Auth Flow".into(), ..Default::default() },
                "2026-06-17T00:00:00Z",
                dir.path(),
            )
            .await
            .unwrap();
        let b = store
            .save_note(
                note_input("p", "See [[Auth Flow]] for details."),
                NoteMeta::default(),
                "2026-06-17T00:01:00Z",
                dir.path(),
            )
            .await
            .unwrap();
        // B's link resolved to A; A has an inbound backlink from B.
        assert_eq!(store.backlinks_for(&a.uid).await.unwrap(), vec![b.uid.clone()]);
    }

    #[tokio::test]
    async fn save_note_unresolved_link_is_not_fatal() {
        let (store, dir) = temp_store().await;
        let rec = store
            .save_note(
                note_input("p", "See [[Nonexistent Note]]."),
                NoteMeta::default(),
                "2026-06-17T00:00:00Z",
                dir.path(),
            )
            .await
            .unwrap();
        // No backlinks for a target that doesn't exist; the save still succeeded.
        assert!(store.backlinks_for(&rec.uid).await.unwrap().is_empty());
        assert_eq!(store.resolve_name("A Title Nobody Indexed").await.unwrap(), None);
        assert!(!rec.uid.is_empty());
    }

    #[tokio::test]
    async fn tiered_query_orders_summary_then_full_and_dedups() {
        let (store, dir) = temp_store().await;
        // A: matches in summary (tier 1). B: matches only in body (tier 3).
        store
            .save_note(
                note_input("p", "long body about sessions"),
                NoteMeta { title: "A".into(), summary: "auth flow design".into(), ..Default::default() },
                "2026-06-17T00:00:00Z",
                dir.path(),
            )
            .await
            .unwrap();
        store
            .save_note(
                note_input("p", "RBAC and auth checks live here"),
                NoteMeta { title: "B".into(), ..Default::default() },
                "2026-06-17T00:01:00Z",
                dir.path(),
            )
            .await
            .unwrap();
        let hits = store
            .tiered_query(TieredQuery { project_id: "p", terms: "auth", limit: 10 })
            .await
            .unwrap();
        // The summary hit (A) ranks first; the body-only hit (B) also appears at a
        // lower tier; each note appears exactly once across tiers.
        assert!(matches!(hits[0], TieredResult::Summary(_)));
        assert!(hits.iter().any(|h| h.record().text.contains("RBAC")));
        let uids: Vec<_> = hits.iter().map(|h| h.record().uid.clone()).collect();
        let mut dedup = uids.clone();
        dedup.sort();
        dedup.dedup();
        assert_eq!(uids.len(), dedup.len(), "no note repeated across tiers");
    }

    #[tokio::test]
    async fn tiered_query_reaches_related_neighbour() {
        let (store, dir) = temp_store().await;
        let b = store
            .save_note(
                note_input("p", "the derived note"),
                NoteMeta { title: "B".into(), ..Default::default() },
                "2026-06-17T00:00:00Z",
                dir.path(),
            )
            .await
            .unwrap();
        // A matches by summary and relates to B.
        store
            .save_note(
                note_input("p", "A body"),
                NoteMeta {
                    title: "A".into(),
                    summary: "auth design".into(),
                    relationships: vec![(b.uid.clone(), "derived_from".into())],
                    ..Default::default()
                },
                "2026-06-17T00:01:00Z",
                dir.path(),
            )
            .await
            .unwrap();
        let hits = store
            .tiered_query(TieredQuery { project_id: "p", terms: "auth", limit: 10 })
            .await
            .unwrap();
        assert!(hits.iter().any(|h| matches!(h, TieredResult::Related(r) if r.uid == b.uid)));
    }

    #[tokio::test]
    async fn bfs_hops_and_cycles() {
        let (store, _d) = temp_store().await;
        // Chain A -> B -> C, plus a cycle C -> A.
        store.write_relationships("A", &[("B".into(), "uses".into())]).await.unwrap();
        store.write_relationships("B", &[("C".into(), "uses".into())]).await.unwrap();
        store.write_relationships("C", &[("A".into(), "uses".into())]).await.unwrap();
        // Records must exist to be returned.
        for id in ["A", "B", "C"] {
            store.db.query("CREATE type::thing('memory', $u) SET uid = $u, user_id = 'me', project_id = 'p', text = '', tags = [], created_at = '', curation_state = 'auto', source_type = '', source_ref = '', summary = '', tier = '', lifecycle = '', provenance = ''")
                .bind(("u", id.to_string())).await.unwrap().check().unwrap();
        }
        let zero = store.related_by_bfs(&["A".into()], 0).await.unwrap();
        assert!(zero.is_empty());
        let one: Vec<_> = store.related_by_bfs(&["A".into()], 1).await.unwrap().iter().map(|r| r.uid.clone()).collect();
        assert_eq!(one, vec!["B".to_string()]);
        let two: Vec<_> = store.related_by_bfs(&["A".into()], 2).await.unwrap().iter().map(|r| r.uid.clone()).collect();
        assert_eq!(two, vec!["B".to_string(), "C".to_string()]);
        // Deep walk terminates despite the C -> A cycle (A is a seed, never re-added).
        let deep = store.related_by_bfs(&["A".into()], 9).await.unwrap();
        assert_eq!(deep.len(), 2); // B and C only
    }

    #[tokio::test]
    async fn wikilink_resolves_to_indexed_uid() {
        let (store, _d) = temp_store().await;
        // Index "Auth Flow" -> 01TARGET, then a link to it records the resolved uid.
        store.upsert_name_index("01TARGET", "Auth Flow").await.unwrap();
        store.write_wikilinks("01SRC", &[link("Auth Flow"), link("Unknown")]).await.unwrap();
        #[derive(serde::Deserialize)]
        struct R {
            display_name: String,
            resolved_uid: String,
        }
        let mut res = store
            .db
            .query("SELECT display_name, resolved_uid FROM vault_wikilink WHERE source_uid = $s")
            .bind(("s", "01SRC".to_string()))
            .await
            .unwrap();
        let rows: Vec<R> = res.take(0).unwrap();
        let auth = rows.iter().find(|r| r.display_name == "Auth Flow").unwrap();
        assert_eq!(auth.resolved_uid, "01TARGET");
        let unknown = rows.iter().find(|r| r.display_name == "Unknown").unwrap();
        assert_eq!(unknown.resolved_uid, ""); // unresolved → empty, not fatal
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
