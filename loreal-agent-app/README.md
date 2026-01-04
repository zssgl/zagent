# Loreal Agent App

This folder contains workflow definitions and supporting configs for the Loreal agent runtime.

## Layout

- `workflows/<workflow_id>/vX.Y.Z/`
  - `workflow.yml`: workflow metadata and file references
  - `spec.md`: business spec, scope, and guardrails
  - `rules.yml`: executable rules and checklist templates
  - `input.schema.json`: input contract (JSON Schema)
  - `output.schema.json`: output contract (JSON Schema)
- `configs/`: shared thresholds and parameters used by rules

## Versioning

Each workflow version is isolated under `workflows/<workflow_id>/vX.Y.Z/`. Update rules or schemas by adding a new version directory and pointing `workflow.yml` to the new files.

## Current workflows

- `meeting_prebrief_daily` (v2.0.0): daily pre-brief for store managers before the evening meeting (aligned to the 夕会前简报模板).

## Editing tips

- Keep `spec.md` aligned with `rules.yml` and the JSON schemas.
- Prefer additive changes; breaking changes should be done in a new version folder.
