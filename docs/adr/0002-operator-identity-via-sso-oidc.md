# ADR-0002: Operator identity via corporate SSO (OIDC)

**Date:** 2026-06-15
**Status:** Accepted

## Context

The wedge attributes every synced run and learning to a verified operator (FR-001/FR-002).
FR-002 deliberately left the mechanism open ("token / SSO / device key… a plan-phase
decision"). Plan `R-1` then picked a hand-rolled "register with email + server-generated
secret + bearer token" and **explicitly deferred SSO to Phase 2**.

Two facts reopen that:

1. The platform is **employees-only** — access must be gated to authorized devs/employees.
   R-1 has no such gate.
2. R-1 makes the **hub a credential authority** (store + hash secrets, issue/verify tokens,
   own reset) — dangerous surface to build and operate. Identity is also load-bearing and
   painful to retrofit once learnings are attributed to it.

The organization's IdPs are **Google** and **JumpCloud**; both speak **OIDC**.

## Decision

Operator identity is established by **OIDC against the org IdPs** — Google and JumpCloud as
configurable issuers behind one OIDC client. The hub **stores no credentials**: it validates
an IdP-issued ID token (signature, issuer, audience) and gates access by verified email
domain / IdP group. `operator_id` is the IdP-issued subject (stable across logins). The Tauri
edge uses the **OIDC Authorization-Code + PKCE** native-app flow (loopback redirect).
Authentication is required **only at the sync/recall boundary**, so edge autonomy is preserved
(Article VI): with the hub or IdP unreachable, the run still completes locally, sync queues,
and recall returns empty until re-auth.

This authenticates *who gets in*; it does not add an insider-malice threat model. The spec's
"operators are mutually trusted; no hostile-operator model" assumption (spec.md:127) stands.

## Alternatives Considered

- **Hand-rolled email + secret + bearer (plan R-1).** Rejected: makes the hub a credential
  authority (larger, more dangerous surface), provides no employees-only gate, and forces an
  identity migration when SSO arrives later.
- **SAML.** Rejected: OIDC + PKCE fits a native desktop/CLI client better than SAML's
  browser-POST assertion model.
- **Device key / mTLS.** Rejected: heavier certificate management, and it proves *device*
  identity, not *person* identity — the wrong unit for attribution.

## Consequences

- **Easier:** an employees-only gate is real from day one; the hub stores no secrets (smaller
  credential surface than R-1); one OIDC integration covers both IdPs; the identity foundation
  won't need migrating as the platform grows into Phase 2 workers.
- **Harder / accepted:** the edge gains an OIDC client + PKCE loopback/browser handoff + a
  refresh-token strategy (wedge work the current tasks lack); sync/recall auth now has a hard
  external dependency on the IdP (offline runs still complete and queue sync).

## Supersedes

Plan `R-1`. Refines FR-002's mechanism and updates the Complexity-Tracking "verified-auth
surface" entry. Applied to the spec tree by a follow-on `/spec amend`.
