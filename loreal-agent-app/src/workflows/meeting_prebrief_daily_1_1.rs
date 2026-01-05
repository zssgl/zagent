use std::collections::HashMap;
use std::path::Path;

use agent_runtime::runtime::{AgentError, WorkflowOutput, WorkflowRunner};
use jsonschema::JSONSchema;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use super::spec::WorkflowSpec;
use crate::tools::{
    assemble_meeting_prebrief_daily_1_1_mysql, merge_json, MysqlAssembleError, SharedTools,
};

const MAX_LIST_ITEMS: usize = 20;
const MIN_CHECKLIST_ITEMS: usize = 3;

#[derive(Debug, Deserialize)]
struct ThresholdMap(HashMap<String, serde_yaml::Value>);

impl ThresholdMap {
    fn get_f64(&self, key: &str) -> Option<f64> {
        match self.0.get(key) {
            Some(serde_yaml::Value::Number(number)) => number.as_f64(),
            Some(serde_yaml::Value::String(text)) => text.parse::<f64>().ok(),
            _ => None,
        }
    }

    fn get_usize(&self, key: &str) -> Option<usize> {
        match self.0.get(key) {
            Some(serde_yaml::Value::Number(number)) => number.as_u64().map(|v| v as usize),
            Some(serde_yaml::Value::String(text)) => text.parse::<usize>().ok(),
            _ => None,
        }
    }

    fn replacements(&self) -> Vec<(String, String)> {
        self.0
            .iter()
            .map(|(key, value)| (key.clone(), yaml_value_to_string(value)))
            .collect()
    }
}

#[derive(Debug, Deserialize)]
struct WorkflowRules {
    risks: Vec<RiskRule>,
    checklist_templates: Vec<ChecklistTemplate>,
}

#[derive(Debug, Deserialize)]
struct RiskRule {
    #[serde(rename = "type")]
    risk_type: String,
    threshold: String,
    evidence_fields: Vec<String>,
    note_template: String,
    evaluator: RiskEvaluator,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum RiskEvaluator {
    MetricDrop {
        metric: String,
        baseline: String,
        ratio_key: String,
    },
    TargetGap {
        metric: String,
        target: String,
        rate_key: String,
    },
    TouchGap {
        no_reply_list_min_key: String,
        no_reply_rate_max_key: String,
    },
    TomorrowLoad {
        count_key: String,
    },
}

#[derive(Debug, Deserialize)]
struct ChecklistTemplate {
    when_risk_types: Option<Vec<String>>,
    when_tomorrow_list: Option<bool>,
    fallback: Option<bool>,
    owner_role: String,
    action_template: String,
    due_template: String,
    evidence_ref: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default)]
struct Metrics {
    visits: f64,
    visits_avg_7d: f64,
    gmv: f64,
    gmv_avg_7d: f64,
    consumption: f64,
    consumption_avg_7d: f64,
    avg_ticket: f64,
    avg_ticket_avg_7d: f64,
    mtd_gmv: f64,
    mtd_consumption: f64,
    mtd_gmv_target: f64,
    mtd_consumption_target: f64,
    mtd_time_progress: f64,
    appointments_count: usize,
    no_reply_list_len: usize,
    contacted: f64,
    replied: f64,
}

impl Metrics {
    fn value(&self, key: &str) -> Option<f64> {
        match key {
            "visits" => Some(self.visits),
            "visits_avg_7d" => Some(self.visits_avg_7d),
            "gmv" => Some(self.gmv),
            "gmv_avg_7d" => Some(self.gmv_avg_7d),
            "consumption" => Some(self.consumption),
            "consumption_avg_7d" => Some(self.consumption_avg_7d),
            "avg_ticket" => Some(self.avg_ticket),
            "avg_ticket_avg_7d" => Some(self.avg_ticket_avg_7d),
            "mtd_gmv" => Some(self.mtd_gmv),
            "mtd_consumption" => Some(self.mtd_consumption),
            "mtd_gmv_target" => Some(self.mtd_gmv_target),
            "mtd_consumption_target" => Some(self.mtd_consumption_target),
            "mtd_time_progress" => Some(self.mtd_time_progress),
            "contacted" => Some(self.contacted),
            "replied" => Some(self.replied),
            _ => None,
        }
    }
}

pub struct MeetingPrebriefDaily1_1Runner {
    version: String,
    thresholds: ThresholdMap,
    rules: WorkflowRules,
    tools: SharedTools,
    output_schema: JSONSchema,
}

struct ExecutionPlan {
    use_mysql_assembly: bool,
}

