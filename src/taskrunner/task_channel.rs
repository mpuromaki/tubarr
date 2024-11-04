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
