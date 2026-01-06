## Workflow 地图 - meeting_postbrief_daily（1-2）

本文档按“输入 → 步骤 → 输出 → 缺口”的结构，明确会后简报的生成流程。

### 说明
- 状态：`已实现`（available）、`部分实现`（partial）、`缺失`（missing）
- 数据来源：`输入`（调用方提供）、`规则`（派生）、`llm`（可选）
- 1-2 仅做“会后补全/纠偏/行动项提炼”，不重新计算 1-1 的指标。

### 输入

**最小输入**
- `prebrief_report`：来自 1-1 的输出（建议同时提供 `report_md` + 结构化 JSON）
- `meeting_transcript`：会议文字记录（完整逐字稿或摘要均可）

**可选输入**
- `meeting_meta`：用于补齐报头/落盘信息（若 1-1 报告已包含，可省略）
  - `biz_date`：业务日期（用于标题与落盘文件名）
  - `store_name`：门店名（用于标题）
  - `data_cutoff_time`：数据截止时间（用于标题）
  - `meeting_time`：会议时间（可选展示）
  - `attendees`：参会人列表（可选展示）
  - `store_id`：仅当需要系统侧关联门店时提供（否则可省略）
- `user_overrides`：显式纠偏/补充（优先级高于 transcript）
- `summary_style`：输出风格控制（精简/详细、是否保留“==已补齐”等标记）

### 步骤地图

| 步骤 | 描述 | 输入（关键字段） | 数据来源 | 输出 | 状态 | 备注 |
| --- | --- | --- | --- | --- | --- | --- |
| S0 请求接收 | 接收原始 input + context | `input`, `context` | 输入 | `raw_request` | 已实现 | 入口。 |
| S1 规范化 | 解析 1-1 报告与 transcript，合并元信息 | `prebrief_report`, `meeting_transcript`, `meeting_meta` | 输入 | `normalized_request` | 部分实现 | 1-1 报告格式允许多种，meeting_meta 可选。 |
| S2 完整性检查 | 校验必填字段 | `prebrief_report`, `meeting_transcript` | 规则 | `complete_request` / `missing_info_list` | 部分实现 | 缺 transcript 时直接失败。 |
| S3 内容拆分 | 将 transcript 按话题/章节切片 | `meeting_transcript` | 规则 + llm | `section_chunks` | 缺失 | 建议与模板章节对齐。 |
| S4 关键信息抽取 | 抽取纠偏信息、行动项、会议总结 | `section_chunks`, `user_overrides` | 规则 + llm | `extractions` | 缺失 | LLM 仅限文本抽取，不生成数字。 |
| S4.5 章节合成 | 将抽取结果回填到模板 | `prebrief_report`, `extractions` | 规则 | `postbrief_report_md` | 缺失 | 允许保留“==已补齐”标记。 |
| S5 结果校验 | 输出 schema 校验 | output JSON | 规则 | `final_result` / `error` | 缺失 | 建议定义输出 schema。 |
| S6 交付与落盘 | 渲染报告 + 持久化 | `postbrief_report_md`, `biz_date` | 规则 | `delivered` | 缺失 | 写入 `reports/`。 |

### 输出

**结构化输出（JSON）**
- `meeting_summary`：全局会议总结（3-5 条）
- `section_summaries`：按章节的会议总结与补充说明
- `corrections`：对 1-1 报告的纠偏（字段路径 + 新值 + 证据）
- `action_items`：行动项列表（责任人/截止时间/任务）
- `open_questions`：待确认项

**人类可读输出**
- `postbrief_report_md`（Markdown 报告）
- 落盘文件：`reports/postbrief_YYYYMMDD.md`

### 输出（按模板章节）

#### 今日经营摘要
- 保留 1-1 报告的指标文本
- 会后补充：会议总结、纠偏信息、升单动作等

#### 各健康管理人完成情况
- 保留 1-1 的人员数据
- 会后补充：当日结论、缺口判断、行动计划

#### 顾客摘要
- 保留 1-1 的新/老客数据与渠道拆分
- 会后补充：单次客、渠道异常、重点顾客跟进

#### 关键品项完成
- 保留 1-1 的品项数据
- 会后补充：品项异常解释、经验分享、促销动作

#### 任务执行情况
- 保留 1-1 的任务指标与名单
- 会后补充：已补齐事项、未完成原因、跟进安排

#### 核心风险提示
- 来自 1-1 风险清单 + 会议确认/澄清
- 可标注“会议未提及/已确认/有行动计划”

#### 明日生意准备
- 保留 1-1 的明日预约与目标
- 会后补充：交接信息、重点顾客名单、注意事项

### 需要补齐的关键缺口（建议）

| 需补齐事项 | 对应章节/步骤 | 影响 |
| --- | --- | --- |
| 输出 schema 定义（含 corrections/action_items） | S5 结果校验 | 无法自动校验与落盘 |
| 章节对齐规则（模板标题映射） | S3 内容拆分 | 难以稳定抽取 |
| 纠偏准入规则（只接受明确口头更正） | S4 关键信息抽取 | 避免 LLM 误改数字 |
| 证据引用格式（引用 transcript 片段） | S4/S4.5 | 可审计性不足 |
| 会后报告命名与落盘路径 | S6 交付与落盘 | 报告管理不一致 |
