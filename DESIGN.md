# Design

> Visual system of record for Wagner. Locked 2026-06-18 to the "biometric
> deep-scan" reference the operator chose. Strategy/why lives in PRODUCT.md;
> this is *how it looks*. Reference implementation: `mocks/_app.css`, `graph.js`,
> `dashboard.html`, `coding.html`.

## Theme

Dark, cinematic, calm, futurist — "not overwhelming." A near-black field with a
faint dot grid; soft dark rounded cards float on it with generous negative space.
One living centerpiece — the **knowledge-graph plexus** (a white wireframe of
nodes + edges that slowly rotates) — which doubles as the assistant's presence:
it pulses and brightens when Wagner speaks. Status lives in color, not chrome.
Physical scene: a developer at night, lights low, talking to and working
alongside an ambient assistant on a large dark screen — focused, a little awe.

## Color (OKLCH)

Strategy: **restrained** — near-black surfaces + one saturated signature
(chartreuse lime), color used only to carry meaning.

| Role | Token | Value |
|---|---|---|
| Field | `--bg` / `--bg-deep` | `oklch(0.115 0.004 140)` / `0.085` |
| Dot grid | `--dot` | `oklch(1 0 0 / 0.035)` |
| Card | `--card` / `--card-2` | `oklch(0.175 0.004 140)` / `0.205` |
| Featured card | `--card-olive` | `oklch(0.26 0.045 122 / 0.45)` |
| Hairline | `--line` / `--line-soft` | `oklch(0.30 …)` / `0.235` |
| Ink | `--ink` / `--ink-2` / `--ink-3` | `0.975` / `0.74` / `0.55` |
| **Signature** | `--lime` | `oklch(0.90 0.185 124)` (text on lime = `--lime-ink` `0.18`) |
| Status | `--ok` `--warn` `--alert` | green `0.80 0.15 150` / amber `0.85 0.15 92` / red `0.64 0.20 25` |

Plexus node groups (canvas, hsla): base near-white, **lime** = verified/active,
**amber** = needs-review, **red** = disputed. Contrast: body ink ≥4.5:1 on the
near-black; lime pills use dark ink. No muted-gray-on-dark body text.

## Typography

- **UI/body:** Hanken Grotesk (calm humanist grotesque — deliberately *not*
  Inter). Weights 300–700.
- **Numbers/metrics:** Hanken Grotesk **300**, large, `letter-spacing:-0.02em`,
  with a small `--ink-3` unit subscript (e.g. `9` `%`, `113.1` `mg/dl`).
- **Data/IDs/code/logs:** Geist Mono — mono only where it earns it.
- Plain sentence-case labels. **No tracked-uppercase eyebrows.** No gradient text.

## Components

- **Card** (`--r-card` 20px, `--line-soft` hairline, soft). Variants: `olive`
  (featured, tinted translucent), `tap` (hover lift). Never nested.
- **Metric card:** colored tick + label / light number + unit + trend arrow /
  mini scatter chart (baseline + dots, one highlighted ring) / optional delta
  pill (lime up, red down).
- **Pill:** `ghost` (dark) and `lime` (filled, dark ink) — tabs, context
  selectors, identity, status.
- **Category tabs:** rounded pills; active = lime filled.
- **Callout (glassy):** translucent + `backdrop-filter: blur` — floats over the
  plexus; title + risk word (lime/amber/red) + key→value rows. Used sparingly
  (glass is purposeful here, not default).
- **Scrubber:** pill-shaped timeline (segments + dot ticks, active = lime).
- **Plexus** (`graph.js`): the knowledge graph + assistant presence. Reduced-
  motion → single static frame.

## Layout

Three columns under a slim top bar (hex mark + context pill · center tabs ·
icon buttons w/ badge): **left** = operating-picture summary + workspace cards +
identity pill; **center** = the plexus with floating callouts, a low-key voice
line, and the bottom scrubber; **right** = category tabs + a 2-col metric grid.
Coding (the heavy workspace) is a separate surface; the plexus shrinks to a
docked companion there. Generous gaps; calm rhythm; semantic z-scale (no magic
9999). `prefers-reduced-motion` alternative is mandatory (the plexus animates).
