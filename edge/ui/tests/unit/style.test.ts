import { describe, it, expect } from "vitest";
import {
  factionColor,
  stateRingColor,
  stateGlyph,
  districtCenter,
  operativeSlot,
  activityVignette,
  truncateBubble,
  districtRadius,
  easeTravel,
  VIGNETTE_ANIMS,
} from "../../world/style";
import type { Activity } from "../../world/districts";

describe("style helpers", () => {
  it("gives factions distinct colours", () => {
    expect(factionColor("architects")).not.toBe(factionColor("forgers"));
  });

  it("gives every state a distinct ring colour and glyph (a11y: not colour-only)", () => {
    const states = ["idle", "thinking", "working", "blocked"] as const;
    const colors = new Set(states.map(stateRingColor));
    const glyphs = new Set(states.map(stateGlyph));
    expect(colors.size).toBe(4);
    expect(glyphs.size).toBe(4);
  });

  it("places a district centre inside the stage", () => {
    const p = districtCenter("gate", 1000, 800);
    expect(p.x).toBeGreaterThan(0);
    expect(p.x).toBeLessThan(1000);
    expect(p.y).toBeGreaterThan(0);
    expect(p.y).toBeLessThan(800);
  });

  it("returns the centre for a single operative", () => {
    const c = districtCenter("forge", 1000, 800);
    expect(operativeSlot("forge", 0, 1, 1000, 800)).toEqual(c);
  });

  it("fans multiple operatives to distinct, non-identical slots", () => {
    const a = operativeSlot("stacks", 0, 3, 1000, 800);
    const b = operativeSlot("stacks", 1, 3, 1000, 800);
    const c = operativeSlot("stacks", 2, 3, 1000, 800);
    expect(a).not.toEqual(b);
    expect(b).not.toEqual(c);
    expect(a).not.toEqual(c);
  });

  it("gives every activity a vignette (glyph + label + known animation)", () => {
    const activities: Activity[] = [
      "read", "edit", "test", "build", "lint", "shell", "review",
      "diff", "judge", "plan", "decompose", "think", "await_permission", "await_question",
    ];
    for (const a of activities) {
      const v = activityVignette(a);
      expect(v.glyph.length).toBeGreaterThan(0);
      expect(v.label.length).toBeGreaterThan(0);
      expect(VIGNETTE_ANIMS).toContain(v.anim);
    }
    // Distinct work reads as distinct props: edit ≠ test ≠ review.
    expect(activityVignette("edit").glyph).not.toBe(activityVignette("test").glyph);
    expect(activityVignette("review").glyph).not.toBe(activityVignette("plan").glyph);
  });

  describe("easeTravel", () => {
    it("pins the endpoints and the midpoint", () => {
      expect(easeTravel(0)).toBe(0);
      expect(easeTravel(1)).toBe(1);
      expect(easeTravel(0.5)).toBeCloseTo(0.5, 5);
    });

    it("clamps out-of-range input", () => {
      expect(easeTravel(-1)).toBe(0);
      expect(easeTravel(2)).toBe(1);
    });

    it("is monotonic and eases in/out (slower than linear at the ends)", () => {
      expect(easeTravel(0.25)).toBeLessThan(0.25);
      expect(easeTravel(0.75)).toBeGreaterThan(0.75);
      expect(easeTravel(0.3)).toBeLessThan(easeTravel(0.7));
    });
  });

  describe("districtRadius", () => {
    it("is positive and scales with the smaller floor dimension", () => {
      expect(districtRadius(1200, 900)).toBeGreaterThan(0);
      expect(districtRadius(1200, 900)).toBeGreaterThan(districtRadius(400, 300));
    });

    it("clamps to sane bounds at extreme sizes", () => {
      const tiny = districtRadius(100, 80);
      const huge = districtRadius(8000, 6000);
      expect(tiny).toBeGreaterThanOrEqual(20);
      expect(huge).toBeLessThanOrEqual(120);
    });
  });

  describe("truncateBubble", () => {
    it("returns empty for missing or blank input", () => {
      expect(truncateBubble(undefined)).toBe("");
      expect(truncateBubble("")).toBe("");
      expect(truncateBubble("   ")).toBe("");
    });

    it("collapses newlines and internal whitespace to a single line", () => {
      expect(truncateBubble("editing\nlib.rs\t now")).toBe("editing lib.rs now");
    });

    it("truncates a long message to one short line with an ellipsis", () => {
      const long = "a".repeat(200);
      const out = truncateBubble(long);
      expect(out.length).toBeLessThanOrEqual(40);
      expect(out.endsWith("…")).toBe(true);
    });

    it("leaves a short message intact (no ellipsis)", () => {
      expect(truncateBubble("editing lib.rs")).toBe("editing lib.rs");
    });
  });
});
