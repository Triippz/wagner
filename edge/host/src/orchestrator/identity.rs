//! Agent identity catalog (Phase 1 — Rust catalog from repo + global + plugins).
//!
//! Wagner runs in the context of a target repository (FR-007), so the agents the
//! engineer already maintains there — `.claude/agents/*.md` and `agents/*.md`,
//! each a markdown file with `name`/`description` frontmatter and a `## Identity`
//! block — become the catalog of operatives you can hire. Picking an identity
//! seeds a roster `Agent`'s display name, role, and standing skill prompt; the
//! engineer still assigns the harness (engine) per agent.
//!
//! Skills are now a **separate** catalog (`SkillRef`), no longer injected as fake
//! agent operatives. `scan_catalog` returns agents only; `scan_skills` returns skills.
//!
//! Pure parsing + filesystem scans with a built-in fallback. No network, no I/O
//! beyond reading the agent/skill files.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ---- types ----------------------------------------------------------------

/// One selectable operative identity, distilled from an agent markdown file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentIdentity {
    /// Stable id — the frontmatter `name` (e.g. `code-reviewer`).
    pub id: String,
    /// Human display name — the first bold token in the `## Identity` block
    /// (e.g. **Code Quality Reviewer**), falling back to a title-cased id.
    pub name: String,
    /// One-line role — the frontmatter `description`.
    pub role: String,
    /// Standing instructions seeded into the agent's skill prompt — the
    /// `**Mandate:**` paragraph if present, else the first Identity paragraph,
    /// else the role.
    pub mandate: String,
    /// Where it came from, relative to the project dir (provenance for the UI).
    /// For global/plugin agents the prefix is `global:` or `plugin:<name>:`.
    pub source: String,
}

/// A discoverable skill — parsed from a `SKILL.md` file in any of the three
/// scan roots (repo, global, plugin). Skills are NOT agents; they are loaded
/// *onto* agents in the roster.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillRef {
    /// Stable id — the frontmatter `name` (e.g. `check-rust`).
    pub id: String,
    /// Display name — title-cased id (skills carry no separate human name).
    pub name: String,
    /// One-line description — frontmatter `description`.
    pub description: String,
    /// The slash command that invokes this skill (e.g. `/check-rust`).
    pub command: String,
    /// Where the SKILL.md lives (relative or absolute path).
    pub source: String,
    /// Provenance: `"repo"`, `"global"`, or `"plugin:<plugin-name>"`.
    pub origin: String,
}

// ---- skip-list for agent meta docs ----------------------------------------

/// Files that are repo meta, not agent identities.
const SKIP_FILES: &[&str] = &["CLAUDE.md", "AGENTS.md", "identity.md", "README.md"];

// ---- public API -----------------------------------------------------------

/// Parse one agent markdown file into an identity. Returns `None` when the file
/// has no usable `name` frontmatter (e.g. a meta doc).
pub fn parse_identity(markdown: &str, source: &str) -> Option<AgentIdentity> {
    let (frontmatter, body) = split_frontmatter(markdown)?;
    let id = frontmatter_value(frontmatter, "name")?;
    if id.is_empty() {
        return None;
    }
    let role = frontmatter_value(frontmatter, "description").unwrap_or_default();
    let identity = section(body, "## Identity");
    let name = identity
        .as_deref()
        .and_then(first_bold)
        .unwrap_or_else(|| title_case(&id));
    let mandate = identity
        .as_deref()
        .and_then(mandate_paragraph)
        .or_else(|| identity.as_deref().and_then(first_paragraph))
        .unwrap_or_else(|| role.clone());
    Some(AgentIdentity {
        id,
        name,
        role,
        mandate,
        source: source.to_string(),
    })
}

