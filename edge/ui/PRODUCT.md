# Product — Wagner Edge

## Register

product

## Users

A single engineer (the operator) running autonomous coding agents against their
own repositories from a native macOS desktop app. They are fluent in
Claude Code / Codex and expect that level of tool craft. Context: long,
unattended agent runs they check in on between other work — they want to glance,
see what each org is doing, answer a permission prompt, and get back out. They
drive several projects at once and resume work across days.

## Product Purpose

Wagner Edge is the edge layer of the Construct: it executes agent work on the
operator's machine ("edge executes, hub remembers"). It assembles the headless
`wagner-edge-host` engine — an oracle that plans and a roster of operatives
(agents) that execute — into a launchable desktop console. Success is when the
operator trusts it to run real work unattended: starting a session is one
gesture, concurrent sessions are glanceable, and nothing is ever lost because
sessions are durable and resumable. The tool should disappear into the task.

## Brand Personality

Calm, terminal-native, precise. Three words: **composed, legible, unattended.**
It is a long-session instrument, not a dashboard demo — it rewards glancing, not
staring. Voice is direct and operational ("needs you", "running", "idle"), never
cute. The org/faction framing (architects, forgers; oracle, operatives) is the
one place personality shows, and it stays subtle — a vocabulary, not a costume.

## Anti-references

- **Config-form-first launchers.** The current "Launch a run" screen — a wall of
  goal + directory + max-iterations + cost-budget + blocked-timeout + test-command
  fields — is the exact thing to move away from. Starting work should not feel
  like filling out a job ticket.
- **SaaS dashboard chrome.** Hero metrics, gradient accents, identical card grids,
  tracked-uppercase eyebrows. This is an operator's tool, not a marketing surface.
- **Anything that hides live state.** If a session needs the operator, that must
  be glanceable without clicking in.

## Design Principles

1. **One gesture to start.** A new session is: pick a folder, type the first
   goal. Configuration is the project's job (its `CLAUDE.md` / `AGENTS.md` /
   MCP config), not a launch form's.
2. **Never lose work.** Sessions are durable on disk and resumable; closing the
   window or quitting the app is safe. Match the Claude Code / Codex resume model.
3. **Glanceable concurrency.** Many sessions run at once; their status (running /
   needs-you / idle / done) reads at a glance from the session rail.
4. **Earned familiarity.** Borrow the proven shape (left session rail + console)
   from the tools the operator already trusts; spend novelty only where it earns
   its keep (the live agent console).
5. **The tool disappears.** Restrained color, consistent component vocabulary,
   motion only to convey state — so attention stays on the agents' work.

## Accessibility & Inclusion

Dark-first (long unattended sessions, low-light operator context). Body text
≥4.5:1 on its surface; status is never conveyed by color alone (dot + label).
Full `prefers-reduced-motion` alternative for every animation. Keyboard-navigable
session switching and primary actions.
