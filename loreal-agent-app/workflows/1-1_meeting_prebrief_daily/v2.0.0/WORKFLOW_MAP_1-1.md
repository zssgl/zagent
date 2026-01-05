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
- 今日开单：`bills.PayAmount` 当日汇总（`ClinicId + CreateTime`，`IsRefund=0`）（已实现）
- 今日消耗：当前等同今日开单（已实现，口径简化；如有独立消耗表需替换）
- 今日预约人数：`appointments` 当日 `StartTime` 计数（`OrginizationId` 匹配门店）（已实现）
- 到店人数：`bills` 当日 `Customer_ID` 去重计数（已实现；未覆盖到诊未开单人群）
- 成交人数：当前等同到店人数（已实现，近似口径；未剔除 0 元单/正负单合并）
- 月度累计开单：当月 `bills.PayAmount` 汇总（已实现）
- 月度累计消耗：当前等同月度开单（已实现，口径简化）
- 月度时间进度：`day_of_month / days_in_month`（已实现）
- 月度开单指标完成度：`mtd.gmv / mtd.gmv_target`（缺 `mtd.gmv_target` 时不展示）
- 月度消耗指标完成度：`mtd.consumption / mtd.consumption_target`（缺目标时不展示）

#### 智能总结
- 总结要点：LLM 生成（已实现，LLM 不可用时为空）

#### 各健康管理人完成情况
- 管理人姓名：`staff_stats.staff_name`（已实现；优先 `billemployees`，为空 fallback `bills.CreateEmpId`）
- 今日开单/消耗：`staff_stats.today_gmv/today_consumption`（部分实现：消耗目前置 0）
- 本月累计开单/消耗：`staff_stats.mtd_gmv/mtd_consumption`（部分实现：消耗目前置 0）
- 达成率/目标：缺失（需员工目标数据）
- R12 回购率：缺失（当前占位 0）
- 智能总结：LLM 生成（已实现）

#### 顾客摘要
- 新客人数/GMV：`customer_summary.new.count/gmv`（已实现；按当日首单判断新客）
- 新客来源分层：`customer_summary.new.sources`（部分实现：仅输出 `customers.LaiYuanID -> customdictionary.DisplayName`，未做业务渠道映射）
- 老带新核验/美丽基金：缺失（需推荐人表/基金表与排除规则）
- 老客人数/GMV：`customer_summary.old.count/gmv`（已实现）
- 单项目顾客：`customer_summary.single_item_customers`（部分实现：近 12 个月品项去重=1，未排除促销/同义品项）
- VIP/VVIP：`customer_summary.vip_customers`（部分实现：`customer_level_historys.new_level LIKE '%VIP%'`，无法区分 VVIP）
- 智能总结：LLM 生成（已实现）

#### 关键品项完成（本月至今）
- 品项开单/消耗金额：`key_items_mtd.gmv_mtd/consumption_mtd`（已实现；来自 `billoperationrecorditems` 聚合）
- 品项人数：缺失（当前未统计人数）
- WOW/同期对比：缺失（需定义同比/环比窗口）
- 单次比例/复购提示：缺失（需顾客项目周期与明细）
- 扫码购明细：缺失（需外部接口）
- 智能总结：LLM 生成（已实现）

#### 任务执行情况
- 回访完成率：`task_execution.followup_done_rate`（部分实现：`returnvisits` 完成/计划）
- 对比照发送完成率：`task_execution.photo_sent_rate`（部分实现：`operation_photo` / 当日到店人数）
- 术后提醒完成率：缺失（无对应表/字段）
- AI 面诊记录生成率：`task_execution.ai_record_rate`（部分实现：`emrs` / 当日到店人数）
- 病历完成比例：`task_execution.emr_done_rate`（部分实现：同上）
- 处方开具比例：`task_execution.prescription_rate`（部分实现：`prescriptions` / 当日到店人数）
- 名单级明细：大多缺失（仅 `missing_photo_list` 示例）
- 群内交接比例/对话质量：缺失（需企微/任务回执）

#### 明日生意准备
- 明日预约人数与清单：`appointments_tomorrow`（已实现；`appointments` + `appointmentlines`）
- 明日业绩目标：缺失（需运营目标输入）
- 预约分群：缺失（需顾客标签/复购周期）
- 当班医生/护士与人手风险：缺失（需排班表）

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

### 需要补齐的关键缺口（基于当前实现）

| 缺口 | 对应章节/步骤 | 影响 |
| --- | --- | --- |
| 月度目标来源（开单/消耗目标） | 今日经营摘要 / S4 执行 | 完成度无法稳定展示 |
| 员工业绩目标与达成率 | 各健康管理人完成情况 / S4 执行 | 达成率与目标差距缺失 |
| R12 回购率口径与数据源 | 各健康管理人完成情况 / S4 执行 | R12 仅占位 0 |
| 新客渠道映射规则（老带新/平台等） | 顾客摘要 / S4 执行 | 渠道分层不准确 |
| 老带新/美丽基金核验表与规则 | 顾客摘要 / S4 执行 | 预警与核验缺失 |
| 单项目/复购口径标准化 | 顾客摘要 / S4 执行 | 单项目占比不可用 |
| VVIP 定义与标签来源 | 顾客摘要 / S4 执行 | 仅能输出 VIP 粗粒度 |
| 关键品项人数与同期/WOW | 关键品项完成 / S4 执行 | 趋势判断缺失 |
| 扫码购数据接口 | 关键品项完成 / S4 执行 | 扫码购模块缺失 |
| 任务名单级明细（回访/对比照/术后/病历/处方） | 任务执行情况 / S4 执行 | 无法输出名单 |
| 企微任务回执与对话质量指标 | 任务执行情况 / S4 执行 | “有效对话比例”等缺失 |
| 明日业绩目标与预约分群 | 明日生意准备 / S4 执行 | 无法输出分群与目标 |
| 排班数据（医生/护士） | 明日生意准备 / S4 执行 | 人手风险无法判断 |
| 未来 7 天/专家日计划数据 | 接下来几天 / S4 执行 | 全段缺失 |
| 缺失信息澄清输出 | S3A 缺失信息处理 | 无法自动生成追问 |
