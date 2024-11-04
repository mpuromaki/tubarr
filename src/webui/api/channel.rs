//! API endpoints for channelS

use std::collections::HashMap;

use rocket::{form::Form, get, post, response::Redirect, serde::json::Json, FromForm, State};
use serde::{Deserialize, Serialize};

use crate::DBPool;

#[derive(FromForm, Deserialize, Serialize)]
struct PostFormChannel {
    url: String,
}

#[post("/channel", data = "<data>")]
pub async fn post_channel(data: Form<PostFormChannel>, db_pool: &State<DBPool>) -> Redirect {
    let conn = db_pool.get().expect("Failed to get DB connection");

    // Parse payload
    let mut outgoing = HashMap::with_capacity(1);
    outgoing.insert("url".to_owned(), data.url.clone());

    // Create task
    conn.execute(
        "INSERT INTO tasks (task_type, task_data, task_state) VALUES (?1, ?2, ?3)",
        [
            "CHANNEL-ADD",
            &serde_json::to_string(&outgoing).unwrap(),
            "WAIT",
        ],
    )
    .expect("Could not write to db.");

    Redirect::to("/channels")
}

#[derive(Serialize)]
struct Channel {
    id: i32,
    domain: String,
    url: String,
    channel_id: String,
    channel_name: String,
    updated_at: String,
}

#[get("/channels")]
pub async fn get_channels(db_pool: &State<DBPool>) -> Json<Vec<Channel>> {
    let conn = db_pool.get().expect("Failed to get DB connection");

    let mut stmt = conn
        .prepare("SELECT id, domain, url, channel_id, channel_name, updated_at FROM channels")
        .expect("Failed to prepare statement");

    let channels = stmt
        .query_map([], |row| {
            Ok(Channel {
                id: row.get(0)?,
                domain: row.get(1)?,
                url: row.get(2)?,
                channel_id: row.get(3)?,
                channel_name: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .expect("Failed to query channels")
        .map(|channel| channel.expect("Failed to map channel"))
        .collect();

    Json(channels)
}
