# ADR-0001: Wedge hub runs SurrealDB on a Deno/TS service

**Date:** 2026-06-15
**Status:** Accepted

## Context

The wedge (`platform/specs/001-shared-runs-and-learnings`) needs a hub datastore + service
runtime. Three facts were not weighed when the wedge spec settled on TypeScript/Hono + SQLite
(FTS5) on Node ≥ 20:

1. The **edge already runs embedded SurrealDB** (SurrealKV, pure-Rust, in-process) with a
   BM25 full-text analyzer doing tag-filtered, recency-ordered recall — i.e. the wedge's
   FR-013 recall already exists in carried code (`apps/wagner/src-tauri/src/memory.rs`).
2. The persistence research (`docs/research/agent-memory-persistence-research.md`)
   recorded SurrealDB as the engineer's choice **specifically for the central-server path**
   ("same engine/schema scales to a central server"). The wedge's SQLite choice silently
   reversed that, and its rejected-alternatives analysis never named SurrealDB.
3. A SQLite hub would run a **second** storage engine with **different recall-ranking
   semantics** than the edge — for a system whose thesis is *shared recall*, edge and hub
   ranking diverging is a correctness smell.

Identity/auth mechanism is a separate decision (see ADR-0002, pending).

## Decision

The wedge hub uses **SurrealDB**, reached from a **Deno + Hono (TypeScript)** service via the
SurrealDB JS SDK (SurrealDB runs as a server process). The edge keeps its carried embedded
SurrealKV store (Article VI offline autonomy; D-PROJ-4 no-rewrite). All edge↔hub payloads stay
JSON-Schema-validated (Article X) — schema sharing is language-neutral, so a TS hub and a Rust
edge interoperate via `platform/shared` schemas. Hub recall is re-implemented in the SDK but
**mirrors the edge's `wagner_en` BM25 analyzer + index**, so ranking matches both sides.

## Alternatives Considered

- **SQLite + FTS5 (the wedge spec's default).** Rejected: a second engine, recall ranking
  that diverges from the edge, and a silent reversal of the research-doc SurrealDB-central
  design with no analysis naming it.
- **Rust hub embedding SurrealKV (reuse `memory.rs` verbatim).** Rejected: it would add the
  only non-Tauri Rust component to the system to save ~20 lines of carried recall, splitting
  the hub's language away from the already-TypeScript edge UI. The one-engine benefit survives
  a TS hub anyway (same SurrealDB + same analyzer ⇒ same ranking); only code reuse is traded.
- **Node runtime.** Superseded by the engineer's Deno preference; Hono runs on both, so this
  is a free swap. (Note: "Deno is Rust-based" does **not** make the carried Rust store cheaply
  callable — FFI/WASM wrapping of an async SurrealDB store is more work than re-writing the
  small recall query in the SDK; the Deno choice stands on DX + TS-consistency, not Rust reuse.)

## Consequences

- **Easier:** one storage-engine family across edge + hub; recall ranking matches both sides;
  the hub shares TypeScript with the edge UI and shared reducer; the SurrealDB-central path the
  research designed is realized rather than reversed.
- **Harder / accepted:** the hub is **two cooperating processes** (the Deno service + a
  SurrealDB server) rather than a single binary — still a "small hub" under the constitution
  (no message broker, no queue, no Temporal); ~20 lines of carried recall are re-implemented in
  the SDK. SurrealDB's relative immaturity is explicitly **not** a concern here — Wagner is an
  internal tool, not a production-hardened external product (engineer, 2026-06-15).
- **Edge autonomy preserved:** the hub/SurrealDB are touched only at the sync/recall boundary;
  with the hub unreachable the run still completes locally, sync queues, recall returns empty.

## Supersedes

Plan `R-2` (SQLite/FTS5 recall) and the Technical-Context "SQLite / Node ≥ 20 / better-sqlite3"
rows for the hub. The wedge spec/plan/tasks are updated by a follow-on `/spec amend`.
