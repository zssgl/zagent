use std::collections::HashMap;
use std::path::{Path, PathBuf};

use agent_runtime::runtime::{AgentError, WorkflowOutput, WorkflowRunner};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

const MAX_LIST_ITEMS: usize = 20;
const MIN_CHECKLIST_ITEMS: usize = 3;

#[derive(Debug, Deserialize)]
struct WorkflowSpecFile {
    workflow_id: String,
    version: String,
    input_schema: String,
    output_schema: String,
    thresholds: String,
    rules: String,
}

pub struct WorkflowSpec {
    workflow_id: String,
    version: String,
    input_schema: String,
    output_schema: String,
    thresholds: String,
    rules: String,
    base_dir: PathBuf,
}

impl WorkflowSpec {
    pub fn load(path: &str) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|err| format!("read workflow spec failed: {}", err))?;
        let spec: WorkflowSpecFile =
            serde_yaml::from_str(&content).map_err(|err| format!("invalid workflow spec: {}", err))?;
        let base_dir = Path::new(path)
            .parent()
            .ok_or_else(|| "workflow spec missing parent dir".to_string())?
            .to_path_buf();
        Ok(Self {
            workflow_id: spec.workflow_id,
            version: spec.version,
            input_schema: spec.input_schema,
            output_schema: spec.output_schema,
            thresholds: spec.thresholds,
            rules: spec.rules,
            base_dir,
        })
    }

    pub fn input_schema_path(&self) -> PathBuf {
        self.base_dir.join(&self.input_schema)
    }

    pub fn output_schema_path(&self) -> PathBuf {
        self.base_dir.join(&self.output_schema)
    }

    fn thresholds_path(&self) -> PathBuf {
        self.base_dir.join(&self.thresholds)
    }

    fn rules_path(&self) -> PathBuf {
        self.base_dir.join(&self.rules)
    }
}

#[derive(Debug, Deserialize)]
struct WorkflowRules {
    risks: Vec<RiskRule>,
    checklist_templates: Vec<ChecklistTemplate>,
    facts_recap: FactsRecapConfig,
    tomorrow_list: TomorrowListConfig,
}

#[derive(Debug, Deserialize)]
struct RiskRule {
    id: String,
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

#[derive(Debug, Deserialize)]
struct FactsRecapConfig {
    fields: Vec<FactField>,
}

#[derive(Debug, Deserialize)]
struct FactField {
    target_path: String,
    source_path: Option<String>,
    compute: Option<ComputeRule>,
    map_array: Option<MapArrayRule>,
}

#[derive(Debug, Deserialize)]
struct ComputeRule {
    kind: String,
    value_path: String,
    baseline_path: String,
}

#[derive(Debug, Deserialize)]
struct MapArrayRule {
    source_path: String,
    fields: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct TomorrowListConfig {
    appointments: ListConfig,
    followups: ListConfig,
}

#[derive(Debug, Deserialize)]
struct ListConfig {
    source_path: String,
    max_items: Option<usize>,
    fields: HashMap<String, String>,
    priority: Option<PriorityRule>,
}

#[derive(Debug, Deserialize)]
struct PriorityRule {
    kind: String,
    source_field: Option<String>,
    true_value: Option<i64>,
    false_value: Option<i64>,
    value: Option<i64>,
}

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

pub struct GenericWorkflowRunner {
    spec: WorkflowSpec,
    thresholds: ThresholdMap,
    rules: WorkflowRules,
    name_static: &'static str,
    version_static: Option<&'static str>,
}

impl GenericWorkflowRunner {
    pub fn from_spec(spec: &WorkflowSpec) -> Result<Self, String> {
        let thresholds_content = std::fs::read_to_string(spec.thresholds_path())
            .map_err(|err| format!("read thresholds failed: {}", err))?;
        let thresholds: HashMap<String, serde_yaml::Value> =
            serde_yaml::from_str(&thresholds_content)
                .map_err(|err| format!("invalid thresholds: {}", err))?;
        let rules_content = std::fs::read_to_string(spec.rules_path())
            .map_err(|err| format!("read rules failed: {}", err))?;
        let rules: WorkflowRules = serde_yaml::from_str(&rules_content)
            .map_err(|err| format!("invalid rules: {}", err))?;
        let name_static: &'static str = Box::leak(spec.workflow_id.clone().into_boxed_str());
        let version_static: Option<&'static str> = if spec.version.is_empty() {
            None
        } else {
            Some(Box::leak(spec.version.clone().into_boxed_str()))
        };
        Ok(Self {
            spec: WorkflowSpec {
                workflow_id: spec.workflow_id.clone(),
                version: spec.version.clone(),
                input_schema: spec.input_schema.clone(),
                output_schema: spec.output_schema.clone(),
                thresholds: spec.thresholds.clone(),
                rules: spec.rules.clone(),
                base_dir: spec.base_dir.clone(),
            },
            thresholds: ThresholdMap(thresholds),
            rules,
            name_static,
            version_static,
        })
    }
}

#[async_trait::async_trait]
impl WorkflowRunner for GenericWorkflowRunner {
    fn name(&self) -> &'static str {
        self.name_static
    }

