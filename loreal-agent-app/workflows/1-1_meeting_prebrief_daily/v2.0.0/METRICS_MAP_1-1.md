## 指标说明 - meeting_prebrief_daily v2.0.0（1-1）

本文档面向“指标口径说明”，按“业务口径 + 字段口径（表/字段/过滤/计算）”展开，便于对齐数据口径与实现细节。

### 通用口径
- 统计门店：`store_id`（对应数据库 `ClinicId / OrganizationId / OrginizationId`）
- 统计日期：`biz_date`（YYYY-MM-DD）
- 时间窗口：
  - 今日：`[biz_date 00:00, biz_date+1 00:00)`
  - 明日：`[biz_date+1 00:00, biz_date+2 00:00)`
  - 当月：`[当月1日 00:00, biz_date+1 00:00)`
  - 近12个月：`[biz_date+1 00:00 - 12个月, biz_date+1 00:00)`
- 退款：统一过滤 `IsRefund = 0`
- “消耗”当前等同“开单金额”（如无消耗表）

### 今日经营摘要（facts_recap.his）

| 指标 | 业务口径 | 字段口径（表/字段/过滤/计算） |
| --- | --- | --- |
| 今日开单金额（GMV） | 当日门店成交金额汇总 | `bills.PayAmount` 求和；过滤 `bills.ClinicId = store_id`、`bills.CreateTime` 在今日、`bills.IsRefund=0` |
| 今日消耗 | 口径暂同开单 | 同“今日开单金额” |
| 今日到店人数 | 当日产生有效开单的顾客数 | `bills.Customer_ID` 去重计数；过滤同上 |
| 今日成交人数 | 口径暂同到店人数 | 同“今日到店人数” |
| 今日预约人数 | 当日预约到店的顾客数 | `appointments.CustomerId` 去重计数；过滤 `appointments.OrginizationId = store_id`、`appointments.StartTime` 在今日、`appointments.IsDelete=0` |
| 今日客单价 | 当日人均开单金额 | `今日开单金额 / 今日到店人数`（分母为 0 返回 0） |
| 今日新客人数 | 当日在本门店首次开单的顾客数 | 对今日开单顾客 `bills.Customer_ID`，若其在该门店 `MIN(bills.CreateTime)` 的日期等于 `biz_date` 计为新客 |
| 今日老客人数 | 当日在本门店非首次开单的顾客数 | 同上，首单日期不等于 `biz_date` |
| 今日新客GMV | 当日新客开单金额汇总 | 今日新客顾客集合在今日 `bills.PayAmount` 求和 |
| 今日老客GMV | 当日老客开单金额汇总 | 今日老客顾客集合在今日 `bills.PayAmount` 求和 |
| 今日Top品项 | 当日开单金额Top品项 | `billoperationrecorditems.PaymentAmount` 按 `ItemName` 求和；联表 `billoperationrecorditems -> billoperationrecords -> bills`；过滤 `bills.ClinicId`、`billoperationrecords.OperationTime` 在今日、`bills.IsRefund=0`；取前 3 |

### 7日滚动基线（facts_recap.baselines.rolling_7d）

| 指标 | 业务口径 | 字段口径（表/字段/过滤/计算） |
| --- | --- | --- |
| 7日均GMV | 近7个自然日（不含当日）的日均开单 | 先按日期聚合 `SUM(bills.PayAmount)`，再对天取平均；过滤 `bills.ClinicId`、`bills.CreateTime` 在 `[biz_date-7, biz_date)`、`bills.IsRefund=0` |
| 7日均到店 | 近7个自然日（不含当日）的日均到店 | 先按日期聚合 `COUNT(DISTINCT bills.Customer_ID)`，再对天取平均；同上过滤 |
| 7日均消耗 | 口径暂同7日均GMV | 同“7日均GMV” |
| 7日均客单价 | 近7日均GMV / 近7日均到店 | `7日均GMV / 7日均到店` |

### 月度累计与完成度（facts_recap.mtd）

| 指标 | 业务口径 | 字段口径（表/字段/过滤/计算） |
| --- | --- | --- |
| 月度累计开单（GMV） | 当月至今门店开单金额汇总 | `bills.PayAmount` 求和；过滤 `bills.ClinicId`、`bills.CreateTime` 在当月、`bills.IsRefund=0` |
| 月度累计消耗 | 口径暂同月度开单 | 同“月度累计开单” |
| 月度时间进度 | 当月已过天数 / 当月总天数 | `day_of_month / days_in_month` |
| 月度开单目标 | 月度开单目标值 | 来自请求输入 `mtd.gmv_target`（无则为 0） |
| 月度消耗目标 | 月度消耗目标值 | 来自请求输入 `mtd.consumption_target`（无则为 0） |
| 月度开单完成度 | 月度开单完成度 | `mtd.gmv / mtd.gmv_target`（目标为 0 时返回 0） |
| 月度消耗完成度 | 月度消耗完成度 | `mtd.consumption / mtd.consumption_target`（目标为 0 时返回 0） |

### 各健康管理人完成情况（facts_recap.staff_stats）