/// Scan a project directory for hireable operative identities. Reads agents from:
/// 1. Repo: `.claude/agents/*.md` and `agents/*.md`
/// 2. Global: `~/.claude/agents/*.md`
/// 3. Plugins: `~/.claude/plugins/**/agents/*.md`
///
/// De-duped by id (repo wins over global wins over plugin); falls back to the
/// built-in catalog when no agents are found anywhere.
pub fn scan_catalog(project_dir: &Path) -> Vec<AgentIdentity> {
    let mut out: Vec<AgentIdentity> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // 1. Repo agents
    scan_agents_from_roots(
        project_dir,
        &[".claude/agents", "agents"],
        "repo",
        &mut out,
        &mut seen,
    );

    // 2. Global + plugin agents (best-effort; HOME may not be set)
    if let Ok(home) = std::env::var("HOME") {
        let home = PathBuf::from(&home);

        // Global ~/.claude/agents
        scan_agents_from_dir_with_prefix(
            &home.join(".claude").join("agents"),
            "global",
            &mut out,
            &mut seen,
        );

        // Plugin agents: ~/.claude/plugins/*/agents and ~/.claude/plugins/*/*/agents
        let plugins_root = home.join(".claude").join("plugins");
        for depth in [1usize, 2] {
            let Ok(top) = std::fs::read_dir(&plugins_root) else {
                break;
            };
            for entry in top.flatten() {
                let plugin_dir = entry.path();
                if !plugin_dir.is_dir() {
                    continue;
                }
                if depth == 1 {
                    let plugin_name = dir_name(&plugin_dir);
                    let origin = format!("plugin:{plugin_name}");
                    scan_agents_from_dir_with_prefix(
                        &plugin_dir.join("agents"),
                        &origin,
                        &mut out,
                        &mut seen,
                    );
                } else {
                    // depth 2: marketplace layout (namespace/plugin-name)
                    let Ok(children) = std::fs::read_dir(&plugin_dir) else {
                        continue;
                    };
                    for child in children.flatten() {
                        let child_dir = child.path();
                        if !child_dir.is_dir() {
                            continue;
                        }
                        let plugin_name = format!("{}/{}", dir_name(&plugin_dir), dir_name(&child_dir));
                        let origin = format!("plugin:{plugin_name}");
                        scan_agents_from_dir_with_prefix(
                            &child_dir.join("agents"),
                            &origin,
                            &mut out,
                            &mut seen,
                        );
                    }
                }
            }
        }
    }

    if out.is_empty() {
        return default_catalog();
    }
    out
}

/// Scan a project directory for available skills. Reads from:
/// 1. Repo: `<project>/.claude/skills/<name>/SKILL.md`, `<project>/skills/<name>/SKILL.md`,
///    `<project>/.agents/skills/<name>/SKILL.md` (Codex mirror)
/// 2. Global: `~/.claude/skills/<name>/SKILL.md`, `~/.codex/skills/<name>/SKILL.md`
/// 3. Plugins: `~/.claude/plugins/**/skills/<name>/SKILL.md`
///
/// De-duped by id (repo wins over global wins over plugin; first occurrence wins).
pub fn scan_skills(project_dir: &Path) -> Vec<SkillRef> {
    let mut roots: Vec<(PathBuf, String)> = Vec::new();

    // Repo roots
    for rel in [".claude/skills", "skills", ".agents/skills"] {
        roots.push((project_dir.join(rel), "repo".to_string()));
    }

    // Global + plugin roots
    if let Ok(home) = std::env::var("HOME") {
        let home = PathBuf::from(&home);

        // ~/.claude/skills
        roots.push((home.join(".claude").join("skills"), "global".to_string()));

        // ~/.codex/skills (Codex global layout)
        roots.push((home.join(".codex").join("skills"), "global".to_string()));

        // ~/.codex/**/skills — one level of nesting (keep it simple)
        if let Ok(entries) = std::fs::read_dir(home.join(".codex")) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    let candidate = p.join("skills");
                    if candidate.is_dir() {
                        roots.push((candidate, "global".to_string()));
                    }
                }
            }
        }

        // Plugin skills: ~/.claude/plugins/*/skills and ~/.claude/plugins/*/*/skills
        let plugins_root = home.join(".claude").join("plugins");
        collect_plugin_skill_roots(&plugins_root, &mut roots);
    }

    scan_skills_from_roots(&roots)
}

