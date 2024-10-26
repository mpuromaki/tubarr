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
use core::time;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::IntoStringError;
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc::channel;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;

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

    // Download the video with yt-dlp
    let filename_template = "%(channel)s - %(upload_date)s - %(title)s - (%(id)s).%(ext)s";
    let filepath = format!("{}/{}", path_tmp, filename_template);
    let output = Command::new("yt-dlp")
        .args([
            "--write-description",
            "--write-thumbnail",
            "--no-playlist",
            "--write-sub",
            "--sub-lang",
            sub_lang,
            "-o",
            &filepath,
            &data.url,
        ])
        .status();

    if output.is_err() {
        let _ = sender.send(TaskResult::Err(task_id, -501));
        return;
    }

    // Move the files to destination

    // And finally, return
    let _ = sender.send(TaskResult::Ok(task_id));
}
