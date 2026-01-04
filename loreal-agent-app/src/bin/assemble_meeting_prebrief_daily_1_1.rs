use chrono::NaiveDate;
use serde_json::json;
use sqlx::MySqlPool;

use loreal_agent_app::tools::assemble_meeting_prebrief_daily_1_1_mysql;

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

    let minimal_input = json!({
        "store_id": store_id,
        "store_name": store_name,
        "biz_date": biz_date.format("%Y-%m-%d").to_string(),
        "data_cutoff_time": cutoff_time
    });
    let input = assemble_meeting_prebrief_daily_1_1_mysql(&pool, &minimal_input)
        .await
        .unwrap_or_else(|err| {
            eprintln!("assemble failed: {}", err);
            std::process::exit(1);
        });
    println!("{}", serde_json::to_string_pretty(&input).unwrap());
}
