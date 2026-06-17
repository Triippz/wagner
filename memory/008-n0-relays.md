---
title: Vault sync uses n0 public relays for now
summary: Live iroh vault sync uses iroh::RelayMode::Default (n0's hosted relays) via the vault_relay_mode() seam; swap to our own relay URLs later (one-line change).
tags: [decision, sync, iroh, network]
tier: supporting
---

# n0 relays for now

**Decided:** 2026-06-17 by the operator ("use the n0 relays for now we have our
own we'll add later").

Live distributed Vault sync (`GossipSyncAdapter`/`IrohDocsStore`) builds its iroh
`Endpoint` with `iroh::RelayMode::Default` — n0's public hosted relays — exposed
through the `vault_relay_mode()` seam (`edge/host/src/vault/sync_adapter.rs`,
re-exported from `vault/mod.rs`). Swapping to our own relay is a one-line change
there (return `RelayMode::Custom(our_urls)`).

**Proven:** `make sync-e2e` runs a REAL two-peer sync over n0
(`edge/host/tests/vault_sync_n0_e2e.rs`, `#[ignore]` so `make verify` stays
offline-safe): peer A broadcasts a loro snapshot, peer B merges and converges.
This is genuine distributed E2E on macOS (answers "can't do E2E on Mac" — the
tauri-driver GUI path is unavailable on macOS, but the system/transport E2E is
real and runs here). See [[007-worktree-lane-integration]] for integration recipe.
