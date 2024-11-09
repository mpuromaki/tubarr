use anyhow::{Context, Ok, Result};
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use tracing::{debug, error, event, info, info_span, span, trace, warn, Level};

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
        info!("DB is up to date.");
        return Ok(());
    }

    // Iterate over the upgrades starting from the specified index
    for (i, upgrade) in upgrades.iter().enumerate().skip(idx) {
        let span = span!(
            Level::INFO,
            "DB upgrade",
            iteration = i + 1,
            total = upgrades.len()
        );
        let _enter = span.enter();
        info!("DB Upgrade {}/{} starting...", i + 1, upgrades.len());
        upgrade(conn)?; // Apply the upgrade
        info!("DB Upgrade {}/{} completed.", i + 1, upgrades.len());
    }

    Ok(())
}

fn insert_version(
    ver: usize,
    desc: &str,
    conn: &PooledConnection<SqliteConnectionManager>,
) -> Result<()> {
    // Insert the first version (e.g., version 1)
    conn.execute(
        "INSERT INTO db_version (version_number, description, date) VALUES (?1, ?2, datetime('now'))",
        params![ver, desc],
    )
    .context("Failed to insert version into db_version table")?;
    Ok(())
}

/// Return a static list of upgrade functions
pub fn upgrades_as_list() -> Vec<fn(&PooledConnection<SqliteConnectionManager>) -> Result<()>> {
    vec![
        upgrade_1_db_versions,
        upgrade_2_users_tables,
        upgrade_3_app_configuration,
        upgrade_4_tasks,
        upgrade_5_channels,
        upgrade_6_videos,
        upgrade_7_channels_normalized_name,
    ]
}

/// Upgrade: Create db_version table and insert initial version
pub fn upgrade_1_db_versions(conn: &PooledConnection<SqliteConnectionManager>) -> Result<()> {
    // Create the db_version table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS db_version (
            version_number INTEGER PRIMARY KEY NOT NULL,
            description TEXT,
            date TEXT NOT NULL
        )",
        [],
    )
    .context("Failed to create db_version table")?;

    // Set DB version
    insert_version(1, "Initial database version", conn)?;
    Ok(())
}

/// Upgrade: Create users and users_local tables
/// User IDs are 16-byte BLOBs of UUID-v4
pub fn upgrade_2_users_tables(conn: &PooledConnection<SqliteConnectionManager>) -> Result<()> {
    // Create users table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (
            id BLOB PRIMARY KEY,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            protected BOOLEAN DEFAULT 0,
            enabled BOOLEAN DEFAULT 1
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
            protected BOOLEAN DEFAULT 0,
            enabled BOOLEAN DEFAULT 1,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )",
        [],
    )
    .context("Failed to create users_local table")?;

    // Insert into users table with 'protected' and 'enabled' fields set to true for the admin

    // Insert the admin user
    let admin_id: [u8; 16] = [
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        0xFF,
    ];
    let admin_password_hash = "replace_with_proper_hashed_password";

    conn.execute(
        "INSERT INTO users (id, protected, enabled) VALUES (?1, 1, 1)",
        params![admin_id],
    )
    .context("Failed to insert admin user into users table")?;

    // Insert into users_local table
    conn.execute(
        "INSERT INTO users_local (username, password_hash, user_id, protected, enabled) VALUES (?1, ?2, ?3, 1, 1)",
        params!["admin", admin_password_hash, admin_id],
    )
    .context("Failed to insert admin user into users_local table")?;

    // Set DB version
    insert_version(2, "Create user handling", conn)?;
    Ok(())
}

/// Upgrade: Create app_configuration table to store key-value pairs
pub fn upgrade_3_app_configuration(conn: &PooledConnection<SqliteConnectionManager>) -> Result<()> {
    // Create app_configuration table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS app_configuration (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
        [],
    )
    .context("Failed to create app_configuration table")?;

    // Insert initial configuration
    let insert_kv = "INSERT INTO app_configuration (key, value) VALUES (?1, ?2)";
    conn.execute(insert_kv, params!["first_time_setup", "true"])?;
    conn.execute(insert_kv, params!["path_temp", "/tmp/tubarr"])?;
    conn.execute(insert_kv, params!["path_media", "/srv/media"])?;
    conn.execute(insert_kv, params!["sub_lang", "en.*,fi"])?;
    conn.execute(insert_kv, params!["retry_limit", "3"])?;

    // Set DB version
    insert_version(3, "Create app configuration", conn)?;
    Ok(())
}

