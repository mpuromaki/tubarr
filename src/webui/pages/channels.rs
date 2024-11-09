//! channels page

use crate::DBPool;
use rocket::http::ContentType;
use rocket::{get, State};
use tracing::{debug, error, event, info, info_span, span, trace, warn, Level};

use super::render_page;

#[get("/channels")]
pub async fn get_channels(db_pool: &State<DBPool>) -> (ContentType, String) {
    // let conn = db_pool.get().expect("Failed to get DB connection");

    let page = render_page("", HTML_CHANNELS);

    (ContentType::HTML, page)
}

const HTML_CHANNELS: &'static str = r#"
<div class="section">
    <h1>Add Channel</h1>
    <form action="/api/channel" method="post">
        <label for="url">URL to add:</label>
        <input type="text" id="url" name="url" required>
        <button type="submit">Add</button>
    </form>
    <p>This will add a background task to fetch the channel's information.</p>
</div>

<div class="section">
    <h2>Followed Channels</h2>
    <ul id="channels-list">
        <!-- Channel items will be populated here by JavaScript -->
    </ul>
</div>

<script>
// Helper function to URL-encode the channel name
function encodeChannelName(name) {
    return encodeURIComponent(name.toLowerCase());
}

// Fetch channels and update the list every 5 seconds
async function fetchChannels() {
    try {
        const response = await fetch('/api/channels');
        if (!response.ok) throw new Error('Network response was not ok');
        
        const channels = await response.json();
        const channelsList = document.getElementById('channels-list');
        channelsList.innerHTML = ''; // Clear the existing list

        channels.forEach(channel => {
            const li = document.createElement('li');
            const encodedName = encodeChannelName(channel.channel_name);
            const link = `/channels/${channel.domain}/${encodedName}`;

            li.innerHTML = `<a href="${link}">
                                <strong>${channel.channel_name}</strong> 
                            </a> 
                            (ID: ${channel.channel_id})`;
            channelsList.appendChild(li);
        });
    } catch (error) {
        console.error('Error fetching channels:', error);
    }
}

// Poll the API every 5 seconds
// setInterval(fetchChannels, 5000);
fetchChannels(); // Initial fetch to load channels right away
</script>
"#;

#[get("/channels/<domain>/<channel>")]
pub async fn get_channel_videos(
    domain: String,
    channel: String,
    _db_pool: &State<DBPool>,
) -> (ContentType, String) {
    let span = span!(Level::DEBUG, "get_channel_videos");
    let _enter = span.enter();

    debug!("Domain: {}", domain);
    debug!("Channel: {}", channel);

    // Render page with dynamic placeholders for JavaScript
    let page_content = HTML_CHANNEL_VIDEOS
        .replace("{{DOMAIN}}", &domain)
        .replace("{{CHANNEL}}", &channel);

    (ContentType::HTML, render_page("", &page_content))
}

// HTML template for channel videos page
const HTML_CHANNEL_VIDEOS: &str = r#"
<div class="section">
    <h1>Channel Videos - {{CHANNEL}}</h1>
    <div id="seasons-container">
        <!-- Videos grouped by season will be populated here by JavaScript -->
    </div>
</div>

<script>
async function fetchVideos() {
    const domain = "{{DOMAIN}}";
    const channel = "{{CHANNEL}}";
    const response = await fetch(`/api/videos/${domain}/${channel}`);
    if (!response.ok) {
        console.error("Failed to fetch videos");
        return;
    }

    const videos = await response.json();
    const seasonsContainer = document.getElementById("seasons-container");
    seasonsContainer.innerHTML = ""; // Clear any existing content

    // Group videos by season
    const videosBySeason = {};
    videos.forEach(video => {
        const season = video.season || "unknown"; // Fallback to "unknown" if season is not defined
        if (!videosBySeason[season]) {
            videosBySeason[season] = [];
        }
        videosBySeason[season].push(video);
    });

    // Create HTML for each season and its videos
    for (const [season, videos] of Object.entries(videosBySeason)) {
        const seasonDiv = document.createElement("div");
        seasonDiv.classList.add("season");
        seasonDiv.innerHTML = `<h2>Season: ${season}</h2>`;

        const videoList = document.createElement("div");
        videoList.classList.add("videos-list");

        videos.forEach(video => {
            const videoDiv = document.createElement("div");
            videoDiv.classList.add("video-item");
            videoDiv.innerHTML = `
                <a href="${video.url}" target="_blank">
                    <strong>${video.name}</strong>
                </a>
                <p>Release Date: ${video.release_date}</p>
                <p>Requested: ${video.is_requested ? "Yes" : "No"}, Downloaded: ${video.is_downloaded ? "Yes" : "No"}</p>
            `;
            videoList.appendChild(videoDiv);
        });

        seasonDiv.appendChild(videoList);
        seasonsContainer.appendChild(seasonDiv);
    }
}

// Initial fetch of videos
fetchVideos();
</script>
"#;
