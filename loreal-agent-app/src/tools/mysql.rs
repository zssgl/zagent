use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde_json::{json, Value};
use sqlx::{MySqlPool, Row};

#[derive(Debug, thiserror::Error)]
pub enum MysqlAssembleError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("db error: {0}")]
    Db(String),
}

fn parse_date(text: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").ok()
}

fn string_field(input: &Value, key: &str) -> Option<String> {
    input.get(key).and_then(|v| v.as_str()).map(|v| v.to_string())
}

fn number_or_zero(value: Option<&Value>) -> f64 {
    match value {
        Some(Value::Number(number)) => number.as_f64().unwrap_or(0.0),
        Some(Value::String(text)) => text.parse::<f64>().unwrap_or(0.0),
        _ => 0.0,
    }
}

fn start_of_month(date: NaiveDate) -> NaiveDate {
    date.with_day(1).unwrap_or(date)
}

fn days_in_month(date: NaiveDate) -> u32 {
    let first = start_of_month(date);
    let next_month = if first.month() == 12 {
        NaiveDate::from_ymd_opt(first.year() + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(first.year(), first.month() + 1, 1)
    }
    .unwrap_or(first);
    (next_month - Duration::days(1)).day()
}

pub async fn assemble_meeting_prebrief_daily_1_1_mysql(
    pool: &MySqlPool,
    minimal_input: &Value,
) -> Result<Value, MysqlAssembleError> {
    let biz_date = string_field(minimal_input, "biz_date")
        .and_then(|v| parse_date(&v))
        .ok_or_else(|| MysqlAssembleError::InvalidInput("missing/invalid biz_date (YYYY-MM-DD)".into()))?;

    let store_id = string_field(minimal_input, "store_id")
        .ok_or_else(|| MysqlAssembleError::InvalidInput("missing store_id".into()))?;
    let store_name = match string_field(minimal_input, "store_name") {
        Some(name) => name,
        None => {
            let name: Option<String> = sqlx::query_scalar("SELECT ClinicName FROM clinics WHERE ID = ? AND IsDeleted = 0")
                .bind(&store_id)
                .fetch_optional(pool)
                .await
                .map_err(|err| MysqlAssembleError::Db(err.to_string()))?;
            name.unwrap_or_else(|| store_id.clone())
        }
    };
    let cutoff_time =
        string_field(minimal_input, "data_cutoff_time").unwrap_or_else(|| "未提供".into());

    let start = biz_date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| MysqlAssembleError::InvalidInput("invalid biz_date".into()))?;
    let end = start + Duration::days(1);

    let tomorrow = biz_date + Duration::days(1);
    let tomorrow_start = tomorrow
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| MysqlAssembleError::InvalidInput("invalid biz_date".into()))?;
    let tomorrow_end = tomorrow_start + Duration::days(1);

    // ---------- Today (HIS-like facts) ----------
    // GMV: use bills.PayAmount scoped by ClinicId + CreateTime.
    let today_gmv: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(PayAmount), 0) \
         FROM bills \
         WHERE ClinicId = ? AND CreateTime >= ? AND CreateTime < ? AND IsRefund = 0",
    )
    .bind(&store_id)
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await
    .map_err(|err| MysqlAssembleError::Db(err.to_string()))?;
    let today_gmv = today_gmv.to_f64().unwrap_or(0.0);

    let today_customers: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT Customer_ID) \
         FROM bills \
         WHERE ClinicId = ? AND CreateTime >= ? AND CreateTime < ? AND IsRefund = 0",
    )
    .bind(&store_id)
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await
    .map_err(|err| MysqlAssembleError::Db(err.to_string()))?;
    let today_visits = today_customers.max(0) as f64;
    let today_avg_ticket = if today_visits > 0.0 {
        today_gmv / today_visits
    } else {
        0.0
    };

    // Top items (today) from bill operation record items.
    let top_item_rows = sqlx::query(
        "SELECT i.ItemName AS item, COALESCE(SUM(i.PaymentAmount), 0) AS amount \
         FROM billoperationrecorditems i \
         JOIN billoperationrecords r ON i.RecordId = r.ID \
         JOIN bills b ON r.BillId = b.ID \
         WHERE b.ClinicId = ? AND r.OperationTime >= ? AND r.OperationTime < ? AND b.IsRefund = 0 \
         GROUP BY i.ItemName \
         ORDER BY amount DESC \
         LIMIT 3",
    )
    .bind(&store_id)
    .bind(start)
    .bind(end)
    .fetch_all(pool)
    .await
    .map_err(|err| MysqlAssembleError::Db(err.to_string()))?;
    let mut top_items = Vec::new();
    for row in top_item_rows {
        let item: Option<String> = row.try_get("item").ok().flatten();
        let amount: Option<Decimal> = row.try_get("amount").ok().flatten();
        top_items.push(json!({
            "item": item.unwrap_or_else(|| "未知品项".to_string()),
            "amount": amount.and_then(|v| v.to_f64()).unwrap_or(0.0)
        }));
    }

    // ---------- Tomorrow appointments ----------
    let today_appointments_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM appointments \
         WHERE OrginizationId = ? AND StartTime >= ? AND StartTime < ? AND IsDelete = 0",
    )
    .bind(&store_id)
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await
    .map_err(|err| MysqlAssembleError::Db(err.to_string()))?;

    let appointment_rows = sqlx::query(
        "SELECT a.CustomerId, a.CustomerName, a.StartTime, a.DoctorName, a.ConsultantName, l.ItemName \
         FROM appointments a \
         LEFT JOIN appointmentlines l ON l.AppoinmentId = a.ID \
         WHERE a.OrginizationId = ? AND a.StartTime >= ? AND a.StartTime < ? AND a.IsDelete = 0 \
         ORDER BY a.StartTime LIMIT 20",
    )
    .bind(&store_id)
    .bind(tomorrow_start)
    .bind(tomorrow_end)
    .fetch_all(pool)
    .await
    .map_err(|err| MysqlAssembleError::Db(err.to_string()))?;

    let mut appointments_tomorrow = Vec::new();
    for row in appointment_rows {
        let customer_id: Option<String> = row.try_get("CustomerId").ok().flatten();
        let start_time: Option<NaiveDateTime> = row.try_get("StartTime").ok().flatten();
        let doctor: Option<String> = row.try_get("DoctorName").ok().flatten();
        let consultant: Option<String> = row.try_get("ConsultantName").ok().flatten();
        let item_name: Option<String> = row.try_get("ItemName").ok().flatten();

        let staff_id = consultant
            .clone()
            .or(doctor.clone())
            .unwrap_or_else(|| "unknown".to_string());

        appointments_tomorrow.push(json!({
            "customer_id": customer_id.unwrap_or_else(|| "unknown".to_string()),
            "time": start_time.map(|t| t.format("%Y-%m-%d %H:%M").to_string()).unwrap_or_else(|| format!("{} 00:00", tomorrow.format("%Y-%m-%d"))),
            "item": item_name.unwrap_or_else(|| "未提供".to_string()),
            "staff_id": staff_id,
            "is_first_visit": false
        }));
    }

    // ---------- Baselines (rolling 7d avg) ----------
    let baseline_start = (biz_date - Duration::days(7))
        .and_hms_opt(0, 0, 0)
        .unwrap_or(start);
    let baseline_end = start;
    let baseline_avg_rows = sqlx::query(
        "SELECT \
            COALESCE(AVG(day_gmv), 0) AS gmv_avg, \
            COALESCE(AVG(day_visits), 0) AS visits_avg \
         FROM ( \
           SELECT DATE(CreateTime) AS d, \
                  COALESCE(SUM(PayAmount), 0) AS day_gmv, \
                  COUNT(DISTINCT Customer_ID) AS day_visits \
           FROM bills \
           WHERE ClinicId = ? AND CreateTime >= ? AND CreateTime < ? AND IsRefund = 0 \
           GROUP BY DATE(CreateTime) \
         ) t",
    )
    .bind(&store_id)
    .bind(baseline_start)
    .bind(baseline_end)
    .fetch_one(pool)
    .await
    .map_err(|err| MysqlAssembleError::Db(err.to_string()))?;
    let gmv_avg_7d: f64 = baseline_avg_rows
        .try_get::<Option<Decimal>, _>("gmv_avg")
        .ok()
        .flatten()
        .and_then(|v| v.to_f64())
        .unwrap_or(0.0);
    let visits_avg_7d: f64 = baseline_avg_rows
        .try_get::<Option<f64>, _>("visits_avg")
        .ok()
        .flatten()
        .unwrap_or(0.0);
    let avg_ticket_avg_7d = if visits_avg_7d > 0.0 {
        gmv_avg_7d / visits_avg_7d
    } else {
        0.0
    };

    // ---------- MTD totals ----------
    let month_start = start_of_month(biz_date).and_hms_opt(0, 0, 0).unwrap();
    let month_end = end;
    let mtd_gmv: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(PayAmount), 0) \
         FROM bills \
         WHERE ClinicId = ? AND CreateTime >= ? AND CreateTime < ? AND IsRefund = 0",
    )
    .bind(&store_id)
    .bind(month_start)
    .bind(month_end)
    .fetch_one(pool)
    .await
    .map_err(|err| MysqlAssembleError::Db(err.to_string()))?;
    let mtd_gmv = mtd_gmv.to_f64().unwrap_or(0.0);
    let mtd_consumption = mtd_gmv;

    let time_progress = {
        let day = biz_date.day() as f64;
        let total = days_in_month(biz_date) as f64;
        if total > 0.0 { day / total } else { 0.0 }
    };

    // ---------- Staff stats (best-effort by bill employees) ----------
    let staff_today_rows = sqlx::query(
        "SELECT COALESCE(e.EmpName, be.EmpId) AS staff_name, COALESCE(SUM(b.PayAmount), 0) AS today_gmv \
         FROM bills b \
         JOIN billemployees be ON be.BillId = b.ID \
         LEFT JOIN employees e ON e.ID = be.EmpId \
         WHERE b.ClinicId = ? AND b.CreateTime >= ? AND b.CreateTime < ? AND b.IsRefund = 0 \
         GROUP BY COALESCE(e.EmpName, be.EmpId) \
         ORDER BY today_gmv DESC \
         LIMIT 10",
    )
    .bind(&store_id)
    .bind(start)
    .bind(end)
    .fetch_all(pool)
    .await
    .map_err(|err| MysqlAssembleError::Db(err.to_string()))?;

    let staff_mtd_rows = sqlx::query(
        "SELECT COALESCE(e.EmpName, be.EmpId) AS staff_name, COALESCE(SUM(b.PayAmount), 0) AS mtd_gmv \
         FROM bills b \
         JOIN billemployees be ON be.BillId = b.ID \
         LEFT JOIN employees e ON e.ID = be.EmpId \
         WHERE b.ClinicId = ? AND b.CreateTime >= ? AND b.CreateTime < ? AND b.IsRefund = 0 \
         GROUP BY COALESCE(e.EmpName, be.EmpId)",
    )
    .bind(&store_id)
    .bind(month_start)
    .bind(month_end)
    .fetch_all(pool)
    .await
    .map_err(|err| MysqlAssembleError::Db(err.to_string()))?;

    let (staff_today_rows, staff_mtd_rows) = if staff_today_rows.is_empty() && staff_mtd_rows.is_empty()
    {
        let fallback_today_rows = sqlx::query(
            "SELECT COALESCE(e.EmpName, b.CreateEmpId) AS staff_name, COALESCE(SUM(b.PayAmount), 0) AS today_gmv \
             FROM bills b \
             LEFT JOIN employees e ON e.ID = b.CreateEmpId \
             WHERE b.ClinicId = ? AND b.CreateTime >= ? AND b.CreateTime < ? AND b.IsRefund = 0 \
             GROUP BY COALESCE(e.EmpName, b.CreateEmpId) \
             ORDER BY today_gmv DESC \
             LIMIT 10",
        )
        .bind(&store_id)
        .bind(start)
        .bind(end)
        .fetch_all(pool)
        .await
        .map_err(|err| MysqlAssembleError::Db(err.to_string()))?;

        let fallback_mtd_rows = sqlx::query(
            "SELECT COALESCE(e.EmpName, b.CreateEmpId) AS staff_name, COALESCE(SUM(b.PayAmount), 0) AS mtd_gmv \
             FROM bills b \
             LEFT JOIN employees e ON e.ID = b.CreateEmpId \
             WHERE b.ClinicId = ? AND b.CreateTime >= ? AND b.CreateTime < ? AND b.IsRefund = 0 \
             GROUP BY COALESCE(e.EmpName, b.CreateEmpId)",
        )
        .bind(&store_id)
        .bind(month_start)
        .bind(month_end)
        .fetch_all(pool)
        .await
        .map_err(|err| MysqlAssembleError::Db(err.to_string()))?;

        (fallback_today_rows, fallback_mtd_rows)
    } else {
        (staff_today_rows, staff_mtd_rows)
    };

    let mut staff_map: std::collections::HashMap<String, serde_json::Map<String, Value>> =
        std::collections::HashMap::new();
    for row in staff_mtd_rows {
        let name: Option<String> = row.try_get("staff_name").ok().flatten();
        let mtd: Option<Decimal> = row.try_get("mtd_gmv").ok().flatten();
        let name = name.unwrap_or_else(|| "未知".to_string());
        let entry = staff_map.entry(name.clone()).or_default();
        entry.insert(
            "staff_name".to_string(),
            Value::String(name),
        );
        entry.insert(
            "mtd_gmv".to_string(),
            json!(mtd.and_then(|v| v.to_f64()).unwrap_or(0.0)),
        );
    }
    for row in staff_today_rows {
        let name: Option<String> = row.try_get("staff_name").ok().flatten();
        let today: Option<Decimal> = row.try_get("today_gmv").ok().flatten();
        let name = name.unwrap_or_else(|| "未知".to_string());
        let entry = staff_map.entry(name.clone()).or_default();
        entry.insert(
            "staff_name".to_string(),
            Value::String(name),
        );
        entry.insert(
            "today_gmv".to_string(),
            json!(today.and_then(|v| v.to_f64()).unwrap_or(0.0)),
        );
    }
    let staff_stats = staff_map
        .into_values()
        .map(|mut obj| {
            obj.entry("today_gmv".to_string()).or_insert(json!(0.0));
            obj.entry("mtd_gmv".to_string()).or_insert(json!(0.0));
            obj.entry("today_consumption".to_string())
                .or_insert(json!(0.0));
            obj.entry("mtd_consumption".to_string())
                .or_insert(json!(0.0));
            obj.entry("r12_rate".to_string()).or_insert(json!(0.0));
            Value::Object(obj)
        })
        .collect::<Vec<_>>();

    // ---------- Customer summary (new vs old, based on first bill date in this clinic) ----------
    let first_bill_rows = sqlx::query(
        "SELECT d.Customer_ID AS customer_id, d.day_gmv AS day_gmv, f.first_time AS first_time \
         FROM ( \
           SELECT Customer_ID, COALESCE(SUM(PayAmount), 0) AS day_gmv \
           FROM bills \
           WHERE ClinicId = ? AND CreateTime >= ? AND CreateTime < ? AND IsRefund = 0 \
           GROUP BY Customer_ID \
         ) d \
         JOIN ( \
           SELECT Customer_ID, MIN(CreateTime) AS first_time \
           FROM bills \
           WHERE ClinicId = ? AND IsRefund = 0 \
           GROUP BY Customer_ID \
         ) f ON f.Customer_ID = d.Customer_ID",
    )
    .bind(&store_id)
    .bind(start)
    .bind(end)
    .bind(&store_id)
    .fetch_all(pool)
    .await
    .map_err(|err| MysqlAssembleError::Db(err.to_string()))?;

    let mut new_count = 0.0;
    let mut old_count = 0.0;
    let mut new_gmv = 0.0;
    let mut old_gmv = 0.0;
    for row in first_bill_rows {
        let first_time: Option<NaiveDateTime> = row.try_get("first_time").ok().flatten();
        let day_gmv: Option<Decimal> = row.try_get("day_gmv").ok().flatten();
        let day_gmv = day_gmv.and_then(|v| v.to_f64()).unwrap_or(0.0);
        if first_time.map(|t| t.date()) == Some(biz_date) {
            new_count += 1.0;
            new_gmv += day_gmv;
        } else {
            old_count += 1.0;
            old_gmv += day_gmv;
        }
    }

    // New customer sources (best-effort).
    let new_source_rows = sqlx::query(
        "SELECT COALESCE(cd.DisplayName, '未知') AS source, COUNT(*) AS cnt, COALESCE(SUM(d.day_gmv), 0) AS gmv \
         FROM ( \
           SELECT Customer_ID, COALESCE(SUM(PayAmount), 0) AS day_gmv \
           FROM bills \
           WHERE ClinicId = ? AND CreateTime >= ? AND CreateTime < ? AND IsRefund = 0 \
           GROUP BY Customer_ID \
         ) d \
         JOIN ( \
           SELECT Customer_ID, MIN(CreateTime) AS first_time \
           FROM bills \
           WHERE ClinicId = ? AND IsRefund = 0 \
           GROUP BY Customer_ID \
         ) f ON f.Customer_ID = d.Customer_ID AND DATE(f.first_time) = ? \
         JOIN customers c ON c.ID = d.Customer_ID \
         LEFT JOIN customdictionary cd ON cd.ID = c.LaiYuanID \
         GROUP BY COALESCE(cd.DisplayName, '未知') \
         ORDER BY cnt DESC \
         LIMIT 10",
    )
    .bind(&store_id)
    .bind(start)
    .bind(end)
    .bind(&store_id)
    .bind(biz_date)
    .fetch_all(pool)
    .await
    .map_err(|err| MysqlAssembleError::Db(err.to_string()))?;
    let mut new_sources = Vec::new();
    for row in new_source_rows {
        let source: Option<String> = row.try_get("source").ok().flatten();
        let cnt: Option<i64> = row.try_get("cnt").ok().flatten();
        let gmv: Option<Decimal> = row.try_get("gmv").ok().flatten();
        new_sources.push(json!({
            "source": source.unwrap_or_else(|| "未知".to_string()),
            "count": (cnt.unwrap_or(0) as f64),
            "gmv": gmv.and_then(|v| v.to_f64()).unwrap_or(0.0)
        }));
    }

    // Single-item customers among today's visitors (best-effort via distinct ItemName in last 12 months).
    let single_item_customers: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ( \
           SELECT b.Customer_ID \
           FROM bills b \
           JOIN billoperationrecords r ON r.BillId = b.ID \
           JOIN billoperationrecorditems i ON i.RecordId = r.ID \
           WHERE b.ClinicId = ? AND b.IsRefund = 0 \
             AND r.OperationTime >= ? AND r.OperationTime < ? \
             AND b.Customer_ID IN ( \
               SELECT DISTINCT Customer_ID \
               FROM bills \
               WHERE ClinicId = ? AND CreateTime >= ? AND CreateTime < ? AND IsRefund = 0 \
             ) \
           GROUP BY b.Customer_ID \
           HAVING COUNT(DISTINCT i.ItemName) = 1 \
         ) t",
    )
    .bind(&store_id)
    .bind(start - Duration::days(365))
    .bind(end)
    .bind(&store_id)
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await
    .map_err(|err| MysqlAssembleError::Db(err.to_string()))?;

    // VIP customers among today's visitors (best-effort via latest customer_level_historys.new_level LIKE '%VIP%').
    let vip_customers: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ( \
           SELECT c.Customer_ID \
           FROM ( \
             SELECT DISTINCT Customer_ID \
             FROM bills \
             WHERE ClinicId = ? AND CreateTime >= ? AND CreateTime < ? AND IsRefund = 0 \
           ) c \
           JOIN ( \
             SELECT customer_id, MAX(create_time) AS last_time \
             FROM customer_level_historys \
             GROUP BY customer_id \
           ) last ON last.customer_id = c.Customer_ID \
           JOIN customer_level_historys h \
             ON h.customer_id = last.customer_id AND h.create_time = last.last_time \
           WHERE h.new_level LIKE '%VIP%' \
         ) t",
    )
    .bind(&store_id)
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await
    .map_err(|err| MysqlAssembleError::Db(err.to_string()))?;

    let customer_summary = json!({
        "new": { "count": new_count, "gmv": new_gmv, "sources": new_sources },
        "old": { "count": old_count, "gmv": old_gmv },
        "single_item_customers": (single_item_customers.max(0) as f64),
        "vip_customers": (vip_customers.max(0) as f64)
    });

    // ---------- Key items (MTD) ----------
    let key_item_rows = sqlx::query(
        "SELECT i.ItemName AS item, COALESCE(SUM(i.PaymentAmount), 0) AS amount \
         FROM billoperationrecorditems i \
         JOIN billoperationrecords r ON i.RecordId = r.ID \
         JOIN bills b ON r.BillId = b.ID \
         WHERE b.ClinicId = ? AND r.OperationTime >= ? AND r.OperationTime < ? AND b.IsRefund = 0 \
         GROUP BY i.ItemName \
         ORDER BY amount DESC \
         LIMIT 10",
    )
    .bind(&store_id)
    .bind(month_start)
    .bind(month_end)
    .fetch_all(pool)
    .await
    .map_err(|err| MysqlAssembleError::Db(err.to_string()))?;
    let mut key_items_mtd = Vec::new();
    for row in key_item_rows {
        let item: Option<String> = row.try_get("item").ok().flatten();
        let amount: Option<Decimal> = row.try_get("amount").ok().flatten();
        let amount = amount.and_then(|v| v.to_f64()).unwrap_or(0.0);
        key_items_mtd.push(json!({
            "item": item.unwrap_or_else(|| "未知品项".to_string()),
            "gmv_mtd": amount,
            "consumption_mtd": amount,
            "wow_gmv": 0.0,
            "wow_consumption": 0.0
        }));
    }

    // ---------- Task execution (best-effort) ----------
    let photos_customers: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT CUSTOMER_ID) \
         FROM operation_photo \
         WHERE ORGANIZATION_ID = ? AND CREATED_DATE >= ? AND CREATED_DATE < ?",
    )
    .bind(&store_id)
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await
    .map_err(|err| MysqlAssembleError::Db(err.to_string()))?;

    let emr_customers: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT CustomerId) \
         FROM emrs \
         WHERE OrganizationId = ? AND EmrDate >= ? AND EmrDate < ? AND IsDeleted = 0",
    )
    .bind(&store_id)
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await
    .map_err(|err| MysqlAssembleError::Db(err.to_string()))?;

    let prescription_customers: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT Customer_ID) \
         FROM prescriptions \
         WHERE ClinicId = ? AND CreateTime >= ? AND CreateTime < ?",
    )
    .bind(&store_id)
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await
    .map_err(|err| MysqlAssembleError::Db(err.to_string()))?;

    let followup_planned: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM returnvisits WHERE ClinicId = ? AND ReturnVisitDate >= ? AND ReturnVisitDate < ?",
    )
    .bind(&store_id)
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await
    .map_err(|err| MysqlAssembleError::Db(err.to_string()))?;
    let followup_done: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM returnvisits WHERE ClinicId = ? AND DoneReturnVisitDate >= ? AND DoneReturnVisitDate < ?",
    )
    .bind(&store_id)
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await
    .map_err(|err| MysqlAssembleError::Db(err.to_string()))?;

    let missing_photo_rows = sqlx::query(
        "SELECT DISTINCT b.Customer_ID AS customer_id \
         FROM bills b \
         LEFT JOIN operation_photo p \
           ON p.CUSTOMER_ID = b.Customer_ID \
          AND p.ORGANIZATION_ID = ? \
          AND p.CREATED_DATE >= ? AND p.CREATED_DATE < ? \
         WHERE b.ClinicId = ? AND b.CreateTime >= ? AND b.CreateTime < ? AND b.IsRefund = 0 \
           AND p.ID IS NULL \
         LIMIT 10",
    )
    .bind(&store_id)
    .bind(start)
    .bind(end)
    .bind(&store_id)
    .bind(start)
    .bind(end)
    .fetch_all(pool)
    .await
    .map_err(|err| MysqlAssembleError::Db(err.to_string()))?;
    let mut missing_photo_list = Vec::new();
    for row in missing_photo_rows {
        let customer_id: Option<String> = row.try_get("customer_id").ok().flatten();
        missing_photo_list.push(json!({
            "customer_id": customer_id.unwrap_or_else(|| "unknown".to_string()),
            "item": ""
        }));
    }

    let denom = if today_visits > 0.0 { today_visits } else { 0.0 };
    let photo_sent_rate = if denom > 0.0 {
        (photos_customers.max(0) as f64) / denom
    } else {
        0.0
    };
    let ai_record_rate = if denom > 0.0 {
        (emr_customers.max(0) as f64) / denom
    } else {
        0.0
    };
    let emr_done_rate = ai_record_rate;
    let followup_done_rate = if followup_planned > 0 {
        (followup_done.max(0) as f64) / (followup_planned as f64)
    } else {
        0.0
    };

    Ok(json!({
        "store_id": store_id,
        "store_name": store_name,
        "biz_date": biz_date.format("%Y-%m-%d").to_string(),
        "data_cutoff_time": cutoff_time,
        "his": {
            "visits": today_visits,
            "appointments": (today_appointments_count.max(0) as f64),
            "deals": today_visits,
            "gmv": today_gmv,
            "consumption": today_gmv,
            "avg_ticket": today_avg_ticket,
            "new_customers": new_count,
            "old_customers": old_count,
            "top_items": top_items
        },
        "baselines": {
            "rolling_7d": {
                "visits_avg": visits_avg_7d,
                "gmv_avg": gmv_avg_7d,
                "consumption_avg": gmv_avg_7d,
                "avg_ticket_avg": avg_ticket_avg_7d
            }
        },
        "mtd": {
            "gmv": mtd_gmv,
            "consumption": mtd_consumption,
            "time_progress": time_progress,
            "gmv_target": number_or_zero(minimal_input.get("mtd").and_then(|v| v.get("gmv_target"))),
            "consumption_target": number_or_zero(minimal_input.get("mtd").and_then(|v| v.get("consumption_target")))
        },
        "appointments_tomorrow": appointments_tomorrow,
        "staff_stats": staff_stats,
        "customer_summary": customer_summary,
        "key_items_mtd": key_items_mtd,
        "task_execution": {
            "followup_done_rate": followup_done_rate,
            "photo_sent_rate": photo_sent_rate,
            "postop_reminder_rate": 0.0,
            "ai_record_rate": ai_record_rate,
            "emr_done_rate": emr_done_rate,
            "missing_photo_list": missing_photo_list,
            "prescription_rate": if denom > 0.0 { (prescription_customers.max(0) as f64) / denom } else { 0.0 }
        }
    }))
}
