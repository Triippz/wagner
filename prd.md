# Wagner — Platform PRD

> **Status:** directional vision + Phase 1 contract. Deep on vision, the edge/hub
> split, the `run` primitive, and Phase 1 (the wedge). Phases 2–3 are intentionally
> *directional* — no fake precision on contracts the wedge will teach us.
>
> **Product-direction note (2026-06-18):** this PRD is historical for the
> engineering-platform wedge. `VISION.md` and `PRODUCT.md` are now the product
> source of truth. Wagner has widened from an engineering/coding platform into a
> local-first personal OS for daily work: agents, deterministic workflows,
> knowledge, search, news/research, media/artifact generation, productivity
> connectors, and dedicated workspaces such as coding.
>
> **Provenance:** evolves `apps/wagner/PRODUCT.md` and `apps/wagner/DESIGN.md`. Carries
> the run/event/transmission engine (`apps/wagner/schemas/*.json`,
> `apps/wagner/src/store/reducer.ts`). The desktop app is the port-from source, then retired.
>
> **Name:** "Wagner" carries over from the desktop app as the working name at platform
> scope; it is provisional and renaming it is a later, cheap edit.

## Register

product

## Summary

Wagner is an engineering organization's **nervous system**: the shared layer that runs
autonomous coding agents, remembers what every run learned, and makes the state of all
that work legible to the people responsible for it.

The metaphor is mechanical, not poetic. A nervous system has three parts and Wagner maps
to each:

- **Reflexes at the edge.** Each engineer runs agents locally, on their own machine,
  through their own subscription `claude`/`codex` CLIs. Execution happens where the code
  and the credentials already are — fast, private, and able to work offline.
- **Memory at the hub.** A small cloud plane remembers across runs and across people:
  a shared event store, a knowledge/recall index, and identity/presence. The hub does
  not hold the org's code; it holds what was *learned* doing the work.
- **One signal vocabulary.** Everything that happens — a coding run, a code review, an
  incident response, an ops chore — is expressed as the same primitive (a `run`) emitting
  the same normalized events. One vocabulary means one way to observe, steer, and recall,
  no matter what the work is.

The job to be done, scaled up from the single operator: **an engineering org stays in
control of many concurrent autonomous runs without paying constant attention to any one of
them, and never loses what a run figured out.**

## The pivot (why this is not just a desktop app anymore)

Wagner began as a single-engineer desktop console: one operator launches a goal, watches
local `claude`/`codex` agents work it, answers the occasional permission prompt, and trusts
guardrails to halt the run. That product is real, built, and tested — and it is the floor
this platform is built on, not a thing being thrown away.

What changed is the realization that the valuable, durable asset is not the *console* — it
is the **run record and its learnings**. A single operator's run teaches something
(this test is flaky, this module owns that invariant, this refactor needs that migration
first). Today that lesson dies in one operator's terminal. If runs and their learnings were
shared, the org would compound knowledge instead of re-deriving it; if runs could also
execute in a small always-on cloud tier, the org could be responded-to around the clock.

So Wagner stops being a single-user run console and becomes the org's engineering platform.
The console survives as one edge surface among several; the center of gravity moves to the
**run** and to **shared memory**.

## Users

| User | Context | What they need from Wagner |
|---|---|---|
| **Edge operator** (every engineer) | Runs agents locally against their own repos on subscription CLIs. Supervises a fleet while doing other work. | Launch a run, glance at its state, answer what's blocked on them, trust guardrails. Get *back* the relevant learnings from prior runs (theirs and the org's) without searching. |
| **The team** | Many operators, many concurrent runs, shared codebases. | See what runs are active and who they belong to. Recall what was learned. Not re-answer questions another run already answered. |
| **On-call / ops engineer** (Phase 2+) | Responsible when something breaks at 3am. | A small always-on tier that can take the first pass at an incident or a chore as a run, with a clear handoff when a human is genuinely needed. |

The edge operator's experience is the one Wagner already serves well, and it must not
regress: quiet until it matters, then unambiguous. The platform adds the org around that
operator; it does not replace the operator's calm.

## Organizing principle: the edge executes, the hub remembers

This is the load-bearing decision. Everything else descends from it.

### The edge executes

A small harness runs on the operator's machine and drives their **subscription** `claude`
and `codex` CLIs against the goal — decomposing it, dispatching subtasks, round-tripping
permission/question prompts, halting on guardrails. This is what `apps/wagner` does today.

Execution lives at the edge because that is where the advantages are:

