// The five districts of the operations floor and the activity→district routing.
// Mirrors the Rust-side R-EVENT mapping so the frontend can place an operative
// even before a district field arrives on an event.

export type District = "stacks" | "forge" | "mirror" | "oracle" | "gate";

export type Activity =
  | "read"
  | "edit"
  | "test"
  | "build"
  | "lint"
  | "shell"
  | "review"
  | "diff"
  | "judge"
  | "plan"
  | "decompose"
  | "think"
  | "await_permission"
  | "await_question";

export interface DistrictZone {
  id: District;
  label: string;
  /** Normalized centre (0..1) on the floor — scene.ts scales to pixels. */
  cx: number;
  cy: number;
}

// Layout: a pentagon-ish arrangement; Gate sits front-centre (where the engineer looks).
export const DISTRICT_ZONES: Record<District, DistrictZone> = {
  oracle: { id: "oracle", label: "The Oracle", cx: 0.5, cy: 0.18 },
  stacks: { id: "stacks", label: "The Stacks", cx: 0.18, cy: 0.45 },
  forge: { id: "forge", label: "The Forge", cx: 0.82, cy: 0.45 },
  mirror: { id: "mirror", label: "The Mirror", cx: 0.32, cy: 0.78 },
  gate: { id: "gate", label: "The Gate", cx: 0.68, cy: 0.78 },
};

export function activityToDistrict(activity: Activity): District {
  switch (activity) {
    case "read":
    case "edit":
      return "stacks";
    case "test":
    case "build":
    case "lint":
    case "shell":
      return "forge";
    case "review":
    case "diff":
    case "judge":
      return "mirror";
    case "plan":
    case "decompose":
    case "think":
      return "oracle";
    case "await_permission":
    case "await_question":
      return "gate";
  }
}
