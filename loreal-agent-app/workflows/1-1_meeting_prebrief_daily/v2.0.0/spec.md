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

## 2.1 Workflow States｜工作流状态机（无 AI 可跑）

| State | Description | Input | Output | Agent Involved |
| --- | --- | --- | --- | --- |
| S0 | Request Received | Raw Input | RawRequest | No |
| S1 | Normalization | RawRequest | NormalizedRequest | No |
| S2 | Completeness Check | NormalizedRequest | CompleteRequest / MissingInfoList | No |
| S3A | Missing Info Handling | MissingInfoList | ClarificationRequest | Optional (language only) |
| S3B | Execution Planning | CompleteRequest | ExecutionPlan | Optional (plan selection only) |
| S4 | Execution | ExecutionPlan | RawResult | No |
| S5 | Result Validation | RawResult | FinalResult / Error | No (LLM output must pass validation) |
| S6 | Delivery & Logging | FinalResult | Delivered + Audit Trail | No |

说明：
- LLM 仅可参与 S3A/S3B 的“措辞或方案选择”，不得越权执行或改写事实数据。
- 关键硬约束（schema 校验/风控/成本/执行权限）全部在非 LLM 步骤完成。

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

- `his` 缺失：当未开启 `context.assemble.source=mysql` 时视为失败（Facts 无根）
- 风险必须可审计：每条风险输出 `threshold` + `evidence_fields` + `note`
- `checklist` 必须输出 ≥ 3 条（缺少时用 fallback 补齐）

---

## 5. Agent Policy Summary｜Agent 使用边界（甲方版）

Agent **不具备**：
- 自主执行权限
- 绕过校验能力
- 编造数字/口径的能力

Agent **仅用于**：
- 澄清问题的自然语言表达（S3A）
- 多方案中选择“性价比更高”的执行计划建议（S3B）

所有 Agent 输出必须通过：
- Schema validation
- Risk policy check（与 `rules.yml` 一致）
