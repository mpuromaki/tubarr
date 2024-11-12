mod home;
pub use home::*;

mod channels;
pub use channels::*;

fn render_page(head: &str, body: &str) -> String {
    let mut page = String::with_capacity(1024);
    page.push_str(TMPL_1);
    page.push_str(head);
    page.push_str(TMPL_2);
    page.push_str(body);
    page.push_str(TMPL_3);
    page
}

const TMPL_1: &'static str = r###"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Tubarr</title>
    <link rel="stylesheet" href="/static/style.css" type="text/css">
"###;

const TMPL_2: &'static str = r###"
</head>
<body>
    <div class="container">
        <header class="top-bar">
            <div class="logo">Tubarr</div>
            <div class="download-video">
                <label for="video-url">Download video:</label>
                <input type="text" id="video-url" placeholder="Enter video URL" />
                <button onclick="downloadVideo()">Download</button>
            </div>
            <div class="user-button">User</div>
        </header>
        <aside class="side-bar">
            <!-- Side Navigation Menu -->
            <nav>
                <ul>
                    <li><a href="/">Home</a></li>
                </ul>
            </nav>
        </aside>
        <main class="content">
"###;

const TMPL_3: &'static str = r###"
        </main>
    </div>
    <script>
        async function downloadVideo() {
            const url = document.getElementById('video-url').value;
            if (!url) {
                alert("Please enter a video URL.");
                return;
            }

            try {
                const response = await fetch('/api/video', {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json'
                    },
                    body: JSON.stringify({ url: url })
                });

                if (response.ok) {
                    alert("Video download request submitted successfully.");
                } else {
                    alert("Failed to submit video download request.");
                }
            } catch (error) {
                alert("An error occurred: " + error.message);
            }
        }
    </script>
</body>
</html>
"###;