| 指标 | 业务口径 | 字段口径（表/字段/过滤/计算） |
| --- | --- | --- |
| 管理人姓名 | 员工姓名（优先员工表） | `employees.EmpName`；通过 `billemployees.EmpId` 或 `bills.CreateEmpId` 关联；为空回退 `EmpId/CreateEmpId` |
| 今日开单 | 按人员的当日开单金额 | `bills.PayAmount` 按人员汇总；联表 `bills -> billemployees -> employees`；过滤 `bills.ClinicId`、`bills.CreateTime` 在今日、`bills.IsRefund=0`；若无 `billemployees` 数据，回退 `bills.CreateEmpId` |
| 本月累计开单 | 按人员的当月累计开单金额 | 同上，时间窗口为当月 |
| 今日消耗/本月消耗 | 口径暂置 0 | 当前固定为 0（待接入消耗表） |
| R12 回购率 | 近12个月同一顾客≥2个不同消费日期的占比 | 以员工为维度，统计近12个月 `bills.Customer_ID` 在不同日期下单次数；分子：`day_count >= 2` 的顾客数；分母：有消费的顾客数 |

### 顾客摘要（facts_recap.customer_summary）

| 指标 | 业务口径 | 字段口径（表/字段/过滤/计算） |
| --- | --- | --- |
| 新客人数/GMV | 今日首单顾客数及其GMV | 今日开单顾客中，若其在该门店首单日期等于 `biz_date` 记为新客；GMV 为该客今日 `bills.PayAmount` |
| 老客人数/GMV | 今日非首单顾客数及其GMV | 今日开单顾客中，首单日期不等于 `biz_date` 记为老客 |
| 新客来源分层 | 今日新客按来源分组统计 | `customers.LaiYuanID -> customdictionary.DisplayName`；仅统计首单日期为 `biz_date` 的顾客 |
| 单项目顾客数 | 今日到店顾客中近12个月仅消费过1个不同品项的顾客数 | `billoperationrecorditems.ItemName` 近12个月（按 `billoperationrecords.OperationTime`）去重计数为 1；并限定顾客出现在今日开单顾客集合 |
| VIP顾客数 | 今日到店顾客中最新会员等级为 VIP 的顾客数 | `customer_level_historys.new_level LIKE '%VIP%'`；取每客最新 `create_time`；限定今日开单顾客集合 |

### 关键品项完成（本月至今，facts_recap.key_items_mtd）

| 指标 | 业务口径 | 字段口径（表/字段/过滤/计算） |
| --- | --- | --- |
| 品项MTD开单金额 | 当月至今品项开单金额汇总 | `billoperationrecorditems.PaymentAmount` 按 `ItemName` 求和；联表 `billoperationrecorditems -> billoperationrecords -> bills`；过滤 `bills.ClinicId`、`billoperationrecords.OperationTime` 在当月、`bills.IsRefund=0` |
| 品项MTD消耗金额 | 口径暂同开单金额 | 同上 |
| WOW/同期 | 口径未实现 | 当前固定为 0（待定义同比/环比窗口） |

### 任务执行情况（facts_recap.task_execution）

| 指标 | 业务口径 | 字段口径（表/字段/过滤/计算） |
| --- | --- | --- |
| 回访完成率 | 当日回访完成 / 当日计划回访 | `returnvisits`：分子 `DoneReturnVisitDate` 在今日的数量；分母 `ReturnVisitDate` 在今日的数量 |
| 对比照发送完成率 | 当日发送对比照人数 / 当日到店人数 | `operation_photo`：`COUNT(DISTINCT CUSTOMER_ID)` 今日发送 / `今日到店人数` |
| AI面诊记录生成率 | 当日生成病历人数 / 当日到店人数 | `emrs`：`COUNT(DISTINCT CustomerId)` 今日生成 / `今日到店人数` |
| 病历完成比例 | 口径暂同AI面诊记录生成率 | 同“AI面诊记录生成率” |
| 处方开具比例 | 当日开具处方人数 / 当日到店人数 | `prescriptions`：`COUNT(DISTINCT Customer_ID)` 今日开具 / `今日到店人数` |
| 术后提醒完成率 | 口径未实现 | 当前固定为 0（待接入数据表） |
| 未发送对比照名单 | 当日到店但未发送对比照的顾客清单（样例） | `bills` 今日顾客左连接 `operation_photo` 今日记录，`p.ID IS NULL` 取前 10 |

### 明日预约清单（facts_recap.appointments_tomorrow）

| 指标 | 业务口径 | 字段口径（表/字段/过滤/计算） |
| --- | --- | --- |
| 明日预约清单 | 明日到店预约列表（最多20条） | `appointments` + `appointmentlines`；字段：`appointments.ID`、`CustomerId`、`CustomerName`、`StartTime`、`DoctorName`、`ConsultantName`；品项为 `appointmentlines.ItemName` 去重拼接 |
| 明日预约人数 | 明日预约顾客数（去重） | `appointments.CustomerId` 去重计数；过滤 `appointments.OrginizationId`、`appointments.StartTime` 在明日、`appointments.IsDelete=0` |

### 说明与已知简化
- “消耗”相关字段目前均等于“开单金额”或固定为 0，需接入消耗明细表后替换。
- 多数“到店人数”来自 `bills`，未覆盖“到店未开单”人群。
- 员工维度以 `billemployees` 为主，缺失时回退 `bills.CreateEmpId`。
- 指标计算为 best-effort，缺失字段返回 0/空数组，不报错。
