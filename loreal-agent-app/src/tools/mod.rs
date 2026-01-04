mod mysql;
mod util;

use std::sync::Arc;

use sqlx::MySqlPool;

pub use mysql::{assemble_meeting_prebrief_daily_1_1_mysql, MysqlAssembleError};
pub use util::merge_json;

#[derive(Clone)]
pub struct ToolManager {
    mysql: Option<MySqlPool>,
}

impl ToolManager {
    pub fn new(mysql: Option<MySqlPool>) -> Self {
        Self { mysql }
    }

    pub fn mysql(&self) -> Option<&MySqlPool> {
        self.mysql.as_ref()
    }
}

pub type SharedTools = Arc<ToolManager>;
