import { describe, it, expect } from "vitest";
import { Roster, ARCHITECT_NAMES, FORGER_NAMES } from "../../world/roster";

describe("Roster", () => {
  it("assigns faction-appropriate handles", () => {
    const r = new Roster();
    const a = r.assign("op1", "architects");
    const f = r.assign("op2", "forgers");
    expect(ARCHITECT_NAMES).toContain(a as (typeof ARCHITECT_NAMES)[number]);
    expect(FORGER_NAMES).toContain(f as (typeof FORGER_NAMES)[number]);
  });

  it("is stable for the same operative id", () => {
    const r = new Roster();
    expect(r.assign("op1", "architects")).toBe(r.assign("op1", "architects"));
  });

  it("hands out unique handles to concurrent operatives", () => {
    const r = new Roster();
    const handles = new Set(
      Array.from({ length: ARCHITECT_NAMES.length }, (_, i) =>
        r.assign(`op${i}`, "architects")
      )
    );
    expect(handles.size).toBe(ARCHITECT_NAMES.length);
  });

  it("recycles a handle after release", () => {
    const r = new Roster();
    const first = r.assign("op1", "architects");
    r.release("op1", "architects");
    const reused = r.assign("op2", "architects");
    expect(reused).toBe(first);
  });

  it("falls back to numbered handles when the pool is exhausted", () => {
    const r = new Roster();
    for (let i = 0; i < FORGER_NAMES.length; i++) r.assign(`op${i}`, "forgers");
    const overflow = r.assign("extra", "forgers");
    expect(overflow).toMatch(/^Forger-\d+$/);
  });
});
