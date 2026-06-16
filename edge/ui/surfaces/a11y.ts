// Surface accessibility helpers (T021a, FR-102, EC-009, D-A11Y-1).
//
// Pure, testable a11y primitives the surface uses:
//  - a `prefers-reduced-motion` alternative for the live-activity strip;
//  - a WCAG contrast-ratio check (AA);
//  - a non-color status mapping (glyph + label) mirroring the Rust tray, so run
//    state is never conveyed by color alone.

export type SurfaceStatus = "idle" | "running" | "needs-you";

/** Whether the live-activity strip should animate. Reduced motion → a static
 *  alternative (a label update instead of a pulsing/looping animation). */
export function shouldAnimateActivity(prefersReducedMotion: boolean): boolean {
  return !prefersReducedMotion;
}

/** Relative luminance of an sRGB color (WCAG 2.x). */
function relativeLuminance({ r, g, b }: { r: number; g: number; b: number }): number {
  const lin = (c: number) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
}

export interface Rgb {
  r: number;
  g: number;
  b: number;
}

/** WCAG contrast ratio between two colors (1..21). */
export function contrastRatio(fg: Rgb, bg: Rgb): number {
  const l1 = relativeLuminance(fg);
  const l2 = relativeLuminance(bg);
  const [light, dark] = l1 >= l2 ? [l1, l2] : [l2, l1];
  return (light + 0.05) / (dark + 0.05);
}

/** AA for normal text requires ≥ 4.5:1; large text ≥ 3:1. */
export function meetsAA(fg: Rgb, bg: Rgb, large = false): boolean {
  return contrastRatio(fg, bg) >= (large ? 3 : 4.5);
}

/** Non-color status presentation — a distinct glyph + text label per state, so
 *  the surface + mobile status never rely on color alone (mirrors the Rust tray). */
export function statusPresentation(status: SurfaceStatus): { glyph: string; label: string } {
  switch (status) {
    case "idle":
      return { glyph: "○", label: "Idle" };
    case "running":
      return { glyph: "◔", label: "Running" };
    case "needs-you":
      return { glyph: "●", label: "Needs you" };
    default: {
      const unreachable: never = status;
      throw new Error(`unknown status: ${String(unreachable)}`);
    }
  }
}