/// Core (pure, testable) skill scanner over an explicit list of `(root, origin)` pairs.
/// Each root is scanned for `<name>/SKILL.md` subdirectories.
/// De-duped by id; first root in the list wins.
pub fn scan_skills_from_roots(roots: &[(PathBuf, String)]) -> Vec<SkillRef> {
    let mut out: Vec<SkillRef> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for (root, origin) in roots {
        let Ok(entries) = std::fs::read_dir(root) else {
            continue;
        };
        let mut subdirs: Vec<_> = entries.flatten().map(|e| e.path()).collect();
        subdirs.sort(); // deterministic order
        for sub in subdirs {
            if !sub.is_dir() {
                continue;
            }
            let skill_md = sub.join("SKILL.md");
            let Ok(text) = std::fs::read_to_string(&skill_md) else {
                continue;
            };
            let Some((frontmatter, _)) = split_frontmatter(&text) else {
                continue;
            };
            let Some(id) = frontmatter_value(frontmatter, "name").filter(|s| !s.is_empty()) else {
                continue;
            };
            if !seen.insert(id.clone()) {
                continue; // de-dup: first wins
            }
            let description = frontmatter_value(frontmatter, "description").unwrap_or_default();
            let name = title_case(&id);
            let command = format!("/{id}");
            let source = skill_md.to_string_lossy().to_string();
            out.push(SkillRef {
                id,
                name,
                description,
                command,
                source,
                origin: origin.clone(),
            });
        }
    }
    out
}

/// The built-in fallback catalog — the default Architect/Forger pair, so the
/// roster editor always has something to offer even against a bare repo.
pub fn default_catalog() -> Vec<AgentIdentity> {
    vec![
        AgentIdentity {
            id: "cipher".into(),
            name: "Cipher".into(),
            role: "Architect — planning, tests, judgement".into(),
            mandate: "Decompose the goal, design the tests, and judge whether the goal is met."
                .into(),
            source: "built-in".into(),
        },
        AgentIdentity {
            id: "vex".into(),
            name: "Vex".into(),
            role: "Forger — scoped implementation".into(),
            mandate: "Implement the scoped subtask you are assigned and nothing beyond it.".into(),
            source: "built-in".into(),
        },
    ]
}

// ---- private helpers ------------------------------------------------------

/// Scan agent `.md` files in a set of relative subdirs under `base`, tagging each
/// with a prefix built from `origin_hint`.
fn scan_agents_from_roots(
    base: &Path,
    rels: &[&str],
    origin_hint: &str,
    out: &mut Vec<AgentIdentity>,
    seen: &mut std::collections::HashSet<String>,
) {
    for rel in rels {
        let dir = base.join(rel);
        let source_prefix = if origin_hint == "repo" {
            rel.to_string()
        } else {
            format!("{origin_hint}:{rel}")
        };
        scan_agents_from_dir_with_prefix(&dir, &source_prefix, out, seen);
    }
}

/// Scan a single agents dir. `source_prefix` is prepended to the filename for provenance.
fn scan_agents_from_dir_with_prefix(
    dir: &Path,
    source_prefix: &str,
    out: &mut Vec<AgentIdentity>,
    seen: &mut std::collections::HashSet<String>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    files.sort(); // deterministic order
    for path in files {
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let fname = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
        if SKIP_FILES.contains(&fname) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let src = format!("{source_prefix}/{fname}");
        if let Some(identity) = parse_identity(&text, &src) {
            if seen.insert(identity.id.clone()) {
                out.push(identity);
            }
        }
    }
}

/// Collect skill roots from `~/.claude/plugins/` at depth 1 and 2.
fn collect_plugin_skill_roots(plugins_root: &Path, roots: &mut Vec<(PathBuf, String)>) {
    let Ok(top) = std::fs::read_dir(plugins_root) else {
        return;
    };
    for entry in top.flatten() {
        let plugin_dir = entry.path();
        if !plugin_dir.is_dir() {
            continue;
        }
        let plugin_name = dir_name(&plugin_dir);

        // depth 1: plugins/<plugin-name>/skills
        let skill_dir = plugin_dir.join("skills");
        if skill_dir.is_dir() {
            roots.push((skill_dir, format!("plugin:{plugin_name}")));
        }

        // depth 2: plugins/<ns>/<plugin-name>/skills (marketplace layout)
        let Ok(children) = std::fs::read_dir(&plugin_dir) else {
            continue;
        };
        for child in children.flatten() {
            let child_dir = child.path();
            if !child_dir.is_dir() {
                continue;
            }
            let child_name = format!("{plugin_name}/{}", dir_name(&child_dir));
            let skill_dir2 = child_dir.join("skills");
            if skill_dir2.is_dir() {
                roots.push((skill_dir2, format!("plugin:{child_name}")));
            }
        }
    }
}