impl MeetingPrebriefDaily1_1Runner {
    pub fn from_spec(spec: &WorkflowSpec, tools: SharedTools) -> Result<Self, String> {
        let rules_path = spec
            .rules_path()
            .ok_or_else(|| "workflow spec missing rules".to_string())?;
        let rules_content =
            std::fs::read_to_string(rules_path).map_err(|err| format!("read rules failed: {}", err))?;
        let rules: WorkflowRules =
            serde_yaml::from_str(&rules_content).map_err(|err| format!("invalid rules: {}", err))?;

        let thresholds_path = spec
            .thresholds_path()
            .ok_or_else(|| "workflow spec missing thresholds".to_string())?;
        let thresholds_content = std::fs::read_to_string(thresholds_path)
            .map_err(|err| format!("read thresholds failed: {}", err))?;
        let thresholds: HashMap<String, serde_yaml::Value> =
            serde_yaml::from_str(&thresholds_content)
                .map_err(|err| format!("invalid thresholds: {}", err))?;
        let output_schema_content = std::fs::read_to_string(spec.output_schema_path())
            .map_err(|err| format!("read output schema failed: {}", err))?;
        let output_schema_json: Value = serde_json::from_str(&output_schema_content)
            .map_err(|err| format!("invalid output schema json: {}", err))?;
        let output_schema = JSONSchema::compile(&output_schema_json)
            .map_err(|err| format!("invalid output schema: {}", err))?;

        Ok(Self {
            version: spec.version.clone(),
            thresholds: ThresholdMap(thresholds),
            rules,
            tools,
            output_schema,
        })
    }
}

#[async_trait::async_trait]
impl WorkflowRunner for MeetingPrebriefDaily1_1Runner {
    fn name(&self) -> &'static str {
        "meeting_prebrief_daily"
    }

    fn version(&self) -> Option<&'static str> {
        Some(Box::leak(self.version.clone().into_boxed_str()))
    }

    async fn run(&self, input: Value) -> Result<WorkflowOutput, AgentError> {
        let plan = build_execution_plan(&input);
        let input = normalize_input(input, &plan, &self.tools).await?;
        validate_input_completeness(&input)?;
        let output = execute_workflow(&input, &self.rules, &self.thresholds);
        let report_md = render_report_md(&input, &output);
        let output = attach_report_md(output, report_md);
        validate_output_schema(&output, &self.output_schema)?;
        let biz_date = output
            .get("biz_date")
            .and_then(|v| v.as_str())
            .unwrap_or("1970-01-01");
        let report_md = output
            .get("report_md")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        persist_report_md(report_md, biz_date)
            .await
            .map_err(AgentError::fatal)?;

        Ok(WorkflowOutput {
            output,
            artifacts: Vec::new(),
        })
    }
}

fn build_execution_plan(input: &Value) -> ExecutionPlan {
    ExecutionPlan {
        use_mysql_assembly: wants_mysql_assembly(input.get("__context")),
    }
}

fn validate_input_completeness(input: &Value) -> Result<(), AgentError> {
    let mut missing = Vec::new();

    if input.get("store_id").and_then(|v| v.as_str()).is_none() {
        missing.push("store_id".to_string());
    }
    if input.get("biz_date").and_then(|v| v.as_str()).is_none() {
        missing.push("biz_date".to_string());
    }
    let his = input.get("his").and_then(|v| v.as_object());
    if his.is_none() {
        missing.push("his".to_string());
    } else {
        let his = his.unwrap();
        for key in [
            "visits",
            "gmv",
            "consumption",
            "avg_ticket",
            "new_customers",
            "old_customers",
        ] {
            if !his.contains_key(key) {
                missing.push(format!("his.{}", key));
            }
        }
    }

    if missing.is_empty() {
        return Ok(());
    }

    let details = json!({
        "kind": "missing_required_fields",
        "missing_fields": missing,
        "missing_info_list": missing.iter().map(|field| {
            json!({
                "path": field,
                "reason": "required"
            })
        }).collect::<Vec<Value>>()
    });
    Err(AgentError::fatal_with_details(
        "missing required fields",
        details,
    ))
}

fn validate_output_schema(output: &Value, schema: &JSONSchema) -> Result<(), AgentError> {
    if let Err(errors) = schema.validate(output) {
        let mapped: Vec<Value> = errors
            .map(|err| json!({ "message": err.to_string() }))
            .collect();
        let details = json!({
            "kind": "output_schema_validation",
            "errors": mapped
        });
        return Err(AgentError::fatal_with_details(
            "output schema validation failed",
            details,
        ));
    }
    Ok(())
}

async fn normalize_input(
    mut input: Value,
    plan: &ExecutionPlan,
    tools: &SharedTools,
) -> Result<Value, AgentError> {
    if plan.use_mysql_assembly {
        let Some(pool) = tools.mysql() else {
            return Err(AgentError::fatal(
                "mysql not configured (DATABASE_URL missing or connection failed)",
            ));
        };
        let assembled = assemble_meeting_prebrief_daily_1_1_mysql(pool, &input)
            .await
            .map_err(|err| match err {
                MysqlAssembleError::InvalidInput(message) => AgentError::fatal(message),
                MysqlAssembleError::Db(message) => AgentError::retryable(message),
            })?;
        let mut merged = assembled;
        merge_json(&mut merged, &input);
        input = merged;
    }
    if let Value::Object(map) = &mut input {
        map.remove("__context");
    }
    Ok(input)
}

fn execute_workflow(input: &Value, rules: &WorkflowRules, thresholds: &ThresholdMap) -> Value {
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

    let (appointments, appointments_count) = build_appointments(input);
    let followups = build_followups(input);
    let tomorrow_list = json!({
        "appointments": appointments,
        "followups": followups
    });

    let (metrics, no_reply_rate, wecom_touch_complete, data_quality_notes) =
        build_metrics(input, appointments_count);

    let thresholds_replacements = thresholds.replacements();
    let risks = build_risks(rules, thresholds, &metrics, no_reply_rate, &thresholds_replacements);

    let checklist = build_checklist(
        &biz_date,
        appointments_count,
        &risks,
        &rules.checklist_templates,
    );
    let facts_recap = build_facts_recap(input);

    json!({
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
    })
}

