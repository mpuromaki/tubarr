use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use tracing::{debug, error, event, info, trace, warn};

use super::update_bgtask_exec_time;
use crate::DBPool;

/// Persisten background task for cleaning old entries from tasks table
pub fn db_clean_tasks(task_id: isize, dbp: DBPool) {
    debug!("Started background task: db_clean_tasks");

    // Connect to the database and delete old completed or failed tasks
    if let Ok(conn) = dbp.get() {
        // Update this persistent task
        update_bgtask_exec_time(task_id, &conn);

        // Do the long processing stuff
        if let Err(e) = conn.execute(
            "DELETE FROM tasks 
                WHERE (task_state = 'DONE' OR task_state = 'FAIL') 
                AND updated_at <= datetime('now', '-24 hours')",
            [],
        ) {
            return;
        }
    } else {
        return;
    }

    debug!("Completed background task: db_clean_tasks");
}
