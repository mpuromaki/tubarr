//! Read tasks from database and execute local yt-dlp binary based on those.
//!
//! New worker thread will be spawned for each task. That thread is responsible for:
//! - Updating the task status to the DB.
//! - Downloading the file with yt-dlp. ("DOWNLOAD" task)
//! - Moving the file to the correct location after download is complete. ("DOWNLOAD" task)
//!
//! Yt-dlp options:
//! yt-dlp --write-description --write-thumbnail --no-playlist --write-sub --sub-lang en,fi -o "%(channel)s - %(upload_date)s - %(title)s - (%(id)s).%(ext)s"  <URL>
//!
//! Output folder structure:
//! <PATH_MEDIA>/<CHANNEL>/Season <YYYY>/<FILENAME>

use anyhow::Result;
use chrono::TimeZone;
use core::{str, time};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::sync::mpsc::channel;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;
use std::{collections::HashMap, fs::create_dir_all};
use std::{ffi::IntoStringError, path::Path};
use std::{fs, path::PathBuf};
use tldextract::{TldExtractor, TldOption};

use crate::common::TaskDownloadData;

use super::DBPool;
use super::FLAG_SHUTDOWN;

pub fn run(dbp: DBPool) {
    let conn = dbp
        .get()
        .expect("Failed to get database connection from pool");
    let (result_tx, result_rx): (Sender<TaskResult>, Receiver<TaskResult>) = channel();
    let conf = match get_configuration(dbp.clone()) {
        Ok(conf) => Arc::from(conf),
        Err(_) => {
            eprintln!("taskrunner could not load configuration.");
            FLAG_SHUTDOWN.store(true, std::sync::atomic::Ordering::Relaxed);
            return;
        }
    };

    loop {
        // Sleep so we don't trash the CPU
        thread::sleep(time::Duration::from_secs(1));

        // Check for tasks
        if let Ok(new_tasks) = get_new_tasks(dbp.clone()) {
            for task in new_tasks {
                println!("Found new task: {:?}", task);
                match task.task_type.as_str() {
                    "DOWNLOAD" => {
                        println!("Processing as download task.");
                        mark_task_wip(dbp.clone(), task.task_id);
                        let thrd_conf = conf.clone();
                        let thrd_tx = result_tx.clone();
                        thread::spawn(move || {
                            worker_download(task.task_id, task.task_data, thrd_conf, thrd_tx)
                        });
                    }
                    _ => continue,
                }
            }
        }

        // Check for worker reports, update tasks state
        while let Ok(result) = result_rx.try_recv() {
            // Update the result to the DB
            println!("TASK RESULT: {:?}", result);
            match result {
                TaskResult::Ok(id) => mark_task_done(dbp.clone(), id),
                TaskResult::Err(id, errcode) => mark_task_error(dbp.clone(), id),
            }
        }

        // Check for shutdown signal
        if FLAG_SHUTDOWN.load(std::sync::atomic::Ordering::Relaxed) == true {
            break;
        }
    }

    // Shutdown, bye!
}

#[derive(Debug)]
struct TaskRaw {
    task_id: isize,
    task_type: String,
    task_data: String,
    task_state: String,
}

fn get_configuration(dbp: DBPool) -> Result<HashMap<String, String>> {
    let conn = dbp
        .get()
        .expect("Failed to get database connection from pool");

    // Query the configuration table and load data into a HashMap
    let mut config_map = HashMap::new();
    let mut stmt = conn.prepare("SELECT key, value FROM app_configuration")?;

    let config_iter = stmt.query_map([], |row| {
        let key: String = row.get(0)?;
        let value: String = row.get(1)?;
        Ok((key, value))
    })?;

    // Collect results into the HashMap
    for config in config_iter {
        let (key, value) = config?;
        config_map.insert(key, value);
    }

    Ok(config_map)
}

