use rocket::http::ContentType;
use rocket::post;
use tracing::{debug, error, event, info, trace, warn};

use crate::FLAG_SHUTDOWN;

#[post("/shutdown")]
pub async fn post_shutdown() -> (ContentType, String) {
    // Send shutdown command to the system
    warn!("Shutdown requested from API");
    FLAG_SHUTDOWN.store(true, std::sync::atomic::Ordering::Relaxed);

    // Return informational page
    let page: String = HTML_SHUTDOWN.into();
    (ContentType::HTML, page)
}

const HTML_SHUTDOWN: &'static str = r#"
<HTML>
    <HEAD>
        <script type="text/javascript">
            function pollFrontPage() {
                fetch('/')
                    .then(response => {
                        if (response.ok) {
                            window.location.href = '/';
                        }
                    })
                    .catch(error => {
                        console.error('Error polling front page:', error);
                    });
            }

            setInterval(pollFrontPage, 5000); // Poll every 5 seconds
        </script>
    </HEAD>
    <BODY>
        <h1>The system is now being shut down.</h1>
        <p> You will be automatically redirected when the system is back online.</p>
    </BODY>
</HTML>
"#;
