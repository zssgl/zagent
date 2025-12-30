use agent_runtime::runtime::{AgentError, WorkflowOutput, WorkflowRunner};
use agent_runtime::types::{Artifact, ArtifactType};
use chrono::{Duration, NaiveDate, NaiveDateTime, Utc};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{MySqlPool, Row};
use tokio::fs;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct DailyBriefingInput {
    category: Option<String>,
    date: Option<String>,
    source: Option<String>,
}

pub struct DailyBriefingWorkflow {
    db: MySqlPool,
}

impl DailyBriefingWorkflow {
    pub fn new(db: MySqlPool) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl WorkflowRunner for DailyBriefingWorkflow {
    fn name(&self) -> &'static str {
        "daily-briefing"
    }

    fn version(&self) -> Option<&'static str> {
        Some("0.1.0")
    }

    async fn run(&self, input: Value) -> Result<WorkflowOutput, AgentError> {
        let parsed: DailyBriefingInput =
            serde_json::from_value(input).map_err(|err| AgentError::Fatal(err.to_string()))?;

        let category = parsed
            .category
            .unwrap_or_else(|| "daily".to_string())
            .to_ascii_lowercase();
        if category == "weekly" {
            return Err(AgentError::Fatal(
                "weekly category is not supported yet".to_string(),
            ));
        }
        if category != "daily" {
            return Err(AgentError::Fatal(
                "category must be daily or weekly".to_string(),
            ));
        }

        let date = parsed
            .date
            .as_deref()
            .and_then(parse_date)
            .unwrap_or_else(|| Utc::now().date_naive());
        let start = date
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| AgentError::Fatal("invalid date".to_string()))?;
        let end = start + Duration::days(1);

        let tomorrow = date + Duration::days(1);
        let tomorrow_start = tomorrow
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| AgentError::Fatal("invalid date".to_string()))?;
        let tomorrow_end = tomorrow_start + Duration::days(1);

        let operation_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM billoperationrecords WHERE OperationTime >= ? AND OperationTime < ?",
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.db)
        .await
        .map_err(|err| AgentError::Retryable(format!("db error: {}", err)))?;

        let payment_total: Decimal = sqlx::query_scalar(
            "SELECT COALESCE(SUM(p.Amount), 0) \
             FROM billoperationrecordpayments p \
             JOIN billoperationrecords r ON p.RecordId = r.ID \
             WHERE r.OperationTime >= ? AND r.OperationTime < ?",
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.db)
        .await
        .map_err(|err| AgentError::Retryable(format!("db error: {}", err)))?;
        let payment_total = payment_total.to_f64().unwrap_or(0.0);

        let appointment_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM appointments \
             WHERE StartTime >= ? AND StartTime < ? AND IsDelete = 0",
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.db)
        .await
        .map_err(|err| AgentError::Retryable(format!("db error: {}", err)))?;

        let return_visit_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM returnvisits \
             WHERE ReturnVisitDate >= ? AND ReturnVisitDate < ?",
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.db)
        .await
        .map_err(|err| AgentError::Retryable(format!("db error: {}", err)))?;

        let wecom_trace_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM customer_trace \
             WHERE trace_day = ? AND is_delete = 0",
        )
        .bind(date)
        .fetch_one(&self.db)
        .await
        .map_err(|err| AgentError::Retryable(format!("db error: {}", err)))?;

        let appointment_rows = sqlx::query(
            "SELECT CustomerName, StartTime, DoctorName, ConsultantName \
             FROM appointments \
             WHERE StartTime >= ? AND StartTime < ? AND IsDelete = 0 \
             ORDER BY StartTime LIMIT 20",
        )
        .bind(tomorrow_start)
        .bind(tomorrow_end)
        .fetch_all(&self.db)
        .await
        .map_err(|err| AgentError::Retryable(format!("db error: {}", err)))?;

        let mut tomorrow_list = Vec::new();
        for row in appointment_rows {
            let name: Option<String> = row.try_get("CustomerName").unwrap_or(None);
            let start_time: Option<NaiveDateTime> = row.try_get("StartTime").unwrap_or(None);
            let doctor: Option<String> = row.try_get("DoctorName").unwrap_or(None);
            let consultant: Option<String> = row.try_get("ConsultantName").unwrap_or(None);

            tomorrow_list.push(json!({
                "customer_name": name.unwrap_or_else(|| "未知客户".to_string()),
                "time": start_time.map(|t| t.format("%H:%M").to_string()),
                "doctor": doctor,
                "consultant": consultant,
            }));
        }

        let mut risks = Vec::new();
        if appointment_count == 0 {
            risks.push("今日预约为 0，需排查获客/预约渠道".to_string());
        }
        if payment_total <= 0.0 {
            risks.push("今日支付金额为 0，需核对账务与支付记录".to_string());
        }
        if return_visit_count == 0 && wecom_trace_count == 0 {
            risks.push("今日无回访/企微跟进记录，需补齐客户触达".to_string());
        }
        if risks.is_empty() {
            risks.push("暂无明显风险".to_string());
        }

        let checklist = vec![
            "核对明日预约客户名单并逐一确认到诊".to_string(),
            "复盘今日未完成回访并安排补回".to_string(),
            "同步关键客户需求和重点风险点".to_string(),
        ];

        let report_md = render_report(
            date,
            operation_count,
            payment_total,
            appointment_count,
            return_visit_count,
            wecom_trace_count,
            &tomorrow_list,
            &risks,
            &checklist,
        );

        let report_dir = "reports";
        fs::create_dir_all(report_dir)
            .await
            .map_err(|err| AgentError::Fatal(format!("create report dir: {}", err)))?;
        let report_path = format!("{}/briefing_{}.md", report_dir, date.format("%Y%m%d"));
        fs::write(&report_path, report_md.as_bytes())
            .await
            .map_err(|err| AgentError::Fatal(format!("write report: {}", err)))?;

        let output = json!({
            "date": date.format("%Y-%m-%d").to_string(),
            "category": "daily",
            "source": parsed.source.unwrap_or_else(|| "HIS".to_string()),
            "facts_recap": {
                "operation_count": operation_count,
                "payment_total": payment_total,
                "appointment_count": appointment_count,
                "return_visit_count": return_visit_count,
                "wecom_trace_count": wecom_trace_count,
            },
            "tomorrow_customers": tomorrow_list,
            "risks": risks,
            "checklist": checklist,
            "report_path": report_path,
        });

        let artifact = Artifact {
            artifact_id: format!("art_{}", Uuid::new_v4()),
            r#type: ArtifactType::File,
            name: Some("daily-briefing".to_string()),
            created_at: Utc::now(),
            mime_type: Some("text/markdown".to_string()),
            data: Some(json!({ "path": report_path })),
            file: None,
        };

        Ok(WorkflowOutput {
            output,
            artifacts: vec![artifact],
        })
    }
}