/// Last path component as a string, or "unknown".
fn dir_name(p: &Path) -> String {
    p.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string()
}

// ---- parsing helpers (pure) -----------------------------------------------

/// Split `---\n<frontmatter>\n---\n<body>`; `None` if no leading frontmatter.
fn split_frontmatter(md: &str) -> Option<(&str, &str)> {
    let rest = md.strip_prefix("---")?;
    // tolerate a leading newline (LF or CRLF) after the opening fence
    let rest = rest.trim_start_matches(['\r', '\n']);
    // the closing fence: "\n---" also matches the "\n" inside a "\r\n---" sequence
    let end = rest.find("\n---")?;
    let frontmatter = &rest[..end];
    let after = &rest[end + 4..]; // past "\n---"
    let body = after.trim_start_matches(['\r', '\n']);
    Some((frontmatter, body))
}

/// Read a `key: value` from frontmatter, trimming surrounding quotes.
fn frontmatter_value(frontmatter: &str, key: &str) -> Option<String> {
    for line in frontmatter.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(key) {
            if let Some(val) = rest.trim_start().strip_prefix(':') {
                let v = val.trim().trim_matches('"').trim_matches('\'').trim();
                return Some(v.to_string());
            }
        }
    }
    None
}

/// Extract the body of a `## Heading` section up to the next `## ` heading.
fn section(body: &str, heading: &str) -> Option<String> {
    let start = body.find(heading)?;
    let after = &body[start + heading.len()..];
    let after = after.strip_prefix('\n').unwrap_or(after);
    let end = after.find("\n## ").unwrap_or(after.len());
    Some(after[..end].trim().to_string())
}

