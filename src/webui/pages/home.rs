//! Home page

use crate::DBPool;
use rocket::get;
use rocket::http::ContentType;

use super::render_page;

#[get("/")]
pub async fn get_home(db_pool: &rocket::State<DBPool>) -> (ContentType, String) {
    // let conn = db_pool.get().expect("Failed to get DB connection");

    let page = render_page("", HTML_HOME);

    (ContentType::HTML, page)
}

const HTML_HOME: &'static str = r#"
<div class="section">
    <h1>Add Channel:</h1>
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

<div class="section">
    <h2>Tasks</h2>
    <ul id="task-list">
        <!-- Task items will be populated here by JavaScript -->
    </ul>
</div>
<script>
function formatElapsedTime(createdAt, updatedAt, taskState) {
    const createdTime = new Date(Date.parse(createdAt + "Z")); // Interpret as UTC
    let endTime;

    if (taskState.toLowerCase() === "done") {
        endTime = new Date(Date.parse(updatedAt + "Z")); // Use updated time for completed tasks
    } else {
        endTime = new Date(); // Use current time for ongoing tasks
    }

    const elapsedMs = endTime - createdTime;
    const seconds = Math.floor(elapsedMs / 1000) % 60;
    const minutes = Math.floor(elapsedMs / (1000 * 60)) % 60;
    const hours = Math.floor(elapsedMs / (1000 * 60 * 60));
    
    if (hours > 0) {
        return `${hours}h ${minutes}m`;
    } else if (minutes > 0) {
        return `${minutes}m ${seconds}s`;
    } else {
        return `${seconds}s`;
    }
}

async function fetchTasks() {
    try {
        const response = await fetch('/api/tasks');
        if (!response.ok) throw new Error('Network response was not ok');
        
        const tasks = await response.json();
        const taskList = document.getElementById('task-list');
        taskList.innerHTML = ''; // Clear the existing list

        tasks
            .filter(task => task.task_type === "VIDEO-DOWNLOAD" && task.task_data.includes('"url":'))
            .sort((a, b) => new Date(b.updated_at) - new Date(a.updated_at)) // Sort by updated_at, latest first
            .forEach(task => {
                const taskData = JSON.parse(task.task_data);
                
                const li = document.createElement('li');
                li.className = 'task-item';

                const url = taskData.url || 'URL not available';
                const elapsed = formatElapsedTime(task.created_at, task.updated_at, task.task_state);

                li.innerHTML = `
                    <div class="task-header">
                        <span class="task-id">Task ID: ${task.id}</span>
                        <span class="task-type">Type: ${task.task_type}</span>
                        <span class="task-state">State: ${task.task_state}</span>
                    </div>
                    <div class="task-details">
                        <span class="task-url">URL: <a href="${url}" target="_blank">${url}</a></span>
                        <span class="task-retries">Retry Count: ${task.retry_count}</span>
                        <span class="task-elapsed">Elapsed: ${elapsed}</span>
                        <span class="task-updated">Last Updated: ${new Date(task.updated_at).toLocaleString()}</span>
                    </div>
                `;

                taskList.appendChild(li);
            });
    } catch (error) {
        console.error('Error fetching tasks:', error);
    }
}

// Poll the API every 5 seconds
setInterval(fetchTasks, 5000);
fetchTasks(); // Initial fetch
</script>

"#;
