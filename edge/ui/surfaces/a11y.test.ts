// T018b — Reduced-motion / contrast / non-color status (EC-009, D-A11Y-1).
//
// The live-activity strip has a `prefers-reduced-motion` alternative; surface +
// mobile status clear AA contrast and never rely on color alone.

import { describe, expect, it } from "vitest";
import {
  contrastRatio,
  meetsAA,
  shouldAnimateActivity,
  statusPresentation,
  type SurfaceStatus,
} from "./a11y";

describe("reduced motion", () => {
  it("animates by default but offers a static alternative under reduced-motion", () => {
    expect(shouldAnimateActivity(false)).toBe(true);
    expect(shouldAnimateActivity(true)).toBe(false);
  });
});

describe("AA contrast", () => {
  it("black on white is the max ratio (21:1) and passes AA", () => {
    const black = { r: 0, g: 0, b: 0 };
    const white = { r: 255, g: 255, b: 255 };
    expect(contrastRatio(black, white)).toBeCloseTo(21, 0);
    expect(meetsAA(black, white)).toBe(true);
  });

  it("the Adyton mint signal (#70fbc7) on black ink clears AA", () => {
    const mint = { r: 0x70, g: 0xfb, b: 0xc7 };
    const ink = { r: 0, g: 0, b: 0 };
    // Mint-needs-dark-ink (the committed Adyton rule) — verify it actually passes.
    expect(meetsAA(ink, mint)).toBe(true);
  });

  it("low-contrast grey-on-grey fails AA", () => {
    const a = { r: 0x88, g: 0x88, b: 0x88 };
    const b = { r: 0x99, g: 0x99, b: 0x99 };
    expect(meetsAA(a, b)).toBe(false);
  });
});

describe("non-color status (never color alone)", () => {
  it("each status carries a distinct glyph + non-empty label", () => {
    const statuses: SurfaceStatus[] = ["idle", "running", "needs-you"];
    const glyphs = statuses.map((s) => statusPresentation(s).glyph);
    expect(new Set(glyphs).size).toBe(3);
    for (const s of statuses) {
      expect(statusPresentation(s).label.length).toBeGreaterThan(0);
    }
  });
});
