//! channels page

use crate::DBPool;
use rocket::get;
use rocket::http::ContentType;

use super::render_page;

#[get("/channels")]
pub async fn get_channels(db_pool: &rocket::State<DBPool>) -> (ContentType, String) {
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
