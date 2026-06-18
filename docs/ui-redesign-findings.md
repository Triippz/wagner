# wagner ui redesign — findings & core stories (2026-06-17)

> Source: operator L2 manual test of the live app (6 screenshots + walkthrough).
> Part 1 = every finding, atomic + ID'd, so we can prove each is addressed.
> Part 2 = the core stories (the refactor narrative). Part 3 = mocks-first process.
> Traces into vision doc §12 (command center) + §13 (v2 backlog).

## part 1 — findings (ID'd)

### product framing & landing
- **F1** — App launches inconsistently: sometimes straight to the goal screen,
  sometimes to Console/Vault. No stable, intentional landing.
- **F2** — Wanted landing is a **dashboard / common operating picture**, not a
  goal-entry screen and not a coding console.
- **F3** — Whole product reads as "trying to be Cursor" — too coding-specific.
  Wants a **general productivity tool** ("run my whole life on it"). Coding is a
  primary workspace, not the only one.
- **F4** — Wants to harness **many agents that aren't about coding**, and build
  **workflows**, and tie into daily software: Slack, Discord, web browsers.
- **F5** — Wants **semantic retrieval across all knowledge**: not just coding /
  project learnings, but Slack interactions, teammate "personalities," Notion
  company docs, GitLab codebases, email.
- **F6** — Eventually **design + run workflows in-app** — the customized harness
  to do everything through.

### terminology / naming
- **F7** — Header shows the current **goal** ("Asdasdas…"); the entire app is
  framed around a *single goal*. Dislikes — should be a dashboard over all daily
  operations, not one run.
- **F8** — **"Sessions"** is ambiguous. A session can mean: a chat session, a
  coding session, a session scoped to a specific project. Needs **levels /
  metadata** so "session" isn't overloaded.
- **F9** — **"Roster"** is undescriptive. If it's a roster of agents, call it
  **"Agents."**
- **F10** — Agent/label names are **too abstract / sci-fi**: "Cipher" means
  nothing; "architects · mirror · review" unclear. Fun but not descriptive.
- **F11** — Empty-state copy is sci-fi/opaque: "No operatives on the floor yet,"
  "the oracle is still planning."

### the assistant & voice (jarvis)
- **F12** — Wants to **drive the whole platform by voice**, not only point-and-
  click (clicking is fine too). Operator is verbal — leans into speaking heavily.
- **F13** — Voice flow: **voice agent grabs intent → hands to an agent → drives
  actions throughout the UI.**
- **F14** — Concrete target workflow (voice *or* drag-and-drop authored):
  "Every Friday 2pm, read our Jira in-progress + recently-completed epics this
  week, post a status page to a Slack channel — at-risk / not / in-progress /
  finished + who's driving."
- **F15** — When enabled, **voice is always active** with a full UX (not a toggle
  that errors).
- **F16** — A **JARVIS-style centerpiece**: a pulsing "essence"/presence in the
  middle of the UI when voice is on — the helper you speak to.
- **F17** — **Customizable assistant identity**: rename it (not just "Wagner"),
  pick its speaking style / personality, and it *only* speaks that way.
- **F18** — Presence needn't live only center, but is **core to interaction when
  enabled**. **Hotword + click** to activate.
- **F19** — Long-term **global "Hey Wagner"** hotword anywhere on the OS (even
  outside the Wagner window) → pops up → ask → it **dispatches agents in the
  background** → returns + **speaks** → optional **notification toast** (corner
  configurable) → click → opens Wagner.
