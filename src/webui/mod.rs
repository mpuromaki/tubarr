use anyhow::Result;
use rocket::{
    fairing::AdHoc,
    form::Form,
    fs::{relative, FileServer},
    post,
    response::Redirect,
    routes, FromForm, Ignite,
};
use rocket::{fs::Options, tokio};
use rocket::{http::ext::IntoCollection, response::content};
use rocket::{Build, Rocket, Shutdown, State};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::time::{self, sleep, Duration};

use super::DBPool;
use super::FLAG_SHUTDOWN;

mod api;
mod pages;
mod statics;

pub fn run(dbp: DBPool) {
    let rt = tokio::runtime::Runtime::new().expect("failed to create runtime");

    rt.block_on(async {
        rocket(dbp)
            .await
            .expect("Failed to build rocket")
            .launch()
            .await
            .expect("failed to launch rocket");
    });

    // Make sure others shutdown as well
    FLAG_SHUTDOWN.store(true, std::sync::atomic::Ordering::Relaxed);
}

// Rocket configuration and setup function
pub async fn rocket(dbp: DBPool) -> Result<Rocket<Ignite>> {
    // Launch Rocket and attach the shutdown monitor
    let rocket = rocket::build()
        .manage(dbp)
        .mount("/static", routes![statics::style_css])
        .mount(
            "/api",
            routes![
                api::post_task,
                api::get_tasks,
                api::post_channel,
                api::get_channels,
                api::get_videos,
                api::post_channel_fetch,
                api::post_video,
            ],
        )
        .mount("/", routes![pages::get_home, pages::get_channel_videos])
        .ignite()
        .await?;

    // Get a handle to Rocket's shutdown mechanism
    let shutdown_handle = rocket.shutdown();

    // Spawn the monitoring task
    tokio::spawn(monitor_shutdown(shutdown_handle.clone()));

    Ok(rocket)
}

async fn monitor_shutdown(shutdown_handle: Shutdown) {
    loop {
        sleep(Duration::from_millis(1500)).await;
        if FLAG_SHUTDOWN.load(Ordering::Relaxed) {
            shutdown_handle.notify();
            break;
        }
    }
}
