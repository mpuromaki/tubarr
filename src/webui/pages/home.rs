//! Home page

use crate::DBPool;
use rocket::get;
use rocket::http::ContentType;
use rocket::response::content;

#[get("/")]
pub async fn get_home(db_pool: &rocket::State<DBPool>) -> (ContentType, &'static str) {
    // let conn = db_pool.get().expect("Failed to get DB connection");

    (ContentType::HTML, HTML_HOME)
}

const HTML_HOME: &'static str = r#"
<!DOCTYPE html>
<html lang="en">

<head>
    <meta charset="UTF-8">
    <title>Download Task</title>
    <link rel="stylesheet" href="static/style.css" type="text/css">
</head>

<body>
    <h1>Download Task</h1>
    <form action="/api/task" method="post">
        <label for="url">URL to download:</label>
        <input type="text" id="url" name="url" required>
        <input type="hidden" name="typ" value="DL-VIDEO">
        <button type="submit">Download</button>
    </form>
</body>

</html>
"#;
