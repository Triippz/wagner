// Sandboxed UI-spec (Phase 5). Agents emit a constrained JSON tree of whitelisted
// primitives; a deterministic renderer maps it to React components. This is
// "LLM-emitted UI" WITHOUT executing model-authored code — no eval, no
// dangerouslySetInnerHTML, no remote scripts. The validator is the security
// boundary: anything off-vocabulary or malformed is dropped before render.

export type BadgeTone = "info" | "ok" | "warn" | "danger";
const TONES: BadgeTone[] = ["info", "ok", "warn", "danger"];

export type TimelineState = "done" | "active" | "pending";
const TL_STATES: TimelineState[] = ["done", "active", "pending"];

export type UiBlock =
  | { kind: "text"; text: string }
  | { kind: "badge"; label: string; tone: BadgeTone }
  | { kind: "progress"; value: number; label?: string }
  | { kind: "list"; items: string[]; ordered: boolean }
  | { kind: "code"; code: string; lang?: string }
  | { kind: "kv"; pairs: { key: string; value: string }[] }
  | { kind: "timeline"; steps: { label: string; state: TimelineState }[] };

export interface UiSpec {
  title?: string;
  blocks: UiBlock[];
}

const isObj = (v: unknown): v is Record<string, unknown> =>
  typeof v === "object" && v !== null && !Array.isArray(v);
const str = (v: unknown): string | null => (typeof v === "string" ? v : null);

/** Validate + sanitize one block, or return null to drop it. */
function validateBlock(raw: unknown): UiBlock | null {
  if (!isObj(raw)) return null;
  switch (raw.kind) {
    case "text": {
      const text = str(raw.text);
      return text === null ? null : { kind: "text", text };
    }
    case "badge": {
      const label = str(raw.label);
      if (label === null) return null;
      const tone = TONES.includes(raw.tone as BadgeTone) ? (raw.tone as BadgeTone) : "info";
      return { kind: "badge", label, tone };
    }
    case "progress": {
      if (typeof raw.value !== "number" || Number.isNaN(raw.value)) return null;
      const value = Math.max(0, Math.min(1, raw.value));
      const label = str(raw.label) ?? undefined;
      return { kind: "progress", value, label };
    }
    case "list": {
      if (!Array.isArray(raw.items)) return null;
      const items = raw.items.filter((i): i is string => typeof i === "string");
      if (items.length === 0) return null;
      return { kind: "list", items, ordered: raw.ordered === true };
    }
    case "code": {
      const code = str(raw.code);
      if (code === null) return null;
      return { kind: "code", code, lang: str(raw.lang) ?? undefined };
    }
    case "kv": {
      if (!Array.isArray(raw.pairs)) return null;
      const pairs = raw.pairs
        .map((p) => (isObj(p) ? { key: str(p.key), value: str(p.value) } : null))
        .filter((p): p is { key: string; value: string } => !!p && p.key !== null && p.value !== null);
      if (pairs.length === 0) return null;
      return { kind: "kv", pairs };
    }
    case "timeline": {
      if (!Array.isArray(raw.steps)) return null;
      const steps = raw.steps
        .map((s) => {
          if (!isObj(s)) return null;
          const label = str(s.label);
          if (label === null) return null;
          const state = TL_STATES.includes(s.state as TimelineState)
            ? (s.state as TimelineState)
            : "pending";
          return { label, state };
        })
        .filter((s): s is { label: string; state: TimelineState } => !!s);
      if (steps.length === 0) return null;
      return { kind: "timeline", steps };
    }
    default:
      return null; // off-vocabulary — dropped
  }
}

/** Validate a raw (LLM-authored) value into a safe `UiSpec`, or null if nothing
 *  renderable survives. Unknown/malformed blocks are silently dropped. */
export function validateUiSpec(raw: unknown): UiSpec | null {
  if (!isObj(raw) || !Array.isArray(raw.blocks)) return null;
  const blocks = raw.blocks.map(validateBlock).filter((b): b is UiBlock => b !== null);
  if (blocks.length === 0) return null;
  const title = str(raw.title) ?? undefined;
  return { title, blocks };
}
