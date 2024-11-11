//! Read tasks from database and execute local yt-dlp binary based on those.
//!
//! New worker thread will be spawned for each task. That thread is responsible for:
//! - Updating the task status to the DB.
//! - Downloading the file with yt-dlp. ("DOWNLOAD" task)
//! - Moving the file to the correct location after download is complete. ("DOWNLOAD" task)
//!
//! Output folder structure:
//! <PATH_MEDIA>/<CHANNEL>/<YYYY>/<FILENAME>

use anyhow::Result;
use core::{str, time};
use rusqlite::params;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::mpsc::channel;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread;
use tldextract::{TldExtractor, TldOption};
use tracing::{debug, error, event, info, trace, warn};

use super::DBPool;
use super::FLAG_SHUTDOWN;

mod task_channel;
mod task_download;

pub fn run(dbp: DBPool) {
    let (result_tx, result_rx): (Sender<TaskResult>, Receiver<TaskResult>) = channel();
    let conf = match get_configuration(dbp.clone()) {
        Ok(conf) => Arc::from(conf),
        Err(_) => {
            error!("Could not load configuration");
            FLAG_SHUTDOWN.store(true, std::sync::atomic::Ordering::Relaxed);
            return;
        }
    };

    let concurrency_limit = 3;
    let mut concurrency = 0;

    loop {
        // Sleep so we don't trash the CPU
        thread::sleep(time::Duration::from_secs(1));

        // Check for tasks
        if concurrency < concurrency_limit {
            if let Ok(new_tasks) = get_new_tasks(dbp.clone()) {
                for task in new_tasks {
                    match task.task_type.as_str() {
                        "DL-VIDEO" => {
                            debug!("DL-VIDEO: {:?}", task);
                            concurrency += 1;
                            mark_task_wip(dbp.clone(), task.task_id);
                            let thrd_conf = conf.clone();
                            let thrd_tx = result_tx.clone();
                            let thrd_dbp = dbp.clone();
                            thread::spawn(move || {
                                task_download::worker(
                                    task.task_id,
                                    task.task_data,
                                    thrd_conf,
                                    thrd_tx,
                                    thrd_dbp,
                                )
                            });
                        }
                        "CHANNEL-ADD" => {
                            debug!("CHANNEL-ADD: {:?}", task);
                            concurrency += 1;
                            mark_task_wip(dbp.clone(), task.task_id);
                            let thrd_conf = conf.clone();
                            let thrd_tx = result_tx.clone();
                            let thrd_dbp = dbp.clone();
                            thread::spawn(move || {
                                task_channel::add(
                                    task.task_id,
                                    task.task_data,
                                    thrd_conf,
                                    thrd_tx,
                                    thrd_dbp,
                                )
                            });
                        }
                        "CHANNEL-FETCH" => {
                            debug!("CHANNEL-FETCH: {:?}", task);
                            concurrency += 1;
                            mark_task_wip(dbp.clone(), task.task_id);
                            let thrd_conf = conf.clone();
                            let thrd_tx = result_tx.clone();
                            let thrd_dbp = dbp.clone();
                            thread::spawn(move || {
                                task_channel::fetch(
                                    task.task_id,
                                    task.task_data,
                                    thrd_conf,
                                    thrd_tx,
                                    thrd_dbp,
                                )
                            });
                        }
                        _ => {
                            error!("Unknown task type: {:?}", task);
                            mark_task_error(dbp.clone(), task.task_id);
                        }
                    }
                }
            }
        }

        // Check for worker reports, update tasks state
        while let Ok(result) = result_rx.try_recv() {
            concurrency -= 1;
            debug!("TASK RESULT: {:?}", result);
            match result {
                TaskResult::Ok(id) => mark_task_done(dbp.clone(), id),
                TaskResult::Err(id, errcode) => mark_task_error(dbp.clone(), id),
            }
        }

        // Check for shutdown signal
        if FLAG_SHUTDOWN.load(std::sync::atomic::Ordering::Relaxed) == true {
            info!("Shutdown requested");
            break;
        }
    }

    // Shutdown, bye!
    // Do we need to join tasks, they can run for long time?
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
        .query_map(params!["WAIT"], |row| {
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
            error!("Error updating task state: {}", e); // Log the error
            return;
        }

        debug!("Task with ID {} marked as WIP.", task_id);
    } else {
        error!("Error getting database connection."); // Log connection error
    }
}

fn mark_task_done(dbp: DBPool, task_id: isize) {
    if let Ok(conn) = dbp.get() {
        // Prepare the SQL statement to update the task
        if let Err(e) = conn.execute(
            "UPDATE tasks SET task_state = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            params!["DONE", task_id],
        ) {
            error!("Error updating task state: {}", e); // Log the error
            return;
        }

        debug!("Task with ID {} marked as DONE.", task_id);
    } else {
        error!("Error getting database connection."); // Log connection error
    }
}

fn mark_task_error(dbp: DBPool, task_id: isize) {
    if let Ok(conn) = dbp.get() {
        // Prepare the SQL statement to update the task
        if let Err(e) = conn.execute(
            "UPDATE tasks SET task_state = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            params!["ERR", task_id],
        ) {
            error!("Error updating task state: {}", e); // Log the error
            return;
        }

        debug!("Task with ID {} marked as ERROR.", task_id);
    } else {
        error!("Error getting database connection."); // Log connection error
    }
}

/// Result of task. Payload is the ID of the task.
#[derive(Debug)]
pub enum TaskResult {
    Ok(isize),         // ID
    Err(isize, isize), // ID, ERRCODE
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
                        info!(
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

fn parse_domain(url: &str) -> String {
    let tldopt = TldOption::default();
    let extractor = TldExtractor::new(tldopt);
    let extracted = extractor.extract(url).expect("Could not extract domain");
    let domain = format!(
        "{}.{}",
        extracted.domain.unwrap(),
        extracted.suffix.unwrap()
    );
    domain
}
