# wagner ui mocks

Static, build-free mockups that define how the UI looks **before** we refactor the
React app. Open any file directly in a browser. Each file = one screen or state.

Source of truth for *what* to mock: `docs/ui-redesign-findings.md` (findings F1–F39,
stories S1–S12).

## convention
- One HTML file per screen/state, lowercase kebab-case: `dashboard.html`,
  `run-activity.html`, `agents.html`, `assistant-active.html`, `vault.html`,
  `settings.html`, …
- Self-contained: inline CSS (or a shared `mocks/_shared.css`), no JS build, no deps.
- `index.html` = contact sheet linking every mock.
- A mock is "signed off" when the operator approves it; then it becomes the spec
  for the React change.

## order (keystone-first)
1. `dashboard.html` — S1, the common operating picture (reframes everything)
2. `run-activity.html` — S7, markdown log + search + model badges + scoped status
3. `agents.html` — S6, create agents / pick models / sub-agents
4. `assistant-active.html` — S8, the voice presence
5. `vault.html` — S9
6. `settings.html` — S10
