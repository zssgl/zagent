use chrono::{Duration, NaiveDate, NaiveDateTime};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde_json::{json, Value};
use sqlx::{MySqlPool, Row};

#[derive(Debug, thiserror::Error)]
pub enum AssembleError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("db error: {0}")]
    Db(String),
}

fn parse_date(text: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").ok()
}

fn string_field(input: &Value, path: &str) -> Option<String> {
    input.get(path).and_then(|v| v.as_str()).map(|v| v.to_string())
}

pub fn merge_json(base: &mut Value, overlay: &Value) {
    match (base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            for (k, v) in overlay_map {
                match base_map.get_mut(k) {
                    Some(existing) => merge_json(existing, v),
                    None => {
                        base_map.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        (base_slot, overlay_value) => {
            *base_slot = overlay_value.clone();
        }
    }
}

pub async fn assemble_meeting_prebrief_daily_1_1_mysql(
    pool: &MySqlPool,
    minimal_input: &Value,
) -> Result<Value, AssembleError> {
    let biz_date = string_field(minimal_input, "biz_date")
        .and_then(|v| parse_date(&v))
        .ok_or_else(|| AssembleError::InvalidInput("missing/invalid biz_date (YYYY-MM-DD)".into()))?;

    let store_id = string_field(minimal_input, "store_id")
        .ok_or_else(|| AssembleError::InvalidInput("missing store_id".into()))?;
    let store_name = string_field(minimal_input, "store_name").unwrap_or_else(|| store_id.clone());
    let cutoff_time = string_field(minimal_input, "data_cutoff_time").unwrap_or_else(|| "未提供".into());

    let start = biz_date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| AssembleError::InvalidInput("invalid biz_date".into()))?;
    let end = start + Duration::days(1);

    let tomorrow = biz_date + Duration::days(1);
    let tomorrow_start = tomorrow
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| AssembleError::InvalidInput("invalid biz_date".into()))?;
    let tomorrow_end = tomorrow_start + Duration::days(1);

    // NOTE: Best-effort queries. Adjust store filters/field mapping once you confirm schema.
    let operation_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM billoperationrecords WHERE OperationTime >= ? AND OperationTime < ?",
    )
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await
    .map_err(|err| AssembleError::Db(err.to_string()))?;

    let payment_total: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(p.Amount), 0) \
         FROM billoperationrecordpayments p \
         JOIN billoperationrecords r ON p.RecordId = r.ID \
         WHERE r.OperationTime >= ? AND r.OperationTime < ?",
    )
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await
    .map_err(|err| AssembleError::Db(err.to_string()))?;
    let payment_total = payment_total.to_f64().unwrap_or(0.0);

    let appointment_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM appointments \
         WHERE StartTime >= ? AND StartTime < ? AND IsDelete = 0",
    )
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await
    .map_err(|err| AssembleError::Db(err.to_string()))?;

    let appointment_rows = sqlx::query(
        "SELECT CustomerId, CustomerName, StartTime, DoctorName, ConsultantName \
         FROM appointments \
         WHERE StartTime >= ? AND StartTime < ? AND IsDelete = 0 \
         ORDER BY StartTime LIMIT 20",
    )
    .bind(tomorrow_start)
    .bind(tomorrow_end)
    .fetch_all(pool)
    .await
    .map_err(|err| AssembleError::Db(err.to_string()))?;

    let mut appointments_tomorrow = Vec::new();
    for row in appointment_rows {
        let customer_id: Option<String> = row
            .try_get("CustomerId")
            .ok()
            .flatten()
            .or_else(|| row.try_get::<Option<String>, _>("CustomerName").ok().flatten());
        let start_time: Option<NaiveDateTime> = row.try_get("StartTime").ok().flatten();
        let doctor: Option<String> = row.try_get("DoctorName").ok().flatten();
        let consultant: Option<String> = row.try_get("ConsultantName").ok().flatten();

        let staff_id = consultant
            .clone()
            .or(doctor.clone())
            .unwrap_or_else(|| "unknown".to_string());

        appointments_tomorrow.push(json!({
            "customer_id": customer_id.unwrap_or_else(|| "unknown".to_string()),
            "time": start_time.map(|t| t.format("%Y-%m-%d %H:%M").to_string()).unwrap_or_else(|| format!("{} 00:00", tomorrow.format("%Y-%m-%d"))),
            "item": "未提供",
            "staff_id": staff_id,
            "is_first_visit": false
        }));
    }

    // Temporary heuristic until store-scoped HIS tables are integrated.
    let visits = appointment_count.max(operation_count) as f64;
    let avg_ticket = if visits > 0.0 { payment_total / visits } else { 0.0 };

    Ok(json!({
        "store_id": store_id,
        "store_name": store_name,
        "biz_date": biz_date.format("%Y-%m-%d").to_string(),
        "data_cutoff_time": cutoff_time,
        "his": {
            "visits": visits,
            "gmv": payment_total,
            "consumption": payment_total,
            "avg_ticket": avg_ticket,
            "new_customers": 0,
            "old_customers": 0
        },
        "appointments_tomorrow": appointments_tomorrow
    }))
}

