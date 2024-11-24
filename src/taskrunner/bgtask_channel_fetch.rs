use std::process::Command;

use chrono::NaiveDate;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use tracing::{debug, error, event, info, trace, warn};

use super::update_bgtask_exec_time;
use crate::DBPool;

/// Persistent background task for fetching new videos for channels
pub fn bg_channel_fetch(task_id: isize, dbp: DBPool) {
    debug!("Started background task: bg_channel_fetch");

    // Connect to the database and delete old completed or failed tasks
    let conn = match dbp.get() {
        Ok(conn) => conn,
        Err(_) => {
            error!("Failed to get database connection");
            return;
        }
    };

    // Update this persistent task
    update_bgtask_exec_time(task_id, &conn);

    // Get every channel videos url to fetch
    let mut stmt = match conn.prepare("SELECT url FROM channels") {
        Ok(stmt) => stmt,
        Err(err) => {
            error!("Failed to prepare statement: {:?}", err);
            return;
        }
    };

    let urls_result: rusqlite::Result<Vec<String>> = stmt
        .query_map([], |row| row.get(0))
        .and_then(|mapped_rows| mapped_rows.collect());

    let urls = match urls_result {
        Ok(urls) => urls
            .into_iter()
            .map(|url| format!("{}/videos", url))
            .collect::<Vec<String>>(),
        Err(err) => {
            error!("Failed to query channels: {:?}", err);
            return;
        }
    };

    // Download the urls
    for url in &urls {
        debug!("bg_channel_fetch: {}", url);

        // Get videos information
        let output = Command::new("yt-dlp")
        .arg("--skip-download")
        .arg("--extractor-args")
        .arg("youtubetab:approximate_date")
        .arg("--print")
        .arg("%(channel_id)s SPLITATTHISPOINT %(channel)s SPLITATTHISPOINT %(webpage_url)s SPLITATTHISPOINT %(upload_date)s SPLITATTHISPOINT %(title)s SPLITATTHISPOINT %(id)s")
        .arg("--dateafter")
        .arg("today-2days")
        .arg("--break-on-reject")
        .arg("--lazy-playlist")
        .arg(&url)
        .output();

        if output.is_err() {
            debug!("Failed yt-dlp fetch");
            continue;
        }

        let files_metadata: Vec<Vec<String>> = match std::str::from_utf8(&output.unwrap().stdout) {
            Ok(rows) => {
                let mut result = Vec::new();
                for row in rows.lines() {
                    //debug!("Processing row: {:?}", row);
                    let parsed_line: Vec<String> = row
                        .split("SPLITATTHISPOINT")
                        .map(|s| s.trim().to_string())
                        .collect();
                    result.push(parsed_line);
                }
                result
            }
            Err(_) => {
                error!("Failed parse yt-dlp fetch");
                continue;
            }
        };

        debug!("Fetching {} new videos", files_metadata.len());

        // Write the data to db
        for row in files_metadata.iter() {
            // Skip rows with unexpected data structure
            if row.len() != 6 {
                error!("Row has unexpected structure: {:?}", row);
                continue;
            }

            // Extract values from the row vector
            let (channel_id, channel_name, webpage_url, upload_date, title, video_id) = (
                row[0].clone(),
                row[1].clone(),
                row[2].clone(),
                row[3].clone(),
                row[4].clone(),
                row[5].clone(),
            );

            info!("New video: {} / {}", channel_name, title);

            // Parse release_date
            let release_date_estimate = match NaiveDate::parse_from_str(&upload_date, "%Y%m%d") {
                Ok(date) => date,
                Err(_) => {
                    error!("Invalid date format for upload_date: {}", upload_date);
                    continue;
                }
            };

            // Check if the channel exists in the channels table
            let mut channel_exists_stmt =
                match conn.prepare("SELECT id FROM channels WHERE channel_id = ?") {
                    Ok(stmt) => stmt,
                    Err(e) => {
                        error!("Failed to prepare channel check statement: {:?}", e);
                        continue;
                    }
                };
            let channel_exists_result = channel_exists_stmt
                .query_row(rusqlite::params![channel_id], |row| row.get::<_, i32>(0));

            let channel_id_db: i32 = match channel_exists_result {
                Ok(id) => id,
                Err(_) => {
                    error!("Channel doesn't exist: {}, skipping.", channel_id);
                    continue;
                }
            };

            // Prepare SQL insertion
            let sql = "INSERT INTO videos (
                       channel_id, domain, url, name, video_id, 
                       is_requested, is_downloaded, release_date, release_date_estimate
                   ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                   ON CONFLICT(domain, video_id) DO UPDATE SET 
                       updated_at = CURRENT_TIMESTAMP";

            // Execute the prepared SQL statement
            let mut stmt = match conn.prepare(sql) {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to prepare statement: {:?}", e);
                    continue;
                }
            };

            if let Err(e) = stmt.execute(rusqlite::params![
                channel_id_db,         // channel_id (from channels table)
                "youtube.com",         // domain (fixed as "youtube.com")
                webpage_url,           // url (now taken from row[2])
                title,                 // name (video title from row[4])
                video_id,              // video_id (from row[5])
                0,                     // is_requested (default to 0)
                0,                     // is_downloaded (default to 0)
                None::<NaiveDate>,     // release_date set to NULL
                release_date_estimate  // release_date_estimate (using same date)
            ]) {
                error!("Failed to execute insert: {:?}", e);
            }
        }
    }

    debug!("Completed background task: bg_channel_fetch");
}
