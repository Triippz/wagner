// T005 — Channel/event schema validation (Article X, D-TEST-3, FR-005).
//
// Every wedge-002 channel message + event kind has a JSON Schema authored with
// draft 2020-12 + `additionalProperties:false`. This test, written FIRST,
// asserts (a) each schema compiles, (b) a valid sample passes, and (c) an
// unknown field is rejected — the structural guarantee F-1 leans on (a hub/
// relay schema can't silently grow a code/diff/file field). T010 authors the
// schemas that make this pass.

import { describe, expect, it } from "vitest";
import Ajv2020 from "ajv/dist/2020";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const DIR = __dirname;
const load = (f: string) => JSON.parse(readFileSync(join(DIR, f), "utf8"));

const ajv = new Ajv2020({ strict: false, allErrors: true });

/** Each schema file + a known-valid sample + the field we expect rejected. */
const CASES: Array<{
  schema: string;
  valid: Record<string, unknown>;
  rejectField: string;
}> = [
  {
    schema: "remote-arm.schema.json",
    valid: {
      schema: "remote-arm.v1",
      operator_id: "op-1",
      node_id: "n0deADdr",
      ticket_id: "tkt-1",
      ts: "2026-06-16T00:00:00Z",
    },
    rejectField: "code",
  },
  {
    schema: "remote-attach.schema.json",
    valid: {
      schema: "remote-attach.v1",
      operator_id: "op-1",
      client_id: "cli-1",
      ticket_id: "tkt-1",
      ts: "2026-06-16T00:00:00Z",
    },
    rejectField: "file_contents",
  },
  {
    schema: "remote-control.schema.json",
    valid: {
      schema: "remote-control.v1",
      client_id: "cli-1",
      kind: "answer_permission",
      ref: "tx-1",
      seq: 1,
      ts: "2026-06-16T00:00:00Z",
    },
    rejectField: "diff",
  },
  {
    schema: "dev-context-cmd.schema.json",
    valid: {
      schema: "dev-context-cmd.v1",
      client_id: "cli-1",
      argv: ["git", "diff"],
      cwd: ".",
      ts: "2026-06-16T00:00:00Z",
    },
    rejectField: "transcript",
  },
  {
    schema: "dev-context-file.schema.json",
    valid: {
      schema: "dev-context-file.v1",
      client_id: "cli-1",
      op: "read",
      path: "src/main.rs",
      ts: "2026-06-16T00:00:00Z",
    },
    rejectField: "contents",
  },
  {
    schema: "remote-event.schema.json",
    valid: {
      schema: "remote-event.v1",
      event_id: "01J0000000000000000000000A",
      run_id: "01J0000000000000000000000B",
      kind: "remote.armed",
      ts: "2026-06-16T00:00:00Z",
    },
    rejectField: "payload",
  },
];

describe("wedge-002 channel + event schemas (Article X)", () => {
  for (const { schema, valid, rejectField } of CASES) {
    describe(schema, () => {
      const validate = ajv.compile(load(schema));

      it("accepts a valid sample", () => {
        const ok = validate(valid);
        expect(validate.errors ?? [], JSON.stringify(validate.errors)).toEqual([]);
        expect(ok).toBe(true);
      });

      it(`rejects an unknown field (${rejectField}) via additionalProperties:false`, () => {
        const bad = { ...valid, [rejectField]: "leak" };
        expect(validate(bad)).toBe(false);
      });

      it("declares draft 2020-12 and additionalProperties:false", () => {
        const raw = load(schema);
        expect(raw.$schema).toBe("https://json-schema.org/draft/2020-12/schema");
        expect(raw.additionalProperties).toBe(false);
      });
    });
  }
});
