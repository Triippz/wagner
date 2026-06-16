import { describe, it, expect } from "vitest";
import {
  activityToDistrict,
  DISTRICT_ZONES,
  type Activity,
} from "../../world/districts";

describe("activityToDistrict (R-EVENT mapping parity with Rust)", () => {
  const cases: Array<[Activity, string]> = [
    ["read", "stacks"],
    ["edit", "stacks"],
    ["test", "forge"],
    ["build", "forge"],
    ["lint", "forge"],
    ["shell", "forge"],
    ["review", "mirror"],
    ["diff", "mirror"],
    ["judge", "mirror"],
    ["plan", "oracle"],
    ["decompose", "oracle"],
    ["think", "oracle"],
    ["await_permission", "gate"],
    ["await_question", "gate"],
  ];

  it.each(cases)("maps %s → %s", (activity, district) => {
    expect(activityToDistrict(activity)).toBe(district);
  });

  it("has a zone for every district with normalized coordinates", () => {
    for (const zone of Object.values(DISTRICT_ZONES)) {
      expect(zone.cx).toBeGreaterThanOrEqual(0);
      expect(zone.cx).toBeLessThanOrEqual(1);
      expect(zone.cy).toBeGreaterThanOrEqual(0);
      expect(zone.cy).toBeLessThanOrEqual(1);
      expect(zone.label.length).toBeGreaterThan(0);
    }
  });
});
