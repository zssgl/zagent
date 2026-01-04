# meeting_prebrief_daily v1.0.0

Daily pre-brief workflow that generates a concise data summary before the evening meeting. It focuses on facts, tomorrow's appointments, risks, and an execution checklist.

## Key files

- `workflow.yml`: workflow metadata and file references
- `spec.md`: business goals, boundaries, and guardrails
- `rules.yml`: risk rules, checklist templates, and field mapping
- `input.schema.json`: input contract (JSON Schema)
- `output.schema.json`: output contract (JSON Schema)
- `../../../../configs/meeting_prebrief_thresholds.yml`: thresholds for risk evaluators

## Trigger

Scheduled daily at 16:30 (Asia/Shanghai). Guard checks:

- `his_data_ready=true` (required)
- `appt_data_ready=true`
- `wecom_touch_ready=true`

Degraded behavior when guards fail is defined in `spec.md`.

## Input overview

Minimum required fields:

- `store_id`, `biz_date`, `his.*`

Optional but recommended:

- `appointments_tomorrow` (tomorrow list)
- `wecom_touch` (touch/follow-up signals)
- `baselines` (7d/28d rolling averages)

See `input.schema.json` for the full JSON Schema.

## Output overview

The output always contains four sections:

- `facts_recap`: factual recap and comparisons
- `tomorrow_list`: appointments + follow-ups
- `risks`: audit-able risks (type, evidence, threshold, note)
- `checklist`: execution checklist with owner and due time

Data quality flags are provided under `data_quality`.

See `output.schema.json` for the full JSON Schema.

## Risk rules and thresholds

Rules are defined in `rules.yml`, thresholds in `configs/meeting_prebrief_thresholds.yml`.

- `metric_drop`: today < 7d_avg * metric_drop_ratio
- `target_gap`: gmv_rate < target_gap_rate
- `touch_gap`: no_reply_list >= no_reply_list_min OR no_reply_rate > no_reply_rate_max
- `tomorrow_load`: appointments_count > tomorrow_load_count

## Checklist rules

Checklist items are generated from:

- risk types (one per triggered risk)
- presence of tomorrow list
- fallback item when none apply

See `rules.yml` for templates and due-time format.

## Notes

- Facts must be factual; no causal statements.
- Missing input must be explained in the output (see `spec.md`).
- Break any contract change into a new version directory.