fn get_new_tasks(dbp: DBPool) -> Result<Vec<TaskRaw>> {
    let conn = dbp
        .get()
        .expect("Failed to get database connection from pool");

    // Prepare SQL statement to select all tasks where task_state is "NEW"
    let mut stmt = conn
        .prepare("SELECT id, task_type, task_data, task_state FROM tasks WHERE task_state = ?1")?;

    // Query for all "NEW" tasks and map each row to a TaskRaw instance
    let tasks = stmt
        .query_map(params!["NEW"], |row| {
            Ok(TaskRaw {
                task_id: row.get(0)?,
                task_type: row.get(1)?,
                task_data: row.get(2)?,
                task_state: row.get(3)?,
            })
        })?
        .filter_map(|res| res.ok())
        .collect();

    Ok(tasks)
}

fn mark_task_wip(dbp: DBPool, task_id: isize) {
    if let Ok(conn) = dbp.get() {
        // Prepare the SQL statement to update the task
        if let Err(e) = conn.execute(
            "UPDATE tasks SET task_state = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            params!["WIP", task_id],
        ) {
            eprintln!("Error updating task state: {}", e); // Log the error
            return;
        }

        println!("Task with ID {} marked as WIP.", task_id);
    } else {
        eprintln!("Error getting database connection."); // Log connection error
    }
}

fn mark_task_done(dbp: DBPool, task_id: isize) {
    if let Ok(conn) = dbp.get() {
        // Prepare the SQL statement to update the task
        if let Err(e) = conn.execute(
            "UPDATE tasks SET task_state = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            params!["DONE", task_id],
        ) {
            eprintln!("Error updating task state: {}", e); // Log the error
            return;
        }

        println!("Task with ID {} marked as DONE.", task_id);
    } else {
        eprintln!("Error getting database connection."); // Log connection error
    }
}

fn mark_task_error(dbp: DBPool, task_id: isize) {
    if let Ok(conn) = dbp.get() {
        // Prepare the SQL statement to update the task
        if let Err(e) = conn.execute(
            "UPDATE tasks SET task_state = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            params!["ERROR", task_id],
        ) {
            eprintln!("Error updating task state: {}", e); // Log the error
            return;
        }

        println!("Task with ID {} marked as ERROR.", task_id);
    } else {
        eprintln!("Error getting database connection."); // Log connection error
    }
}

/// Result of task. Payload is the ID of the task.
#[derive(Debug)]
enum TaskResult {
    Ok(isize),         // ID
    Err(isize, isize), // ID, ERRCODE
}

