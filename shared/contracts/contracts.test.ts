// T016 [US3] — the generated TS bindings compile (`tsc -p shared/tsconfig.json`)
// for every stable-tier core type: representative values are constructed against
// the generated barrel, so a Rust→TS shape drift fails compilation. Covers
// SC-007, AS-1.
// T017 [US3] — a representative payload validates against its committed
// `edge/host/schemas/bus/*.json` via ajv (the 2020-12 dialect). Covers AS-2,
// D-TEST-3.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import Ajv2020 from "ajv/dist/2020.js";
import { describe, expect, it } from "vitest";
import type { Command, Envelope, Event, PluginManifest } from "./index";

// T016: representative values of every stable-tier core type. The annotations
// are the compile-time proof (SC-007); a drift in the generated shape makes tsc
// reject these.
const event: Event = {
  type: "vault",
  data: { type: "note_updated", data: { path: "n.md", rev: 1 } },
};
const command: Command = {
  type: "run",
  data: { type: "start", data: { goal: "build" } },
};
const manifest: PluginManifest = {
  participants_provided: ["ui-gateway"],
  emits: [],
  subscribes: ["ui"],
  registered_schemas: [],
  capabilities: [],
  stability: "stable",
};
const envelope: Envelope = {
  schema: "envelope.v1",
  id: "01ARZ3NDEKTSV4RRFFQ69G5FAV",
  ts: "2026-06-19T00:00:00Z",
  origin: { node: "z32-node-id", kind: "system", name: "host", instance: "01ARZ3NDEKTSV4RRFFQ69G5FAV" },
  stream: { type: "run", data: "01J0RUN" },
  seq: 0,
  scope: { user: "u", workspace: "w" },
  payload: event,
};

const schemasDir = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "edge", "host", "schemas", "bus");
function committedSchema(name: string): object {
  return JSON.parse(readFileSync(join(schemasDir, `${name}.json`), "utf8")) as object;
}

describe("US3 — generated bus contracts", () => {
  it("stable core types are constructible against the generated bindings (SC-007)", () => {
    expect(event.type).toBe("vault");
    expect(command.type).toBe("run");
    expect(manifest.stability).toBe("stable");
    expect(envelope.schema).toBe("envelope.v1");
  });

  it("a representative payload validates against its committed schema via ajv (D-TEST-3)", () => {
    const ajv = new Ajv2020({ strict: false });
    const cases: ReadonlyArray<readonly [string, unknown]> = [
      ["event", event],
      ["command", command],
      ["plugin_manifest", manifest],
      ["envelope", envelope],
    ];
    for (const [name, payload] of cases) {
      const validate = ajv.compile(committedSchema(name));
      expect(validate(payload), `${name}: ${ajv.errorsText(validate.errors)}`).toBe(true);
    }
  });
});
