use rocket::{get, serde::json::Json, State};
use serde::Serialize;
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

    debug!("Found: {:?}", videos);

    Json(videos)
}
