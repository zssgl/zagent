## Workflow 地图 - meeting_prebrief_daily v2.0.0（1-1）

本文档按“输入 → 步骤 → 输出 → 缺口”的结构，明确每个步骤所需数据与完成度。

### 说明
- 状态：`已实现`（available）、`部分实现`（partial）、`缺失`（missing）
- 数据来源：`输入`（调用方提供）、`mysql`（装配）、`规则`（派生）、`llm`（可选）
- `部分实现`/“best-effort”的含义：会尽力从现有表聚合，但**口径不完整、字段可能缺失、缺失时会返回 0/空数组，不会报错**。

### 输入

**最小输入（默认 mysql 装配）**
- `store_id`
- `biz_date`

**可选输入**
- `store_name`, `data_cutoff_time`
- `mtd.*_target`（月度目标）
- 其它结构化字段：`staff_stats`, `customer_summary`, `key_items_mtd`, `task_execution`
- 外部目标与计划输入（XLS）：`daily_targets`, `staff_targets`, `schedule_plan`（来源 `reference/*.xlsx`，后续可替换为接口/系统接入）

### 步骤地图

| 步骤 | 描述 | 输入（关键字段） | 数据来源 | 输出 | 状态 | 备注 |
| --- | --- | --- | --- | --- | --- | --- |
| S0 请求接收 | 接收原始 input + context | `input`, `context` | 输入 | `raw_request` | 已实现 | 入口。 |
| S1 规范化 | MySQL 装配 + input 覆盖合并，剥离 context | raw input | mysql + 输入 | `normalized_request` | 已实现 | 默认装配。 |
| S2 完整性检查 | 校验必填字段 | `store_id`, `biz_date`, `his.*` | 输入或 mysql | `complete_request` / `missing_info_list` | 已实现 | `his` 由装配补齐，调用方可不传。 |
| S3A 缺失信息处理 | 生成澄清请求（可选） | `missing_info_list` | 规则 | `clarification_request` | 缺失 | 暂无显式输出。 |
| S4 执行 | 计算 facts/risks/checklist | `his`, `baselines`, `mtd`, `appointments_tomorrow`, `wecom_touch`, `staff_stats`, `customer_summary`, `key_items_mtd`, `task_execution`, `daily_targets`, `staff_targets`, `schedule_plan` | mysql + 输入 + 规则 | `raw_result` | 已实现 | 多字段为 best-effort。 |
| S4.5 智能总结 | 基于 facts/risks/checklist 生成总结要点 | `facts_recap`, `risks`, `checklist` | 规则 + llm | `agent_summary` | 已实现 | LLM 失败时回退规则总结。 |
| S5 结果校验 | 输出 schema 校验 | output JSON | 规则 | `final_result` / `error` | 已实现 | 不通过直接失败。 |
| S6 交付与落盘 | 渲染报告 + 持久化 | `report_md`, `biz_date` | 规则 | `delivered` | 已实现 | 写入 `reports/`。 |

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
- 今日预约人数：`appointments` 当日 `StartTime` **去重 CustomerId**（`OrginizationId` 匹配门店）（已实现）
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
- 达成率/目标：缺失（可补齐：门诊业绩跟进表/业绩分解表等目标表，见 `reference/*.xlsx`，需导入并建立门店/员工映射）
- R12 回购率：已实现（近 12 个月内同一顾客有≥2个不同消费日期；按 staff 去重顾客计算）
- 智能总结：LLM 生成（已实现）

#### 顾客摘要
- 新客人数/GMV：`customer_summary.new.count/gmv`（已实现；按当日首单判断新客）
- 新客来源分层：`customer_summary.new.sources`（部分实现：仅输出 `customers.LaiYuanID -> customdictionary.DisplayName`；可补齐：`reference/sql代码/14、创建病历编号和健康人关系直接写入表.sql` 已落 `bi.wechat_cu_service.客户来源`）
- 老带新核验/美丽基金：缺失（需推荐人表/基金表与排除规则）
- 老客人数/GMV：`customer_summary.old.count/gmv`（已实现）
- 单项目顾客：`customer_summary.single_item_customers`（部分实现：近 12 个月品项去重=1，未排除促销/同义品项）
- VIP/VVIP：`customer_summary.vip_customers`（部分实现：`customer_level_historys.new_level LIKE '%VIP%'`；可补齐：`bi.wechat_cu_service.会员等级`）
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

