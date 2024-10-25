//! TUBARR - ARR app for Youtube
//!
//! Server software with web-UI for monitoring youtube subscriptions and downloading
//! videos.
//! Requires local installation of yt-dlp.

use std::{
    io, sync::{mpsc::channel, Arc}, thread
};
use std::io::Write;

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use anyhow::{Context, Result};

static DB_PATH: &'static str = "./db.sqlite";

mod database;
mod taskrunner;

pub type DBPool = Arc<Pool<SqliteConnectionManager>>;

fn main() {
    // Initialize the database
    let dbp = database::init_from(DB_PATH);

    // First_time_setup
    if is_first_time_setup(dbp.clone()) {
        match get_configuration_from_user(dbp.clone()) {
            Ok(_) => set_first_time_setup(dbp.clone()),
            Err(_) => exit_with_error(-1),
        };
    }

    // Start subprograms
    let mut subprogs = Vec::new();

    subprogs.push(thread::spawn(|| taskrunner::start()));
}

/// Exit the program with no errors. Sends shutdown to subprograms.
fn exit_with_ok() {
    std::process::exit(0);
}

/// Exit the program with error code. Sends shutdown to subprograms.
fn exit_with_error(errcode: i32) {
    std::process::exit(errcode);
}

/// Check if the application is in its first-time setup state by querying the app_configuration table.
fn is_first_time_setup(dbp: DBPool) -> bool {
    // Get a database connection from the pool
    let conn = dbp
        .get()
        .expect("Failed to get database connection from pool");

    // Query for the "first_time_setup" configuration value
    match conn.query_row(
        "SELECT value FROM app_configuration WHERE key = ?1",
        params!["first_time_setup"],
        |row| row.get::<_, String>(0),
    ) {
        Ok(value) => value == "true",
        Err(_) => false,
    }
}

/// Ask configuration from user, store the settings to database.
fn get_configuration_from_user(dbp: DBPool) -> Result<()> {
    let conn = dbp.get().expect("Failed to get database connection from pool");

    // Retrieve all configuration rows except for "first_time_setup"
    let mut stmt = conn.prepare("SELECT key, value FROM app_configuration WHERE key != 'first_time_setup'")?;
    let config_entries = stmt.query_map([], |row| {
        let key: String = row.get(0)?;
        let value: String = row.get(1)?;
        Ok((key, value))
    })?;

    let mut updates = Vec::new();

    println!("First time setup. Values will be stored to the database.");
    println!("Press enter to accept default values.");

    for entry in config_entries {
        let (key, default_value) = entry?;

        // Prompt user with the current value as the default
        print!("'{}', default [{}]: ", key, default_value);
        io::stdout().flush().expect("Failed to flush stdout");

        // Read user input
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read input");

        // Use the default value if the input is empty; otherwise, use the user's input
        let value = if input.trim().is_empty() {
            default_value.clone()
        } else {
            input.trim().to_string()
        };

        // Add the configuration entry to the list of updates
        updates.push((key, value));
    }

    // Write all updated configuration values back to the database
    for (key, value) in updates {
        conn.execute(
            "UPDATE app_configuration SET value = ?1 WHERE key = ?2",
            params![value, key],
        )?;
    }

    Ok(())
}

/// Set the first_time_setup configuration key to "false".
/// Skips if errors are found.
fn set_first_time_setup(dbp: DBPool) {
    if let Ok(conn) = dbp.get() {
        if let Err(_) = conn.execute(
            "UPDATE app_configuration SET value = 'false' WHERE key = ?1",
            params!["first_time_setup"],
        ) {
            return;
        }
    } else {
        return;
    }
}