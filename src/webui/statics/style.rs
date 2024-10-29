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
    padding-top: 20px;
    padding-bottom: 50px;
}
"#;
