use chrono::{NaiveDate, NaiveDateTime};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::{collections::HashMap, process::Command};
use std::{fs::create_dir_all, thread, time};
use tldextract::{TldExtractor, TldOption};
use tracing::{debug, error, event, info, info_span, span, trace, warn, Level};

use crate::DBPool;

use super::{move_files_with_prefix, parse_domain, TaskResult};

/// Worker for DOWNLOAD tasks.
pub fn worker(
    task_id: isize,
    data: String,
    conf: Arc<HashMap<String, String>>,
    sender: Sender<TaskResult>,
    dbp: DBPool,
) {
    debug!("task_download::worker() started for task {}", task_id);

    // Unpack data
    let data: TaskDownloadData = match serde_json::from_str(&data) {
        Ok(data) => data,
        Err(_) => {
            let _ = sender.send(TaskResult::Err(task_id, -500));
            return;
        }
    };

    let domain = parse_domain(&data.url);

    let path_tmp = conf
        .get("path_temp")
        .expect("Could not get configuration: path_temp");
    let path_media = conf
        .get("path_media")
        .expect("Could not get configuration: path_media");
    let sub_lang = conf
        .get("sub_lang")
        .expect("Could not get configuration: sub_lang");

    // Resolve file name with yt-dlp
    let filename_template = "%(channel_id)s SPLITATTHISPOINT %(channel)s SPLITATTHISPOINT %(upload_date)s SPLITATTHISPOINT %(title)s SPLITATTHISPOINT %(id)s";
    let url_no_list_query = match &data.url.split_once("&list") {
        Some(url) => url.0,
        None => &data.url.clone(),
    };

    let filename_output = Command::new("yt-dlp")
        .args([
            "--print",
            "filename",
            "-o",
            &filename_template,
            url_no_list_query,
        ])
        .output();

    if filename_output.is_err() {
        let _ = sender.send(TaskResult::Err(task_id, -502));
        return;
    }
    let filename_output = filename_output.unwrap();
    let filename_parts = match std::str::from_utf8(&filename_output.stdout) {
        Ok(name) => name.trim().to_string(),
        Err(_) => {
            let _ = sender.send(TaskResult::Err(task_id, -503));
            return;
        }
    };
    debug!("FILENAME_PARTS: {}", filename_parts);

    let mut channel_id = Some(
        filename_parts
            .split("SPLITATTHISPOINT")
            .nth(0)
            .unwrap()
            .trim()
            .to_owned(),
    );
    if channel_id == Some("NA".to_string()) {
        channel_id = None;
    } else {
        channel_id = channel_id;
    }

    let mut channel_name = Some(
        filename_parts
            .split("SPLITATTHISPOINT")
            .nth(1)
            .unwrap()
            .trim()
            .to_owned(),
    );
    if channel_name == Some("NA".to_string()) {
        channel_name = None;
    } else {
        channel_name = channel_name;
    }

    let mut upload_date = Some(
        filename_parts
            .split("SPLITATTHISPOINT")
            .nth(2)
            .unwrap()
            .trim()
            .to_owned(),
    );
    if upload_date == Some("NA".to_string()) {
        upload_date = None;
    } else {
        upload_date = upload_date;
    }

    let mut year = Some(
        filename_parts
            .split("SPLITATTHISPOINT")
            .nth(2)
            .unwrap()
            .trim()
            .to_owned(),
    );
    if year == Some("NA".to_string()) {
        year = None;
    } else {
        year = Some(year.unwrap().trim()[..4].to_string());
    }

    let mut title = Some(
        filename_parts
            .split("SPLITATTHISPOINT")
            .nth(3)
            .unwrap()
            .trim()
            .to_owned(),
    );
    if title == Some("NA".to_string()) {
        title = None;
    } else {
        title = title;
    }

    let mut video_id = Some(
        filename_parts
            .split("SPLITATTHISPOINT")
            .nth(4)
            .unwrap()
            .trim()
            .to_owned(),
    );
    if video_id == Some("NA".to_string()) {
        video_id = None;
    } else {
        video_id = video_id;
    }

    let mut filename = "".to_string();
    if let Some(txt) = &channel_name {
        filename.push_str(txt);
        filename.push_str(" - ");
    }
    if let Some(txt) = &upload_date {
        filename.push_str(txt);
        filename.push_str(" - ");
    }
    if let Some(txt) = &title {
        filename.push_str(txt);
        filename.push_str(" - ");
    }
    if let Some(txt) = &video_id {
        filename.push_str(txt);
    }

    debug!("FILENAME: {:?}", filename);

    // Download the files with yt-dlp
    let filename_template = match (&channel_name, &year) {
        (Some(_), Some(_)) => "%(channel)s - %(upload_date)s - %(title)s - %(id)s.%(ext)s",
        (Some(_), None) => "%(channel)s - %(title)s - %(id)s.%(ext)s",
        (None, Some(_)) => "%(upload_date)s - %(title)s - %(id)s.%(ext)s",
        (None, None) => "%(title)s - %(id)s.%(ext)s",
    };
    let filepath = format!("{}/{}", path_tmp, filename_template);

    // Refer: https://github.com/yt-dlp/yt-dlp/issues/630#issuecomment-893659460
    let output = Command::new("yt-dlp")
        .args([
            "--no-playlist",
            "--add-metadata",
            "--embed-metadata",
            "--write-thumbnail",
            "--convert-thumbnails",
            "jpg",
            "--write-subs",
            "--write-auto-subs",
            "--convert-subs",
            "srt",
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
    path_media_full.push(&domain);
    if channel_name.is_some() {
        path_media_full.push(channel_name.unwrap());
    }
    if year.is_some() {
        path_media_full.push(year.unwrap());
    } else {
        path_media_full.push("other");
    }
    let _ = create_dir_all(&path_media_full);

    debug!("PATH_MEDIA_FULL: {:?}", path_media_full);

    // Move the files
    let path_tmp = PathBuf::from(path_tmp);
    let _ = move_files_with_prefix(&path_tmp, &path_media_full, &filename);

    // Record this as known video
    let span = span!(Level::DEBUG, "Insert video");
    let _enter = span.enter();
    if let Ok(conn) = dbp.get() {
        let channel_id: Option<i64> = conn
            .query_row(
                "SELECT id FROM channels WHERE domain = ?1 AND channel_id = ?2",
                params![domain, channel_id],
                |row| row.get(0),
            )
            .optional()
            .ok()
            .flatten();
        debug!("Channel ID: {:?}", channel_id);

        let mut naive_datetime_upload: Option<NaiveDateTime> = None;
        if upload_date.is_some() {
            let naive_date_upload = NaiveDate::parse_from_str(&upload_date.unwrap(), "%Y%m%d")
                .expect("Could not parse datetime");
            naive_datetime_upload = Some(naive_date_upload.and_hms(0, 0, 0));
        }

        conn.execute(
        "INSERT OR REPLACE INTO videos (channel_id, domain, url, name, video_id, release_date, release_date_estimate, is_requested, is_downloaded)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            channel_id,
            domain,
            &data.url,
            title,
            video_id,
            naive_datetime_upload,
            naive_datetime_upload,
            true,
            true,
        ]).unwrap();
    }
    drop(_enter);

    // And finally, return
    let _ = sender.send(TaskResult::Ok(task_id));
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TaskDownloadData {
    pub url: String,
}
