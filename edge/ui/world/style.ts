// Pure visual-style helpers for the operations floor. Kept free of PixiJS so the
// colour/position/label logic is unit-testable; the floor renderer consumes it.

import type { Faction, OperativeState } from "../store/types";
import { DISTRICT_ZONES, type Activity, type District } from "./districts";

/** Faction palette — Architects cyan, Forgers magenta (cyberpunk duotone). */
export function factionColor(faction: Faction): number {
  return faction === "architects" ? 0x21e6c1 : 0xff3caf;
}

/** State-ring colour. Distinct hues so state is not conveyed by motion alone (a11y). */
export function stateRingColor(state: OperativeState): number {
  switch (state) {
    case "idle":
      return 0x4a5568;
    case "thinking":
      return 0xffd54a;
    case "working":
      return 0x4ade80;
    case "blocked":
      return 0xff4242;
  }
}

/** A non-colour glyph per state, so colour-blind users still read state (a11y, CHK026). */
export function stateGlyph(state: OperativeState): string {
  switch (state) {
    case "idle":
      return "○";
    case "thinking":
      return "◔";
    case "working":
      return "◉";
    case "blocked":
      return "✕";
  }
}

export interface Point {
  x: number;
  y: number;
}

/** Pixel position of a district centre on a `width`×`height` stage. */
export function districtCenter(district: District, width: number, height: number): Point {
  const z = DISTRICT_ZONES[district];
  return { x: z.cx * width, y: z.cy * height };
}

/** Seconds an operative takes to glide from one district to another. */
export const TRAVEL_SECS = 0.6;

/** Smoothstep easing (0→1) for inter-district travel — slow-in/slow-out so a
 *  bot accelerates off its old slot and settles into the new one. Clamps input. */
export function easeTravel(t: number): number {
  const c = Math.max(0, Math.min(1, t));
  return c * c * (3 - 2 * c);
}

/** District-circle radius in pixels, proportional to the smaller floor dimension
 *  and clamped to sane bounds — so the circles grow/shrink with the window instead
 *  of staying a fixed 70px (the non-responsive bug). */
export function districtRadius(width: number, height: number): number {
  const base = Math.min(width, height) * 0.11;
  return Math.max(28, Math.min(110, base));
}

/**
 * Fan multiple operatives around their district centre so they don't overlap.
 * Deterministic given (index, count): even ring placement.
 */
export function operativeSlot(
  district: District,
  index: number,
  count: number,
  width: number,
  height: number,
  radius = 46
): Point {
  const c = districtCenter(district, width, height);
  if (count <= 1) return c;
  const angle = (index / count) * Math.PI * 2;
  return { x: c.x + Math.cos(angle) * radius, y: c.y + Math.sin(angle) * radius };
}

/** Animation kinds a vignette prop can play (driven per-frame in scene.ts). */
export const VIGNETTE_ANIMS = ["pulse", "spin", "blink", "scan"] as const;
export type VignetteAnim = (typeof VIGNETTE_ANIMS)[number];

/** A "bot doing real work" prop: a glyph, a short caption, and how it animates.
 *  Maps the activity verb (edit→terminal, test→bench, review→scanner, …) to a
 *  small visual the operative carries so the floor shows *what* it's doing. */
export interface Vignette {
  glyph: string;
  label: string;
  anim: VignetteAnim;
}

/** Longest bubble shown above an operative — one short line, not a wrapped block. */
const BUBBLE_MAX = 40;

/** Reduce an agent message to a single short floor caption. Collapses newlines and
 *  runs of whitespace, then truncates with an ellipsis. The raw message is the whole
 *  agent output; rendering it verbatim (wrapped at 160px) produced the garbled wall. */
export function truncateBubble(message: string | undefined): string {
  const oneLine = (message ?? "").replace(/\s+/g, " ").trim();
  if (oneLine.length <= BUBBLE_MAX) return oneLine;
  return oneLine.slice(0, BUBBLE_MAX - 1) + "…";
}

export function activityVignette(activity: Activity): Vignette {
  switch (activity) {
    case "read":
      return { glyph: "▤", label: "reading", anim: "blink" };
    case "edit":
      return { glyph: "⌨", label: "editing", anim: "pulse" };
    case "test":
      return { glyph: "⚗", label: "testing", anim: "pulse" };
    case "build":
      return { glyph: "⚙", label: "building", anim: "spin" };
    case "lint":
      return { glyph: "✓", label: "linting", anim: "blink" };
    case "shell":
      return { glyph: "❯", label: "shell", anim: "blink" };
    case "review":
      return { glyph: "⊙", label: "reviewing", anim: "scan" };
    case "diff":
      return { glyph: "±", label: "diffing", anim: "blink" };
    case "judge":
      return { glyph: "⚖", label: "judging", anim: "pulse" };
    case "plan":
      return { glyph: "◇", label: "planning", anim: "pulse" };
    case "decompose":
      return { glyph: "⑃", label: "decomposing", anim: "spin" };
    case "think":
      return { glyph: "✦", label: "thinking", anim: "blink" };
    case "await_permission":
      return { glyph: "⏏", label: "awaiting clearance", anim: "blink" };
    case "await_question":
      return { glyph: "?", label: "question", anim: "blink" };
  }
}