- **Cost.** Subscription CLIs use the operator's existing logged-in session. The edge sets
  no API key and never calls a metered LLM API. (See *Open questions* for the one place this
  breaks down — always-on cloud workers.)
- **Privacy.** The code and the agent transcripts never have to leave the machine. What
  leaves is metadata and learnings, by explicit default (see *Privacy model*).
- **Resilience.** A run can start, work, and finish with no hub reachable. The hub is an
  amplifier, not a dependency, for edge runs.
- **Tooling fidelity.** The CLIs run in the real repo context, so the operator's installed
  skills/agents/commands/hooks/plugins (the capability library) are already in scope.

### The hub remembers

The hub is a **small** cloud plane. Its job is memory and coordination, not execution of
edge work. It holds:

- **A shared event store** — the durable record of runs across the org (metadata, outcomes,
  learnings; not raw code or transcripts).
- **A knowledge / recall index** — learnings made retrievable, so a starting run can be
  handed what prior runs figured out about the same area.
- **Identity & presence** — who ran what, who is online, which runs are active.
- **A worker runtime** (Phase 2+) — the one place the hub *does* execute, for always-on
  work (on-call, scheduled chores, chat agents).

"Small" is a requirement, not an aspiration: the hub is deliberately not a heavyweight
control plane that edge runs route through. Edge runs execute locally and *report to* the
hub; they do not *run on* it.

## The `run` primitive

Everything Wagner does is a **run**. A run is:

```
run = goal + workflow + agents + guardrails + event stream + outcome + learnings
```

- **goal** — the human-stated objective ("make the auth tests pass", "triage this alert").
- **workflow** — the shape of the work (a single goal-loop today; a composed graph or a
  durable Temporal workflow later).
- **agents** — the hired roster for this run, each bound to an engine (`claude`/`codex`/a
  local endpoint).
- **guardrails** — the limits that halt the run (max iterations, cost budget, blocked-too-long).
- **event stream** — the normalized, append-only record of what happened, moment to moment.
- **outcome** — the terminal status (met / halted / aborted) and its reason.
- **learnings** — the durable lessons extracted from the run, the thing the hub remembers.

The same primitive expresses every kind of work:

- A **coding run** is a run. (This exists today.)
- A **code review** is a run whose goal is "review this diff" and whose outcome is findings.
- An **incident response** is a run whose goal is "diagnose this alert".
- An **ops chore** is a run whose goal is "rotate these creds / clear this queue".

One primitive means one event vocabulary, one way to observe, one way to steer, one way to
recall. A new capability is a new *kind of run*, not a new subsystem.

### The engine is already built — carry it, rename it

The run/event/transmission engine exists and is tested in `apps/wagner`. It is the spine,
and it ports directly:

- `schemas/run-state.schema.json`, `wagner-event.schema.json`, `transmission.schema.json`
  — every payload is schema-validated before it is written or emitted.
- `src/store/reducer.ts` — pure, unit-tested folds of the event stream into UI state
  (no UI framework, no rendering dependency).
- The existing memory store (`save_memory` / `recall_memory` → `MemoryRecord` with
  `project_id`, `text`, `tags`, `curation_state`) is the seed of the hub's knowledge index.

**One required change on the way in: rename the floor-era vocabulary.** The desktop app
inherited names from its retired "operations floor" visualization. At platform scope those
names are jargon (they violate the *speak plainly* principle) and must be replaced:

| Floor-era term | Platform term | Note |
|---|---|---|
| operative | **agent** | A worker in a run's roster. (`subtask.agent_id` already uses this.) |
| faction (`architects` / `forgers`) | **engine class** | A claude-led vs codex-led lane; collapse onto the existing `engine` tag rather than a separate axis. |
| district (`stacks` / `forge` / `mirror` / `oracle` / `gate`) | **activity / stage** | The kind of work happening (code / build-test / review / plan / waiting-on-you), expressed as activity, not a spatial zone. |
| oracle (the planning pass) | **planner** | Plain name for the decomposition step. |

Exact field renames are an implementation decision for the wedge spec; this table fixes the
intent so the rename is consistent everywhere it lands.

## Architecture: one monorepo, two layers, one-directional dependency

Wagner is **not** a separate repository. It lives in this monorepo as a new layer that
*consumes* the existing capability library. The dependency runs one way only.