/// Worker for DOWNLOAD tasks.
fn worker_download(
    task_id: isize,
    data: String,
    conf: Arc<HashMap<String, String>>,
    sender: Sender<TaskResult>,
) {
    println!("Worker started for task ID {}", task_id);

    // Unpack data
    let data: TaskDownloadData = match serde_json::from_str(&data) {
        Ok(data) => data,
        Err(_) => {
            let _ = sender.send(TaskResult::Err(task_id, -500));
            return;
        }
    };

    let tldopt = TldOption::default();
    let extractor = TldExtractor::new(tldopt);
    let extracted = extractor
        .extract(&data.url)
        .expect("Could not extract domain");
    let domain = format!(
        "{}.{}",
        extracted.domain.unwrap(),
        extracted.suffix.unwrap()
    );

    let path_tmp = conf
        .get("path_temp")
        .expect("Could not get configuration: path_temp");
    let path_media = conf
        .get("path_media")
        .expect("Could not get configuration: path_media");
    let sub_lang = conf
        .get("sub_lang")
        .expect("Could not get configuration: sub_lang");

    println!("Task created successfully!");
    println!("PATH TMP: {:?}", path_tmp);
    println!("PATH_MEDIA: {:?}", path_media);

    // Resolve file name with yt-dlp
    let filename_template = "%(channel)s SPLITATTHISPOINT %(upload_date)s SPLITATTHISPOINT %(title)s SPLITATTHISPOINT (%(id)s)";

    let filename_output = Command::new("yt-dlp")
        .args(["--print", "filename", "-o", &filename_template, &data.url])
        .output();

    if filename_output.is_err() {
        let _ = sender.send(TaskResult::Err(task_id, -502));
        return;
    }
    let filename_output = filename_output.unwrap();
    let filename_parts = match str::from_utf8(&filename_output.stdout) {
        Ok(name) => name.trim().to_string(),
        Err(_) => {
            let _ = sender.send(TaskResult::Err(task_id, -503));
            return;
        }
    };
    println!("FILENAME_PARTS: {}", filename_parts);

    let mut filename = filename_parts.replace("SPLITATTHISPOINT", "-");
    let mut channel = Some(
        filename_parts
            .split("SPLITATTHISPOINT")
            .nth(0)
            .unwrap()
            .trim()
            .to_owned(),
    );
    if channel == Some("NA".to_string()) {
        channel = None;
    } else {
        channel = channel;
    }

    let mut year = Some(
        filename_parts
            .split("SPLITATTHISPOINT")
            .nth(1)
            .unwrap()
            .trim()
            .to_owned(),
    );
    if year == Some("NA".to_string()) {
        year = None;
    } else {
        year = Some(year.unwrap().trim()[..4].to_string());
    }

    filename = filename.replace("NA -", "").trim().to_owned();

    println!("FILENAME: {:?}", filename);
    println!("CHANNEL: {:?}", channel);
    println!("YEAR: {:?}", year);

    // Download the files with yt-dlp
    let filename_template = match (&channel, &year) {
        (Some(_), Some(_)) => "%(channel)s - %(upload_date)s - %(title)s - (%(id)s).%(ext)s",
        (Some(_), None) => "%(channel)s - %(title)s - (%(id)s).%(ext)s",
        (None, Some(_)) => "%(upload_date)s - %(title)s - (%(id)s).%(ext)s",
        (None, None) => "%(title)s - (%(id)s).%(ext)s",
    };
    let filepath = format!("{}/{}", path_tmp, filename_template);

    // Refer: https://github.com/yt-dlp/yt-dlp/issues/630#issuecomment-893659460
    let output = Command::new("yt-dlp")
        .args([
            "--no-playlist",
            "--add-metadata",
            "--embed-metadata",
            "--embed-thumbnail",
            "--write-subs",
            "--write-auto-subs",
            "--embed-subs",
            "--compat-options",
            "no-keep-subs",
            "--sub-lang",
            sub_lang,
            "-o",
            &filepath,
            &data.url,
        ])
        .output();

    if output.is_err() {
        let _ = sender.send(TaskResult::Err(task_id, -504));
        return;
    }
    let output = output.unwrap();

    // 10s delay to let things stablize
    thread::sleep(time::Duration::from_secs(10));

    // Set up the media storage location
    let mut path_media_full: PathBuf = path_media.into();
    path_media_full.push(domain);
    if channel.is_some() {
        path_media_full.push(channel.unwrap());
    }
    if year.is_some() {
        path_media_full.push(year.unwrap());
    } else {
        path_media_full.push("other");
    }
    let _ = create_dir_all(&path_media_full);

    println!("PATH_MEDIA_FULL: {:?}", path_media_full);

    // Move the files
    let path_tmp = PathBuf::from(path_tmp);
    let _ = move_files_with_prefix(&path_tmp, &path_media_full, &filename);

    // And finally, return
    let _ = sender.send(TaskResult::Ok(task_id));
}

fn move_files_with_prefix(
    path_tmp: &Path,
    path_media_full: &Path,
    filename_prefix: &str,
) -> Result<()> {
    // Iterate over entries in the `path_tmp` directory
    for entry in fs::read_dir(path_tmp)? {
        let entry = entry?;
        let path = entry.path();

        // Check if it's a file and if its name starts with the `filename_prefix`
        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
                if file_name.starts_with(filename_prefix) {
                    // Define the destination path in `path_media_full`
                    let destination = path_media_full.join(file_name);

                    // Move the file
                    fs::copy(&path, &destination)?;

                    // Verify that `path` is within `path_tmp` before removing
                    let canonical_path = path.canonicalize()?;
                    if canonical_path.starts_with(&path_tmp) {
                        fs::remove_file(&canonical_path)?;
                    } else {
                        eprintln!(
                            "Warning: Skipped deletion for file outside `path_tmp`: {:?}",
                            path
                        );
                    }
                }
            }
        }
    }
    Ok(())
}