fn parse_date(input: &str) -> Option<NaiveDate> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }
    NaiveDate::parse_from_str(input, "%Y-%m-%d")
        .ok()
        .or_else(|| NaiveDate::parse_from_str(input, "%Y/%m/%d").ok())
        .or_else(|| NaiveDate::parse_from_str(input, "%Y%m%d").ok())
}

fn render_report(
    date: NaiveDate,
    operation_count: i64,
    payment_total: f64,
    appointment_count: i64,
    return_visit_count: i64,
    wecom_trace_count: i64,
    tomorrow_list: &[Value],
    risks: &[String],
    checklist: &[String],
) -> String {
    let mut lines = Vec::new();
    lines.push(format!("# 夕会数据简报 ({})", date.format("%Y-%m-%d")));
    lines.push("".to_string());
    lines.push("## Facts Recap".to_string());
    lines.push(format!(
        "- 当日经营：操作记录 {} 笔，支付金额 {:.2}",
        operation_count, payment_total
    ));
    lines.push(format!("- 预约：{} 笔", appointment_count));
    lines.push(format!(
        "- 回访：{} 条；企微跟进：{} 条",
        return_visit_count, wecom_trace_count
    ));
    lines.push("".to_string());
    lines.push("## 明日客户清单".to_string());
    if tomorrow_list.is_empty() {
        lines.push("- 暂无预约".to_string());
    } else {
        for item in tomorrow_list {
            let name = item
                .get("customer_name")
                .and_then(|v| v.as_str())
                .unwrap_or("未知客户");
            let time = item.get("time").and_then(|v| v.as_str()).unwrap_or("--:--");
            let doctor = item.get("doctor").and_then(|v| v.as_str()).unwrap_or("-");
            let consultant = item.get("consultant").and_then(|v| v.as_str()).unwrap_or("-");
            lines.push(format!(
                "- {} {}（医生：{} / 咨询：{}）",
                time, name, doctor, consultant
            ));
        }
    }
    lines.push("".to_string());
    lines.push("## 风险提示".to_string());
    for risk in risks {
        lines.push(format!("- {}", risk));
    }
    lines.push("".to_string());
    lines.push("## 执行 checklist".to_string());
    for item in checklist {
        lines.push(format!("- {}", item));
    }
    lines.join("\n")
}
