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

function normalizeChannelName(channelName) {
    // Replace apostrophes and other unwanted characters with empty string
    let normalized = channelName.replace(/['"]/g, ""); // Remove quotes
    normalized = normalized.replace(/\s+/g, "-"); // Replace spaces with hyphens
    normalized = normalized.toLowerCase(); // Optional: Normalize to lowercase
    return normalized;
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
            const encodedName = normalizeChannelName(channel.channel_name);
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
    db_pool: &State<DBPool>,
) -> (ContentType, String) {
    let span = span!(Level::DEBUG, "get_channel_videos");
    let _enter = span.enter();

    debug!("Domain: {}", domain);
    debug!("Channel: {}", channel);

    // Get additional data from DB
    let conn = db_pool.get().expect("Failed to get DB connection");
    let mut stmt = conn
        .prepare("SELECT channel_id FROM channels WHERE domain = ? AND channel_name_normalized = ?")
        .expect("Failed to prepare query");

    let channel_id: String = stmt
        .query_row(rusqlite::params![domain, channel], |row| row.get(0))
        .expect("Failed to retrieve channel_id");

    // Render page with dynamic placeholders for JavaScript
    let page_content = HTML_CHANNEL_VIDEOS
        .replace("{{DOMAIN}}", &domain)
        .replace("{{CHANNEL}}", &channel)
        .replace("{{CHANNEL_ID}}", &channel_id);

    (ContentType::HTML, render_page("", &page_content))
}

// HTML template for channel videos page
const HTML_CHANNEL_VIDEOS: &str = r#"
<div class="section">
    <h1>Channel Videos - {{CHANNEL}}</h1>
    
    <!-- Fetch All Videos Button -->
    <button id="fetch-videos-button" onclick="fetchAllVideos()">Fetch All Videos</button>
    
    <div id="seasons-container">
        <!-- Videos grouped by season will be populated here by JavaScript -->
    </div>
</div>

<script>
// Fetch videos grouped by season
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

    // Sort seasons in descending order by season number
    const sortedSeasons = Object.keys(videosBySeason)
        .sort((a, b) => {
            // Convert season numbers to integers and handle "unknown" case
            const seasonA = a === "unknown" ? -1 : parseInt(a);
            const seasonB = b === "unknown" ? -1 : parseInt(b);
            return seasonB - seasonA; // Descending order
        });

    // Create HTML for each season and its videos
    for (const season of sortedSeasons) {
        const seasonDiv = document.createElement("div");
        seasonDiv.classList.add("season");
        seasonDiv.innerHTML = `<h2>Season: ${season}</h2>
            <button onclick="requestSeason('${season}')">Request Season</button>`; // New Request Season Button

        const videoList = document.createElement("div");
        videoList.classList.add("videos-list");

        // Sort videos within each season by release date (newest first)
        videosBySeason[season].sort((a, b) => new Date(b.release_date) - new Date(a.release_date));

        videosBySeason[season].forEach(video => {
            const videoDiv = document.createElement("div");
            videoDiv.classList.add("video-item");
            videoDiv.innerHTML = `
                <a href="${video.url}" target="_blank">
                    <strong>${video.name}</strong>
                </a>
                <p>Release Date: ${video.release_date}</p>
                <p>Requested: ${video.is_requested ? "Yes" : "No"}, Downloaded: ${video.is_downloaded ? "Yes" : "No"}</p>
                ${!video.is_requested ? `<button onclick="requestVideo('${video.url}')">Request</button>` : ""}
            `;
            videoList.appendChild(videoDiv);
        });

        seasonDiv.appendChild(videoList);
        seasonsContainer.appendChild(seasonDiv);
    }
}

// Function to request all videos in a season
async function requestSeason(season) {
    const domain = "{{DOMAIN}}";
    const channel = "{{CHANNEL}}";
    
    // Fetch videos for the current channel to get videos by season
    const response = await fetch(`/api/videos/${domain}/${channel}`);
    if (!response.ok) {
        console.error("Failed to fetch videos");
        return;
    }

    const videos = await response.json();
    const videosInSeason = videos.filter(video => (video.season || "unknown") === season && !video.is_requested);

    if (videosInSeason.length === 0) {
        alert("All videos in this season have already been requested.");
        return;
    }

    // Send a request for each video in the season that hasn't been requested
    for (const video of videosInSeason) {
        await requestVideo(video.url);
    }

    alert(`Requested all videos in season ${season}`);
}

// Function to send a request to download a video
async function requestVideo(url) {
    try {
        const response = await fetch("/api/task", {
            method: "POST",
            headers: {
                "Content-Type": "application/x-www-form-urlencoded",
            },
            body: new URLSearchParams({
                url: url,
                typ: "DL-VIDEO"
            })
        });

        if (response.ok) {
            console.log("Request sent successfully");
            await fetchVideos(); // Refresh videos to reflect "Requested" status
        } else {
            console.error("Failed to request video download");
            alert("Failed to request download. Please try again.");
        }
    } catch (error) {
        console.error("Error in requestVideo:", error);
    }
}

// Function to fetch all videos
async function fetchAllVideos() {
    const domain = "{{DOMAIN}}";
    const channelId = "{{CHANNEL_ID}}";

    try {
        const response = await fetch("/api/channel/fetch", {
            method: "POST",
            headers: {
                "Content-Type": "application/x-www-form-urlencoded",
            },
            body: new URLSearchParams({
                domain: domain,
                channel_id: channelId
            })
        });

        if (response.ok) {
            console.log("Fetch request sent successfully");
            await fetchVideos(); // Update page content after fetching
        } else {
            console.error("Failed to send fetch request");
        }
    } catch (error) {
        console.error("Error in fetchAllVideos:", error);
    }
}

// Initial fetch of videos
fetchVideos();
</script>
"#;
