# Wagner Platform Spec Constitution

**Version:** 0.1.0
**Ratified:** 2026-06-14
**Last Amended:** 2026-06-14
**Maintainer:** Mark Tripoli (mtripoli@adyton.io)

## Purpose

The Wagner platform is the engineering-org runtime layer of the dev-ai-utilities monorepo:
an **edge** harness that runs autonomous `claude`/`codex` coding agents locally on an
operator's subscription CLIs, and a **small hub** that remembers across runs and people
(shared event store, recall index, identity, and later an always-on worker tier). Everything
the platform does is expressed as a `run` (goal + workflow + agents + guardrails + event
stream + outcome + learnings). The platform *consumes* the capability library
(`skills/`, `agents/`, `commands/`, `hooks/`, `plugins/`, `profiles/`) and never the reverse.
This constitution governs platform specs only; the library has its own at `docs/spec/`.
Full vision and architecture: `platform/prd.md`.

## Articles

Articles are non-negotiable. They use RFC 2119 keywords (MUST, MUST NOT, SHOULD, SHOULD NOT,
MAY). The first three articles are required by this repo and may not be removed; they only
ever expand. Articles VI–X are the platform's defining non-negotiables and descend directly
from `platform/prd.md`.

### Article I — Test-First (NON-NEGOTIABLE)

Production code MUST NOT be written before a failing test exists. The Red-Green-Refactor cycle
is enforced. The /tdd skill is the implementation of this article.

**Why:** Without this article, AI-generated code is reviewed against a specification it has
already shaped its own assumptions around. Tests written first are the only defensible record
of what behaviour was actually asked for.

**How to verify:** spec-validator inspects each task in tasks.md and confirms test tasks
precede implementation tasks for the same behaviour.

### Article II — Specs Are Evidence-Driven

Every requirement, constraint, performance target, and architectural choice MUST be either
(a) quantified, (b) cited to a source (file:line, RFC, benchmark, prior incident, or
stakeholder decision), or (c) marked as an explicit Assumption with the engineer's rationale.
Vague adjectives (fast, scalable, secure, robust, intuitive, lightweight, modern, simple,
efficient) MUST NOT appear without an adjacent quantification.

**Why:** Vague language is the single largest source of agent misinterpretation. "Fast"
cannot generate code; "p95 latency under 150 ms at 1000 RPS" can.

**How to verify:** spec-challenger runs a vague-adjective scan over spec.md and reports each
violation as a HIGH finding.

### Article III — Hard-Fail on CRITICAL Findings

A spec MUST NOT advance to implementation while any CRITICAL finding from spec-challenger or
spec-validator is unresolved. CRITICAL findings include: constitution-article violations
without recorded Complexity Tracking justification; functional requirements with zero task
coverage; user stories without an Independent Test definition; contradictions between
spec.md, plan.md, and tasks.md.

**Why:** Soft-failing CRITICAL findings is how SDD degrades into vibe coding with extra
paperwork.

**How to verify:** spec-validator's verdict is `BLOCKED` if any CRITICAL is open.

### Article IV — Independent User Stories

Every user story prioritised P1 MUST be independently testable: implementing only that story
MUST yield a viable MVP that delivers the user the value the spec promises.

**Why:** A platform built as one monolithic story cannot ship the wedge before the workers.
Independent stories are what let Phase 1 deliver value with no Phase 2 present.

**How to verify:** spec-validator confirms each P1 user story has a non-empty Independent
Test field stated as a concrete action + value-delivered.

### Article V — Simplicity Gate

The implementation MUST use the smallest project structure that solves the problem. Adding a
new project, library, service, or external engine (e.g. Temporal, a new datastore) requires a
recorded Complexity Tracking entry with a rejected-alternative analysis.

**Why:** "Small hub" is a platform requirement (Article VI). Every service, queue, or engine
added to the hub is permanent operational surface. The wedge in particular MUST NOT pull in
Temporal or a managed datastore it does not yet need.

**How to verify:** plan.md's Project Structure uses the chosen structure unmodified, OR
Complexity Tracking has an entry per added piece with a rejected alternative.

### Article VI — Edge Executes, Hub Remembers (NON-NEGOTIABLE)

Edge runs MUST execute locally and MUST be able to start, progress, and complete with **no hub
reachable**. The hub MUST NOT execute edge-run work; its role is memory and coordination
(event store, recall, identity). Edge runs MUST drive the operator's subscription CLIs and
MUST set no metered LLM API key. The only component permitted to execute is the Phase-2+ hub
**worker tier**, and it executes hub-native runs only — never edge runs on an operator's behalf.

**Why:** This is the platform's load-bearing decision (`platform/prd.md` §"the edge executes,
the hub remembers"). Cost (subscription vs metered), privacy, and offline resilience all
follow from it. A hub that edge runs route through would forfeit all three.

**How to verify:** plan.md's architecture shows no hub round-trip on the edge-run critical
path; an integration test starts and completes an edge run with the hub unreachable; the edge
harness sets no API-key env var (carried check from `apps/wagner` installer preflight).

