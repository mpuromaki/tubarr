//! API endpoints for TASKS

use std::collections::HashMap;

use rocket::{form::Form, get, post, response::Redirect, serde::json::Json, FromForm, State};
use serde::{Deserialize, Serialize};

use crate::DBPool;

#[derive(FromForm, Deserialize, Serialize)]
struct FromPostTask {
    url: String,
    typ: String,
}

#[post("/task", data = "<data>")]
pub async fn post_task(data: Form<FromPostTask>, db_pool: &State<DBPool>) -> Redirect {
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

#[derive(Serialize, Deserialize)]
struct Task {
    id: i64,
    task_type: String,
    task_data: String,
    task_state: String,
    retry_count: i32,
    created_at: String,
    updated_at: String,
}

#[get("/tasks")]
pub async fn get_tasks(db_pool: &State<DBPool>) -> Json<Vec<Task>> {
    let conn = db_pool.get().expect("Failed to get DB connection");

    let mut stmt = conn.prepare("SELECT id, task_type, task_data, task_state, retry_count, created_at, updated_at FROM tasks")
        .expect("Failed to prepare statement");

    let tasks = stmt
        .query_map([], |row| {
            Ok(Task {
                id: row.get(0)?,
                task_type: row.get(1)?,
                task_data: row.get(2)?,
                task_state: row.get(3)?,
                retry_count: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })
        .expect("Failed to query tasks")
        .map(|task| task.expect("Failed to map task"))
        .collect();

    Json(tasks)
}