```
dev-ai-utilities/                  (this monorepo)
│
├─ <capability library>            Layer 1 — the existing product, unchanged in role
│  skills/ agents/ commands/       Installable into any repo via install.sh.
│  hooks/ plugins/ profiles/       Project-agnostic workflow capability.
│        ▲
│        │  consumed by  (one-directional — the library NEVER depends on the runtime)
│        │
├─ platform/                       Layer 2 — the new runtime (this PRD)
│  ├─ edge/                        local harness: drives subscription CLIs, runs the goal loop
│  ├─ hub/                         small cloud plane: event store, recall index, identity
│  ├─ workers/                     always-on cloud execution (Phase 2+: on-call, chat, chores)
│  ├─ shared/                      the run/event/transmission schemas + reducer (ported)
│  ├─ docs/spec/                   platform SDD governance (constitution.md, defaults.md)
│  └─ specs/                       platform feature specs (NNN-slug/)
│
└─ apps/wagner/                    port-from source — retired once the engine has moved
```

Why one-directional:

- The **library stays independently installable.** Any repo can `install.sh` the skills,
  agents, and hooks with zero knowledge that the platform exists. The platform is one
  consumer of the library, not its owner.
- The **runtime is free to evolve** without destabilizing the library that other projects
  depend on.
- A dependency from the library *into* the runtime would couple every consuming repo to the
  platform — the opposite of what the library is for. This is a hard rule, not a preference.

## Phase 1 — the wedge: shared coding runs + learnings

The first thing built is the smallest slice that proves the platform thesis end to end:
**a coding run at the edge, whose metadata and learnings sync to the hub, and which can
recall the org's prior learnings on its way in — tied to a person's identity.**

It is chosen as the wedge because it exercises all three platform-defining mechanisms
(edge→hub sync, shared recall, identity) while needing **no Temporal and only a minimal
hub**. It delivers value on day one (an operator gets back relevant prior learnings) and
every later phase builds on the same spine.

### What the wedge does

1. **Run at the edge (carried over).** An operator launches a coding run locally, exactly as
   the desktop app does today: subscription CLIs, goal loop, guardrails, transmissions. This
   behavior is ported, not reinvented.
2. **Sync metadata + learnings to the hub.** When a run progresses and completes, its
   *metadata* (goal, status, outcome, guardrail headroom, timing) and its *learnings* (the
   curated `MemoryRecord`s) sync to the hub's shared event store. Code and transcripts do
   not (see *Privacy model*).
3. **Recall on the way in.** When a run starts, the edge asks the hub for learnings relevant
   to the goal/area and surfaces them to the operator and the planning step — so the run
   starts informed by what the org already knows.
4. **Identity.** Every run, event, and learning is attributed to a person. The hub knows who
   ran what; recall and presence are scoped by identity.

### What the wedge proves

- **Edge→hub sync works** over the real run/event schema, including the privacy boundary
  (metadata + learnings cross; code + transcripts stay).
- **Shared recall works** — a learning saved by one run is retrievable by another.
- **Identity works** — runs and learnings are attributable and scoped.

If these three hold, the platform's spine is real and Phases 2–3 are extensions of it rather
than new inventions.

### Explicitly out of scope for the wedge

- No Temporal, no always-on cloud workers (Phase 2).
- No sensors / automatic ingestion of the org's engineering surface (Phase 3).
- No code or transcript sync (privacy default; deferred and gated on explicit opt-in).
- No new run *kinds* (review/incident/ops) — those are later runs over the same spine.

## Phases 2–3 — directional

Held deliberately loose. The wedge will sharpen these; committing to contracts now would be
fake precision.

**Phase 2 — durable cloud workers.** A small always-on worker tier in the hub takes runs that
must execute without an operator present: on-call first-pass triage, scheduled chores, chat
(Slack) agents. The workflow engine here is **Temporal.io** (durable, retryable, observable
long-running workflows); edge runs stay event-sourced on the existing log. The known
tension: always-on workers likely cannot use subscription CLIs (headless auth / ToS) and so
likely run on **API keys** — a real cost decision, revisited when Phase 2 starts, not now.

**Phase 3 — sensors and shared knowledge.** The org's engineering surface (CI, alerts, PRs,
incidents) becomes a source of events that can open runs automatically — automatic code
review as a run, an alert as an incident run. The hub's recall index broadens from coding
learnings into a fuller engineering knowledge base (RAG over runs, decisions, and outcomes).

## Principles

Carried from the desktop product (these are proven and must not regress), and extended for
the platform.

**From the console (still binding):**

1. **Color is meaning, never decoration.** Accent and semantic states are the only color.
2. **Summarize first, detail on demand.** The most-needed state is always visible and
   digestible at a glance; depth is one interaction away.
