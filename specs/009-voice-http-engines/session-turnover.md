# Session turnover: Voice HTTP Engines — HttpStt / HttpTts

## 2026-06-17T00:00:00Z — plan drafted

**Phase:** Plan (pre-approval)
**Last commit:** dca39f2 (docs: [repo] build-kickoff handoff for the first session in the standalone repo)
**Branch:** feat/autonomous-build
**In-flight task:** n/a (planning phase only; no implementation started)
**Resume at:** on operator approval
**Notes:**

Spec tree created at `specs/009-voice-http-engines/`. All artifacts written:

- `spec.md` — three P1 stories (A: HttpStt, B: HttpTts, C: re-exports + router)
- `research.md` — five external URLs (faster-whisper-server, Kokoro-FastAPI, reqwest multipart, Tokio TcpListener) + three internal prior-art references
- `plan.md` — constitution check (all pass), technical context, approach, project structure
- `tasks.md` — 23 tasks across 5 phases: Foundational (reqwest multipart) → P1-A → P1-B → P1-C → Integration/Commit
- `quickstart.md` — per-story test commands
- `contracts/adr-pointers.md` — ADR context for sidecar transport, multipart flag, timeout values, confidence convention
- `session-turnover.md` — this file

Key findings from research:
1. `reqwest` `multipart` feature not yet enabled in workspace `Cargo.toml` — must be added before `HttpStt` can compile.
2. Mock server pattern already exists in `edge/host/tests/permission_server.rs` — exact same `TcpListener::bind("127.0.0.1:0")` approach; no new test framework needed.
3. STT endpoint: `POST /v1/audio/transcriptions`, multipart/form-data, fields `file` + `model`, response `{"text":"..."}`.
4. TTS endpoint: `POST /v1/audio/speech`, JSON body, raw binary response.
5. All constitution articles pass. No blockers.

Next action after operator approval: begin at Phase 0 (bd-TBD-1, add `multipart` to workspace `Cargo.toml`), then proceed through tasks in order.