fn build_metrics(
    input: &Value,
    appointments_count: usize,
) -> (Metrics, f64, bool, Vec<String>) {
    let wecom_touch = get_value_at_path(input, "wecom_touch").and_then(|v| v.as_object());
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

    let metrics = Metrics {
        visits: number_or_zero(get_value_at_path(input, "his.visits")),
        visits_avg_7d: number_or_zero(get_value_at_path(input, "baselines.rolling_7d.visits_avg")),
        gmv: number_or_zero(get_value_at_path(input, "his.gmv")),
        gmv_avg_7d: number_or_zero(get_value_at_path(input, "baselines.rolling_7d.gmv_avg")),
        consumption: number_or_zero(get_value_at_path(input, "his.consumption")),
        consumption_avg_7d: number_or_zero(get_value_at_path(
            input,
            "baselines.rolling_7d.consumption_avg",
        )),
        avg_ticket: number_or_zero(get_value_at_path(input, "his.avg_ticket")),
        avg_ticket_avg_7d: number_or_zero(get_value_at_path(
            input,
            "baselines.rolling_7d.avg_ticket_avg",
        )),
        mtd_gmv: number_or_zero(get_value_at_path(input, "mtd.gmv")),
        mtd_consumption: number_or_zero(get_value_at_path(input, "mtd.consumption")),
        mtd_gmv_target: number_or_zero(get_value_at_path(input, "mtd.gmv_target")),
        mtd_consumption_target: number_or_zero(get_value_at_path(input, "mtd.consumption_target")),
        mtd_time_progress: number_or_zero(get_value_at_path(input, "mtd.time_progress")),
        appointments_count,
        no_reply_list_len,
        contacted,
        replied,
    };

    let wecom_touch_complete = get_value_at_path(input, "wecom_touch").is_some();
    let mut data_quality_notes = Vec::new();
    if !wecom_touch_complete {
        data_quality_notes.push("wecom_touch missing".to_string());
    }

    (
        metrics,
        no_reply_rate,
        wecom_touch_complete,
        data_quality_notes,
    )
}

fn build_risks(
    rules: &WorkflowRules,
    thresholds: &ThresholdMap,
    metrics: &Metrics,
    no_reply_rate: f64,
    thresholds_replacements: &[(String, String)],
) -> Vec<Value> {
    let mut risks = Vec::new();
    for rule in rules.risks.iter() {
        let Some(mut note_replacements) =
            evaluate_rule(rule, metrics, no_reply_rate, thresholds)
        else {
            continue;
        };
        let threshold_text = render_template(&rule.threshold, thresholds_replacements);
        note_replacements.extend(thresholds_replacements.to_vec());
        let note_text = render_template(&rule.note_template, &note_replacements);
        push_risk(
            &mut risks,
            &rule.risk_type,
            threshold_text,
            &rule.evidence_fields,
            note_text,
        );
    }
    risks
}

fn attach_report_md(mut output: Value, report_md: String) -> Value {
    if let Value::Object(map) = &mut output {
        map.insert("report_md".to_string(), Value::String(report_md));
    }
    output
}

fn wants_mysql_assembly(context: Option<&Value>) -> bool {
    let Some(context) = context else {
        return false;
    };
    match context.get("assemble") {
        Some(Value::Bool(true)) => true,
        Some(Value::Object(map)) => map
            .get("source")
            .and_then(|v| v.as_str())
            .is_some_and(|v| v.eq_ignore_ascii_case("mysql")),
        _ => false,
    }
}

async fn persist_report_md(report_md: &str, biz_date: &str) -> Result<(), String> {
    let file_suffix = chrono::NaiveDate::parse_from_str(biz_date, "%Y-%m-%d")
        .map(|d| d.format("%Y%m%d").to_string())
        .unwrap_or_else(|_| biz_date.replace('-', ""));
    let report_dir = std::env::var("REPORTS_DIR").unwrap_or_else(|_| "reports".to_string());
    tokio::fs::create_dir_all(&report_dir)
        .await
        .map_err(|err| format!("create reports dir failed: {}", err))?;
    let report_path = format!("{}/briefing_{}.md", report_dir, file_suffix);
    tokio::fs::write(&report_path, report_md.as_bytes())
        .await
        .map_err(|err| format!("write report failed: {}", err))?;
    Ok(())
}

