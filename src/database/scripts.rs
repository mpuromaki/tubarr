use anyhow::{Context, Ok, Result};
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;

/// Apply upgrades from specific index
pub fn upgrade_from(idx: usize, conn: &PooledConnection<SqliteConnectionManager>) -> Result<()> {
    // Get the list of upgrade functions
    let upgrades = upgrades_as_list();

    // Ensure the provided index is within bounds
    if idx > upgrades.len() {
        return Err(anyhow::anyhow!(
            "Invalid upgrade index: {}. No such upgrade exists.",
            idx
        ));
    }

    if idx == upgrades.len() {
        println!("DB is up to date.");
        return Ok(());
    }

    // Iterate over the upgrades starting from the specified index
    for (i, upgrade) in upgrades.iter().enumerate().skip(idx) {
        println!("DB Upgrade {}/{}", i + 1, upgrades.len()); // Logging the upgrade number
        upgrade(conn)?; // Apply the upgrade
    }

    Ok(())
}

/// Return a static list of upgrade functions
pub fn upgrades_as_list() -> Vec<fn(&PooledConnection<SqliteConnectionManager>) -> Result<()>> {
    vec![upgrade_0_db_versions, upgrade_1_users_tables]
}

/// Upgrade: Create db_version table and insert initial version
pub fn upgrade_0_db_versions(conn: &PooledConnection<SqliteConnectionManager>) -> Result<()> {
    // Create the db_version table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS db_version (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            version_number INTEGER NOT NULL,
            description TEXT,
            date TEXT NOT NULL
        )",
        [],
    )
    .context("Failed to create db_version table")?;

    // Insert the first version (e.g., version 1)
    conn.execute(
        "INSERT INTO db_version (version_number, description, date) VALUES (?1, ?2, datetime('now'))",
        params![1, "Initial database version"],
    )
    .context("Failed to insert the initial version into db_version table")?;

    Ok(())
}

/// Upgrade: Create users and users_local tables
/// User IDs are 16-byte BLOBs of UUID-v4
pub fn upgrade_1_users_tables(conn: &PooledConnection<SqliteConnectionManager>) -> Result<()> {
    // Create users table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (
            id BLOB PRIMARY KEY,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .context("Failed to create users table")?;

    // Create users_local table, this allows local usernames to be used to login
    // Use this template to create for example ldap login
    conn.execute(
        "CREATE TABLE IF NOT EXISTS users_local (
            username TEXT PRIMARY KEY NOT NULL,
            password_hash TEXT NOT NULL,
            user_id BLOB NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )",
        [],
    )
    .context("Failed to create users_local table")?;

    // Insert the admin user
    let admin_id: [u8; 16] = [
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    ];
    let admin_password_hash = "replace_with_proper_hashed_password";

    conn.execute("INSERT INTO users (id) VALUES (?1)", params![admin_id])
        .context("Failed to insert admin user into users table")?;

    conn.execute(
        "INSERT INTO users_local (username, password_hash, user_id) VALUES (?1, ?2, ?3)",
        params!["admin", admin_password_hash, admin_id],
    )
    .context("Failed to insert admin user into users_local table")?;

    Ok(())
}
