use std::sync::Arc;

use agent_runtime::runtime::WorkflowRunner;
use serde_json::json;
use uuid::Uuid;

use loreal_agent_app::tools::ToolManager;
use loreal_agent_app::workflows::{load_latest_active_spec_path, MeetingPrebriefDaily1_1Runner, WorkflowSpec};

#[tokio::test]
async fn persists_report_md_to_reports_dir() {
    let tmp_dir = format!("target/tmp/reports_test_{}", Uuid::new_v4());
    std::fs::create_dir_all(&tmp_dir).expect("create tmp reports dir");
    std::env::set_var("REPORTS_DIR", &tmp_dir);

    let spec_path = load_latest_active_spec_path().expect("discover active spec");
    let spec = WorkflowSpec::load(&spec_path).expect("load spec");
    let tools = Arc::new(ToolManager::new(None));
    let runner = MeetingPrebriefDaily1_1Runner::from_spec(&spec, tools).expect("runner");

    let input = json!({
        "store_id": "test_store",
        "store_name": "测试门店",
        "biz_date": "2025-12-30",
        "data_cutoff_time": "16:12",
        "his": { "visits": 15, "gmv": 186000, "consumption": 142000, "avg_ticket": 13286, "new_customers": 9, "old_customers": 5 },
        "mtd": { "gmv": 986000, "consumption": 865000, "time_progress": 0.58, "gmv_target": 2200000, "consumption_target": 2000000 }
    });

    runner.run(input).await.expect("run workflow");

    let expected = format!("{}/briefing_20251230.md", tmp_dir);
    let content = std::fs::read_to_string(&expected).expect("report exists");
    assert!(content.contains("## 今日经营摘要"));
}

