## Workflow 地图 - meeting_prebrief_daily v2.0.0（1-1）

本文档按“输入 → 步骤 → 输出 → 缺口”的结构，明确每个步骤所需数据与完成度。

### 说明
- 状态：`已实现`（available）、`部分实现`（partial）、`缺失`（missing）
- 数据来源：`输入`（调用方提供）、`mysql`（装配）、`规则`（派生）、`llm`（可选）
- `部分实现`/“best-effort”的含义：会尽力从现有表聚合，但**口径不完整、字段可能缺失、缺失时会返回 0/空数组，不会报错**。

### 输入

**最小输入（无装配）**
- `store_id`（门店 ID）
- `biz_date`（YYYY-MM-DD）
- `his.*`（营业事实字段：visits/gmv/consumption/avg_ticket/new_customers/old_customers）

**最小输入（开启 mysql 装配）**
- `store_id`
- `biz_date`

**可选输入**
- `store_name`, `data_cutoff_time`
- `mtd.*_target`（月度目标）
- 其它结构化字段：`staff_stats`, `customer_summary`, `key_items_mtd`, `task_execution`

**上下文（context）**
- 默认启用 MySQL 装配（不再需要传 `context.assemble.source`）

### 步骤地图

| 步骤 | 描述 | 输入（关键字段） | 数据来源 | 工具 | 输出 | 状态 | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| S0 请求接收 | 接收原始 input + context | `input`, `context` | 输入 | 无 | `raw_request` | 已实现 | 入口。 |
| S1 规范化 | MySQL 装配 + input 覆盖合并，剥离 context | raw input | mysql + 输入 | MySQL | `normalized_request` | 已实现 | 默认装配。 |
| S2 完整性检查 | 校验必填字段 | `store_id`, `biz_date`, `his.*` | 输入或 mysql | 无 | `complete_request` / `missing_info_list` | 已实现 | `his` 由装配补齐，调用方可不传。 |
| S3A 缺失信息处理 | 生成澄清请求（可选） | `missing_info_list` | 规则 | 无 | `clarification_request` | 缺失 | 暂无显式输出。 |
| S4 执行 | 计算 facts/risks/checklist | `his`, `baselines`, `mtd`, `appointments_tomorrow`, `wecom_touch`, `staff_stats`, `customer_summary`, `key_items_mtd`, `task_execution` | mysql + 输入 + 规则 | MySQL | `raw_result` | 已实现 | 多字段为 best-effort。 |
| S4.5 智能总结 | 基于 facts/risks/checklist 生成总结要点 | `facts_recap`, `risks`, `checklist` | 规则 + llm | LLM（可选） | `agent_summary` | 已实现 | LLM 失败时回退规则总结。 |
| S5 结果校验 | 输出 schema 校验 | output JSON | 规则 | JSON Schema | `final_result` / `error` | 已实现 | 不通过直接失败。 |
| S6 交付与落盘 | 渲染报告 + 持久化 | `report_md`, `biz_date` | 规则 | 文件系统 | `delivered` | 已实现 | 写入 `reports/`。 |

### 输出

**结构化输出（JSON）**
- `facts_recap`, `tomorrow_list`, `risks`, `checklist`, `data_quality`
- 可选：`agent_summary`
- 可选：`agent_risk_summary`, `agent_staff_summary`, `agent_customer_summary`, `agent_key_items_summary`

**人类可读输出**
- `report_md`（Markdown 报告）
- 落盘文件：`reports/briefing_YYYYMMDD.md`

### 输出（按模板章节）

#### 今日经营摘要
- 今日开单：`bills.PayAmount` 当日汇总（已实现）
- 今日消耗：当前等同今日开单（已实现，口径简化）
- 今日预约人数：`appointments` 当日计数（已实现）
- 到店人数：`bills` 当日去重客户（已实现）
- 成交人数：当前等同到店人数（已实现，近似口径）
- 月度累计开单：当月 `bills.PayAmount` 汇总（已实现）
- 月度累计消耗：当前等同月度开单（已实现，口径简化）
- 月度时间进度：`day_of_month / days_in_month`（已实现）
- 月度开单指标完成度：`mtd.gmv / mtd.gmv_target`（缺 `mtd.gmv_target` 时不展示）
- 月度消耗指标完成度：`mtd.consumption / mtd.consumption_target`（缺目标时不展示）

