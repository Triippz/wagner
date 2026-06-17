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
  /** Every live/known session, keyed by run id (concurrent sessions). */
  runs: Record<string, RunSnapshot>;
  /** The session the console is focused on (the rail's selection). */
  selectedRunId: string | null;
  operatives: Record<string, Operative>;
  transmissions: Transmission[];
  /** The operative the inspector is open on, if any. */
  selectedOperativeId: string | null;
  /** Latest LLM-authored panel per operative (validated; off-vocabulary dropped). */
  panels: Record<string, UiSpec>;
}

export const initialState: WagnerState = {
  runs: {},
  selectedRunId: null,
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

/** Fold a run snapshot into the session map, keyed by its run id (concurrent
 *  sessions). The first session to arrive auto-focuses so the console isn't blank
 *  before the rail makes a selection. */
export function applyRun(
  state: WagnerState,
  run: RunSnapshot
): WagnerState {
  const runs = { ...state.runs, [run.run_id]: run };
  const selectedRunId = state.selectedRunId ?? run.run_id;
  return { ...state, runs, selectedRunId };
}

/** The focused session (what TopBar/Inspector render), or null if none. */
export function activeRun(state: WagnerState): RunSnapshot | null {
  if (state.selectedRunId == null) return null;
  return state.runs[state.selectedRunId] ?? null;
}

/** All sessions for the rail, newest-first by updated_at (created order falls
 *  back to insertion when timestamps are absent/equal). */
export function runList(state: WagnerState): RunSnapshot[] {
  return Object.values(state.runs).sort((a, b) =>
    (b.updated_at ?? "").localeCompare(a.updated_at ?? "")
  );
}

/** Focus a session by id (the rail's click). */
export function selectRun(state: WagnerState, runId: string): WagnerState {
  return { ...state, selectedRunId: runId };
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
