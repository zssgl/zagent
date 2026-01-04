# workflows/meeting_prebrief_daily/v1.0.0/spec.md

## 0. Meta｜元信息

```yaml
workflow_id: meeting_prebrief_daily
version: v1.0.0
status: active
owner: Ops / Store Manager
last_updated: 2025-12-30
delivery_channel: wecom
run_timezone: Asia/Shanghai
```

---

## 1. Purpose｜目的与边界

### 1.1 目标（必须明确）

在夕会开始前自动生成**会前数据简报**，让店长“开会前就知道今天该盯什么”。输出必须包含四块：

1. **Facts Recap**：今日经营事实回顾（只讲事实）
2. **Tomorrow List**：明日客户清单（预约/需跟进）
3. **Risk Alerts**：风险提示（异常、缺口、漏项）
4. **Execution Checklist**：执行 checklist（会后要落地的动作）

### 1.2 非目标（明确不做什么）

* ❌ 不编造原因；原因只能写成“待验证假设”
* ❌ 不输出话术全文（只输出“沟通方向/意图”）
* ❌ 不做策略创新/专项活动创意（那是周会或 B 类）
* ❌ 不做医疗判断

一句话：**这是“会前聚焦器”，不是“万能运营脑”。**

---

## 2. Trigger｜触发（什么时候跑）

```yaml
trigger:
  type: scheduled
  schedule: "daily 16:30"
  guard:
    his_data_ready: true
    appt_data_ready: true
    wecom_touch_ready: true
```

降级策略（guard 不满足时）

* `wecom_touch_ready=false`：不出触达相关风险，只出经营与预约
* `appt_data_ready=false`：不出明日清单，只出经营与触达
* `his_data_ready=false`：直接失败并报警（因为 Facts Recap 没根）

---

## 3. Input Contract｜输入契约

### 3.1 输入来源

* HIS：当日经营数据
* 预约系统：明日预约数据
* 企微：回访/触达数据（任务流 + 会话回执）

### 3.2 最小输入 Schema（摘要）

> 这是“能产出四块内容”的最小集合。

```json
{
  "store_id": "string",
  "biz_date": "2025-12-30",

  "his": {
    "visits": 0,
    "gmv": 0,
    "consumption": 0,
    "avg_ticket": 0,
    "new_customers": 0,
    "old_customers": 0,
    "top_items": [{"item": "string", "amount": 0}],
    "targets": {"gmv_target": 0, "consumption_target": 0}
  },

  "appointments_tomorrow": [
    {
      "customer_id": "string",
      "time": "2025-12-31 10:30",
      "item": "string",
      "staff_id": "string",
      "is_first_visit": true
    }
  ],

  "wecom_touch": {
    "tasks_sent": 0,
    "contacted": 0,
    "replied": 0,
    "no_reply_list": [
      {"customer_id": "string", "last_touch_at": "2025-12-29 15:10", "staff_id": "string"}
    ]
  },

  "baselines": {
    "rolling_7d": {"visits_avg": 0, "gmv_avg": 0, "consumption_avg": 0},
    "rolling_28d": {"visits_avg": 0, "gmv_avg": 0}
  }
}
```

### 3.3 不可信输入（必须标注）

* 任何手工备注自由文本：只能展示，不能触发风险或结论
* 企微回执延迟：允许空，且触达类指标需要标注“可能不完整”

---

## 4. Context Assembly｜上下文拼装规则

允许使用：

* 近 7 日 / 28 日滚动均值（用于对比异常）
* 目标（target）与当日达成（用于差距提示）

禁止使用：

* 历史聊天全文
* 其它门店数据（除非明确要求做对标）

---

## 5. Policy｜规则（硬规则 + 触发阈值）

### 5.1 输出结构硬约束（必须四块齐全，缺数据则解释）

* Facts Recap：必须输出（若缺 HIS 则直接失败）
* Tomorrow List：预约数据缺失则输出“数据未同步”
* Risk Alerts：无风险也要输出“无明显风险”
* Execution Checklist：必须输出 ≥ 3 条（否则视为失败）

### 5.2 Facts Recap（只允许事实）

必须包含：

* 到店人数、开单金额、消耗金额
* 同比/环比（若无同比则用 7D/28D 对比替代）
* 目标达成进度（如有 targets）
* 新老客结构
* Top 品项（至少 Top3）

禁止：