fn build_facts_recap(input: &Value) -> Value {
    let gmv = number_or_zero(get_value_at_path(input, "his.gmv"));
    let consumption = number_or_zero(get_value_at_path(input, "his.consumption"));
    let visits = number_or_zero(get_value_at_path(input, "his.visits"));
    let avg_ticket = number_or_zero(get_value_at_path(input, "his.avg_ticket"));

    let gmv_vs_7d = ratio_delta(
        gmv,
        number_or_zero(get_value_at_path(input, "baselines.rolling_7d.gmv_avg")),
    );
    let consumption_vs_7d = ratio_delta(
        consumption,
        number_or_zero(get_value_at_path(input, "baselines.rolling_7d.consumption_avg")),
    );
    let visits_vs_7d = ratio_delta(
        visits,
        number_or_zero(get_value_at_path(input, "baselines.rolling_7d.visits_avg")),
    );
    let avg_ticket_vs_7d = ratio_delta(
        avg_ticket,
        number_or_zero(get_value_at_path(input, "baselines.rolling_7d.avg_ticket_avg")),
    );

    let mtd_gmv = number_or_zero(get_value_at_path(input, "mtd.gmv"));
    let mtd_consumption = number_or_zero(get_value_at_path(input, "mtd.consumption"));
    let mtd_gmv_target = number_or_zero(get_value_at_path(input, "mtd.gmv_target"));
    let mtd_consumption_target = number_or_zero(get_value_at_path(input, "mtd.consumption_target"));
    let mtd_time_progress = number_or_zero(get_value_at_path(input, "mtd.time_progress"));

    json!({
        "today": {
            "visits": visits,
            "gmv": gmv,
            "consumption": consumption,
            "avg_ticket": avg_ticket,
            "structure": {
                "new_customers": number_or_zero(get_value_at_path(input, "his.new_customers")),
                "old_customers": number_or_zero(get_value_at_path(input, "his.old_customers"))
            },
            "vs_7d": {
                "gmv_delta": gmv_vs_7d,
                "consumption_delta": consumption_vs_7d,
                "visits_delta": visits_vs_7d,
                "avg_ticket_delta": avg_ticket_vs_7d
            },
            "top_items": get_value_at_path(input, "his.top_items").cloned().unwrap_or(Value::Array(vec![]))
        },
        "mtd": {
            "gmv_mtd": mtd_gmv,
            "consumption_mtd": mtd_consumption,
            "time_progress": mtd_time_progress,
            "gmv_target": mtd_gmv_target,
            "consumption_target": mtd_consumption_target,
            "gmv_rate": ratio(mtd_gmv, mtd_gmv_target),
            "consumption_rate": ratio(mtd_consumption, mtd_consumption_target)
        },
        "staff_stats": get_value_at_path(input, "staff_stats").cloned().unwrap_or(Value::Array(vec![])),
        "customer_summary": get_value_at_path(input, "customer_summary").cloned().unwrap_or(Value::Object(serde_json::Map::new())),
        "key_items_mtd": get_value_at_path(input, "key_items_mtd").cloned().unwrap_or(Value::Array(vec![])),
        "task_execution": get_value_at_path(input, "task_execution").cloned().unwrap_or(Value::Object(serde_json::Map::new()))
    })
}

fn build_appointments(input: &Value) -> (Vec<Value>, usize) {
    let mut items = get_value_at_path(input, "appointments_tomorrow")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let total = items.len();
    items.sort_by(|a, b| {
        let pa = a.get("is_first_visit").and_then(|v| v.as_bool()).unwrap_or(false);
        let pb = b.get("is_first_visit").and_then(|v| v.as_bool()).unwrap_or(false);
        pb.cmp(&pa)
    });
    let mut mapped = Vec::new();
    for item in items.into_iter().take(MAX_LIST_ITEMS) {
        let mut obj = serde_json::Map::new();
        obj.insert(
            "customer_id".to_string(),
            item.get("customer_id").cloned().unwrap_or(Value::Null),
        );
        obj.insert("time".to_string(), item.get("time").cloned().unwrap_or(Value::Null));
        obj.insert("item".to_string(), item.get("item").cloned().unwrap_or(Value::Null));
        obj.insert(
            "staff_id".to_string(),
            item.get("staff_id").cloned().unwrap_or(Value::Null),
        );
        let priority = if item.get("is_first_visit").and_then(|v| v.as_bool()).unwrap_or(false) {
            1
        } else {
            2
        };
        obj.insert("priority".to_string(), json!(priority));
        mapped.push(Value::Object(obj));
    }
    (mapped, total)
}

fn build_followups(input: &Value) -> Vec<Value> {
    let list = get_value_at_path(input, "wecom_touch.no_reply_list")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    list.into_iter()
        .take(MAX_LIST_ITEMS)
        .filter_map(|item| item.as_object().cloned())
        .map(|item| {
            let mut obj = serde_json::Map::new();
            obj.insert(
                "customer_id".to_string(),
                item.get("customer_id").cloned().unwrap_or(Value::Null),
            );
            obj.insert(
                "last_touch_at".to_string(),
                item.get("last_touch_at").cloned().unwrap_or(Value::Null),
            );
            obj.insert(
                "staff_id".to_string(),
                item.get("staff_id").cloned().unwrap_or(Value::Null),
            );
            obj.insert("priority".to_string(), json!(2));
            Value::Object(obj)
        })
        .collect()
}

