# Product

## Register

product

## Source Of Truth

`VISION.md` is the canonical product vision. This file is the shorter product
register used when designing UI, copy, and interaction behavior.

## Users

Mark — a software engineer/founder (Adyton) who wants **one place to run daily
work**: coding and development, agentic and deterministic workflows, knowledge,
search, research, image/artifact generation, and connected tools (Slack, Discord,
Jira, Notion, email, GitLab, calendars, docs, browsers). A **verbal,
voice-first** thinker who describes things better out loud than in writing.
**ADHD** — needs explicit, well-defined, low-metaphor communication: clear
status, defined answers, predictable structure, low cognitive load. Often works
at night, focused, wants an ambient assistant he talks to. Solo operator today;
**self-hostable + multi-tenant later** so the same vault/workflow/agent system
can sync across his own devices or be shared with a team.

## Product Purpose

Wagner is a **local-first, cloud-connected personal OS** — a single operating
picture over agents, workflows, knowledge, search, connectors, and dedicated
workspaces. It should feel like a Maven-style smart system for one person's work:
typed entities, source-aware analysis, clear status, and coordinated action. It
is not an OSINT product; the reference is the seriousness and integrated
operating picture, not the domain.

Work is **not a fixed taxonomy of types** (generative AI is open-ended). The
boundary that matters is *environment*: most work is light and can happen from
the dashboard/assistant; heavy work earns a dedicated **workspace**. **Coding is
the first heavy workspace**, not the product boundary. Wagner should also handle
research, news briefings, general search, image generation, documents, workflow
automation, and productivity connectors.

It is **voice-first and text-equal**: you talk or type to Wagner, it captures
intent, dispatches agents or deterministic workflows, uses tools like Codex and
Claude Code where they are valuable, then reports back in UI, speech, or both.
Success = Mark runs daily operations through Wagner instead of juggling a dozen
separate tools.

## Brand Personality

- **Three words: cinematic, grounded, precise.**
- **The assistant** is engineer-like: explicit, well-defined, low-metaphor,
  ADHD-friendly. It states exactly what happened and what's needed — no fluff, no
  implicit acknowledgement, no "I've gone ahead and…". Calm command, not a
  chatty companion and not a theatrical hype-bot.
- **The product** is a living, cinematic *presence* (the orb you talk to) wrapped
  around real, legible, professional work surfaces. **The sci-fi is in the
  atmosphere; the language is always plain.**
- **Bespoke, never template.** Must not feel "made by AI." Professional yet with
  personality ("tickles the fancy"), never obnoxious or loud.

## Anti-references

- **Enterprise SaaS admin dashboards** — card grids, KPI tiles, generic. ("Too
  enterprisey.")
- **Opaque sci-fi jargon** — Cipher / oracle / "operatives on the floor." Fun but
  not descriptive. Keep the cinematic *feel*, kill the cryptic *words*.
- **Generic AI-generated UI tells** — cream/sand palettes, gradient text,
  glassmorphism-by-default, the hero-metric template, identical card grids,
  tracked-uppercase eyebrows on every section, Inter-everywhere sameness. (Note:
  the V.A.U.L.T. reference itself leans on tracked-caps + a hero number — we take
  its *soul*, not those tropes.)
- **A passive status mirror / pure voice-bot toy** — Wagner is a workbench you
  *do work in*, not just look at and talk to.
- **An IDE clone (Cursor) that's only about code** — coding is one room, not the
  whole house.
- **An OSINT / military-intelligence console** — the smart-system reference is
  operating-picture quality and entity-aware reasoning, not the domain.

## Design Principles

1. **Cinematic soul, plain words.** Atmosphere can be dramatic and alive; labels
   and copy are always plain English. Sci-fi in the feel, never the vocabulary.
2. **The operating picture is home.** The default surface is a rollup of *all*
   activity — open-ended and untyped — not a goal screen and not a coding console.
3. **The assistant is present, not in the way.** The orb is home *and* a
   persistent companion that shrinks when you work — always reachable, never
   blocking the work.
4. **Voice-first, click-equal.** Anything you can do by speaking you can do by
   clicking, and vice-versa.
5. **Local-first, cloud-connected.** Local execution and local knowledge are the
   default; cloud sync/share extends the vault and coordination model without
   becoming the required path for local work.
6. **Explicit over clever.** Engineer-grade communication: defined answers,
   structured status, no metaphor-soup — built for an operator who wants clarity,
   not vibes.
7. **Bespoke craft.** Deliberate type and color that read as hand-made and
   professional — proof a person designed this, not a generator.

## Accessibility & Inclusion

- **WCAG 2.2 AA** contrast: body text ≥4.5:1 against the dark field; no
  muted-gray-on-dark that fails. (Doubles as ADHD legibility.)
- **`prefers-reduced-motion`**: the orb and all ambient motion have a calm static
  fallback. Non-negotiable — the orb is animation-heavy by design.
- **ADHD-oriented clarity**: explicit state, well-defined copy, predictable layout,
  low cognitive load; never hide critical status behind motion or metaphor.
- **Keyboard + voice parity** with pointer; visible focus states throughout.