### Article VII — One-Directional Dependency (NON-NEGOTIABLE)

The dependency runs one way only: `platform/` MAY consume the capability library; the library
(`skills/`, `agents/`, `commands/`, `hooks/`, `plugins/`, `profiles/`, and everything outside
`platform/`) MUST NOT import, reference, or depend on anything under `platform/`. The library
MUST remain installable via `install.sh` into any repo that has no `platform/` tree.

**Why:** The library is a product depended on by 5+ external repos. A dependency from the
library into the runtime would couple every consuming repo to the platform — the opposite of
the library's purpose (`platform/prd.md` §"one monorepo, two layers").

**How to verify:** a dependency-direction test asserts no file outside `platform/` imports
from `platform/`; an install test runs `install.sh` against a fixture repo with no `platform/`
present and asserts success.

### Article VIII — Event-Sourced Truth (NON-NEGOTIABLE)

Edge run state MUST be derived from an append-only event log. The log is the source of truth;
run snapshots are projections of it. Events MUST be immutable once written. Every state change
MUST be expressible as an event folded by a **pure reducer** — no run-state mutation outside
the fold.

**Why:** Event sourcing is what makes a run replayable, syncable, and recall-able. The pure
reducer (`apps/wagner/src/store/reducer.ts`, already unit-tested in isolation) is the carried
spine; a run's history must reconstruct identically from its log.

**How to verify:** the reducer has no I/O and is unit-tested in isolation; a test replays a
run's event log from empty and asserts the projection equals the live snapshot.

### Article IX — The Privacy Boundary (NON-NEGOTIABLE)

By default, only run **metadata** (goal, status, phase, outcome, guardrail usage, timing) and
curated **learnings** MAY sync to the hub. The codebase, file diffs, and full agent transcripts
MUST NOT leave the edge unless the operator explicitly opts in **per item**. The default code
path MUST be the privacy-preserving one; sharing more is always an explicit action, never a
default or a global toggle.

**Why:** An operator must be able to adopt Wagner without auditing what leaves their machine
(`platform/prd.md` §"privacy model"). "Shared memory" means shared *learnings*, not a feed of
everyone's code. Privacy is a default, not a feature.

**How to verify:** the hub-sync payload has a declared schema containing only metadata +
learning fields with `additionalProperties: false`; a test asserts a representative run's
default sync transmits no transcript or code field, and that opt-in sharing is a distinct,
per-item call.

### Article X — Schema-Validated Payloads (NON-NEGOTIABLE)

Every run, event, transmission, and learning payload MUST validate against a declared JSON
Schema (draft 2020-12, `additionalProperties: false` by default) before it is written to disk,
emitted to a UI surface, or synced to the hub.

**Why:** Unvalidated structured output is the primary source of silent failures in
multi-agent systems and the chief risk at a sync boundary. This article carries forward the
desktop app's schema discipline (`apps/wagner/schemas/*.json`) to every platform boundary.

**How to verify:** every write/emit/sync call site is preceded by a schema-validation call;
CI validates each committed schema file and a representative payload against it.

## Gates

| Gate | Article | Pass Criteria | Failure Remedy |
|------|---------|----------------|----------------|
| Test-First | I | tasks.md shows test task IDs preceding the first implementation task ID for the same behaviour | Re-order tasks.md or add missing test tasks |
| Evidence | II | spec-challenger vague-adjective scan returns zero HIGH findings | Quantify each flagged adjective |
| CRITICAL-Resolved | III | spec-validator verdict is READY | Resolve every CRITICAL finding or rewrite the spec |
| Independent-MVP | IV | Each P1 user story has a non-empty Independent Test (action + value delivered) | Rewrite the Independent Test concretely |
| Simplicity | V | plan.md Project Structure unmodified, OR Complexity Tracking entry per added project/service/engine | Justify each addition or remove it |
| Edge-Autonomy | VI | plan.md shows no hub round-trip on the edge-run critical path; an offline-completion test exists; no API-key env var set | Move hub calls off the critical path; add the offline test |
| Dependency-Direction | VII | a test asserts nothing outside `platform/` imports from `platform/`; install.sh succeeds without `platform/` | Remove the reverse dependency |
| Event-Sourced | VIII | reducer is pure and isolated-tested; a log-replay-equals-snapshot test exists | Move mutation into the fold; add the replay test |
| Privacy-Boundary | IX | sync payload schema is metadata+learnings only; default-sync-transmits-no-code test exists | Remove code/transcript fields from the default sync path |
| Schema-Validated | X | every write/emit/sync is schema-validated; CI validates schemas + sample payloads | Add the missing validation call or schema |

## Amendment Process

The constitution is amended via PR to `platform/docs/spec/constitution.md` with reviewer
approval. It is not amended via the spec-generation flow. Amendments increment the Version
field per semantic versioning:

- **MAJOR:** an article is removed or its meaning is reversed.
- **MINOR:** a new article is added or an existing article gains a new gate.
- **PATCH:** wording, clarifications, typo fixes.

The Last Amended field is set to the date of merge.
