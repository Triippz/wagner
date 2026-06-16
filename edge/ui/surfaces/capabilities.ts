// Capability availability by host-reachability (T031, FR-204/211, R5).
//
// The surface splits capabilities by what they need:
//  - HUB-SIDE (browse shared learnings, recall) work on any surface with NO host
//    reachable — they hit the hub directly.
//  - HOST-SIDE (run-control ①, dev-context ②) need a reachable host over iroh.
// When a host-side capability is unavailable the surface shows it disabled WITH A
// REASON — never a silent dead-end (US2-AS-5).

export type Capability = "browse" | "recall" | "run-control" | "dev-context";

export interface CapabilityAvailability {
  capability: Capability;
  available: boolean;
  /** Why it is unavailable — shown to the operator (never silent). */
  reason?: string;
}

const HOST_SIDE: Capability[] = ["run-control", "dev-context"];
const HUB_SIDE: Capability[] = ["browse", "recall"];

/** Compute availability for every capability given host reachability. */
export function capabilityAvailability(hostReachable: boolean): CapabilityAvailability[] {
  const hub = HUB_SIDE.map((capability) => ({ capability, available: true }));
  const host = HOST_SIDE.map((capability) => ({
    capability,
    available: hostReachable,
    ...(hostReachable ? {} : { reason: "Host unreachable — reconnect to act on this machine." }),
  }));
  return [...hub, ...host];
}

/** Convenience: is a single capability available right now? */
export function isAvailable(capability: Capability, hostReachable: boolean): boolean {
  return HUB_SIDE.includes(capability) || hostReachable;
}
