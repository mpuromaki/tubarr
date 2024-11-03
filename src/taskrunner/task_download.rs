use serde::{Deserialize, Serialize};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::{collections::HashMap, process::Command};
use std::{fs::create_dir_all, thread, time};
use std::{path::PathBuf, sync::mpsc::channel};
use tldextract::{TldExtractor, TldOption};

use super::{move_files_with_prefix, TaskResult};

/// Worker for DOWNLOAD tasks.
pub fn worker(
    task_id: isize,
    data: String,
    conf: Arc<HashMap<String, String>>,
    sender: Sender<TaskResult>,
) {
    println!("task_download::worker started for task {}", task_id);

    // Unpack data
    let data: TaskDownloadData = match serde_json::from_str(&data) {
        Ok(data) => data,
        Err(_) => {
            let _ = sender.send(TaskResult::Err(task_id, -500));
            return;
        }
    };

    let domain = parse_domain(&data);

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
    let filename_template = "%(channel)s SPLITATTHISPOINT %(upload_date)s SPLITATTHISPOINT %(title)s SPLITATTHISPOINT (%(id)s)";

    let filename_output = Command::new("yt-dlp")
        .args(["--print", "filename", "-o", &filename_template, &data.url])
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

fn parse_domain(data: &TaskDownloadData) -> String {
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
    domain
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TaskDownloadData {
    pub url: String,
}
