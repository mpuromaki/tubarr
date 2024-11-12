use std::collections::HashMap;

use rocket::{get, http::Status, post, serde::json::Json, State};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, event, info, info_span, span, trace, warn, Level};

use crate::DBPool;

#[derive(Debug, Serialize)]
struct Video {
    video_id: i32,        // Row ID of the video
    channel_id: i32,      // Row ID of the channel
    url: String,          // URL of the video
    name: String,         // Name of the video
    is_requested: i32,    // is_requested of the video
    is_downloaded: i32,   // is_downloaded of the video
    release_date: String, // Actual release date or release_date_estimate if release_date is NULL
    season: String,       // Year, or some other string in some cases
    updated_at: String,   // Last update timestamp
}

#[get("/videos/<domain>/<channel>")]
pub async fn get_videos(
    domain: String,
    channel: String,
    db_pool: &State<DBPool>,
) -> Json<Vec<Video>> {
    let span = span!(Level::DEBUG, "get_videos");
    let _enter = span.enter();
    debug!("Domain: {}", domain);
    debug!("Channel: {}", channel);

    let conn = db_pool.get().expect("Failed to get DB connection");

    let mut stmt = conn
        .prepare(
            "SELECT v.id, v.channel_id, v.url, v.name, v.is_requested, 
                    v.is_downloaded, 
                    COALESCE(v.release_date, v.release_date_estimate) as release_date,
                    COALESCE(strftime('%Y', COALESCE(v.release_date, v.release_date_estimate)), 'unknown') as season,
                    v.updated_at
             FROM videos v 
             JOIN channels c ON v.channel_id = c.id 
             WHERE c.domain = ?1 AND c.channel_name_normalized = LOWER(?2)"
        )
        .expect("Failed to prepare statement");

    let videos = stmt
        .query_map([&domain, &channel], |row| {
            Ok(Video {
                video_id: row.get(0)?,      // Row ID of the video
                channel_id: row.get(1)?,    // Row ID of the channel
                url: row.get(2)?,           // URL of the video
                name: row.get(3)?,          // Name of the video
                is_requested: row.get(4)?,  // is_requested of the video
                is_downloaded: row.get(5)?, // is_downloaded of the video
                release_date: row.get(6)?,  // release_date or release_date_estimate
                season: row.get(7)?,        // Year as string or "unknown"
                updated_at: row.get(8)?,    // Last update timestamp
            })
        })
        .expect("Failed to query videos")
        .map(|video| video.expect("Failed to map video"))
        .collect();

    //debug!("Found: {:?}", videos);

    Json(videos)
}

#[derive(Deserialize, Serialize)]
struct FromPostVideo {
    url: String,
}

#[post("/video", format = "json", data = "<data>")]
pub async fn post_video(data: Json<FromPostVideo>, db_pool: &State<DBPool>) -> Status {
    let conn = db_pool.get().expect("Failed to get DB connection");

    // Check if the URL already exists in the `videos` table
    let mut stmt = conn
        .prepare("SELECT COUNT(*) FROM videos WHERE url = ?1")
        .expect("Failed to prepare statement");

    let exists: bool = stmt
        .query_row([&data.url], |row| {
            row.get::<_, i64>(0).map(|count| count > 0)
        })
        .expect("Failed to execute query");

    if exists {
        // Update the `is_requested` field to 1 for the existing video entry
        conn.execute(
            "UPDATE videos SET is_requested = 1 WHERE url = ?1",
            params![&data.url],
        )
        .expect("Failed to update video record");
    }

    // Prepare the data for inserting a new task in the tasks table
    let mut outgoing = HashMap::new();
    outgoing.insert("url".to_owned(), data.url.clone());

    // Insert a new row into the `tasks` table
    conn.execute(
        "INSERT INTO tasks (task_type, task_data, task_state) VALUES (?1, ?2, ?3)",
        params![
            "VIDEO-DOWNLOAD",
            serde_json::to_string(&outgoing).unwrap(),
            "WAIT"
        ],
    )
    .expect("Could not insert task into db");

    // Return a 200 OK response
    Status::Ok
}
