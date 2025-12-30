use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use sqlx::MySqlPool;

#[tokio::test]
#[ignore = "requires DATABASE_URL and network access"]
async fn read_mysql_tables() {
    dotenvy::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = MySqlPool::connect(&db_url)
        .await
        .expect("connect mysql");

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM appointments")
        .fetch_one(&pool)
        .await
        .expect("query appointments");

    assert!(count >= 0);

    let payment_total: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(p.Amount), 0) \
         FROM billoperationrecordpayments p \
         JOIN billoperationrecords r ON p.RecordId = r.ID \
         WHERE r.OperationTime >= ? AND r.OperationTime < ?",
    )
    .bind("2025-01-06 00:00:00")
    .bind("2025-01-07 00:00:00")
    .fetch_one(&pool)
    .await
    .expect("query payment_total");

    let payment_total = payment_total.to_f64().unwrap_or(0.0);
    assert!(payment_total >= 0.0);
}
