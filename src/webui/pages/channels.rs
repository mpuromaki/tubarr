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
<div class=section>
    <h1>Add Channel</h1>
    <form action="/api/channel" method="post">
        <label for="url">URL to add:</label>
        <input type="text" id="url" name="url" required>
        <button type="submit">Add</button>
    </form>
</div>


<div class=section>
    <h2>Followed channels</h2>
    <p>To be implemented..</p>
</div>
"#;
