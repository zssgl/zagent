# 1-1 meeting_prebrief_daily（会前）对接说明（简版）

## 目的

每日夕会前自动生成门店会前简报：

- `report_md`（企微可直接发送的 Markdown）
- 结构化输出（`facts_recap` / `tomorrow_list` / `risks` / `checklist`）
- 自动落盘 `reports/briefing_YYYYMMDD.md`（可用 `REPORTS_DIR` 覆盖目录）

## Workflow 步骤

1. **接收请求**：输入最小字段（门店 + 日期 + 可选目标）
2. **数据装配（默认）**：默认从 MySQL 取数补齐缺失字段（见下文）
3. **指标计算**：组装 `facts_recap`（今日、7D 基线、MTD、结构、Top 品项）
4. **风险评估**：按 `rules.yml` 规则生成 `risks`（可审计：阈值 + 证据字段）
5. **清单生成**：根据风险与明日预约生成 `checklist`（至少 3 条）
6. **渲染简报**：生成 `report_md`（对齐 `../【夕会前】日会数据简报模板.md` 的段落结构）
7. **持久化**：保存到 `reports/briefing_YYYYMMDD.md`

## Workflow 状态机（甲方版）

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

Agent policy 简述：
- Agent 不具备自主执行、绕过校验、编造数字权限
- 仅用于澄清语言表达或方案选择建议
- 产出必须经 schema 校验与风险规则检查

## 需要的数据（输入契约）

### 最小输入（推荐）

- `store_id`：门店 ID（建议用 `clinics.ID`；也与 `appointments.OrginizationId` 对齐）
- `biz_date`：业务日期（YYYY-MM-DD）
- `data_cutoff_time`：数据截止时间（展示用）
- `store_name`：门店名称（可选，不传则从 `clinics` 补）
- `mtd.gmv_target` / `mtd.consumption_target`：月度目标（可选；用于目标差距风险与“月度完成度”展示）

### 自动装配会补齐的字段（Best-effort）

默认从 MySQL 组装：

- `his`：`gmv` / `visits` / `avg_ticket` / `top_items`，以及 `appointments`（今日预约数）/ `deals`（成交人数 best-effort）
- `baselines.rolling_7d`：7D 日均（按天聚合后取 AVG）
- `mtd`：`gmv` / `consumption` / `time_progress`（目标仍建议由调用方传入）
- `appointments_tomorrow`：明日预约（含 `appointmentlines.ItemName`）
- `staff_stats`：今日/MTD 员工业绩（`billemployees`，fallback `bills.CreateEmpId`）
- `customer_summary`：新/老客（按门店首单日期），新客来源（`customers.LaiYuanID -> customdictionary.DisplayName`），单项目/ VIP（best-effort）
- `key_items_mtd`：MTD top items（`billoperationrecorditems`）
- `task_execution`：回访/对比照/病历/处方等完成率（best-effort）

## 仍缺的数据 / 需要甲方补充或确认

以下项在当前数据库装配中**无法稳定补齐**，需要甲方提供数据源/字段映射/口径确认（否则只能留空或做 best-effort 近似）：

### 目标与进度

- 月度目标（模板提到外部 Excel/运营下发）：建议调用方在请求中传入 `mtd.gmv_target` / `mtd.consumption_target`（或提供目标表/接口）
- 月度消耗目标/开单目标口径确认（是否与 `bills.PayAmount` 一致）

### 经营口径（需要确认）

- “消耗”严格口径（是否来自 `consumption_details/consumption_project_details`，还是与开单一致）
- “到店人数”口径（HIS/预约/实际到诊）与“成交人数”口径（去除 0 元/合并正负单等）
- 异店结算/手工调整逻辑（模板中提到异店调整、群内确认机制）：当前未接入相关表/流程

### 渠道与推荐（模板中的老带新/平台）

- 新客来源的业务映射：当前只输出 `customers.LaiYuanID -> customdictionary.DisplayName`，若需固定分类（老带新/小红书/大众点评等）需要甲方给 mapping 表或规则
- 老带新推荐人字段/表（模板要求校验推荐人信息、基金金额等）：当前未接入推荐人/基金相关表（如有）
- “美丽基金/折扣码排除集”等业务规则与数据源：当前未接入

### 顾客结构与标签

- VIP/VVIP 精确定义（当前仅用 `customer_level_historys.new_level LIKE '%VIP%'` best-effort；无法区分 VVIP）
- “12个月仅消耗一个项目/单项目顾客”严格口径（当前以 `billoperationrecorditems.ItemName` 去重 best-effort；未排除促销/疗程/同义品项）
- RFMC / Category-RFM 等标签（用于 CRM/运营诊断）：当前未接入

### 关键品项对比（WOW / 同期）

- WOW 定义（上月同期/近三月同期）与窗口：当前只输出 MTD top items，未实现同期对比

### 任务执行明细

- 企微任务流“下发/回执”数据源（模板里的“有效对话比例/三来回对话”等）：当前仅用 `customer_trace` / `returnvisits` best-effort，未接企微 API 回执表
- 术后提醒/交接比例/AI 面诊摘要等：需明确对应表与完成判定（当前只覆盖照片/病历/处方/回访等部分表）

### 人员与归属

- “健康管理人/咨询/医生”归属的唯一口径：当前使用 `billemployees` 或 `bills.CreateEmpId` best-effort；若需按组织架构/排班/顾客归属统计，需要甲方提供归属规则与字段

## 工具（Tools）

### DB（MySQL）

用于数据装配。当前主要表（best-effort）：

- `clinics`（门店名）
- `bills`（GMV/到店、MTD）
- `billoperationrecords` / `billoperationrecorditems`（Top 品项、关键品项）
- `appointments` / `appointmentlines`（明日预约）
- `billemployees` / `employees`（员工维度）
- `customers` / `customdictionary`（新客来源）
- `customer_level_historys`（VIP best-effort）
- `operation_photo` / `emrs` / `prescriptions` / `returnvisits`（任务执行 best-effort）

### LLM（可选）

当前实现未强依赖 LLM。建议的 LLM 使用点（后续接入）：

- 生成“智能总结/建议措辞”（严格约束：只能引用已计算字段，禁止编造数字）
- 将 `facts_recap + risks + checklist` 做更自然的叙述优化（不改变数值）

## 开关与约定

- 默认开启 MySQL 装配（无需传 `context.assemble.source`）
- 输出落盘目录：默认 `reports/`，可用环境变量 `REPORTS_DIR` 覆盖
