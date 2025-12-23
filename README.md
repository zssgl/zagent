# Agent Runtime API (Draft)

This repo currently contains an initial OpenAPI draft for a workflow-first agent runtime that can be called from frontends and SDKs in any language.

- OpenAPI spec: `openapi/agent-runtime.yaml`

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

## Notes

- The `/v1/runs/{run_id}/events` endpoint supports both SSE and JSON pagination; clients should prefer SSE when available.
- File artifacts return a `download_url` (typically pre-signed) via `GET /v1/artifacts/{artifact_id}`.