fn build_checklist(
    biz_date: &str,
    appointments_count: usize,
    risks: &[Value],
    templates: &[ChecklistTemplate],
) -> Vec<Value> {
    let mut checklist = Vec::new();

    for risk in risks.iter().take(3) {
        let risk_type = risk.get("type").and_then(|v| v.as_str()).unwrap_or("unknown");
        let risk_id = risk.get("risk_id").and_then(|v| v.as_str()).unwrap_or("unknown");
        if let Some(template) = find_risk_template(templates, risk_type) {
            let replacements = checklist_replacements(biz_date, Some(risk_type), appointments_count);
            let action = render_template(&template.action_template, &replacements);
            let due = render_template(&template.due_template, &replacements);
            let mut evidence_ref = template.evidence_ref.clone().unwrap_or_default();
            if evidence_ref.is_empty() {
                evidence_ref.push(format!("risks:{}", risk_id));
            }
            checklist.push(json!({
                "item_id": format!("item_{}", Uuid::new_v4()),
                "owner_role": template.owner_role,
                "action": action,
                "due": due,
                "evidence_ref": evidence_ref
            }));
        }
    }

    if appointments_count > 0 {
        if let Some(template) = find_tomorrow_template(templates) {
            let replacements = checklist_replacements(biz_date, None, appointments_count);
            let action = render_template(&template.action_template, &replacements);
            let due = render_template(&template.due_template, &replacements);
            let evidence_ref = template
                .evidence_ref
                .clone()
                .unwrap_or_else(|| vec!["tomorrow_list:appointments".to_string()]);
            checklist.push(json!({
                "item_id": format!("item_{}", Uuid::new_v4()),
                "owner_role": template.owner_role,
                "action": action,
                "due": due,
                "evidence_ref": evidence_ref
            }));
        }
    }

    while checklist.len() < MIN_CHECKLIST_ITEMS {
        let Some(template) = find_fallback_template(templates) else {
            break;
        };
        let replacements = checklist_replacements(biz_date, None, appointments_count);
        let action = render_template(&template.action_template, &replacements);
        let due = render_template(&template.due_template, &replacements);
        let evidence_ref = template.evidence_ref.clone().unwrap_or_default();
        checklist.push(json!({
            "item_id": format!("item_{}", Uuid::new_v4()),
            "owner_role": template.owner_role,
            "action": action,
            "due": due,
            "evidence_ref": evidence_ref
        }));
    }

    checklist
}

fn checklist_replacements(
    biz_date: &str,
    risk_type: Option<&str>,
    appointments_count: usize,
) -> Vec<(String, String)> {
    let mut replacements = vec![
        ("biz_date".to_string(), biz_date.to_string()),
        ("appointments_count".to_string(), appointments_count.to_string()),
    ];
    if let Some(risk_type) = risk_type {
        replacements.push(("risk_type".to_string(), risk_type.to_string()));
    }
    replacements
}

fn find_risk_template<'a>(
    templates: &'a [ChecklistTemplate],
    risk_type: &str,
) -> Option<&'a ChecklistTemplate> {
    templates.iter().find(|template| {
        template
            .when_risk_types
            .as_ref()
            .map(|types| types.iter().any(|item| item == risk_type))
            .unwrap_or(false)
    })
}

fn find_tomorrow_template<'a>(templates: &'a [ChecklistTemplate]) -> Option<&'a ChecklistTemplate> {
    templates.iter().find(|template| template.when_tomorrow_list.unwrap_or(false))
}

fn find_fallback_template<'a>(templates: &'a [ChecklistTemplate]) -> Option<&'a ChecklistTemplate> {
    templates.iter().find(|template| template.fallback.unwrap_or(false))
}

fn evaluate_rule(
    rule: &RiskRule,
    metrics: &Metrics,
    no_reply_rate: f64,
    thresholds: &ThresholdMap,
) -> Option<Vec<(String, String)>> {
    let mut replacements = vec![(
        "appointments_count".to_string(),
        metrics.appointments_count.to_string(),
    )];
    match &rule.evaluator {
        RiskEvaluator::MetricDrop {
            metric,
            baseline,
            ratio_key,
        } => {
            let metric_value = metrics.value(metric)?;
            let baseline_value = metrics.value(baseline)?;
            let ratio = thresholds.get_f64(ratio_key)?;
            if baseline_value > 0.0 && metric_value < baseline_value * ratio {
                replacements.push((
                    "delta_pct".to_string(),
                    format!("{:.0}", ratio_delta(metric_value, baseline_value) * 100.0),
                ));
                Some(replacements)
            } else {
                None
            }
        }
        RiskEvaluator::TargetGap {
            metric,
            target,
            rate_key,
        } => {
            let metric_value = metrics.value(metric)?;
            let target_value = metrics.value(target)?;
            let rate = thresholds.get_f64(rate_key)?;
            if target_value > 0.0 && ratio(metric_value, target_value) < rate {
                replacements.push((
                    "rate_pct".to_string(),
                    format!("{:.0}", ratio(metric_value, target_value) * 100.0),
                ));
                Some(replacements)
            } else {
                None
            }
        }
        RiskEvaluator::TouchGap {
            no_reply_list_min_key,
            no_reply_rate_max_key,
        } => {
            let min_count = thresholds.get_usize(no_reply_list_min_key)?;
            let max_rate = thresholds.get_f64(no_reply_rate_max_key)?;
            if metrics.no_reply_list_len >= min_count || no_reply_rate > max_rate {
                Some(replacements)
            } else {
                None
            }
        }
        RiskEvaluator::TomorrowLoad { count_key } => {
            let max_count = thresholds.get_usize(count_key)?;
            if metrics.appointments_count > max_count {
                Some(replacements)
            } else {
                None
            }
        }
    }
}

