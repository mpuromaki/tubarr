use chrono::NaiveDate;
use regex::Regex;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::{collections::HashMap, process::Command};
use std::{fs::create_dir_all, thread, time};
use tldextract::{TldExtractor, TldOption};
use tracing::{debug, error, event, info, trace, warn};

use crate::DBPool;

use super::{move_files_with_prefix, parse_domain, TaskResult};

/// Worker for CHANNEL-ADD tasks.
/// We only know the URL of the channel, we have to fill row in "channels" table.
pub fn add(
    task_id: isize,
    data: String,
    conf: Arc<HashMap<String, String>>,
    sender: Sender<TaskResult>,
    dbp: DBPool,
) {
    debug!("task_channel::add() started for task {}", task_id);

    // Unpack data
    let data: TaskChannelAddData = match serde_json::from_str(&data) {
        Ok(data) => data,
        Err(_) => {
            debug!("Failed to parse data for task {}", task_id);
            let _ = sender.send(TaskResult::Err(task_id, -500));
            return;
        }
    };

    // Get domain
    let domain = parse_domain(&data.url);

    if &domain != "youtube.com" {
        error!("Adding channels is not supported for domain: {}", domain);
        let _ = sender.send(TaskResult::Err(task_id, -400));
        return;
    }

    // Get channel ID
    let output = Command::new("yt-dlp")
        .args([
            "--skip-download",
            "--playlist-items",
            "1",
            "--print",
            "%(channel_id)s",
            &data.url,
        ])
        .output();

    if output.is_err() {
        debug!("Failed to get channel id for task {}", task_id);
        let _ = sender.send(TaskResult::Err(task_id, -500));
        return;
    }
    let channel_id = match std::str::from_utf8(&output.unwrap().stdout) {
        Ok(chid) => chid.trim().to_string(),
        Err(_) => {
            debug!("Failed to parse channel id for task {}", task_id);
            let _ = sender.send(TaskResult::Err(task_id, -500));
            return;
        }
    };

    // Get url
    let url = format!("{}/channel/{}", domain, channel_id);

    // Get channel name
    let output = Command::new("yt-dlp")
        .args([
            "--skip-download",
            "--playlist-items",
            "1",
            "--print",
            "%(channel)s",
            &data.url,
        ])
        .output();

    if output.is_err() {
        debug!("Failed to get channel name for task {}", task_id);
        let _ = sender.send(TaskResult::Err(task_id, -500));
        return;
    }
    let channel_name = match std::str::from_utf8(&output.unwrap().stdout) {
        Ok(chnm) => chnm.trim().to_string(),
        Err(_) => {
            debug!("Failed to parse channel name for task {}", task_id);
            let _ = sender.send(TaskResult::Err(task_id, -500));
            return;
        }
    };

    let normalized_channel_name = normalize_channel_name(&channel_name);

    // Write the data to db
    if let Ok(conn) = dbp.get() {
        // Prepare the SQL statement to update the task
        if let Err(e) = conn.execute(
            "INSERT INTO channels (domain, url, channel_id, channel_name) VALUES (?1, ?2, ?3, ?4)",
            params![domain, url, channel_id, channel_name],
        ) {
            error!("Error inserting channel for task {}: {}", task_id, e);
            let _ = sender.send(TaskResult::Err(task_id, -500));
            return;
        }
    } else {
        error!("Error connecting to database for task {}", task_id);
        let _ = sender.send(TaskResult::Err(task_id, -500));
        return;
    }

    // And finally, return
    let _ = sender.send(TaskResult::Ok(task_id));
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TaskChannelAddData {
    pub url: String,
}

fn normalize_channel_name(channel_name: &str) -> String {
    let re = Regex::new(r#"['\"]"#).unwrap(); // This works for removing quotes

    let mut normalized = re.replace_all(channel_name, "");

    // Replace spaces with hyphens
    normalized = normalized.replace(" ", "-").into();

    // Convert to lowercase
    normalized.to_lowercase()
}

/// Worker for CHANNEL-FETCH tasks.
/// Download the metadata for every video on specific channel and populate videos table.
pub fn fetch(
    task_id: isize,
    data: String,
    conf: Arc<HashMap<String, String>>,
    sender: Sender<TaskResult>,
    dbp: DBPool,
) {
    debug!("task_channel::fetch() started for task {}", task_id);

    // Unpack data
    let data: TaskChannelFetchData = match serde_json::from_str(&data) {
        Ok(data) => data,
        Err(_) => {
            debug!("Failed to parse data for task {}", task_id);
            let _ = sender.send(TaskResult::Err(task_id, -500));
            return;
        }
    };

    // Get domain
    let domain = parse_domain(&data.domain);

    if &domain != "youtube.com" {
        error!("Fetching channels is not supported for domain: {}", domain);
        let _ = sender.send(TaskResult::Err(task_id, -400));
        return;
    }

    // Construct URLs
    let url = match domain.as_ref() {
        "youtube.com" => format!("youtube.com/channel/{}", data.channel_id),
        _ => {
            error!("Fetching channels is not supported for domain: {}", domain);
            let _ = sender.send(TaskResult::Err(task_id, -400));
            return;
        }
    };
    let videos_url = format!("https://www.{}/videos", url);
    debug!("Downloading url: {:?}", videos_url);

    // Get videos information
    let output = Command::new("yt-dlp")
        .arg("--skip-download")
        .arg("--extractor-args")
        .arg("youtubetab:approximate_date") // Separate this as a distinct argument
        .arg("--print")
        .arg("%(channel_id)s SPLITATTHISPOINT %(channel)s SPLITATTHISPOINT %(webpage_url)s SPLITATTHISPOINT %(upload_date)s SPLITATTHISPOINT %(title)s SPLITATTHISPOINT %(id)s")
        .arg(&videos_url) // The URL at the very end
        .output();

    if output.is_err() {
        debug!("Failed to get channel id for task {}", task_id);
        let _ = sender.send(TaskResult::Err(task_id, -500));
        return;
    }
    //debug!("YT-DLP: {:?}", output);

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
            error!("Failed parse channels metadata for task {}", task_id);
            let _ = sender.send(TaskResult::Err(task_id, -500));
            return;
        }
    };
    //debug!("FILES METADATA[0]: {:?}", files_metadata[0]);
    debug!("Received {} rows", files_metadata.len());
    debug!("Example row: {:?}", files_metadata[0]);

    // Write the data to db
    if let Ok(conn) = dbp.get() {
        for row in files_metadata.iter() {
            //debug!("Inserting row: {:?}", row);

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
    } else {
        error!("Error connecting to database for task {}", task_id);
        let _ = sender.send(TaskResult::Err(task_id, -500));
        return;
    }

    // And finally, return
    let _ = sender.send(TaskResult::Ok(task_id));
}

#[derive(Deserialize, Serialize)]
struct TaskChannelFetchData {
    domain: String,
    channel_id: String,
}
