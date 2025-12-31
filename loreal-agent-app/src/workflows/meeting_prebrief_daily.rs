use agent_runtime::runtime::{AgentError, WorkflowOutput, WorkflowRunner};
use once_cell::sync::Lazy;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

const MAX_LIST_ITEMS: usize = 20;
const MIN_CHECKLIST_ITEMS: usize = 3;
const THRESHOLDS_YAML: &str = include_str!("../../configs/meeting_prebrief_thresholds.yml");

#[derive(Debug, Deserialize)]
struct Thresholds {
    metric_drop_ratio: f64,
    target_gap_rate: f64,
    no_reply_list_min: usize,
    no_reply_rate_max: f64,
    tomorrow_load_count: usize,
}

static THRESHOLDS: Lazy<Thresholds> = Lazy::new(|| {
    serde_yaml::from_str(THRESHOLDS_YAML)
        .expect("meeting_prebrief_thresholds.yml must be valid")
});

pub struct MeetingPrebriefDailyWorkflow;

#[async_trait::async_trait]
impl WorkflowRunner for MeetingPrebriefDailyWorkflow {
    fn name(&self) -> &'static str {
        "meeting_prebrief_daily"
    }

    fn version(&self) -> Option<&'static str> {
        Some("v1.0.0")
    }

    async fn run(&self, input: Value) -> Result<WorkflowOutput, AgentError> {
        let store_id = input
            .get("store_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let biz_date = input
            .get("biz_date")
            .and_then(|v| v.as_str())
            .unwrap_or("1970-01-01")
            .to_string();

        let his = input.get("his").and_then(|v| v.as_object());
        let visits = number_or_zero(his.and_then(|h| h.get("visits")));
        let gmv = number_or_zero(his.and_then(|h| h.get("gmv")));
        let consumption = number_or_zero(his.and_then(|h| h.get("consumption")));
        let new_customers = number_or_zero(his.and_then(|h| h.get("new_customers")));
        let old_customers = number_or_zero(his.and_then(|h| h.get("old_customers")));
        let top_items = extract_top_items(his.and_then(|h| h.get("top_items")));
        let target_gmv = number_or_zero(
            his.and_then(|h| h.get("targets"))
                .and_then(|t| t.get("gmv_target")),
        );

        let baselines = input.get("baselines").and_then(|v| v.as_object());
        let gmv_avg_7d = number_or_zero(
            baselines
                .and_then(|b| b.get("rolling_7d"))
                .and_then(|r| r.get("gmv_avg")),
        );

        let facts_recap = json!({
            "visits": visits,
            "gmv": gmv,
            "consumption": consumption,
            "mom_or_baseline": {
                "gmv_vs_7d": ratio_delta(gmv, gmv_avg_7d)
            },
            "target_progress": {
                "gmv_rate": ratio(gmv, target_gmv)
            },
            "structure": {
                "new": new_customers,
                "old": old_customers
            },
            "top_items": top_items
        });

        let (appointments, appointments_count) =
            extract_appointments(input.get("appointments_tomorrow"));
        let followups = extract_followups(
            input
                .get("wecom_touch")
                .and_then(|v| v.get("no_reply_list")),
        );

        let tomorrow_list = json!({
            "appointments": appointments,
            "followups": followups
        });

        let mut risks = Vec::new();
        if gmv_avg_7d > 0.0 && gmv < gmv_avg_7d * THRESHOLDS.metric_drop_ratio {
            push_risk(
                &mut risks,
                "metric_drop",
                &format!("today < {} * 7d_avg", THRESHOLDS.metric_drop_ratio),
                vec!["his.gmv", "baselines.rolling_7d.gmv_avg"],
                format!(
                    "GMV below 7d average by {:.0}%",
                    ratio_delta(gmv, gmv_avg_7d) * 100.0
                ),
            );
        }

        if target_gmv > 0.0 && ratio(gmv, target_gmv) < THRESHOLDS.target_gap_rate {
            push_risk(
                &mut risks,
                "target_gap",
                &format!("gmv_rate < {}", THRESHOLDS.target_gap_rate),
                vec!["his.gmv", "his.targets.gmv_target"],
                format!("GMV progress at {:.0}%", ratio(gmv, target_gmv) * 100.0),
            );
        }

        let wecom_touch = input.get("wecom_touch").and_then(|v| v.as_object());
        let no_reply_list_len = wecom_touch
            .and_then(|w| w.get("no_reply_list"))
            .and_then(|v| v.as_array())
            .map(|v| v.len())
            .unwrap_or(0);

        let contacted = number_or_zero(wecom_touch.and_then(|w| w.get("contacted")));
        let replied = number_or_zero(wecom_touch.and_then(|w| w.get("replied")));
        let no_reply_rate = if contacted > 0.0 {
            (contacted - replied) / contacted
        } else {
            0.0
        };

        if no_reply_list_len >= THRESHOLDS.no_reply_list_min
            || no_reply_rate > THRESHOLDS.no_reply_rate_max
        {
            push_risk(
                &mut risks,
                "touch_gap",
                &format!(
                    "no_reply_list >= {} or no_reply_rate > {}",
                    THRESHOLDS.no_reply_list_min, THRESHOLDS.no_reply_rate_max
                ),
                vec![
                    "wecom_touch.no_reply_list",
                    "wecom_touch.contacted",
                    "wecom_touch.replied",
                ],
                "WeCom follow-up backlog is high".to_string(),
            );
        }

        if appointments_count > THRESHOLDS.tomorrow_load_count {
            push_risk(
                &mut risks,
                "tomorrow_load",
                &format!(
                    "appointments_count > {}",
                    THRESHOLDS.tomorrow_load_count
                ),
                vec!["appointments_tomorrow"],
                format!("High appointment load: {}", appointments_count),
            );
        }

        let mut checklist = Vec::new();
        for risk in risks.iter().take(2) {
            let risk_id = risk
                .get("risk_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            checklist.push(json!({
                "item_id": format!("item_{}", Uuid::new_v4()),
                "owner_role": "store_manager",
                "action": format!(
                    "Review risk {}",
                    risk.get("type").and_then(|v| v.as_str()).unwrap_or("unknown")
                ),
                "due": format!("{} 20:30", biz_date),
                "evidence_ref": [format!("risks:{}", risk_id)]
            }));
        }

        if appointments_count > 0 {
            checklist.push(json!({
                "item_id": format!("item_{}", Uuid::new_v4()),
                "owner_role": "store_manager",
                "action": "Confirm tomorrow appointment staffing",
                "due": format!("{} 20:30", biz_date),
                "evidence_ref": ["tomorrow_list:appointments"]
            }));
        }

        while checklist.len() < MIN_CHECKLIST_ITEMS {
            checklist.push(json!({
                "item_id": format!("item_{}", Uuid::new_v4()),
                "owner_role": "store_manager",
                "action": "Prepare evening meeting highlights",
                "due": format!("{} 20:30", biz_date),
                "evidence_ref": []
            }));
        }

        let wecom_touch_complete = input.get("wecom_touch").is_some();
        let mut data_quality_notes = Vec::new();
        if !wecom_touch_complete {
            data_quality_notes.push("wecom_touch missing".to_string());
        }

        let output = json!({
            "run_id": format!("run_{}", Uuid::new_v4()),
            "biz_date": biz_date,
            "store_id": store_id,
            "facts_recap": facts_recap,
            "tomorrow_list": tomorrow_list,
            "risks": risks,
            "checklist": checklist,
            "data_quality": {
                "wecom_touch_complete": wecom_touch_complete,
                "notes": data_quality_notes
            }
        });

        Ok(WorkflowOutput {
            output,
            artifacts: Vec::new(),
        })
    }
}