fn push_risk(
    risks: &mut Vec<Value>,
    risk_type: &str,
    threshold: String,
    evidence_fields: &[String],
    note: String,
) {
    risks.push(json!({
        "risk_id": format!("risk_{}", Uuid::new_v4()),
        "type": risk_type,
        "threshold": threshold,
        "evidence_fields": evidence_fields,
        "note": note
    }));
}

fn yaml_value_to_string(value: &serde_yaml::Value) -> String {
    match value {
        serde_yaml::Value::Number(number) => number.to_string(),
        serde_yaml::Value::String(text) => text.clone(),
        serde_yaml::Value::Bool(value) => value.to_string(),
        _ => String::new(),
    }
}

fn render_template(template: &str, replacements: &[(String, String)]) -> String {
    let mut rendered = template.to_string();
    for (key, value) in replacements {
        rendered = rendered.replace(&format!("{{{}}}", key), value);
    }
    rendered
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

fn get_value_at_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for key in path.split('.') {
        match current {
            Value::Object(map) => {
                current = map.get(key)?;
            }
            _ => return None,
        }
    }
    Some(current)
}

fn format_int_like(value: f64) -> String {
    if value.is_finite() {
        format!("{:.0}", value)
    } else {
        "0".to_string()
    }
}

fn format_currency(value: f64) -> String {
    format!("￥{}", format_int_like(value))
}

fn format_pct_ratio(value: f64) -> String {
    if value.is_finite() {
        format!("{:.0}%", value * 100.0)
    } else {
        "0%".to_string()
    }
}

fn format_pct_delta(value: f64) -> String {
    if !value.is_finite() {
        return "0%".to_string();
    }
    let pct = value * 100.0;
    if pct >= 0.0 {
        format!("+{:.0}%", pct)
    } else {
        format!("{:.0}%", pct)
    }
}

fn risk_label(risk_type: &str) -> std::borrow::Cow<'static, str> {
    match risk_type {
        "gmv_drop" => std::borrow::Cow::Borrowed("开单下滑"),
        "consumption_drop" => std::borrow::Cow::Borrowed("消耗下滑"),
        "visits_drop" => std::borrow::Cow::Borrowed("到店下滑"),
        "avg_ticket_drop" => std::borrow::Cow::Borrowed("客单下滑"),
        "gmv_target_gap" => std::borrow::Cow::Borrowed("开单进度落后"),
        "consumption_target_gap" => std::borrow::Cow::Borrowed("消耗进度落后"),
        "touch_gap" => std::borrow::Cow::Borrowed("触达未回积压"),
        "tomorrow_load" => std::borrow::Cow::Borrowed("明日承接压力"),
        _ => std::borrow::Cow::Owned(risk_type.to_string()),
    }
}