/// Upgrade: Create tasks table for tracking download tasks
/// This is for one-and-done tasks. New tasks are always created with task_state="WAIT".
/// Workers lock the table when looking for tasks. They'll update single row they take at a time.
/// When ever task_state is updated, updated_at must be updated as well.
/// Separate background tasks will clean old tasks and restart stuck tasks based on task_state and updated_at.
/// Exceeding retry-limit will cause the task to be set "FAIL".
pub fn upgrade_4_tasks(conn: &PooledConnection<SqliteConnectionManager>) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_type TEXT NOT NULL,  /* DOWNLOAD */
            task_data TEXT NOT NULL,  /* content varied by task_type */
            task_state TEXT NOT NULL, /* WAIT, WIP, ERR, DONE, FAIL */
            retry_count INTEGER DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .context("Failed to create tasks table")?;

    // Set DB version
    insert_version(4, "Create tasks", conn)?;
    Ok(())
}

/// Upgrade: Create channels table for tracking known channels
/// Only youtube is supported at this point.
pub fn upgrade_5_channels(conn: &PooledConnection<SqliteConnectionManager>) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS channels (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            domain TEXT NOT NULL,
            url TEXT UNIQUE NOT NULL,
            channel_id TEXT NOT NULL,
            channel_name TEXT NOT NULL,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(domain, channel_id),
            UNIQUE(domain, channel_name)
        )",
        [],
    )
    .context("Failed to create channels table")?;

    // Set DB version
    insert_version(5, "Create channels", conn)?;
    Ok(())
}

/// Upgrade: Create videos table for tracking known videos
/// When fetching all videos for a channel, yt-dlp can only give
/// estimated release_dates. Then separate background process should
/// get more precise dates for those. Aggregation into seasons will be
/// by release_date, or estimate if that's null. Season is just release year.
pub fn upgrade_6_videos(conn: &PooledConnection<SqliteConnectionManager>) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS videos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            channel_id INTEGER,
            domain TEXT NOT NULL,
            url TEXT UNIQUE NOT NULL,
            name TEXT NOT NULL,
            video_id TEXT NOT NULL,
            is_requested INTEGER NOT NULL, 
            is_downloaded INTEGER NOT NULL,
            release_date DATETIME,
            release_date_estimate DATETIME,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (channel_id) REFERENCES channels(id),
            UNIQUE(domain, video_id)
        )",
        [],
    )
    .context("Failed to create channels table")?;

    // Set DB version
    insert_version(6, "Create videos", conn)?;
    Ok(())
}

/// Upgrade: Add channel_name_normalized column to channels table for case-insensitive matching
pub fn upgrade_7_channels_normalized_name(
    conn: &PooledConnection<SqliteConnectionManager>,
) -> Result<()> {
    // Add channel_name_normalized column
    conn.execute(
        "ALTER TABLE channels ADD COLUMN channel_name_normalized TEXT;",
        [],
    )
    .context("Failed to add channel_name_normalized column")?;

    // Populate the channel_name_normalized column with lowercase values of channel_name
    conn.execute(
        "UPDATE channels SET channel_name_normalized = LOWER(channel_name);",
        [],
    )
    .context("Failed to populate channel_name_normalized")?;

    // Add a trigger to keep channel_name_normalized in sync with channel_name on updates
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_channel_name_normalized
         AFTER UPDATE OF channel_name ON channels
         FOR EACH ROW
         BEGIN
             UPDATE channels SET channel_name_normalized = LOWER(NEW.channel_name)
             WHERE id = NEW.id;
         END;",
        [],
    )
    .context("Failed to create update trigger for channel_name_normalized")?;

    // Set DB version
    insert_version(
        7,
        "Add normalized channel name for case-insensitive matching",
        conn,
    )?;
    Ok(())
}
