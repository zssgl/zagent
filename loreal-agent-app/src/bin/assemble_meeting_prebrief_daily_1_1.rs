use chrono::{Duration, NaiveDate, NaiveDateTime};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde_json::{json, Value};
use sqlx::{MySqlPool, Row};

fn usage_and_exit() -> ! {
    eprintln!(
        "Usage: assemble_meeting_prebrief_daily_1_1 --biz-date YYYY-MM-DD [--store-id ID] [--store-name NAME] [--cutoff-time HH:MM]"
    );
    std::process::exit(2);
}

fn arg_value(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|v| v == name)
        .and_then(|idx| args.get(idx + 1))
        .cloned()
}

fn parse_date(text: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").ok()
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let args: Vec<String> = std::env::args().collect();
    let biz_date = arg_value(&args, "--biz-date")
        .or_else(|| arg_value(&args, "-d"))
        .and_then(|v| parse_date(&v))
        .unwrap_or_else(|| usage_and_exit());

    let store_id = arg_value(&args, "--store-id").unwrap_or_else(|| "unknown".to_string());
    let store_name = arg_value(&args, "--store-name").unwrap_or_else(|| store_id.clone());
    let cutoff_time = arg_value(&args, "--cutoff-time").unwrap_or_else(|| "未提供".to_string());

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        eprintln!("DATABASE_URL must be set (e.g. via .env)");
        std::process::exit(1);
    });
    let pool = MySqlPool::connect(&db_url)
        .await
        .unwrap_or_else(|err| {
            eprintln!("connect mysql failed: {}", err);
            std::process::exit(1);
        });

    let start = biz_date.and_hms_opt(0, 0, 0).unwrap();
    let end = start + Duration::days(1);
    let tomorrow = biz_date + Duration::days(1);
    let tomorrow_start = tomorrow.and_hms_opt(0, 0, 0).unwrap();
    let tomorrow_end = tomorrow_start + Duration::days(1);

    // NOTE: These queries are best-effort and may need adjustment to match your schema/store filter.
    let operation_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM billoperationrecords WHERE OperationTime >= ? AND OperationTime < ?",
    )
    .bind(start)
    .bind(end)
    .fetch_one(&pool)
    .await
    .unwrap_or(0);

    let payment_total: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(p.Amount), 0) \
         FROM billoperationrecordpayments p \
         JOIN billoperationrecords r ON p.RecordId = r.ID \
         WHERE r.OperationTime >= ? AND r.OperationTime < ?",
    )
    .bind(start)
    .bind(end)
    .fetch_one(&pool)
    .await
    .unwrap_or_else(|_| Decimal::ZERO);
    let payment_total = payment_total.to_f64().unwrap_or(0.0);

    let appointment_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM appointments \
         WHERE StartTime >= ? AND StartTime < ? AND IsDelete = 0",
    )
    .bind(start)
    .bind(end)
    .fetch_one(&pool)
    .await
    .unwrap_or(0);

    let appointment_rows = sqlx::query(
        "SELECT CustomerId, CustomerName, StartTime, DoctorName, ConsultantName \
         FROM appointments \
         WHERE StartTime >= ? AND StartTime < ? AND IsDelete = 0 \
         ORDER BY StartTime LIMIT 20",
    )
    .bind(tomorrow_start)
    .bind(tomorrow_end)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

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

    let visits = appointment_count.max(operation_count) as f64;
    let avg_ticket = if visits > 0.0 { payment_total / visits } else { 0.0 };

    let input: Value = json!({
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
    });

    println!("{}", serde_json::to_string_pretty(&input).unwrap());
}

