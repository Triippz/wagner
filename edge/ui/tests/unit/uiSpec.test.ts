// P5 — the sandboxed UI-spec validator. An agent emits a JSON tree of whitelisted
// primitives; the validator drops anything off-vocabulary so a deterministic
// renderer can map only known, safe blocks (no eval, no raw HTML).

import { describe, it, expect } from "vitest";
import { validateUiSpec, type UiSpec } from "../../store/uiSpec";

describe("validateUiSpec", () => {
  it("keeps the seven whitelisted block kinds", () => {
    const raw = {
      title: "Slugify work",
      blocks: [
        { kind: "text", text: "Implemented slugify." },
        { kind: "badge", label: "tests green", tone: "ok" },
        { kind: "progress", value: 0.5, label: "halfway" },
        { kind: "list", items: ["a", "b"], ordered: true },
        { kind: "code", code: "fn slug() {}", lang: "rust" },
        { kind: "kv", pairs: [{ key: "files", value: "2" }] },
        { kind: "timeline", steps: [{ label: "plan", state: "done" }] },
      ],
    };
    const spec = validateUiSpec(raw) as UiSpec;
    expect(spec.title).toBe("Slugify work");
    expect(spec.blocks).toHaveLength(7);
    expect(spec.blocks.map((b) => b.kind)).toEqual([
      "text", "badge", "progress", "list", "code", "kv", "timeline",
    ]);
  });

  it("drops off-vocabulary block kinds (e.g. a script/html injection attempt)", () => {
    const raw = {
      blocks: [
        { kind: "script", src: "evil.js" },
        { kind: "html", html: "<img onerror=alert(1)>" },
        { kind: "text", text: "safe" },
      ],
    };
    const spec = validateUiSpec(raw) as UiSpec;
    expect(spec.blocks).toHaveLength(1);
    expect(spec.blocks[0]!.kind).toBe("text");
  });

  it("clamps progress to 0..1 and coerces bad fields", () => {
    const spec = validateUiSpec({ blocks: [{ kind: "progress", value: 9 }] }) as UiSpec;
    const block = spec.blocks[0]!;
    expect(block.kind).toBe("progress");
    if (block.kind === "progress") expect(block.value).toBe(1);
  });

  it("drops malformed blocks of a known kind (missing required field)", () => {
    const spec = validateUiSpec({
      blocks: [
        { kind: "text" }, // missing text
        { kind: "badge", label: "ok" },
      ],
    }) as UiSpec;
    expect(spec.blocks).toHaveLength(1);
    expect(spec.blocks[0]!.kind).toBe("badge");
  });

  it("returns null when there is no valid spec at all", () => {
    expect(validateUiSpec(null)).toBeNull();
    expect(validateUiSpec({})).toBeNull();
    expect(validateUiSpec({ blocks: [{ kind: "script" }] })).toBeNull();
    expect(validateUiSpec("not an object")).toBeNull();
  });

  it("ignores an unknown badge tone, falling back to info", () => {
    const spec = validateUiSpec({
      blocks: [{ kind: "badge", label: "x", tone: "rainbow" }],
    }) as UiSpec;
    const b = spec.blocks[0]!;
    if (b.kind === "badge") expect(b.tone).toBe("info");
  });
});
