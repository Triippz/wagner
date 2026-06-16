// Pure reducers that fold the host's event/run/transmission stream into UI state.
// Kept pure (no zustand, no PixiJS) so they are unit-testable in isolation.

import type {
  WagnerEvent,
  Operative,
  RunSnapshot,
  Transmission,
} from "./types";
import { validateUiSpec, type UiSpec } from "./uiSpec";

export interface WagnerState {
  run: RunSnapshot | null;
  operatives: Record<string, Operative>;
  transmissions: Transmission[];
  /** The operative the inspector is open on, if any. */
  selectedOperativeId: string | null;
  /** Latest LLM-authored panel per operative (validated; off-vocabulary dropped). */
  panels: Record<string, UiSpec>;
}

export const initialState: WagnerState = {
  run: null,
  operatives: {},
  transmissions: [],
  selectedOperativeId: null,
  panels: {},
};

/** Fold one WagnerEvent into state, creating/updating its operative. */
export function applyEvent(
  state: WagnerState,
  e: WagnerEvent
): WagnerState {
  const prev = state.operatives[e.operative_id];
  // Stale-event guard: never let an older event clobber a newer one.
  if (prev && prev.lastTs > e.ts) return state;

  // Append a transcript line only when the event carries a message (the
  // inspector's "thinking" stream); message-less events just update position.
  const transcript = prev?.transcript ?? [];
  const nextTranscript = e.message
    ? [...transcript, { ts: e.ts, activity: e.activity, message: e.message }]
    : transcript;

  const operative: Operative = {
    id: e.operative_id,
    name: e.operative_name || prev?.name || e.operative_id,
    faction: e.faction,
    district: e.district,
    activity: e.activity,
    state: e.state,
    bubble: e.message ?? prev?.bubble,
    handoffTo: e.handoff_target_operative_id,
    lastTs: e.ts,
    transcript: nextTranscript,
  };
  return {
    ...state,
    operatives: { ...state.operatives, [e.operative_id]: operative },
  };
}

/** Minimal launch-roster shape the floor seeds from (subset of bridge `AgentSpec`).
 *  `engine` is the tagged harness: "claude" leads as an Architect, everything else
 *  (codex / local endpoint) is a Forger. */
export interface SeedAgent {
  id: string;
  name: string;
  engine: "claude" | "codex" | { endpoint: { base_url: string; model: string } };
}

/** Seed the hired roster as idle operatives at the Gate so the floor is populated
 *  the instant a run launches — before any backend event arrives (the empty-floor
 *  bug). An operative already present (from an event) is left untouched. Seeds carry
 *  an empty `lastTs` so the first real event always supersedes the seed. */
export function seedRoster(
  state: WagnerState,
  agents: SeedAgent[]
): WagnerState {
  const operatives = { ...state.operatives };
  for (const a of agents) {
    if (operatives[a.id]) continue;
    operatives[a.id] = {
      id: a.id,
      name: a.name || a.id,
      faction: a.engine === "claude" ? "architects" : "forgers",
      district: "gate",
      activity: "think",
      state: "idle",
      handoffTo: null,
      lastTs: "",
      transcript: [],
    };
  }
  return { ...state, operatives };
}

/** Select (or clear, with null) the operative the inspector is open on. */
export function selectOperative(
  state: WagnerState,
  id: string | null
): WagnerState {
  return { ...state, selectedOperativeId: id };
}

/** Resolve the selected operative, or null if none / not present. */
export function selectedOperative(state: WagnerState): Operative | null {
  if (!state.selectedOperativeId) return null;
  return state.operatives[state.selectedOperativeId] ?? null;
}

/** Store an operative's latest LLM-authored panel. The raw value is validated +
 *  sanitized; an unrenderable spec is ignored (state unchanged). */
export function applyPanel(
  state: WagnerState,
  operativeId: string,
  raw: unknown
): WagnerState {
  const spec = validateUiSpec(raw);
  if (!spec) return state;
  return { ...state, panels: { ...state.panels, [operativeId]: spec } };
}

/** Replace the run snapshot (HUD: status, iteration, cost). */
export function applyRun(
  state: WagnerState,
  run: RunSnapshot
): WagnerState {
  return { ...state, run };
}

/** Open or update a transmission (dedup by id; newest fields win). */
export function applyTransmission(
  state: WagnerState,
  t: Transmission
): WagnerState {
  const idx = state.transmissions.findIndex((x) => x.id === t.id);
  const transmissions =
    idx === -1
      ? [...state.transmissions, t]
      : state.transmissions.map((x) => (x.id === t.id ? t : x));
  return { ...state, transmissions };
}

/** Mark a transmission answered locally (optimistic; host confirms via event). */
export function answerTransmission(
  state: WagnerState,
  id: string,
  response: string
): WagnerState {
  return {
    ...state,
    transmissions: state.transmissions.map((t) =>
      t.id === id ? { ...t, state: "answered", response } : t
    ),
  };
}

/** The currently-open transmission, if any (the one the modal renders). */
export function openTransmission(state: WagnerState): Transmission | null {
  return state.transmissions.find((t) => t.state === "open") ?? null;
}

/** Operatives grouped by district — what the world renderer consumes. */
export function operativesByDistrict(
  state: WagnerState
): Record<string, Operative[]> {
  const out: Record<string, Operative[]> = {};
  for (const op of Object.values(state.operatives)) {
    (out[op.district] ??= []).push(op);
  }
  return out;
}
