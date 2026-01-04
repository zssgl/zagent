# workflows/meeting_prebrief_daily/v2.0.0/spec.md

## 0. Meta｜元信息

```yaml
workflow_id: meeting_prebrief_daily
version: v2.0.0
status: active
owner: Ops / Store Manager
last_updated: 2026-01-04
delivery_channel: wecom
run_timezone: Asia/Shanghai
```

---

## 1. Purpose｜目的与边界

在夕会开始前自动生成**会前数据简报**（对齐 `【夕会前】日会数据简报模板.md`），让店长“开会前就知道今天该盯什么”。

本版本输出两层内容：

1. `report_md`：面向企微直接发送的 Markdown 简报（缺数据会明确提示）
2. 结构化 JSON：用于 UI 展示/自动化（`facts_recap` / `tomorrow_list` / `risks` / `checklist`）

明确不做：

- 不编造原因；只能陈述事实与“风险提示”
- 不输出医疗判断
- 不依赖历史聊天全文（只用汇总信号）

---

## 2. Sections｜简报结构（与模板对齐）

`report_md` 以如下顺序组织（数据缺失则输出“未提供/未同步”）：

1. 今日经营摘要：开单/消耗/到店/客单价 + 7D 对比 + 月度完成度（若提供）
2. 核心风险提示：基于阈值触发的可审计风险（`risks`）
3. 明日生意准备：明日预约人数 + Top10 预约清单（`tomorrow_list.appointments`）
4. 任务执行情况：从 `checklist` 输出“会后要落地的动作”

更多模板字段（如“各健康管理人完成情况/顾客摘要/关键品项完成/任务执行明细/接下来几天”）通过扩展 `facts_recap` 结构化字段承载，方便后续迭代渲染。

---

## 3. Input Contract｜输入契约（v2 增量）

在 v1 的基础上，新增/扩展了以下可选输入（缺失不影响主流程）：

- `store_name`：门店名称（用于 `report_md`）
- `data_cutoff_time`：数据截止时间（用于 `report_md`）
- `mtd.*`：月累计与目标/时间进度（用于“月度完成度”）
- `staff_stats[]`：健康管理人/咨询维度的当日与月累计（用于“各健康管理人完成情况”）
- `customer_summary.*`：新老客结构、VIP、单项目等（用于“顾客摘要”）
- `key_items_mtd[]`：关键品项 MTD 指标（用于“关键品项完成”）
- `task_execution.*`：任务执行率与缺失清单（用于“任务执行情况”）

HIS/预约/企微数据仍为主输入来源，且 `his` 为最小必需。

---

## 4. Policy｜规则（硬约束）

- `his` 缺失：流程应视为失败（Facts 无根）
- 风险必须可审计：每条风险输出 `threshold` + `evidence_fields` + `note`
- `checklist` 必须输出 ≥ 3 条（缺少时用 fallback 补齐）