fn number_or_zero(value: Option<&Value>) -> f64 {
    match value {
        Some(Value::Number(number)) => number.as_f64().unwrap_or(0.0),
        Some(Value::String(text)) => text.parse::<f64>().unwrap_or(0.0),
        _ => 0.0,
    }
}

fn ratio(value: f64, baseline: f64) -> f64 {
    if baseline > 0.0 {
        value / baseline
    } else {
        0.0
    }
}

fn ratio_delta(value: f64, baseline: f64) -> f64 {
    if baseline > 0.0 {
        (value - baseline) / baseline
    } else {
        0.0
    }
}

fn extract_top_items(value: Option<&Value>) -> Vec<Value> {
    let Some(items) = value.and_then(|v| v.as_array()) else {
        return Vec::new();
    };

    items
        .iter()
        .filter_map(|item| {
            let item_obj = item.as_object()?;
            let name = item_obj.get("item").and_then(|v| v.as_str()).unwrap_or("");
            let amount = number_or_zero(item_obj.get("amount"));
            Some(json!({
                "item": name,
                "amount": amount
            }))
        })
        .collect()
}

fn extract_appointments(value: Option<&Value>) -> (Vec<Value>, usize) {
    let Some(items) = value.and_then(|v| v.as_array()) else {
        return (Vec::new(), 0);
    };
    let total = items.len();
    let appointments = items
        .iter()
        .take(MAX_LIST_ITEMS)
        .map(|item| {
            let customer_id = item
                .get("customer_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let time = item.get("time").and_then(|v| v.as_str()).unwrap_or("");
            let item_name = item.get("item").and_then(|v| v.as_str()).unwrap_or("");
            let staff_id = item
                .get("staff_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let is_first_visit = item
                .get("is_first_visit")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let priority = if is_first_visit { 1 } else { 2 };
            json!({
                "customer_id": customer_id,
                "time": time,
                "item": item_name,
                "staff_id": staff_id,
                "priority": priority
            })
        })
        .collect();

    (appointments, total)
}

fn extract_followups(value: Option<&Value>) -> Vec<Value> {
    let Some(items) = value.and_then(|v| v.as_array()) else {
        return Vec::new();
    };

    items
        .iter()
        .take(MAX_LIST_ITEMS)
        .map(|item| {
            let customer_id = item
                .get("customer_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let last_touch_at = item
                .get("last_touch_at")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let staff_id = item
                .get("staff_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            json!({
                "customer_id": customer_id,
                "last_touch_at": last_touch_at,
                "staff_id": staff_id,
                "priority": 2
            })
        })
        .collect()
}

fn push_risk(
    risks: &mut Vec<Value>,
    risk_type: &str,
    threshold: &str,
    evidence_fields: Vec<&str>,
    note: String,
) {
    let evidence_fields: Vec<String> = evidence_fields
        .into_iter()
        .map(|field| field.to_string())
        .collect();
    risks.push(json!({
        "risk_id": format!("risk_{}", Uuid::new_v4()),
        "type": risk_type,
        "threshold": threshold,
        "evidence_fields": evidence_fields,
        "note": note
    }));
}
