//! API endpoints for TASKS

use std::collections::HashMap;

use rocket::{form::Form, post, response::Redirect, FromForm};
use serde::{Deserialize, Serialize};

use crate::DBPool;

#[derive(FromForm, Deserialize, Serialize)]
struct FromPostTask {
    url: String,
    typ: String,
}

#[post("/task", data = "<data>")]
pub async fn post_task(data: Form<FromPostTask>, db_pool: &rocket::State<DBPool>) -> Redirect {
    let conn = db_pool.get().expect("Failed to get DB connection");

    let mut outgoing = HashMap::with_capacity(1);
    outgoing.insert("url".to_owned(), data.url.clone());

    conn.execute(
        "INSERT INTO tasks (task_type, task_data, task_state) VALUES (?1, ?2, ?3)",
        [
            data.typ.clone(),
            serde_json::to_string(&outgoing).unwrap(),
            "WAIT".to_string(),
        ],
    )
    .expect("Could not write to db.");

    Redirect::to("/")
}