/// First `**bold**` token in a string.
fn first_bold(s: &str) -> Option<String> {
    let open = s.find("**")?;
    let after = &s[open + 2..];
    let close = after.find("**")?;
    let token = after[..close].trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

/// The paragraph beginning with `**Mandate:**`, with the label stripped.
fn mandate_paragraph(s: &str) -> Option<String> {
    for para in s.split("\n\n") {
        let p = para.trim();
        if let Some(rest) = p.strip_prefix("**Mandate:**") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

/// First non-empty paragraph.
fn first_paragraph(s: &str) -> Option<String> {
    s.split("\n\n")
        .map(str::trim)
        .find(|p| !p.is_empty())
        .map(str::to_string)
}

/// `code-reviewer` → `Code Reviewer`.
fn title_case(id: &str) -> String {
    id.split(['-', '_'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// ---- tests ----------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const CODE_REVIEWER: &str = "---\nname: code-reviewer\ndescription: Code quality reviewer that checks SOLID principles\n---\n\n# Code Reviewer Agent\n\n<role>You are the reviewer.</role>\n\n## Identity\n\nYou are the **Code Quality Reviewer** — you read code the way the next developer will.\n\n**Mandate:** Own code-level quality — SOLID, naming, function size — on the changed diff only.\n\n**Voice:** Blunt.\n\n## Context\n\nMore stuff.\n";

    // ---- existing agent-parsing tests (must stay green) -------------------

    #[test]
    fn parses_name_role_display_and_mandate() {
        let id = parse_identity(CODE_REVIEWER, "agents/code-reviewer.md").unwrap();
        assert_eq!(id.id, "code-reviewer");
        assert_eq!(id.name, "Code Quality Reviewer");
        assert_eq!(
            id.role,
            "Code quality reviewer that checks SOLID principles"
        );
        assert!(id.mandate.starts_with("Own code-level quality"));
        assert!(
            !id.mandate.contains("Voice"),
            "mandate stops at its paragraph"
        );
        assert_eq!(id.source, "agents/code-reviewer.md");
    }

    #[test]
    fn falls_back_to_title_cased_id_when_no_bold_token() {
        let md = "---\nname: test-automator\ndescription: Finds coverage gaps\n---\n\n## Identity\n\nNo bold here, just prose.\n";
        let id = parse_identity(md, "agents/test-automator.md").unwrap();
        assert_eq!(id.name, "Test Automator");
        assert_eq!(id.mandate, "No bold here, just prose.");
    }

    #[test]
    fn rejects_files_without_name_frontmatter() {
        assert!(parse_identity("# Just a doc\n\nNo frontmatter.", "agents/CLAUDE.md").is_none());
        assert!(parse_identity("---\ndescription: x\n---\nbody", "x.md").is_none());
    }

    #[test]
    fn mandate_falls_back_to_description_without_identity_block() {
        let md = "---\nname: lonely\ndescription: A solo role\n---\n\n# Lonely\n\nNo identity section.\n";
        let id = parse_identity(md, "agents/lonely.md").unwrap();
        assert_eq!(id.mandate, "A solo role");
    }

    #[test]
    fn scan_returns_builtin_catalog_for_a_bare_dir() {
        let dir = std::env::temp_dir().join(format!("wagner-cat-empty-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let cat = scan_catalog(&dir);
        assert_eq!(cat.len(), 2);
        assert!(cat.iter().any(|i| i.id == "cipher"));
        assert!(cat.iter().any(|i| i.id == "vex"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn scan_reads_agent_files_and_dedups_across_dirs() {
        let root = std::env::temp_dir().join(format!("wagner-cat-{}", std::process::id()));
        let dot = root.join(".claude/agents");
        let plain = root.join("agents");
        std::fs::create_dir_all(&dot).unwrap();
        std::fs::create_dir_all(&plain).unwrap();
        std::fs::write(dot.join("code-reviewer.md"), CODE_REVIEWER).unwrap();
        // same id in the second dir — must not duplicate
        std::fs::write(plain.join("code-reviewer.md"), CODE_REVIEWER).unwrap();
        std::fs::write(
            plain.join("debugger.md"),
            "---\nname: debugger\ndescription: Bug hunter\n---\n\n## Identity\n\nYou are the **Debugger**.\n",
        )
        .unwrap();
        std::fs::write(plain.join("CLAUDE.md"), "# meta, must be skipped").unwrap();

        let cat = scan_catalog(&root);
        let ids: Vec<_> = cat.iter().map(|i| i.id.as_str()).collect();
        assert!(ids.contains(&"code-reviewer"));
        assert!(ids.contains(&"debugger"));
        assert_eq!(
            ids.iter().filter(|i| **i == "code-reviewer").count(),
            1,
            "deduped across .claude/agents and agents"
        );
        assert!(!ids.contains(&"CLAUDE"));
        let _ = std::fs::remove_dir_all(&root);
    }

    // ---- NEW: scan_catalog must NOT contain skill ids ----------------------

    #[test]
    fn scan_catalog_does_not_contain_skill_ids() {
        // A repo with only skills and no agents → should return the built-in catalog,
        // not skills masquerading as agents.
        let root = std::env::temp_dir().join(format!("wagner-cat-noskill-{}", std::process::id()));
        let skills = root.join("skills");
        std::fs::create_dir_all(skills.join("check-rust")).unwrap();
        std::fs::write(
            skills.join("check-rust/SKILL.md"),
            "---\nname: check-rust\ndescription: Scan Rust for non-idiomatic patterns.\n---\n# body\n",
        )
        .unwrap();

        let cat = scan_catalog(&root);
        // No skill id should appear as an agent id
        assert!(
            !cat.iter().any(|i| i.id == "check-rust"),
            "check-rust skill must not appear in agent catalog"
        );
        // With no real agents, falls back to built-in
        assert!(cat.iter().any(|i| i.id == "cipher"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn scan_catalog_agents_and_skills_coexist_without_mixing() {
        let root =
            std::env::temp_dir().join(format!("wagner-cat-mixed-{}", std::process::id()));
        let agents = root.join(".claude/agents");
        let skills = root.join("skills");
        std::fs::create_dir_all(&agents).unwrap();
        std::fs::create_dir_all(skills.join("tdd")).unwrap();
        std::fs::write(agents.join("code-reviewer.md"), CODE_REVIEWER).unwrap();
        std::fs::write(
            skills.join("tdd/SKILL.md"),
            "---\nname: tdd\ndescription: Tests before implementation.\n---\n# body\n",
        )
        .unwrap();

        let cat = scan_catalog(&root);
        // Agent present, skill absent from agent catalog
        assert!(cat.iter().any(|i| i.id == "code-reviewer"));
        assert!(
            !cat.iter().any(|i| i.id == "tdd"),
            "tdd skill must not appear in agent catalog"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    // ---- NEW: SkillRef tests -----------------------------------------------

    #[test]
    fn skill_ref_has_correct_command_and_no_codename() {
        let root = std::env::temp_dir().join(format!("wagner-skill-cmd-{}", std::process::id()));
        let skills = root.join("skills");
        std::fs::create_dir_all(skills.join("check-rust")).unwrap();
        std::fs::write(
            skills.join("check-rust/SKILL.md"),
            "---\nname: check-rust\ndescription: Scan Rust for non-idiomatic patterns.\n---\n# body\n",
        )
        .unwrap();

        let roots = vec![(skills, "repo".to_string())];
        let skills = scan_skills_from_roots(&roots);
        let skill = skills.iter().find(|s| s.id == "check-rust").expect("check-rust found");

        assert_eq!(skill.command, "/check-rust", "command must be /<id>");
        assert_eq!(skill.name, "Check Rust", "name is title-cased id, no codename");
        assert_eq!(skill.description, "Scan Rust for non-idiomatic patterns.");
        assert_eq!(skill.origin, "repo");
    }

    #[test]
    fn skill_ref_id_not_in_scan_catalog() {
        // Verify the two catalogs are fully disjoint on a repo with both.
        // Use IDs that are guaranteed not to exist in any real global/plugin dirs.
        let root = std::env::temp_dir().join(format!("wagner-disjoint-{}", std::process::id()));
        let agents_dir = root.join("agents");
        let skills_dir = root.join("skills");
        std::fs::create_dir_all(&agents_dir).unwrap();
        std::fs::create_dir_all(skills_dir.join("xtest-unique-skill-z9q")).unwrap();
        // Agent: use an id that will never appear as a skill in ~/.codex/skills or plugins
        std::fs::write(
            agents_dir.join("xtest-unique-agent-z9q.md"),
            "---\nname: xtest-unique-agent-z9q\ndescription: Test-only operative.\n---\n## Identity\n**Test Agent**\n",
        )
        .unwrap();
        std::fs::write(
            skills_dir.join("xtest-unique-skill-z9q/SKILL.md"),
            "---\nname: xtest-unique-skill-z9q\ndescription: Test-only skill.\n---\n# body\n",
        )
        .unwrap();

        let agents = scan_catalog(&root);
        let skills = scan_skills(&root);

        let agent_ids: std::collections::HashSet<_> = agents.iter().map(|a| a.id.as_str()).collect();
        let skill_ids: std::collections::HashSet<_> = skills.iter().map(|s| s.id.as_str()).collect();
        assert!(
            agent_ids.is_disjoint(&skill_ids),
            "agent and skill catalogs must be disjoint; overlap: {:?}",
            agent_ids.intersection(&skill_ids).collect::<Vec<_>>()
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn scan_skills_from_roots_deduplicates_by_id_first_wins() {
        let base = std::env::temp_dir().join(format!("wagner-skilldup-{}", std::process::id()));
        let repo_skills = base.join("repo_skills");
        let global_skills = base.join("global_skills");
        let codex_skills = base.join("codex_skills");

        // tdd exists in all three roots; repo must win
        for dir in [&repo_skills, &global_skills, &codex_skills] {
            std::fs::create_dir_all(dir.join("tdd")).unwrap();
            std::fs::write(
                dir.join("tdd/SKILL.md"),
                format!(
                    "---\nname: tdd\ndescription: From {}.\n---\n# body\n",
                    dir.file_name().unwrap().to_str().unwrap()
                ),
            )
            .unwrap();
        }
        // check-rust only in global
        std::fs::create_dir_all(global_skills.join("check-rust")).unwrap();
        std::fs::write(
            global_skills.join("check-rust/SKILL.md"),
            "---\nname: check-rust\ndescription: Global Rust review.\n---\n# body\n",
        )
        .unwrap();

        let roots = vec![
            (repo_skills, "repo".to_string()),
            (global_skills, "global".to_string()),
            (codex_skills, "global".to_string()),
        ];
        let skills = scan_skills_from_roots(&roots);

        // tdd appears exactly once
        assert_eq!(
            skills.iter().filter(|s| s.id == "tdd").count(),
            1,
            "tdd must be de-duped to one entry"
        );
        // repo wins: description from repo_skills
        let tdd = skills.iter().find(|s| s.id == "tdd").unwrap();
        assert_eq!(tdd.origin, "repo");
        assert!(tdd.description.contains("repo_skills"), "repo root wins, got: {}", tdd.description);

        // check-rust present from global
        assert!(skills.iter().any(|s| s.id == "check-rust"));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn scan_skills_from_roots_reads_agents_skills_codex_mirror() {
        // .agents/skills is the Codex mirror root; it must be scanned
        let root = std::env::temp_dir().join(format!("wagner-codex-{}", std::process::id()));
        let agents_skills = root.join(".agents/skills");
        std::fs::create_dir_all(agents_skills.join("workspace-map")).unwrap();
        std::fs::write(
            agents_skills.join("workspace-map/SKILL.md"),
            "---\nname: workspace-map\ndescription: Projects ICM routing map.\n---\n# body\n",
        )
        .unwrap();

        let roots = vec![(agents_skills, "repo".to_string())];
        let skills = scan_skills_from_roots(&roots);
        assert!(skills.iter().any(|s| s.id == "workspace-map"), "Codex mirror skill found");
        let s = skills.iter().find(|s| s.id == "workspace-map").unwrap();
        assert_eq!(s.command, "/workspace-map");
        assert_eq!(s.origin, "repo");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn scan_skills_from_roots_plugin_origin_tagged_correctly() {
        let base = std::env::temp_dir().join(format!("wagner-plugin-{}", std::process::id()));
        let plugin_skills = base.join("my-plugin").join("skills");
        std::fs::create_dir_all(plugin_skills.join("check-ucan")).unwrap();
        std::fs::write(
            plugin_skills.join("check-ucan/SKILL.md"),
            "---\nname: check-ucan\ndescription: UCAN auth review.\n---\n# body\n",
        )
        .unwrap();

        let roots = vec![(plugin_skills, "plugin:my-plugin".to_string())];
        let skills = scan_skills_from_roots(&roots);
        let s = skills.iter().find(|s| s.id == "check-ucan").unwrap();
        assert_eq!(s.origin, "plugin:my-plugin");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn scan_skills_from_roots_skips_dirs_without_skill_md() {
        let root = std::env::temp_dir().join(format!("wagner-noskillmd-{}", std::process::id()));
        let skills_dir = root.join("skills");
        std::fs::create_dir_all(skills_dir.join("no-skill-md")).unwrap();
        // No SKILL.md inside — must be skipped
        std::fs::write(skills_dir.join("no-skill-md/README.md"), "# not a skill").unwrap();

        let roots = vec![(skills_dir, "repo".to_string())];
        let skills = scan_skills_from_roots(&roots);
        assert!(skills.is_empty(), "dir without SKILL.md must be skipped");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn scan_skills_tolerates_unreadable_roots() {
        // Passing a nonexistent root must not panic; just yields empty
        let roots = vec![(PathBuf::from("/no/such/path/skills"), "repo".to_string())];
        let skills = scan_skills_from_roots(&roots);
        assert!(skills.is_empty());
    }

    #[test]
    fn skill_name_is_title_cased_not_random_codename() {
        // Regression: skills must use title_case(id), never a codename pool entry
        let root = std::env::temp_dir().join(format!("wagner-title-{}", std::process::id()));
        let skills_dir = root.join("skills");
        std::fs::create_dir_all(skills_dir.join("my-skill")).unwrap();
        std::fs::write(
            skills_dir.join("my-skill/SKILL.md"),
            "---\nname: my-skill\ndescription: Does things.\n---\n# body\n",
        )
        .unwrap();

        let roots = vec![(skills_dir, "repo".to_string())];
        let skills = scan_skills_from_roots(&roots);
        let s = skills.iter().find(|s| s.id == "my-skill").unwrap();
        assert_eq!(s.name, "My Skill");
        let _ = std::fs::remove_dir_all(&root);
    }

    // ---- old test that expected skills-as-agents: REPLACED ----------------
    // The old test `scan_turns_skills_into_named_operatives_that_invoke_the_skill`
    // tested the now-removed conflation. It is replaced by:
    // - scan_catalog_does_not_contain_skill_ids
    // - scan_catalog_agents_and_skills_coexist_without_mixing
    // - skill_ref_has_correct_command_and_no_codename
}
