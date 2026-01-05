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

**人类可读输出**
- `report_md`（Markdown 报告）
- 落盘文件：`reports/briefing_YYYYMMDD.md`

### 输出（按模板章节）

#### 今日经营摘要（模板指定字段）
- 今日开单：金额 + 日同比/对比百分比
- 今日消耗：金额 + 日同比/对比百分比
- 今日预约人数 / 到店人数 / 成交人数
- 月度累计开单
- 月度累计消耗
- 月度时间进度
- 月度开单指标完成度（含目标值）
- 月度消耗指标完成度（含目标值）

**指标口径（当前实现）**
- 今日开单：`bills.PayAmount` 当日汇总（`ClinicId + CreateTime`，`IsRefund=0`）
- 今日消耗：当前等同今日开单（`consumption = gmv`）
- 今日预约人数：`appointments` 当日 `StartTime` 计数（`OrginizationId` 匹配门店）
- 到店人数：`bills` 当日 `Customer_ID` 去重计数
- 成交人数：当前等同到店人数（近似口径）
- 月度累计开单：当月 `bills.PayAmount` 汇总
- 月度累计消耗：当前等同月度开单
- 月度时间进度：当月天数进度 `day_of_month / days_in_month`
- 月度开单指标完成度：`mtd.gmv / mtd.gmv_target`
- 月度消耗指标完成度：`mtd.consumption / mtd.consumption_target`

#### 智能总结
- 规则总结 | 规则 | 已实现
- LLM 总结 | llm | 已实现（可选）

#### 各健康管理人完成情况
- `staff_stats`（今日/MTD gmv）| mysql | 已实现（best-effort：优先 `billemployees`，为空则 fallback `bills.CreateEmpId`，无员工归属规则）
- R12 回购率 | mysql | 缺失（占位）
- 目标达成 | 输入 | 缺失

#### 顾客摘要
- 新/老客人数 + GMV | mysql | 已实现
- 新客来源 | mysql | 已实现（best-effort：仅输出 `customers.LaiYuanID -> customdictionary.DisplayName`，未做业务渠道映射）
- 单项目顾客 | mysql | 部分实现（best-effort：按近 12 个月 `billoperationrecorditems.ItemName` 去重计数，未排除促销/同义项目）
- VIP 顾客 | mysql | 部分实现（best-effort：`customer_level_historys.new_level LIKE '%VIP%'`，无法区分 VVIP）
- 老带新/美丽基金核验 | mysql | 缺失

#### 关键品项完成（本月至今）
- 关键品项 + MTD GMV | mysql | 已实现（best-effort：按 `billoperationrecorditems` 汇总，不做品项口径清洗）
- WOW/同期对比 | 规则 | 缺失
- 扫码购 | 外部 API | 缺失

#### 任务执行情况
- 基础比例（照片/病历/回访/处方）| mysql | 部分实现（best-effort：按今日到店人数作分母，非严格业务口径）
- 名单级明细 | mysql | 部分实现（best-effort：仅 `missing_photo_list` 示例，其他名单未接入）
- 企微触达/对话比例 | wecom API | 缺失

#### 明日生意准备
- 明日预约清单 | mysql | 已实现
- 明日目标 & 人员排班 | 输入 | 缺失

#### 接下来几天
- 专家日 / 7 天目标 / VIP 回店 | 输入 + 规则 | 缺失

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