3. **Earned calm.** Quiet by default; escalate visually only for states that need a human.
4. **Keep the long, silent turn alive.** A message-sparse turn must never look frozen.
5. **The tool disappears into the run.** Earned familiarity over novelty; one vocabulary.
6. **Speak plainly.** No unexplained acronyms, no internal jargon, no raw machine verbs in
   human-facing surfaces. (This principle is *why* the floor vocabulary is being renamed.)

**New at platform scope:**

7. **The edge executes; the hub remembers.** Execution is local by default; the hub holds
   memory and coordination, not the org's code.
8. **The hub is small.** Deliberately minimal surface — event store, recall, identity, and
   (later) a worker tier. Not a control plane edge runs route through.
9. **Privacy is a default, not a feature.** Metadata and learnings sync; code and transcripts
   stay local unless explicitly shared. The default is the safe one.
10. **The library never depends on the runtime.** One-directional dependency; the capability
    library stays independently installable.
11. **Everything is a run.** New capability is a new *kind of run* over the shared spine, not
    a new subsystem.

## Anti-references

Carried from the desktop product:

- **Neon glow soup.** No colored `box-shadow`, no cyan/magenta duotone, no decorative motion.
  The retired floor is an anti-reference, not a fallback.
- **Generic SaaS dashboard.** No hero-metric template, no identical icon-card grids, no
  gradient accents. A serious operator tool, not a marketing surface.
- **Toy / game UI.** The run surface reads as a real operations instrument, never a gimmick.

New at platform scope:

- **Heavyweight enterprise control plane.** Wagner is not a centralized orchestration server
  that all work routes through. The edge is primary; the hub is small.
- **Metered-API cost center.** The edge is subscription-first and sets no API key. Only the
  Phase-2 always-on tier may use metered keys, and only as an explicit, costed decision.
- **Surveillance tool.** Shared memory is shared *learnings*, not a feed of everyone's code
  and transcripts. Privacy is the default; visibility is opt-in beyond metadata + learnings.

## Privacy model

The boundary is a first-class contract, not a setting buried in a menu:

- **Syncs to the hub by default:** run *metadata* (goal, status, phase, outcome, guardrail
  usage, timing) and curated *learnings* (`MemoryRecord`s and their tags).
- **Stays local by default:** the codebase, file diffs, and the full agent transcripts.
- **Crosses only on explicit opt-in:** anything beyond metadata + learnings (e.g. sharing a
  transcript for debugging) is a deliberate, per-item action by the operator.

The default is chosen so an operator can adopt Wagner without auditing what leaves their
machine: the answer is "what the run learned, and the facts about the run — never the code."

## Workflow engine

- **Edge runs:** event-sourced on the existing append-only log (the run-state + event schema
  already in `apps/wagner`). No external engine; the log is the source of truth.
- **Cloud worker tier (Phase 2+):** **Temporal.io** for durable, retryable, observable
  long-running workflows that must survive process restarts and span hours/days.

Two engines is intentional: edge runs are short-lived, local, and offline-capable (event
sourcing fits); always-on cloud workflows are long-lived and must be durable across
infrastructure (Temporal fits). The wedge uses only the first.

## Non-goals (current)

- **Not replacing the agents or the CLIs.** Wagner orchestrates and remembers; `claude`/
  `codex` still do the reasoning and editing.
- **Not a metered-API product** at the edge. Subscription-first is a defining constraint.
- **Not a second home for the org's source code.** The hub remembers learnings, not code.
- **Not a from-scratch rewrite.** The run engine ports from `apps/wagner`; the wedge reuses it.

## Open questions & deferred decisions

- **Always-on workers and API keys.** 24/7 cloud workers likely cannot use subscription CLIs
  (headless auth / ToS) and so likely run on API keys — a real cost decision. Deferred to
  Phase 2; flagged here so it is not a surprise.
- **Legacy palette remap.** The desktop app's neon "dual palette" is pervasive in live
  components and carries state meaning; remap to semantic tokens during the platform rebuild,
  ideally with the app running for visual verification.
- **Retired components.** The benched desktop components (GoalEntry/Console/AgentInspector/
  MissionBar) do not port to `platform/edge`; their tests retire with them.
- **`impeccable` as a first-class repo skill.** Currently untracked; decide during platform
  work whether to adopt it.
- **Naming.** "Wagner" is provisional at platform scope; renaming is cheap and deferred.
