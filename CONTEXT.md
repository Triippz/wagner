# Domain Glossary

Platform-scoped domain language for the Wagner runtime (`platform/`). Domain terms only —
no implementation detail. Established during design interrogation; each term is accepted by
the engineer before it lands here.

## Learning

**Definition:** A durable, operator-authored lesson distilled from a run — the unit of
knowledge the hub remembers and recall returns. A learning is content a person decided was
worth keeping, not a raw transcript or an automatic capture.
**Avoid:** "memory" (overloaded — the store is not the lesson), "note", "insight".
**Related:** Curation state, Mark shareable.

## Curation state

**Definition:** A learning's shareability lifecycle — the control that decides whether a
learning may leave the operator's machine. Three states: **auto** (machine-suggested,
stays local), **captured** (operator-authored, stays local), **curated** (operator has
deliberately marked it shareable — the only state that syncs to the hub). The lifecycle is
introduced by the wedge; earlier local-only work used a single unconstrained value.
**Avoid:** "status" (collides with a run's status), "published", "promoted".
**Related:** Learning, Mark shareable.

## Mark shareable

**Definition:** The deliberate operator act that moves a learning to the **curated** state,
making it eligible to sync. It is the per-learning privacy decision Article IX requires:
nothing a person jots down leaves the machine until they choose to share that specific item.
**Avoid:** "publish", "sync" (sync is the mechanism that follows; marking shareable is the
human decision that authorises it).
**Related:** Curation state, Learning.

## Shared project

**Definition:** The unit of enrollment and org-wide recall — a codebase identified by where
it comes from (its source-control origin), so every operator working on the same repository
agrees on the same project automatically, with no coordination. Only enrolled shared
projects participate in the hub, in either direction.
**Avoid:** "repo path", "workspace" (the on-disk path is the *local project*, not the shared
identity); "project" unqualified.
**Related:** Local project, Learning.

## Local project

**Definition:** An operator's on-machine working copy of a codebase — the directory a run
executes in. It is an edge-local handle only and is never the identity used to share or
recall across operators (two people's local projects for the same codebase differ).
**Avoid:** using it as the cross-operator key; "project" unqualified.
**Related:** Shared project.

## Operator

**Definition:** The verified person a run, event, and learning is attributed to —
authenticated to the hub through the organization's single sign-on (an employee). Identity is
established externally, never self-asserted, and scopes what each person may share and recall.
**Avoid:** "user" (unscoped), "agent" (an agent is not a person — see Agent).
**Related:** Agent, Learning, Shared project.

## Agent

**Definition:** A worker hired into a run's roster, bound to an engine — the thing that does
the reasoning and editing. An agent is not the operator: many agents may act on one operator's
behalf within a single run.
**Avoid:** "operative" (retired floor-era term — renamed to agent); "operator" (that's the
person, not the worker).
**Related:** Operator.

## Recall

**Definition:** Surfacing prior learnings to a run as it starts, so it begins informed. Two
complementary sources, kept distinct rather than merged: **local recall** (the operator's own
prior learnings for the project at hand, any curation state, folded into the run's goal) and
**org recall** (curated learnings shared across the org's enrolled projects, surfaced to the
operator and the planner).
**Avoid:** "search" (recall is automatic at run start, not a query the operator types);
"memory" (that's the store — recall is the read of it).
**Related:** Learning, Operator, Shared project.

---
