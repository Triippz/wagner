// T000c — Ephemeral node-discovery registry. Test FIRST.
//
// The hub holds, per armed operator, the reachable iroh NodeId + a signaling
// ticket so a verified OWNING peer can resolve it. The registry is ephemeral
// (in-memory, TTL'd) — armed hosts re-register; it stores no run-bearing data.
// Ownership is enforced on resolve: only the same operator_id can resolve their
// own host (FR-201/212).

import { assertEquals } from "@std/assert";
import { DiscoveryRegistry } from "../../src/discovery/registry.ts";

const NOW = 1_000_000;
const clock = { now: NOW };
const reg = () => new DiscoveryRegistry({ nowMs: () => clock.now });

Deno.test("register then owner resolves the armed host", () => {
  clock.now = NOW;
  const r = reg();
  r.register({ operatorId: "op-1", nodeId: "node-aaa", ticket: "tkt-1", ttlMs: 60_000 });
  const got = r.resolve({ operatorId: "op-1", requesterId: "op-1" });
  assertEquals(got?.nodeId, "node-aaa");
  assertEquals(got?.ticket, "tkt-1");
});

Deno.test("non-owner cannot resolve another operator's host", () => {
  clock.now = NOW;
  const r = reg();
  r.register({ operatorId: "op-1", nodeId: "node-aaa", ticket: "tkt-1", ttlMs: 60_000 });
  const got = r.resolve({ operatorId: "op-1", requesterId: "op-2" });
  assertEquals(got, null);
});

Deno.test("resolve of a never-armed operator is null", () => {
  clock.now = NOW;
  const r = reg();
  assertEquals(r.resolve({ operatorId: "ghost", requesterId: "ghost" }), null);
});

Deno.test("expired registration is not resolvable", () => {
  clock.now = NOW;
  const r = reg();
  r.register({ operatorId: "op-1", nodeId: "node-aaa", ticket: "tkt-1", ttlMs: 30_000 });
  clock.now = NOW + 30_001; // strictly past expiry
  assertEquals(r.resolve({ operatorId: "op-1", requesterId: "op-1" }), null);
});

Deno.test("re-register refreshes node/ticket/expiry without duplicating", () => {
  clock.now = NOW;
  const r = reg();
  r.register({ operatorId: "op-1", nodeId: "node-aaa", ticket: "tkt-1", ttlMs: 30_000 });
  clock.now = NOW + 10_000;
  r.register({ operatorId: "op-1", nodeId: "node-bbb", ticket: "tkt-2", ttlMs: 30_000 });
  const got = r.resolve({ operatorId: "op-1", requesterId: "op-1" });
  assertEquals(got?.nodeId, "node-bbb");
  assertEquals(got?.ticket, "tkt-2");
  assertEquals(r.size(), 1);
});

Deno.test("disarm removes the registration", () => {
  clock.now = NOW;
  const r = reg();
  r.register({ operatorId: "op-1", nodeId: "node-aaa", ticket: "tkt-1", ttlMs: 60_000 });
  r.disarm("op-1");
  assertEquals(r.resolve({ operatorId: "op-1", requesterId: "op-1" }), null);
  assertEquals(r.size(), 0);
});