**部分实现说明**
- 当前仅输出“比例类指标”，无名单级明细
- 分母多用“当日到店人数”近似，未按严格业务口径校准
- 企微对话与任务回执未接入，相关指标为空

#### 明日生意准备
- 明日预约人数与清单：`appointments_tomorrow`（已实现；`appointments` + `appointmentlines`）
- 明日业绩目标：缺失（可补齐：业绩跟进表/业绩分解表含每日预约/业绩目标，见 `reference/*业绩*跟进表*.xlsx`）
- 预约分群：缺失（需顾客标签/复购周期）
- 当班医生/护士与人手风险：缺失（可补齐：部分门诊业绩跟进表含“排班/当班”，见 `reference/*业绩*跟进表*.xlsx`）

#### 接下来几天
- 专家日目标/预约：缺失（可补齐：业绩跟进表若包含“专家日/活动日”字段）
- 未来 7 天目标与预约量：缺失（可补齐：业绩跟进表的每日目标/预约目标）
- 客单差距测算：缺失
- 单次客回店邀约：缺失
- VIP 维护到店：缺失

### 需要补齐的关键缺口（含已提供参考）

| 需甲方提供/现有参考 | 对应章节/步骤 | 影响 |
| --- | --- | --- |
| 月度目标（开单/消耗目标）已提供：`reference/2025年8月指标-0723.xlsx`（作为外部可选输入） | 今日经营摘要 / S4 执行 | 完成度可展示，但需导入与门店映射 |
| 员工业绩目标与达成率口径：部分提供（门店级/每日目标见 `reference/*业绩*跟进表*.xlsx`，作为外部可选输入），员工维度仍缺 | 各健康管理人完成情况 / S4 执行 | 达成率与目标差距缺失 |
| R12 回购率口径与数据源已提供：`reference/sql代码/17、lifetime_detail 查询.sql`（workflow 已内置近12月复购口径，仍可对齐/替换为标准SQL） | 各健康管理人完成情况 / S4 执行 | 已可展示，口径需确认一致性 |
| 新客渠道映射规则（老带新/平台等）：部分提供（`reference/sql代码/14、创建病历编号和健康人关系直接写入表.sql` 含客户来源原值） | 顾客摘要 / S4 执行 | 渠道分层仍需映射规则 |
| 老带新/美丽基金核验规则与数据表 | 顾客摘要 / S4 执行 | 预警与核验缺失 |
| 单项目/复购口径标准 | 顾客摘要 / S4 执行 | 单项目占比不可用 |
| VVIP 定义与标签规则：部分提供（会员等级见 `reference/sql代码/14、创建病历编号和健康人关系直接写入表.sql`，可纳入 workflow 数据装配） | 顾客摘要 / S4 执行 | 仅能输出 VIP 粗粒度 |
| 关键品项 WOW/同期对比规则 | 关键品项完成 / S4 执行 | 趋势判断缺失 |
| 扫码购数据接口与字段说明 | 关键品项完成 / S4 执行 | 扫码购模块缺失 |
| 任务名单级明细口径与字段 | 任务执行情况 / S4 执行 | 无法输出名单 |
| 企微任务回执与对话质量口径 | 任务执行情况 / S4 执行 | “有效对话比例”等缺失 |
| 明日业绩目标与预约分群规则：部分提供（每日目标见 `reference/*业绩*跟进表*.xlsx`，作为外部可选输入），分群规则缺 | 明日生意准备 / S4 执行 | 无法输出分群与目标 |
| 排班数据（医生/护士）与风险规则：部分提供（部分跟进表含“排班/当班”栏位，作为外部可选输入） | 明日生意准备 / S4 执行 | 人手风险判断规则缺失 |
| 未来 7 天/专家日计划数据：部分提供（部分跟进表含每日/活动日目标，作为外部可选输入） | 接下来几天 / S4 执行 | 需定义展示口径与字段 |
| 智能总结输出规则（风控/事实约束/引用字段范围） | 各章节智能总结 / S4.5 | LLM 输出不可审计 |
