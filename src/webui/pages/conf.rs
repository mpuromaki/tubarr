//! Configuration page

use crate::DBPool;
use rocket::get;
use rocket::http::ContentType;

use super::render_page;

#[get("/configuration")]
pub async fn get_configuration(db_pool: &rocket::State<DBPool>) -> (ContentType, String) {
    // let conn = db_pool.get().expect("Failed to get DB connection");

    let version = env!("CARGO_PKG_VERSION");
    let page = render_page("", &HTML_CONFIGURATION.replace("{version}", version));

    (ContentType::HTML, page)
}

const HTML_CONFIGURATION: &'static str = r#"
<div class="section">
    <h1>Information</h1>

    <p>Current system version is: {version}</p>

</div>

<div class="section">
    <h1>Admin Actions:</h1>

    <p>This will shutdown the system. Restarting should be configured on the host system.</p>
    <form action="/api/shutdown" method="post">
        <button type="submit">Shutdown the system</button>
    </form>

</div>
"#;