fn render_report_md(input: &Value, output: &Value) -> String {
    let biz_date = output
        .get("biz_date")
        .and_then(|v| v.as_str())
        .unwrap_or("1970-01-01");
    let store_id = output
        .get("store_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let store_name = input
        .get("store_name")
        .and_then(|v| v.as_str())
        .unwrap_or(store_id);
    let cutoff = input
        .get("data_cutoff_time")
        .and_then(|v| v.as_str())
        .unwrap_or("未提供");

    let facts = output.get("facts_recap").unwrap_or(&Value::Null);
    let visits = number_or_zero(get_value_at_path(facts, "today.visits"));
    let gmv = number_or_zero(get_value_at_path(facts, "today.gmv"));
    let consumption = number_or_zero(get_value_at_path(facts, "today.consumption"));
    let avg_ticket = number_or_zero(get_value_at_path(facts, "today.avg_ticket"));
    let gmv_vs_7d = number_or_zero(get_value_at_path(facts, "today.vs_7d.gmv_delta"));
    let consumption_vs_7d =
        number_or_zero(get_value_at_path(facts, "today.vs_7d.consumption_delta"));
    let visits_vs_7d = number_or_zero(get_value_at_path(facts, "today.vs_7d.visits_delta"));
    let avg_ticket_vs_7d = number_or_zero(get_value_at_path(facts, "today.vs_7d.avg_ticket_delta"));
    let gmv_rate = number_or_zero(get_value_at_path(facts, "mtd.gmv_rate"));
    let consumption_rate = number_or_zero(get_value_at_path(facts, "mtd.consumption_rate"));
    let time_progress = number_or_zero(get_value_at_path(facts, "mtd.time_progress"));

    let risks = output.get("risks").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let checklist = output
        .get("checklist")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let appts = output
        .get("tomorrow_list")
        .and_then(|v| v.get("appointments"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut smart_summary = Vec::new();
    if time_progress > 0.0 && (gmv_rate > 0.0 || consumption_rate > 0.0) {
        let gap_threshold = 0.05;
        if gmv_rate - time_progress < -gap_threshold {
            smart_summary.push("开单完成进度落后于时间进度，需重点盯当晚可落地的补缺动作".to_string());
        }
        if consumption_rate - time_progress < -gap_threshold {
            smart_summary.push("消耗完成进度落后于时间进度，关注明日承接与当日消耗转化".to_string());
        }
    }
    if avg_ticket_vs_7d <= -0.1 {
        smart_summary.push("客单价低于近7日平均，关注升单/组合项目与高客单顾客推进".to_string());
    }
    if visits_vs_7d <= -0.1 {
        smart_summary.push("到店人数低于近7日平均，关注明日预约承接与当晚邀约补量".to_string());
    }
    if gmv_vs_7d >= 0.5 {
        smart_summary.push("今日开单显著高于近7日平均，关注大客/大单结构与交付承接".to_string());
    }
    if consumption_vs_7d >= 0.5 {
        smart_summary.push("今日消耗显著高于近7日平均，关注疗程交付与复购承接".to_string());
    }
    if risks.iter().any(|r| r.get("type").and_then(|v| v.as_str()) == Some("touch_gap")) {
        smart_summary.push("触达未回积压偏高，建议在夕会明确二触达 owner 与截止时间".to_string());
    }

    let mut lines = Vec::new();
    lines.push(format!("日期：{}", biz_date));
    lines.push(String::new());
    lines.push(format!("数据截止时间：{}", cutoff));
    lines.push(String::new());
    lines.push(format!("门店：{}", store_name));
    lines.push(String::new());

    lines.push("## 今日经营摘要".to_string());
    lines.push(format!("- 今日开单：{}（{}）", format_currency(gmv), format_pct_delta(gmv_vs_7d)));
    lines.push(format!(
        "- 今日消耗：{}（{}）",
        format_currency(consumption),
        format_pct_delta(consumption_vs_7d)
    ));
    let today_appts = number_or_zero(get_value_at_path(input, "his.appointments"));
    let today_deals = number_or_zero(get_value_at_path(input, "his.deals"));
    if today_appts > 0.0 || today_deals > 0.0 {
        lines.push(format!(
            "- 今日预约人数：{}，到店人数：{}；成交人数：{}",
            format_int_like(today_appts),
            format_int_like(visits),
            format_int_like(today_deals)
        ));
    }
    lines.push(format!(
        "- 今日到店人数：{}；客单价：{}（{}）",
        format_int_like(visits),
        format_currency(avg_ticket),
        format_pct_delta(avg_ticket_vs_7d)
    ));
    if gmv_rate > 0.0 || consumption_rate > 0.0 {
        lines.push(format!(
            "- 月度指标完成度：开单 {}；消耗 {}",
            format_pct_ratio(gmv_rate),
            format_pct_ratio(consumption_rate)
        ));
    }
    lines.push(String::new());
    lines.push("<font color=\"RED\">智能总结</font>".to_string());
    if smart_summary.is_empty() {
        lines.push("暂无（关键上下文不足或未触发总结规则）".to_string());
    } else {
        for item in smart_summary.iter().take(6) {
            lines.push(item.to_string());
        }
    }
    lines.push(String::new());

    lines.push("## 各健康管理人完成情况".to_string());
    let staff_stats = facts
        .get("staff_stats")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    if staff_stats.is_empty() {
        lines.push("- 数据未提供".to_string());
    } else {
        for staff in staff_stats.iter().take(8) {
            let name = staff.get("staff_name").and_then(|v| v.as_str()).unwrap_or("-");
            let today_gmv = number_or_zero(staff.get("today_gmv"));
            let today_cons = number_or_zero(staff.get("today_consumption"));
            let mtd_gmv = number_or_zero(staff.get("mtd_gmv"));
            let mtd_cons = number_or_zero(staff.get("mtd_consumption"));
            let r12 = number_or_zero(staff.get("r12_rate"));
            lines.push(format!(
                "- {}：今日开单{}，消耗{}；本月累计开单{}，累计消耗{}；R12回购率 {}",
                name,
                format_currency(today_gmv),
                format_currency(today_cons),
                format_currency(mtd_gmv),
                format_currency(mtd_cons),
                format_pct_ratio(r12)
            ));
        }
    }
    lines.push(String::new());

    lines.push("## 顾客摘要".to_string());
    let customer_summary = facts.get("customer_summary").cloned().unwrap_or(Value::Null);
    if customer_summary.is_null() {
        lines.push("- 数据未提供".to_string());
    } else {
        let new_count = number_or_zero(get_value_at_path(&customer_summary, "new.count"));
        let new_gmv = number_or_zero(get_value_at_path(&customer_summary, "new.gmv"));
        let old_count = number_or_zero(get_value_at_path(&customer_summary, "old.count"));
        let old_gmv = number_or_zero(get_value_at_path(&customer_summary, "old.gmv"));
        let single_item = number_or_zero(get_value_at_path(&customer_summary, "single_item_customers"));
        let vip = number_or_zero(get_value_at_path(&customer_summary, "vip_customers"));
        if new_count > 0.0 || old_count > 0.0 {
            lines.push(format!("- 新客：{}人，{}", format_int_like(new_count), format_currency(new_gmv)));
            let sources = get_value_at_path(&customer_summary, "new.sources")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            if !sources.is_empty() {
                for s in sources.iter().take(5) {
                    let source = s.get("source").and_then(|v| v.as_str()).unwrap_or("未知");
                    let cnt = number_or_zero(s.get("count"));
                    let gmv = number_or_zero(s.get("gmv"));
                    lines.push(format!(
                        "  - {}：{}人，{}",
                        source,
                        format_int_like(cnt),
                        format_currency(gmv)
                    ));
                }
            }
            lines.push(format!("- 老客：{}人，{}", format_int_like(old_count), format_currency(old_gmv)));
        }
        if single_item > 0.0 {
            lines.push(format!(
                "- 今日到店顾客中，12个月仅消耗一个项目的顾客：{}人",
                format_int_like(single_item)
            ));
        }
        if vip > 0.0 {
            lines.push(format!(
                "- 今日到店顾客中，VIP顾客：{}人",
                format_int_like(vip)
            ));
        }
        if new_count == 0.0 && old_count == 0.0 && single_item == 0.0 && vip == 0.0 {
            lines.push("- 数据未提供".to_string());
        }
    }
    lines.push(String::new());

    lines.push("## 关键品项完成（本月至今）".to_string());
    let key_items = facts
        .get("key_items_mtd")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    if key_items.is_empty() {
        lines.push("- 数据未提供".to_string());
    } else {
        for item in key_items.iter().take(10) {
            let name = item.get("item").and_then(|v| v.as_str()).unwrap_or("-");
            let gmv_mtd = number_or_zero(item.get("gmv_mtd"));
            let cons_mtd = number_or_zero(item.get("consumption_mtd"));
            let wow_gmv = number_or_zero(item.get("wow_gmv"));
            let wow_cons = number_or_zero(item.get("wow_consumption"));
            lines.push(format!(
                "- {}：开单{}（{}WOW），消耗{}（{}WOW）",
                name,
                format_currency(gmv_mtd),
                format_pct_delta(wow_gmv),
                format_currency(cons_mtd),
                format_pct_delta(wow_cons)
            ));
        }
    }
    lines.push(String::new());

    lines.push("## 任务执行情况".to_string());
    let task_execution = facts.get("task_execution").cloned().unwrap_or(Value::Null);
    if task_execution.is_null() {
        lines.push("- 数据未提供".to_string());
    } else {
        let followup = number_or_zero(get_value_at_path(&task_execution, "followup_done_rate"));
        let photo = number_or_zero(get_value_at_path(&task_execution, "photo_sent_rate"));
        let postop = number_or_zero(get_value_at_path(&task_execution, "postop_reminder_rate"));
        let ai_record = number_or_zero(get_value_at_path(&task_execution, "ai_record_rate"));
        let emr = number_or_zero(get_value_at_path(&task_execution, "emr_done_rate"));
        // If task_execution is provided, always print the KPIs (0% is still meaningful).
        lines.push(format!("- 回访完成率：{}", format_pct_ratio(followup)));
        lines.push(format!("- 对比照发送完成率：{}", format_pct_ratio(photo)));
        lines.push(format!("- 术后提醒发送完成率：{}", format_pct_ratio(postop)));
        lines.push(format!("- AI面诊记录生成率：{}", format_pct_ratio(ai_record)));
        lines.push(format!("- 病历完成比例：{}", format_pct_ratio(emr)));
    }
    lines.push(String::new());

    lines.push("## 核心风险提示".to_string());
    if risks.is_empty() {
        lines.push("- 无明显风险（按当前规则）".to_string());
    } else {
        for risk in risks.iter().take(10) {
            let note = risk.get("note").and_then(|v| v.as_str()).unwrap_or("");
            let risk_type = risk.get("type").and_then(|v| v.as_str()).unwrap_or("risk");
            if note.is_empty() {
                lines.push(format!("- {}", risk_label(risk_type)));
            } else {
                lines.push(format!("- {}：{}", risk_label(risk_type), note));
            }
        }
    }
    lines.push(String::new());

    lines.push("## 明日生意准备".to_string());
    lines.push(format!("- 明日预约人数：{}", appts.len()));
    if appts.is_empty() {
        lines.push("- 明日预约清单：数据未同步/为空".to_string());
    } else {
        lines.push("- 明日预约清单（Top10）：".to_string());
        for (idx, item) in appts.iter().take(10).enumerate() {
            let time = item.get("time").and_then(|v| v.as_str()).unwrap_or("-");
            let customer_id = item.get("customer_id").and_then(|v| v.as_str()).unwrap_or("-");
            let appt_item = item.get("item").and_then(|v| v.as_str()).unwrap_or("-");
            lines.push(format!("  {}. {} {} {}", idx + 1, time, customer_id, appt_item));
        }
    }
    lines.push(String::new());

    lines.push("## 会后执行 checklist".to_string());
    if checklist.is_empty() {
        lines.push("- 清单为空（需要检查 rules.yml）".to_string());
    } else {
        for item in checklist.iter().take(12) {
            let owner = item.get("owner_role").and_then(|v| v.as_str()).unwrap_or("owner");
            let action = item.get("action").and_then(|v| v.as_str()).unwrap_or("");
            let due = item.get("due").and_then(|v| v.as_str()).unwrap_or("");
            if due.is_empty() {
                lines.push(format!("- [{}] {}", owner, action));
            } else {
                lines.push(format!("- [{}] {}（截止 {}）", owner, action, due));
            }
        }
    }

    lines.join("\n")
}

pub fn load_latest_active_spec_path() -> Result<std::path::PathBuf, String> {
    let workflow_root = Path::new("loreal-agent-app/workflows/1-1_meeting_prebrief_daily");
    super::spec::discover_latest_active_version(workflow_root)
}
