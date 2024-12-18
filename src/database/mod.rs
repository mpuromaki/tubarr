use std::{fs, path::PathBuf, sync::Arc};

use anyhow::{anyhow, Context, Ok, Result};
use r2d2::{self, Pool, PooledConnection};
use r2d2_sqlite::{self, SqliteConnectionManager};
use rusqlite::{params, OptionalExtension};
use tracing::{debug, error, event, info, trace, warn};

mod scripts;

pub fn init_from(filepath: &PathBuf) -> Arc<r2d2::Pool<SqliteConnectionManager>> {
    info!("Loading database from: {:#?}", filepath);

    // Initialize the database connection
    let sqlite_conman = SqliteConnectionManager::file(filepath);
    let sqlite_pool = r2d2::Pool::new(sqlite_conman).expect("Failed to start SQLITE pool.");
    let pool = Arc::new(sqlite_pool);

    // Upgrade the database
    let _ = db_upgrade(pool.clone());

    return pool;
}

fn db_upgrade(pool: Arc<Pool<SqliteConnectionManager>>) -> Result<()> {
    let conn = pool.get().expect("Could not connect to SQLITE.");

    // Get DB version
    let db_version = db_get_version(&conn);

    let idx = match db_version {
        Result::Ok(idx) => idx,
        Result::Err(_) => 0,
    };

    scripts::upgrade_from(idx as usize, &conn).expect("SQLITE Upgrade failed.");

    Ok(())
}

fn db_get_version(conn: &PooledConnection<SqliteConnectionManager>) -> Result<u32> {
    // SQL query to fetch the latest version number by date
    let mut stmt = conn
        .prepare("SELECT version_number FROM db_version ORDER BY version_number DESC LIMIT 1")
        .context("Failed to prepare the SQL statement.")?;

    // Execute the query and fetch the version_number
    let version_number: u32 = stmt
        .query_row(params![], |row| row.get(0))
        .optional()
        .context("Failed to query the version number.")?
        .ok_or_else(|| anyhow!("No version information found in the database"))?;

    Ok(version_number)
}
