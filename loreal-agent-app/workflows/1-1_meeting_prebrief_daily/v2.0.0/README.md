# meeting_prebrief_daily v2.0.0

This version is redesigned to align with `../【夕会前】日会数据简报模板.md` and provides both:

- `report_md`: a WeCom-ready Markdown brief (human readable)
- structured JSON sections for UI/automation (`facts_recap`, `risks`, `tomorrow_list`, `checklist`)

## Key files

- `workflow.yml`: workflow metadata and file references
- `spec.md`: business goals, boundaries, and guardrails
- `rules.yml`: risk rules, checklist templates, and field mapping
- `input.schema.json`: input contract (JSON Schema)
- `output.schema.json`: output contract (JSON Schema)
- `../../../configs/meeting_prebrief_thresholds.yml`: shared thresholds for risk evaluators

## What changed vs v1.0.0

- Expanded `facts_recap` to better match the briefing template (today, MTD, staff, customers, key items, execution).
- Added `report_md` generation in the workflow runner (still keeps the structured output).
- Added more risk types (gmv/consumption/visits/avg_ticket + target gaps), still audit-able with thresholds and evidence fields.

## Data assembly (MySQL)

This workflow expects a structured `input` JSON. For local testing, you can assemble the input from MySQL using:

- `cargo run -p loreal-agent-app --bin assemble_meeting_prebrief_daily_1_1 -- --biz-date 2025-12-30 --store-id <store_id> --store-name <store_name> --cutoff-time 16:12`

Then `POST /v1/runs` with the printed JSON as the `input`.

### Single-request mode (recommended for POC)

You can also send a single `POST /v1/runs` request with minimal input, and let the server assemble missing fields from MySQL:

```json
{
  "workflow": { "name": "meeting_prebrief_daily" },
  "context": { "assemble": { "source": "mysql" } },
  "input": {
    "store_id": "hz_xizi",
    "store_name": "杭州西子诊所",
    "biz_date": "2025-12-30",
    "data_cutoff_time": "16:12"
  }
}
```

If `context.assemble.source=mysql` is set, the server reads `DATABASE_URL` and fills `his` + `appointments_tomorrow` best-effort, then merges your provided `input` as overrides.
