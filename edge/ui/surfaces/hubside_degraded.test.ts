// T026 — Hub-side works with no host; host-side shows unavailable-with-reason
// (FR-204/211, SC-005, US2-AS-5, EC-002).

import { describe, expect, it } from "vitest";
import { capabilityAvailability, isAvailable } from "./capabilities";

describe("capability availability by host reachability", () => {
  it("hub-side (browse, recall) work even with no host reachable", () => {
    const caps = capabilityAvailability(false);
    const browse = caps.find((c) => c.capability === "browse")!;
    const recall = caps.find((c) => c.capability === "recall")!;
    expect(browse.available).toBe(true);
    expect(recall.available).toBe(true);
  });

  it("host-side (run-control, dev-context) are unavailable WITH A REASON when no host", () => {
    const caps = capabilityAvailability(false);
    for (const cap of ["run-control", "dev-context"] as const) {
      const c = caps.find((x) => x.capability === cap)!;
      expect(c.available).toBe(false);
      expect(c.reason, `${cap} must explain why (never silent)`).toBeTruthy();
    }
  });

  it("everything is available when the host is reachable", () => {
    const caps = capabilityAvailability(true);
    expect(caps.every((c) => c.available)).toBe(true);
    expect(caps.every((c) => c.reason === undefined)).toBe(true);
  });

  it("isAvailable mirrors the table", () => {
    expect(isAvailable("recall", false)).toBe(true);
    expect(isAvailable("run-control", false)).toBe(false);
    expect(isAvailable("run-control", true)).toBe(true);
  });
});