* ❌ “因为…所以…”的确定性因果句

### 5.3 Risk Alerts（必须可审计：阈值 + 证据字段）

风险类型建议固定枚举（便于统计）：

* `metric_drop` 指标下滑
* `target_gap` 目标差距
* `structure_issue` 结构异常（新客/老客占比）
* `touch_gap` 触达缺口（未回多）
* `tomorrow_load` 明日承接风险（预约集中/空档过大）

默认阈值（可后续参数化，但 v1 先固化）：

* 指标下滑：当日值 < 7D 均值 * 0.8
* 目标差距：达成率 < 70% 且当日剩余时间不足以自然追平
* 未回积压：`no_reply_list` ≥ 10 或 未回率 > 60%
* 明日集中：某时段（例如 10-12）预约占比 > 40%
* 明日空档：全天空档率 > 50%（如果能算出可预约时段）

输出每条风险必须带：

* `type`
* `evidence_fields`（来自输入的字段路径）
* `threshold`（触发阈值）
* `note`（一句解释，不许编原因）

### 5.4 Tomorrow List（明日客户清单）

来源必须可追溯：

* 预约清单：直接列出明日预约
* 需跟进清单：从 `no_reply_list` 派生（可加优先级）

规则：

* 优先级排序：新客首访 > 高客单项目 > 未回需二触达
* 每人最多 N 条（避免刷屏，默认 N=20；超出要汇总）

### 5.5 Execution Checklist（行动清单：必须可执行）

每条 checklist 必须具备：

* `owner_role`（店长/咨询/医生/运营）
* `action`（动词开头：确认/跟进/补齐/提醒/复盘…）
* `due`（默认：当日夕会结束后 2 小时内；或次日 12:00）
* `evidence_link`（对应风险或明日清单的引用 id）

禁止：

* ❌ “加强…、提升…、重视…”这种无落点句子

---

## 6. State Machine｜状态机

```text
INIT
 → INPUT_VALIDATED
 → FACTS_BUILT
 → RISKS_EVALUATED
 → TOMORROW_LIST_BUILT
 → CHECKLIST_BUILT
 → OUTPUT_VALIDATED
 → DELIVERED
 → COMPLETED

ERROR
DEGRADED (deliver_partial)
```

---

## 7. Output Contract｜输出契约（人读 + 机读）

### 7.1 机读输出 JSON

```json
{
  "run_id": "string",
  "biz_date": "2025-12-30",
  "store_id": "string",

  "facts_recap": {
    "visits": 0,
    "gmv": 0,
    "consumption": 0,
    "mom_or_baseline": {"gmv_vs_7d": -0.12},
    "target_progress": {"gmv_rate": 0.68},
    "structure": {"new": 0, "old": 0},
    "top_items": [{"item": "string", "amount": 0}]
  },

  "tomorrow_list": {
    "appointments": [
      {"customer_id": "string", "time": "string", "item": "string", "staff_id": "string", "priority": 1}
    ],
    "followups": [
      {"customer_id": "string", "last_touch_at": "string", "staff_id": "string", "priority": 2}
    ]
  },

  "risks": [
    {
      "risk_id": "string",
      "type": "metric_drop",
      "threshold": "today < 0.8 * 7d_avg",
      "evidence_fields": ["his.gmv", "baselines.rolling_7d.gmv_avg"],
      "note": "GMV 较 7 日均值下降 23%"
    }
  ],

  "checklist": [
    {
      "item_id": "string",
      "owner_role": "store_manager",
      "action": "确认明日10:00-12:00预约集中安排与医生排班",
      "due": "2025-12-30 20:30",
      "evidence_ref": ["risks:risk_id_xxx"]
    }
  ],

  "data_quality": {
    "wecom_touch_complete": true,
    "notes": []
  }
}
```

### 7.2 企微输出格式（给人看）

* 标题：`{门店}｜{biz_date} 夕会会前简报`
* 四段固定结构（Facts / Tomorrow / Risks / Checklist）
* 每段最多 8 行，超出用“展开查看”链接（避免刷屏）

---

## 8. Observability｜可观测性

必须记录：

* 输入 hash（用于复现）
* 风险命中分布（按 type）
* checklist 完成率（如果你们后续做回写）
* 明日预约覆盖（是否都被提及）

必须支持追问：

* “今天为什么提示 metric_drop？”
* “为什么某顾客进入明日清单？”
