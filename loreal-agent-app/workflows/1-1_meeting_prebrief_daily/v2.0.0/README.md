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
- Added `report_md` generation in the generic runner (additive; still keeps the structured output).
- Added more risk types (gmv/consumption/visits/avg_ticket + target gaps), still audit-able with thresholds and evidence fields.
