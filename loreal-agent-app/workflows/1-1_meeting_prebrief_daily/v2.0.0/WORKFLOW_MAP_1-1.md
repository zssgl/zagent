## Workflow 地图 - meeting_prebrief_daily v2.0.0（1-1）

本文档按步骤映射本 workflow 所需数据、工具与完成度。

### 说明
- 状态：`已实现`（available）、`部分实现`（partial）、`缺失`（missing）
- 数据来源：`输入`（调用方提供）、`mysql`（装配）、`规则`（派生）、`llm`（可选）

### 步骤地图

| 步骤 | 描述 | 输入（关键字段） | 数据来源 | 工具 | 状态 | 备注 |
| --- | --- | --- | --- | --- | --- | --- |
| S0 请求接收 | 接收原始 input + context | `input`, `context` | 输入 | 无 | 已实现 | 入口。 |
| S1 规范化 | MySQL 装配 + input 覆盖合并，剥离 context | `context.assemble.source`, raw input | mysql + 输入 | MySQL | 已实现 | 开启装配时使用。 |
| S2 完整性检查 | 校验必填字段 | `store_id`, `biz_date`, `his.*` | 输入或 mysql | 无 | 已实现 | 未开启装配时 `his` 必填。 |
| S3A 缺失信息处理 | 生成澄清请求（可选） | missing fields list | 规则 | 无 | 缺失 | 暂无显式输出。 |
| S3B 执行规划 | 选择执行计划 | `context.plan_candidates` | 输入 + llm | LLM（可选） | 已实现 | 无 LLM 时回退确定性方案。 |
| S4 执行 | 计算 facts/risks/checklist | `his`, `baselines`, `mtd`, `appointments_tomorrow`, `wecom_touch`, `staff_stats`, `customer_summary`, `key_items_mtd`, `task_execution` | mysql + 输入 + 规则 | MySQL | 已实现 | 多字段为 best-effort。 |
| S5 结果校验 | 输出 schema 校验 | output JSON | 规则 | JSON Schema | 已实现 | 不通过直接失败。 |
| S6 交付与落盘 | 渲染报告 + 持久化 | `report_md`, `biz_date` | 规则 | 文件系统 | 已实现 | 写入 `reports/`。 |

### 数据覆盖（模板 vs 当前）

#### 今日经营摘要
- `his.gmv`, `his.consumption`, `his.visits`, `his.avg_ticket` | mysql | 已实现
- `his.appointments`, `his.deals` | mysql | 部分实现（best-effort）
- 7D 对比（`baselines.rolling_7d.*`）| mysql | 已实现
- 月度累计（`mtd.gmv`, `mtd.consumption`, `mtd.time_progress`）| mysql | 已实现
- 月度目标（`mtd.*_target`）| 输入 | 部分实现（需调用方提供）

#### 智能总结
- 规则总结 | 规则 | 已实现
- LLM 总结 | llm | 已实现（可选）

#### 健康管理人
- `staff_stats`（今日/MTD gmv）| mysql | 已实现（best-effort）
- R12 回购率 | mysql | 缺失（占位）
- 目标达成 | 输入 | 缺失

#### 顾客摘要
- 新/老客人数 + GMV | mysql | 已实现
- 新客来源 | mysql | 已实现（best-effort）
- 单项目顾客 | mysql | 部分实现（best-effort）
- VIP 顾客 | mysql | 部分实现（best-effort）
- 老带新/美丽基金核验 | mysql | 缺失

#### 关键品项（MTD）
- 关键品项 + MTD GMV | mysql | 已实现
- WOW/同期对比 | 规则 | 缺失
- 扫码购 | 外部 API | 缺失

#### 任务执行
- 基础比例（照片/病历/回访/处方）| mysql | 部分实现
- 名单级明细 | mysql | 部分实现（多数缺失）
- 企微触达/对话比例 | wecom API | 缺失

#### 明日准备
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
- 月度目标来源（表或 API）
- 渠道映射 + 美丽基金校验
- 关键品项 WOW/同期对比逻辑
- 企微任务执行与对话质量指标
- 员工业绩目标与达成率