#### 智能总结
- 总结要点：LLM 生成（已实现，LLM 不可用时为空）

#### 各健康管理人完成情况
- 管理人姓名：`staff_stats.staff_name`（已实现）
- 今日开单/消耗：`staff_stats.today_gmv/today_consumption`（已实现，消耗为占位 0）
- 本月累计开单/消耗：`staff_stats.mtd_gmv/mtd_consumption`（已实现，消耗为占位 0）
- 达成率/目标：缺失（需目标数据）
- R12 回购率：缺失（占位 0）
- 智能总结：LLM 生成（已实现）

#### 顾客摘要
- 新客人数/GMV：`customer_summary.new.count/gmv`（已实现）
- 新客来源分层：`customer_summary.new.sources`（部分实现，仅原始来源名）
- 老带新核验/美丽基金：缺失（需业务规则/表）
- 老客人数/GMV：`customer_summary.old.count/gmv`（已实现）
- 单项目顾客：`customer_summary.single_item_customers`（部分实现，仅人数）
- VIP/VVIP：`customer_summary.vip_customers`（部分实现，仅 VIP 人数）
- 智能总结：LLM 生成（已实现）

#### 关键品项完成（本月至今）
- 品项开单/消耗金额：`key_items_mtd.gmv_mtd/consumption_mtd`（已实现）
- 品项人数：缺失
- WOW/同期对比：缺失
- 单次比例/复购提示：缺失
- 扫码购明细：缺失
- 智能总结：LLM 生成（已实现）

#### 任务执行情况
- 回访完成率：`task_execution.followup_done_rate`（部分实现）
- 对比照发送完成率：`task_execution.photo_sent_rate`（部分实现）
- 术后提醒完成率：缺失
- AI 面诊记录生成率：`task_execution.ai_record_rate`（部分实现）
- 病历完成比例：`task_execution.emr_done_rate`（部分实现）
- 处方开具比例：`task_execution.prescription_rate`（部分实现）
- 名单级明细（回访/对比照/术后/病历/处方）：大多缺失（仅 `missing_photo_list` 示例）
- 群内交接比例/对话质量：缺失

#### 明日生意准备
- 明日预约人数与清单：`appointments_tomorrow`（已实现）
- 明日业绩目标：缺失
- 预约分群：缺失
- 当班医生/护士与人手风险：缺失

#### 接下来几天
- 专家日目标/预约：缺失
- 未来 7 天目标与预约量：缺失
- 客单差距测算：缺失
- 单次客回店邀约：缺失
- VIP 维护到店：缺失

### 工具清单

| 工具 | 用途 | 状态 | 备注 |
| --- | --- | --- | --- |
| MySQL | 数据装配 | 已实现 | `context.assemble.source=mysql` |
| LLM | 总结 + 计划选择 | 已实现 | 可选；使用 env 配置 |
| 文件系统 | 报告落盘 | 已实现 | 可用 `REPORTS_DIR` 覆盖 |

### 需要补齐的关键缺口

| 缺口 | 对应步骤 | 影响 |
| --- | --- | --- |
| 月度目标来源（表或 API） | S4 执行 | 月度完成度/风险提示不完整 |
| 渠道映射 + 美丽基金校验 | S4 执行 | 顾客摘要无法输出“老带新/基金预警” |
| 关键品项 WOW/同期对比逻辑 | S4 执行 | 关键品项趋势缺失 |
| 企微任务执行与对话质量指标 | S4 执行 | 任务执行细节不足 |
| 员工业绩目标与达成率 | S4 执行 | 管理人维度达成率缺失 |
| 缺失信息澄清输出 | S3A 缺失信息处理 | 无法自动生成追问 |
