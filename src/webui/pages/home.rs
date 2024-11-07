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
<div class=section>
    <h1>Download video</h1>
    <form action="/api/task" method="post">
        <label for="url">URL to download:</label>
        <input type="text" id="url" name="url" required>
        <input type="hidden" name="typ" value="DL-VIDEO">
        <button type="submit">Download</button>
    </form>
</div>


<div class=section>
    <h2>Running Tasks</h2>
    <ul id="task-list">
        <!-- Task items will be populated here by JavaScript -->
    </ul>
</div>

<script>
async function fetchTasks() {
    try {
        const response = await fetch('/api/tasks');
        if (!response.ok) throw new Error('Network response was not ok');
        
        const tasks = await response.json();
        const taskList = document.getElementById('task-list');
        taskList.innerHTML = ''; // Clear the existing list

        tasks.forEach(task => {
            const li = document.createElement('li');
            li.textContent = `Task ID: ${task.id}, Type: ${task.task_type}, State: ${task.task_state}`;
            taskList.appendChild(li);
        });
    } catch (error) {
        console.error('Error fetching tasks:', error);
    }
}

// Poll the API every 5 seconds
// setInterval(fetchTasks, 5000);
fetchTasks(); // Initial fetch
</script>
"#;
