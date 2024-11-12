//! Style.css, default stylesheet

use rocket::get;
use rocket::http::ContentType;
use rocket::response::content;

#[get("/style.css")]
pub async fn style_css() -> (ContentType, &'static str) {
    // let conn = db_pool.get().expect("Failed to get DB connection");

    (ContentType::CSS, CSS_STYLE)
}

const CSS_STYLE: &'static str = r#"
/* style.css - Dark Theme with Bluish Accent */

* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

body, html {
    height: 100%;
    font-family: Arial, sans-serif;
    background-color: #1a1a1a; /* Dark background for the entire page */
    color: #e0e0e0; /* Light text color */
}

.container {
    display: grid;
    grid-template-areas:
        "top-bar top-bar"
        "side-bar content";
    grid-template-columns: 200px 1fr;
    grid-template-rows: 60px 1fr;
    height: 100vh;
}

/* Header - Top Bar */
.top-bar {
    grid-area: top-bar;
    background-color: #222; /* Darker background for top bar */
    color: #e0e0e0;
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0 20px;
    border-bottom: 1px solid #333; /* Border to separate top bar */
}

.logo {
    font-size: 1.5em;
    font-weight: bold;
    color: #b0c7ff; /* Bluish accent color for logo */
}

.user-button {
    cursor: pointer;
    padding: 10px;
    background-color: #2a2f3b; /* Slightly darker background for button */
    color: #b0c7ff; /* Bluish accent for text */
    border: none;
    border-radius: 4px;
    text-align: center;
    transition: background-color 0.3s;
}

.user-button:hover {
    background-color: #3a4b66; /* Bluish hover effect for user button */
}

/* Sidebar Navigation */
.side-bar {
    grid-area: side-bar;
    background-color: #1c1e26; /* Dark sidebar background */
    color: #e0e0e0;
    padding: 20px;
    border-right: 1px solid #333;
}

.side-bar nav ul {
    list-style: none;
}

.side-bar nav ul li {
    margin-bottom: 15px;
}

.side-bar nav ul li a {
    color: #b0c7ff; /* Bluish accent color for links */
    text-decoration: none;
    font-size: 1.1em;
    transition: color 0.3s;
}

.side-bar nav ul li a:hover {
    color: #8aa7ff; /* Slightly lighter blue on hover */
    text-decoration: underline;
}

/* Main Content Area */
.content {
    grid-area: content;
    padding: 20px;
    background-color: #1f2029; /* Dark background for main content */
    overflow-y: auto;
    color: #e0e0e0;
}

.content ul {
    padding-left: 20px;
}


/* Scrollbar Styling (optional, for dark theme consistency) */
.content::-webkit-scrollbar {
    width: 8px;
}

.content::-webkit-scrollbar-thumb {
    background-color: #3a4b66; /* Bluish scrollbar thumb */
    border-radius: 4px;
}

.content::-webkit-scrollbar-track {
    background-color: #2a2a2a; /* Dark scrollbar track */
}

.section {
    padding: 20px 0;
    border-bottom: 1px solid #333;
}

/* Form Styling */
form {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 10px;
}

form label {
    color: #e0e0e0;
    font-size: 1em;
}

form input[type="text"] {
    padding: 8px;
    font-size: 1em;
    border-radius: 4px;
    border: 1px solid #444;
    background-color: #2a2a2a;
    color: #e0e0e0;
}

form input[type="text"]:focus {
    outline: none;
    border-color: #8aa7ff; /* Bluish border on focus */
}

form button {
    padding: 8px 16px;
    font-size: 1em;
    color: #e0e0e0;
    background-color: #3a4b66;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    transition: background-color 0.3s;
}

form button:hover {
    background-color: #4a5b76; /* Slightly lighter blue on hover */
}

/* Channel List Styling */
#channels-list {
    list-style-type: none;
    padding: 0;
}

#channels-list li {
    padding: 15px;
    margin: 10px 0;
    border: 1px solid #333;
    border-radius: 5px;
    background-color: #2a2f3b;
    color: #e0e0e0;
}

#channels-list li a {
    color: #8aa7ff; /* Bluish accent for clickable channel names */
    text-decoration: none;
    font-weight: bold;
}

#channels-list li a:hover {
    text-decoration: underline;
}

/* Task List Styling (For Reference) */
#task-list {
    list-style-type: none;
    padding: 0;
}

.task-item {
    border: 1px solid #333;
    border-radius: 5px;
    padding: 15px;
    margin: 10px 0;
    background-color: #2a2f3b;
    color: #e0e0e0;
}

.task-header {
    font-weight: bold;
    display: flex;
    justify-content: space-between;
    color: #b0c7ff;
}

.task-details {
    margin-top: 10px;
    font-size: 0.9em;
}

.task-url a {
    color: #8aa7ff;
    text-decoration: none;
}

.task-url a:hover {
    text-decoration: underline;
}

.task-elapsed, .task-retries, .task-updated {
    display: block;
    margin-top: 5px;
    color: #d0d0d0;
}

/* Fetch All Videos Button */
#fetch-videos-button {
    padding: 10px 20px;
    font-size: 1em;
    background-color: #3a4b66;
    color: #e0e0e0;
    border: none;
    border-radius: 5px;
    cursor: pointer;
    transition: background-color 0.3s;
}

#fetch-videos-button:hover {
    background-color: #4a5b76;
}

/* Seasons Container */
#seasons-container {
    margin-top: 20px;
}

/* Styling for the <details> element */
details {
    margin-bottom: 20px;
    padding: 15px;
    border: 1px solid #333;
    border-radius: 5px;
    background-color: #2a2f3b;
    color: #e0e0e0;
    font-size: 1.1em;
}

details summary {
    font-size: 1.4em;
    font-weight: bold;
    color: #8aa7ff;
    margin-bottom: 10px;
}

details summary:hover {
    cursor: pointer;
}

/* Request Season Button */
details button {
    padding: 8px 16px;
    font-size: 0.9em;
    background-color: #3a4b66;
    color: #e0e0e0;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    transition: background-color 0.3s;
    margin-bottom: 15px;
}

details button:hover {
    background-color: #4a5b76;
}

/* Video List and Video Item Styling */
.videos-list {
    margin-top: 10px;
}

.video-item {
    padding: 10px;
    margin-bottom: 10px;
    border: 1px solid #444;
    border-radius: 5px;
    background-color: #1e2330;
    color: #d0d0d0;
}

.video-item a {
    color: #8aa7ff;
    text-decoration: none;
    font-weight: bold;
}

.video-item a:hover {
    text-decoration: underline;
}

.video-item p {
    margin: 5px 0;
}

/* Request Video Button */
.video-item button {
    padding: 6px 12px;
    font-size: 0.9em;
    background-color: #4a5b76;
    color: #e0e0e0;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    transition: background-color 0.3s;
}

.video-item button:hover {
    background-color: #5a6b86;
}

/* Scrollbar Styling (optional, for dark theme consistency) */
details::-webkit-scrollbar, .videos-list::-webkit-scrollbar {
    width: 8px;
}

details::-webkit-scrollbar-thumb, .videos-list::-webkit-scrollbar-thumb {
    background-color: #3a4b66;
    border-radius: 4px;
}

details::-webkit-scrollbar-track, .videos-list::-webkit-scrollbar-track {
    background-color: #2a2a2a;
}
    
"#;
