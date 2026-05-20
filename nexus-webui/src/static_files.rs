//! Static file serving for web UI

use warp::Filter;

/// Serve embedded static files
pub fn embedded_files() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
{
    // For now, serve a basic index page
    warp::path::end()
        .map(|| {
            warp::reply::html(
                r#"
<!DOCTYPE html>
<html>
<head>
    <title>Nexus Web UI</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        .container { max-width: 800px; margin: 0 auto; }
        .status { color: #28a745; }
    </style>
</head>
<body>
    <div class="container">
        <h1>Rust-Nexus Web UI</h1>
        <p class="status">✅ Web UI is running</p>
        <p>This is the integrated web management interface combining:</p>
        <ul>
            <li>Tauri-Executor web interface technology</li>
            <li>Real-time agent management</li>
            <li>Task orchestration</li>
            <li>Infrastructure monitoring</li>
        </ul>
        <p><strong>WebSocket endpoint:</strong> <code>/ws</code></p>
        <p><strong>API endpoint:</strong> <code>/api/*</code></p>
    </div>
    <script>
        // Basic WebSocket connection for testing
        const ws = new WebSocket('ws://localhost:8080/ws');
        ws.onopen = () => console.log('WebSocket connected');
        ws.onmessage = (event) => console.log('WebSocket message:', event.data);
    </script>
</body>
</html>
"#,
            )
        })
        .or(warp::path("health").map(|| {
            warp::reply::json(&serde_json::json!({
                "status": "healthy",
                "service": "nexus-webui",
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        }))
}