    fn version(&self) -> Option<&'static str> {
        self.version_static
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

        let facts_recap = build_facts_recap(&self.rules.facts_recap, &input);

        let (appointments, appointments_count) =
            build_list(&self.rules.tomorrow_list.appointments, &input);
        let (followups, _) = build_list(&self.rules.tomorrow_list.followups, &input);

        let tomorrow_list = json!({
            "appointments": appointments,
            "followups": followups
        });

        let wecom_touch = get_value_at_path(&input, "wecom_touch").and_then(|v| v.as_object());
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
            visits: number_or_zero(get_value_at_path(&input, "his.visits")),
            visits_avg_7d: number_or_zero(get_value_at_path(
                &input,
                "baselines.rolling_7d.visits_avg",
            )),
            gmv: number_or_zero(get_value_at_path(&input, "his.gmv")),
            gmv_avg_7d: number_or_zero(get_value_at_path(&input, "baselines.rolling_7d.gmv_avg")),
            consumption: number_or_zero(get_value_at_path(&input, "his.consumption")),
            consumption_avg_7d: number_or_zero(get_value_at_path(
                &input,
                "baselines.rolling_7d.consumption_avg",
            )),
            avg_ticket: number_or_zero(get_value_at_path(&input, "his.avg_ticket")),
            avg_ticket_avg_7d: number_or_zero(get_value_at_path(
                &input,
                "baselines.rolling_7d.avg_ticket_avg",
            )),
            mtd_gmv: number_or_zero(get_value_at_path(&input, "mtd.gmv")),
            mtd_consumption: number_or_zero(get_value_at_path(&input, "mtd.consumption")),
            mtd_gmv_target: number_or_zero(get_value_at_path(&input, "mtd.gmv_target")),
            mtd_consumption_target: number_or_zero(get_value_at_path(&input, "mtd.consumption_target")),
            mtd_time_progress: number_or_zero(get_value_at_path(&input, "mtd.time_progress")),
            target_gmv: number_or_zero(get_value_at_path(&input, "his.targets.gmv_target")),
            target_consumption: number_or_zero(get_value_at_path(
                &input,
                "his.targets.consumption_target",
            )),
            appointments_count,
            no_reply_list_len,
            contacted,
            replied,
        };

        let thresholds_replacements = self.thresholds.replacements();
        let mut risks = Vec::new();
        for rule in self.rules.risks.iter() {
            let Some(mut note_replacements) =
                evaluate_rule(rule, &metrics, no_reply_rate, &self.thresholds)
            else {
                continue;
            };
            let threshold_text = render_template(&rule.threshold, &thresholds_replacements);
            note_replacements.extend(thresholds_replacements.clone());
            let note_text = render_template(&rule.note_template, &note_replacements);
            push_risk(
                &mut risks,
                &rule.risk_type,
                threshold_text,
                &rule.evidence_fields,
                note_text,
            );
        }

        let mut checklist = Vec::new();
        let templates = &self.rules.checklist_templates;
        for risk in risks.iter().take(2) {
            let risk_type = risk
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let risk_id = risk
                .get("risk_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            if let Some(template) = find_risk_template(templates, risk_type) {
                let replacements =
                    checklist_replacements(&biz_date, Some(risk_type), appointments_count);
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
                let replacements =
                    checklist_replacements(&biz_date, None, appointments_count);
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
            let replacements = checklist_replacements(&biz_date, None, appointments_count);
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

        let wecom_touch_complete = get_value_at_path(&input, "wecom_touch").is_some();
        let mut data_quality_notes = Vec::new();
        if !wecom_touch_complete {
            data_quality_notes.push("wecom_touch missing".to_string());
        }

        let mut output = json!({
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
        let report_md =
            render_prebrief_report_md(&input, &output, appointments_count, &risks, &checklist);
        if let Value::Object(map) = &mut output {
            map.insert("report_md".to_string(), Value::String(report_md));
        }

        Ok(WorkflowOutput {
            output,
            artifacts: Vec::new(),
        })
    }
}

fn yaml_value_to_string(value: &serde_yaml::Value) -> String {
    match value {
        serde_yaml::Value::Number(number) => number.to_string(),
        serde_yaml::Value::String(text) => text.clone(),
        serde_yaml::Value::Bool(value) => value.to_string(),
        _ => String::new(),
    }
}

fn number_or_zero(value: Option<&Value>) -> f64 {
    match value {
        Some(Value::Number(number)) => number.as_f64().unwrap_or(0.0),
        Some(Value::String(text)) => text.parse::<f64>().unwrap_or(0.0),
        _ => 0.0,
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

fn set_value_at_path(value: &mut Value, path: &str, new_value: Value) {
    let mut current = value;
    let mut parts = path.split('.').peekable();
    while let Some(part) = parts.next() {
        let is_last = parts.peek().is_none();
        if is_last {
            if let Value::Object(map) = current {
                map.insert(part.to_string(), new_value);
            }
            return;
        }
        match current {
            Value::Object(map) => {
                current = map
                    .entry(part)
                    .or_insert_with(|| Value::Object(serde_json::Map::new()));
            }
            _ => return,
        }
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

fn build_facts_recap(config: &FactsRecapConfig, input: &Value) -> Value {
    let mut output = Value::Object(serde_json::Map::new());
    for field in &config.fields {
        if let Some(source_path) = &field.source_path {
            let value = number_or_zero(get_value_at_path(input, source_path));
            set_value_at_path(&mut output, &field.target_path, json!(value));
            continue;
        }
        if let Some(compute) = &field.compute {
            let value = match compute.kind.as_str() {
                "ratio" => {
                    let numerator = number_or_zero(get_value_at_path(input, &compute.value_path));
                    let denominator =
                        number_or_zero(get_value_at_path(input, &compute.baseline_path));
                    ratio(numerator, denominator)
                }
                "ratio_delta" => {
                    let numerator = number_or_zero(get_value_at_path(input, &compute.value_path));
                    let denominator =
                        number_or_zero(get_value_at_path(input, &compute.baseline_path));
                    ratio_delta(numerator, denominator)
                }
                _ => 0.0,
            };
            set_value_at_path(&mut output, &field.target_path, json!(value));
            continue;
        }
        if let Some(map_array) = &field.map_array {
            let items = get_value_at_path(input, &map_array.source_path)
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let mapped = items
                .iter()
                .filter_map(|item| item.as_object())
                .map(|item| {
                    let mut obj = serde_json::Map::new();
                    for (target_key, source_key) in map_array.fields.iter() {
                        let value = item.get(source_key).cloned().unwrap_or(Value::Null);
                        obj.insert(target_key.to_string(), value);
                    }
                    Value::Object(obj)
                })
                .collect::<Vec<_>>();
            set_value_at_path(&mut output, &field.target_path, Value::Array(mapped));
        }
    }
    output
}

fn build_list(config: &ListConfig, input: &Value) -> (Vec<Value>, usize) {
    let items = get_value_at_path(input, &config.source_path)
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let total = items.len();
    let limit = config.max_items.unwrap_or(MAX_LIST_ITEMS);
    let mapped = items
        .iter()
        .take(limit)
        .filter_map(|item| item.as_object())
        .map(|item| {
            let mut obj = serde_json::Map::new();
            for (target_key, source_key) in config.fields.iter() {
                let value = item.get(source_key).cloned().unwrap_or(Value::Null);
                obj.insert(target_key.to_string(), value);
            }
            if let Some(priority) = &config.priority {
                if let Some(value) = priority_value(priority, item) {
                    obj.insert("priority".to_string(), json!(value));
                }
            }
            Value::Object(obj)
        })
        .collect::<Vec<_>>();
    (mapped, total)
}

fn priority_value(priority: &PriorityRule, item: &serde_json::Map<String, Value>) -> Option<i64> {
    match priority.kind.as_str() {
        "fixed" => priority.value,
        "boolean_flag" => {
            let source = priority.source_field.as_deref()?;
            let flag = item.get(source).and_then(|v| v.as_bool()).unwrap_or(false);
            if flag {
                priority.true_value
            } else {
                priority.false_value
            }
        }
        _ => None,
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
    target_gmv: f64,
    target_consumption: f64,
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
            "target_gmv" => Some(self.target_gmv),
            "target_consumption" => Some(self.target_consumption),
            "contacted" => Some(self.contacted),
            "replied" => Some(self.replied),
            _ => None,
        }
    }
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

fn render_prebrief_report_md(
    input: &Value,
    output: &Value,
    appointments_count: usize,
    risks: &[Value],
    checklist: &[Value],
) -> String {
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

    let risk_label = |risk_type: &str| -> std::borrow::Cow<'static, str> {
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
    };

    let mut smart_summary = Vec::new();
    if time_progress > 0.0 && (gmv_rate > 0.0 || consumption_rate > 0.0) {
        let gap_gmv = gmv_rate - time_progress;
        let gap_cons = consumption_rate - time_progress;
        let gap_threshold = 0.05;
        if gap_gmv < -gap_threshold {
            smart_summary.push("开单完成进度落后于时间进度，需重点盯当晚可落地的补缺动作".to_string());
        }
        if gap_cons < -gap_threshold {
            smart_summary.push("消耗完成进度落后于时间进度，关注明日承接与当日消耗转化".to_string());
        }
        if gap_gmv > gap_threshold {
            smart_summary.push("开单进度领先于时间进度，继续稳态推进并关注结构质量".to_string());
        }
        if gap_cons > gap_threshold {
            smart_summary.push("消耗进度领先于时间进度，注意保持预约承接与交付效率".to_string());
        }
    }
    if avg_ticket_vs_7d <= -0.1 {
        smart_summary.push("客单价低于近7日平均，关注升单/组合项目与高客单顾客推进".to_string());
    }
    if visits_vs_7d <= -0.1 {
        smart_summary.push("到店人数低于近7日平均，关注明日预约承接与当晚邀约补量".to_string());
    }
    if risks.iter().any(|r| r.get("type").and_then(|v| v.as_str()) == Some("touch_gap")) {
        smart_summary.push("触达未回积压偏高，建议在夕会明确二触达 owner 与截止时间".to_string());
    }
    if smart_summary.is_empty() && !risks.is_empty() {
        smart_summary.push("风险已触发但缺少关键上下文，建议夕会补齐原因验证与行动分工".to_string());
    }

    let mut lines = Vec::new();
    lines.push(format!("日期：{}", biz_date));
    lines.push(format!("数据截止时间：{}", cutoff));
    lines.push(format!("门店：{}", store_name));
    lines.push(String::new());

    lines.push("## 今日经营摘要".to_string());
    lines.push(format!(
        "- 今日开单：{}（{} vs 7D均值）",
        format_currency(gmv),
        format_pct_delta(gmv_vs_7d)
    ));
    lines.push(format!(
        "- 今日消耗：{}（{} vs 7D均值）",
        format_currency(consumption),
        format_pct_delta(consumption_vs_7d)
    ));
    lines.push(format!(
        "- 今日到店人数：{}；今日客单价：{}",
        format_int_like(visits),
        format_currency(avg_ticket)
    ));
    if gmv_rate > 0.0 || consumption_rate > 0.0 {
        lines.push(format!(
            "- 月度指标完成度：开单 {}；消耗 {}",
            format_pct_ratio(gmv_rate),
            format_pct_ratio(consumption_rate)
        ));
    }
    if !smart_summary.is_empty() {
        lines.push(String::new());
        lines.push("<font color=\"RED\">智能总结</font>".to_string());
        for item in smart_summary.iter().take(4) {
            lines.push(item.to_string());
        }
    }
    lines.push(String::new());

    lines.push("## 核心风险提示".to_string());
    if risks.is_empty() {
        lines.push("- 无明显风险（按当前规则）".to_string());
    } else {
        for risk in risks.iter().take(6) {
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
    lines.push(format!("- 明日预约人数：{}", appointments_count));
    let appts = output
        .get("tomorrow_list")
        .and_then(|v| v.get("appointments"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    if appts.is_empty() {
        lines.push("- 明日预约清单：数据未同步/为空".to_string());
    } else {
        lines.push("- 明日预约清单（Top10）：".to_string());
        for (idx, item) in appts.iter().take(10).enumerate() {
            let time = item.get("time").and_then(|v| v.as_str()).unwrap_or("-");
            let customer_id = item
                .get("customer_id")
                .and_then(|v| v.as_str())
                .unwrap_or("-");
            let appt_item = item.get("item").and_then(|v| v.as_str()).unwrap_or("-");
            lines.push(format!("  {}. {} {} {}", idx + 1, time, customer_id, appt_item));
        }
    }
    lines.push(String::new());

    lines.push("## 任务执行情况（自动生成清单）".to_string());
    if checklist.is_empty() {
        lines.push("- 清单为空（需要检查 rules.yml）".to_string());
    } else {
        for item in checklist.iter().take(10) {
            let owner = item
                .get("owner_role")
                .and_then(|v| v.as_str())
                .unwrap_or("owner");
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

fn render_template(template: &str, replacements: &[(String, String)]) -> String {
    let mut rendered = template.to_string();
    for (key, value) in replacements {
        rendered = rendered.replace(&format!("{{{}}}", key), value);
    }
    rendered
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

fn checklist_replacements(
    biz_date: &str,
    risk_type: Option<&str>,
    appointments_count: usize,
) -> Vec<(String, String)> {
    let mut replacements = vec![
        ("biz_date".to_string(), biz_date.to_string()),
        (
            "appointments_count".to_string(),
            appointments_count.to_string(),
        ),
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

fn find_tomorrow_template<'a>(
    templates: &'a [ChecklistTemplate],
) -> Option<&'a ChecklistTemplate> {
    templates.iter().find(|template| template.when_tomorrow_list.unwrap_or(false))
}

fn find_fallback_template<'a>(
    templates: &'a [ChecklistTemplate],
) -> Option<&'a ChecklistTemplate> {
    templates.iter().find(|template| template.fallback.unwrap_or(false))
}