- **F20** — Wagner lives in the **macOS menu bar**: closing the window
  **backgrounds** the app (doesn't quit); quit only from the tray.

### agents, models & skills
- **F21** — **(MANDATORY)** No way to use the **skills already defined** in Claude
  Code / Codex when setting a goal. Must be able to use them.
- **F22** — **Discover skills wherever they live** — `.claude/`, cursor rules,
  `.agents/`, etc. — and use them when present, discovered per session/project.
- **F23** — **Install skill repos from GitHub** and use those skills **anywhere in
  Wagner** (not locked to Claude/Cursor) — skills as portable prompts.
- **F24** — No way to **create multiple agents** or request **sub-agents**
  (blocked partly by no skills + no model choice).
- **F25** — Want **multiple models in one harness**: Cursor, Codex, Claude models
  — chosen per task/purpose.
- **F26** — Want **auto model-routing** skills: pick the best model by
  cost-effectiveness, thinking depth, etc.

### activity log / run view
- **F27** — **LIKE (keep):** timestamps, the `review/think/shell/edit` labels,
  text broken down into entries.
- **F28** — **Render all log/agent text as markdown** — key requirement,
  everywhere.
- **F29** — Want **search** in the log.
- **F30** — Want to **see which models** are being used (per entry / per agent).
- **F31** — Top-right status (**Running · phase · iter · spend**) is good but must
  be **scoped to the specific session**, not the global header.

### vault
- **F32** — Vault renders **empty** — nothing shows up.
- **F33** — No visibility into **what learnings/memories exist** or how they work.
- **F34** — Unclear whether **auto-capture** of learnings actually happens, and how.
- **F35** — No way to **add your own data** to the vault.

### settings
- **F36** — Settings is **voice-only**; should be a real multi-section settings
  surface (much more coming).

### asks / questions
- **F37** — **LIKE (keep):** the agent question/ask pop-ups. Could be **more
  explicit** when a question appears, but good.

### window / os
- **F38** — **Fullscreen is broken** — only goes partial, not true native
  macOS fullscreen.
- **F39** — **Abort button does nothing** — clicking "Abort" on a running run is
  inert (see B3).

### concrete bugs observed (fix regardless of redesign)
- **B1** — **Voice "error" despite models READY.** STT (whisper tiny.en) + TTS
  (Kokoro q8) both show `READY`, but header = "Voice error." Root cause:
  `wagner-tts-sidecar` isn't built/on PATH under `make edge` (dev path); enabling
  voice tries to spawn a missing binary. Fix: build/ship the sidecar for dev, and
  make the error state name *what* failed (sidecar missing vs. model missing).
- **B2** — **Discoverability:** operator couldn't tell how to "see my sessions"
  even though the SESSIONS list was present — weak visual hierarchy (F2/F8).
- **B3** — **Abort is inert** (F39). Clicking Abort doesn't stop the run. Fix the
  abort path (IPC + run-loop cancellation) independent of the redesign.

## part 2 — core stories (refined)

### 2.0 the reframe, in one line
Wagner is built as a **single coding-run console**; the operator wants a
**dashboard-first, voice-driven, general-purpose command center** where coding is
one workspace among many. Everything below serves that shift.

### 2.1 information architecture (DECIDED 2026-06-17)

**Principle (operator):** work is **not a fixed enum of types** — that's
narrow-sighted and not future-facing; generative AI does open-ended things. The
unit is simply *"an agent/workflow doing a task."* The boundary that matters is
**environment**: most work is *light* (an ask, an automation) and needs no special
surface; *some* work is *heavy* and earns a dedicated **workspace** with
specialized tools. **Coding is the canonical heavy workspace** (repo + files +
diffs + oracle/operative runs) — encoding the operator's "I code differently than
I Slack." New heavy modes become new workspaces later; light work is never
pre-typed. Open-ended by default, bounded only where it earns it.

```
┌ Assistant (omnipresent: top bar + center presence when voice on) ──────┐
│ NAV            │  SURFACE (changes with nav)                           │
│  ◆ Dashboard   │   operating picture of ALL work (untyped feed) +      │
│  ◆ Coding      │   quick-launch ("ask Wagner / start anything")        │
│  ◆ Agents      │                                                       │
│  ◆ Workflows   │   Coding = the one heavy WORKSPACE (folder-scoped      │
│  ◆ Knowledge   │     sessions, files, diffs, runs)                     │
│  ◆ Connectors  │   Agents / Workflows / Knowledge / Connectors =       │
│  ◆ Settings    │     capabilities, not work-types                      │
└────────────────┴───────────────────────────────────────────────────────┘
```
- **Dashboard** = open-ended operating picture; any agent/workflow/run shows here
  regardless of where it executes. Not typed.
- **Coding** = the dedicated heavy workspace (today's oracle/operative engine).
- **Agents / Workflows / Knowledge / Connectors** = capabilities the open-ended
  work draws on. **Settings** holds the **global per-user assistant identity**
  (name/voice/personality — DECIDED: one identity app-wide).
- Adding a future heavy mode = add a workspace; it does **not** require a new
  "work type" taxonomy for light work.

### 2.2 epics (S1–S12 regrouped; IDs preserved for traceability)

**E1 — The new shell** *(reframe everything hangs on; mock first)*
- **S1 Dashboard-first home** — operating picture is the landing surface, not a
  goal screen. *(F1,F2,F7 → §12.4)*
- **S2 De-code-ify the shell** — coding = one workspace; layout/copy stop
  assuming "a repo + a run." *(F3,F4 → §13.3)*
- **S3 Plain-language domain** — Roster→**Agents**; kill sci-fi names + opaque
  empty-state copy; keep the idea, drop the theme. *(F9,F10,F11)*
- **S4 Session-model clarity** — define session **kinds** (chat / coding /
  project) + their metadata; surface the kind. *(F8,B2)*

**E2 — Legible work view** *(the most-used inner surface)*
- **S7 Run/activity view** — markdown everywhere, search, per-entry model badges,
  **session-scoped** status (running/phase/iter/spend). Keep timestamps + role
  labels. *(F27–F31)*

**E3 — Agents, models & skills** *(the engine made usable)*
- **S5 Skills everywhere (MANDATORY)** — discover from `.claude/`, cursor rules,
  `.agents/`; install skill repos from GitHub; use any skill anywhere; surface
  when authoring. *(F21,F22,F23 → §13.7,§13.9)*
- **S6 Multi-agent & multi-model** — create agents + sub-agents; per-task model
  (Claude/Codex/Cursor/self-hosted/GLM); skill-driven auto-routing. *(F24,F25,F26
  → §12.A,§13.5,§13.9)*

**E4 — The Assistant (voice-first JARVIS)** *(split near vs far)*
- **S8a In-app assistant** — customizable identity + voice + personality;
  always-on when enabled; pulsing center presence; hotword + click. *(F12–F18)*
- **S8b Global assistant** *(far)* — system-wide "Hey Wagner" outside the window,
  background dispatch, speak-back, notification toast, **menu-bar background app**
  (close = background, quit from tray). *(F19,F20 → §13.3)*

**E5 — Knowledge** 
- **S9 Vault made real** — show learnings/memories, explain + verify auto-capture,
  manual add, non-empty useful graph. *(F32–F35 → §13.1)*

**E6 — Workflows** *(the "do everything" payoff)*
- **S12 Workflow builder** — drag-and-drop *and* voice-authored, scheduled
  triggers (Friday-Jira→Slack example). *(F6,F14 → §12.B,§13.5)*

**E7 — Plumbing**
- **S10 Settings as a surface** — multi-section (voice is one). *(F36)*
- **S11 OS integration** — true fullscreen; tray background; notifications.
  *(F20,F38)* — overlaps S8b.

### 2.2.1 dashboard mock — iteration log
- **v1 (2026-06-17): REJECTED** — "too enterprisey." Card-grid SaaS dashboard.
- **v2 (2026-06-17): right genre, too literal** — V.A.U.L.T./JARVIS HUD clone
  (central voice orb, tracked-caps metrics left, command-deck right, hero number).
  Operator: good start, but "don't literally copy," "make it mine," and it's "more
  a voice bot" — must also cover software dev, workflows, complex work.
- **v3 (2026-06-18): current direction** — *cinematic soul, plain words.* Built on
  a bespoke design system (`mocks/_app.css`, `orb.js`): Geist type (not Inter),
  deep blue-black + one teal signature, high-contrast ink (AA/ADHD), engineer-like
  assistant copy. Two states prove the model:
  - `dashboard.html` — **Home**: the living orb is the centerpiece + the operating
    picture (Today / Needs you / Now / Activity) in plain language around it.
  - `coding.html` — **Coding workspace** (the heavy environment): file tree + live
    diff + agents + role-labeled activity log, with the orb shrunk to a **docked
    corner companion** ("present, not in the way"). Proves Wagner is a workbench,
    not a voice-bot toy. Resolves §2.1: Home = orb; heavy work = a real workspace.
  Dropped V.A.U.L.T. tropes flagged as slop in PRODUCT.md anti-refs (tracked-caps
  eyebrows, hero-metric number). Decisions: persona = engineer-like/explicit/
  ADHD-friendly; aesthetic = bespoke, "not made by AI."
- **v4 (2026-06-18): LOCKED — "biometric deep-scan" direction.** Operator gave a
  precise reference set (medical full-body deep-scan dashboards) and said "that
  whole thing is screaming what I love." Adopted wholesale, Wagner-ified:
  near-black + dot grid, **chartreuse-lime** signature, soft dark rounded cards
  with light-weight numbers + tick + mini scatter + delta pill, category tab
  pills, glassy floating callouts, bottom timeline scrubber. **The reference's 3D
  anatomy plexus → the knowledge graph** (`graph.js`): white wireframe, lime
  verified-cluster / amber needs-review / red disputed node; it also *is* the
  voice presence (pulses when Wagner speaks). Type → Hanken Grotesk. Captured as
  **`PRODUCT.md`** (strategy) + **`DESIGN.md`** (visual system of record).
  `mocks/dashboard.html` = the locked home. Next: re-theme `coding.html` to this
  language; refine plexus density; then translate into the real React app.

### 2.3 mock / build sequence (dependency-ordered)
1. **Lock the IA (2.1)** — one decision, blocks all mocks.
2. **E1 shell** → `dashboard.html` (+ nav, renamed surfaces, empty states).
3. **E2** → `run-activity.html` (markdown log, search, model badges).
4. **E3** → `agents.html` (create agent, model picker, skills picker).
5. **E4** → `assistant-active.html` (S8a presence; S8b is later/native, not a mock).
6. **E5** → `vault.html`; **E7** → `settings.html`.
7. **E6** → `workflow-builder.html` (last — biggest, depends on agents+connectors).

### 2.4 just-fix bugs (parallel, no mock needed)
B1 (voice error message + dev sidecar), B3 (Abort inert), F38 (fullscreen). These
are correctness fixes we can land independent of the redesign.

## part 3 — process: mocks before code

Per operator: **define how everything looks before coding.** Plan:
- `mocks/` directory at repo root — static HTML/CSS mockups (no build, open in a
  browser), one file per screen/state, plus a `mocks/index.html` contact sheet.
- Mock order follows story priority: **dashboard (S1)** first — it's the keystone
  that reframes everything else — then run/activity view (S7), agents+models
  (S6), the assistant presence (S8), vault (S9), settings (S10).
- Once a mock is signed off, it becomes the spec for the React refactor.

Decided (operator, 2026-06-17): mock fidelity = **styled to final look**; IA =
**§2.1** (work-by-environment, Coding = heavy workspace); assistant identity =
**global per-user**.

### open clarifications
1. **Connectors** — which first proves the seam (Slack? Jira? Notion?) — drives
   the Workflows mock content.
