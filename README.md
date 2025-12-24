# Agent Runtime API (Draft)

This repo currently contains an initial OpenAPI draft for a workflow-first agent runtime that can be called from frontends and SDKs in any language.

- OpenAPI spec: `openapi/agent-runtime.yaml`
- Embedded runtime app: `agent-runtime-app/src/main.rs` (HTTP server over in-memory runtime)
- Dev plan: `TODO.md`
- Rust SDK (initial): `agent-sdk/src/client.rs`

## Design goals (contract-level)

- **Cross-language SDKs**: HTTP + OpenAPI as the source of truth.
- **Structured-first**: inputs/outputs/artifacts are JSON, with schema discovery.
- **Observable-by-default**: events are streamable via SSE (`text/event-stream`).
- **HITL-ready**: human checkpoints are first-class endpoints (approve/reject/provide input).

## How clients use it

- Create a run: `POST /v1/runs` (optionally with `Idempotency-Key`)
- Stream events: `GET /v1/runs/{run_id}/events` with `Accept: text/event-stream`
- Poll status/result: `GET /v1/runs/{run_id}`
- Discover schemas for UI/validation: `GET /v1/workflows/{name}/schemas`

## Local prototype

Run the embedded runtime app:

```bash
cargo run -p agent-runtime-app
```

Minimal request:

```bash
curl -sS -X POST http://127.0.0.1:9000/v1/runs \
  -H 'Content-Type: application/json' \
  -d '{"workflow":{"name":"echo","version":"0.1.0"},"input":{"hello":"world"}}'
```

Stream events (SSE):

```bash
curl -N http://127.0.0.1:9000/v1/runs/<run_id>/events \
  -H 'Accept: text/event-stream'
```

## Notes

- The `/v1/runs/{run_id}/events` endpoint supports both SSE and JSON pagination; clients should prefer SSE when available.
- File artifacts return a `download_url` (typically pre-signed) via `GET /v1/artifacts/{artifact_id}`.
- The gRPC draft (`proto/agent_runtime.proto`) mirrors the HTTP resources and avoids breaking changes by:
  - Keeping names aligned with OpenAPI (`Run`, `Event`, `Artifact`, `HumanCheckpoint`).
  - Using additive-only field evolution and reserving deprecated fields when needed.
  - Treating `WorkflowRef` + `schema_hash` as compatibility anchors.
