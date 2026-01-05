# meeting_prebrief_daily v2.0.0 (1-1｜门店店长·每日夕会智能体·会前)

Aligned to `../【夕会前】日会数据简报模板.md`, this workflow generates:

- `report_md`: WeCom-ready Markdown briefing (human readable)
- Structured JSON for UI/automation: `facts_recap`, `tomorrow_list`, `risks`, `checklist`
- A local copy: `reports/briefing_YYYYMMDD.md` (override with `REPORTS_DIR`)

## Key files

- `workflow.yml`: workflow metadata
- `spec.md`: business goals/boundaries
- `rules.yml`: risk thresholds + checklist templates
- `input.schema.json`, `output.schema.json`: contracts
- `INTEGRATION.md`: integration checklist (client-facing)

## Run (recommended: single request)

Start the service:

- `cargo run -p loreal-agent-app`

Send one request (minimal input + MySQL assembly on):

```json
{
  "workflow": { "name": "meeting_prebrief_daily" },
  "context": { "assemble": { "source": "mysql" } },
  "input": {
    "store_id": "<clinics.ID>",
    "store_name": "<optional>",
    "biz_date": "2025-12-30",
    "data_cutoff_time": "16:12",
    "mtd": { "gmv_target": 2200000, "consumption_target": 2000000 }
  }
}
```

Notes:

- `store_id` should be `clinics.ID` (it also matches `appointments.OrginizationId` in this DB).
- `mtd.*_target` is not reliably available from the current DB schema; pass it in if you want target-gap risks and “月度完成度”.

## MySQL data assembly coverage (best-effort)

When `context.assemble.source=mysql` is present, the workflow calls the MySQL tool to fill missing fields, then merges your provided `input` as overrides.

Currently filled:

- `his`: `gmv`, `visits`, `avg_ticket`, `top_items` + `appointments`(today) + `deals`(today)
- `baselines.rolling_7d`: daily-avg of `gmv` and `visits` (from `bills`)
- `mtd`: `gmv`, `consumption`, `time_progress` (targets come from request input if provided)
- `tomorrow_list` source: `appointments` + `appointmentlines` (tomorrow window)
- `staff_stats`: today/MTD GMV by staff (via `billemployees` or fallback to `bills.CreateEmpId`)
- `customer_summary`: new vs old (by first bill date in this clinic) + new-customer source breakdown (via `customers.LaiYuanID -> customdictionary`)
- `customer_summary.single_item_customers`: today visitors whose last-12m distinct `ItemName` == 1 (best-effort)
- `customer_summary.vip_customers`: today visitors whose latest `customer_level_historys.new_level` matches `%VIP%` (best-effort)
- `key_items_mtd`: top items MTD (via `billoperationrecorditems`)
- `task_execution`: follow-up/photo/emr/prescription rates (best-effort) + `missing_photo_list` (sample)

## Remaining gaps vs template (needs definition or extra data sources)

- **口径确认**
  - “消耗”与“开单”的严格口径（目前两者都用 `bills.PayAmount` best-effort）
  - “到店人数/成交人数”的定义（目前基于当日有账单的 distinct customer）
- **目标来源**
  - 月度目标（模板提到外部 Excel/运营下发）；DB 内无稳定表/字段
- **渠道与老带新**
  - “老带新/小红书/大众点评”等需要 `LaiYuanID` 与业务映射字典；目前只输出 `customdictionary.DisplayName`
  - “美丽基金”校验相关表/规则未接入
- **VIP/VVIP 定义**
  - 目前仅用 `customer_level_historys.new_level LIKE '%VIP%'` best-effort；需明确等级/标签标准
- **关键品项 WOW / 同期对比**
  - 需要明确 “上月同期/过往三月同期” 窗口与对比口径
- **任务执行明细**
  - 目前只覆盖部分表（照片/病历/处方/回访）；其它任务（术后提醒、交接、AI面诊摘要等）需明确数据源与字段

## Where LLM helps (optional, guarded)

LLM is not used for numbers. Recommended usage is strictly for narrative/summary, with hard rules:

- Generate “智能总结”段落：只允许引用已计算的事实字段；禁止编造数字
- Convert `risks + facts` into concise “会前重点/行动建议”措辞（仍要可追溯到字段）
- (Later) For meeting audio workflows (1-2/1-5): summarization and action extraction from transcript
