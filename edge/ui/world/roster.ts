// Operative name roster (FR-021). Each faction has a fixed pool of handles;
// the assigner hands out unique names per active operative and recycles them
// when an operative retires.

import type { Faction } from "../store/types";

export const ARCHITECT_NAMES = [
  "Cipher",
  "Echo",
  "Halcyon",
  "Vesper",
  "Solace",
  "Quill",
] as const;

export const FORGER_NAMES = [
  "Raze",
  "Glitch",
  "Onyx",
  "Nyx",
  "Forge",
  "Ember",
] as const;

export function namesFor(faction: Faction): readonly string[] {
  return faction === "architects" ? ARCHITECT_NAMES : FORGER_NAMES;
}

/** Hands out unique faction handles and recycles them on release. */
export class Roster {
  private assigned = new Map<string, string>(); // operativeId → handle
  private used: Record<Faction, Set<string>> = {
    architects: new Set(),
    forgers: new Set(),
  };

  /** Assign (or return the existing) handle for an operative id. */
  assign(operativeId: string, faction: Faction): string {
    const existing = this.assigned.get(operativeId);
    if (existing) return existing;

    const pool = namesFor(faction);
    const free = pool.find((n) => !this.used[faction].has(n));
    // Fall back to a numbered handle if the pool is exhausted.
    const handle = free ?? `${faction === "architects" ? "Architect" : "Forger"}-${this.used[faction].size + 1}`;
    this.used[faction].add(handle);
    this.assigned.set(operativeId, handle);
    return handle;
  }

  /** Release an operative's handle back to the pool. */
  release(operativeId: string, faction: Faction): void {
    const handle = this.assigned.get(operativeId);
    if (!handle) return;
    this.used[faction].delete(handle);
    this.assigned.delete(operativeId);
  }

  handleOf(operativeId: string): string | undefined {
    return this.assigned.get(operativeId);
  }
}
