//! WebSocket handler for real-time updates

use tokio_tungstenite::{tungstenite::Message, WebSocketStream};
use futures_util::{SinkExt, StreamExt};
use crate::WebUIState;

/// Handle WebSocket connection
pub async fn handle_websocket(
    ws: warp::ws::Ws,
    state: WebUIState,
) -> Result<impl warp::Reply, warp::Rejection> {
    Ok(ws.on_upgrade(move |websocket| handle_websocket_connection(websocket, state)))
}

/// Handle individual WebSocket connection
async fn handle_websocket_connection(
    ws: warp::ws::WebSocket,
    state: WebUIState,
) {
    let (mut ws_tx, mut ws_rx) = ws.split();
    let mut broadcast_rx = state.broadcast_tx.subscribe();

    // Handle incoming messages
    let state_clone = state.clone();
    let receive_task = tokio::spawn(async move {
        while let Some(result) = ws_rx.next().await {
            match result {
                Ok(msg) if msg.is_text() => {
                    // Handle incoming WebSocket messages
                    log::debug!("Received WebSocket message: {:?}", msg);
                }
                Ok(msg) if msg.is_close() => {
                    log::info!("WebSocket connection closed");
                    break;
                }
                Err(e) => {
                    log::error!("WebSocket receive error: {}", e);
                    break;
                }
                _ => {}
            }
        }
    });

    // Handle outgoing messages (broadcasts)
    let broadcast_task = tokio::spawn(async move {
        while let Ok(event) = broadcast_rx.recv().await {
            let message = match serde_json::to_string(&event) {
                Ok(json) => warp::ws::Message::text(json),
                Err(e) => {
                    log::error!("Failed to serialize broadcast event: {}", e);
                    continue;
                }
            };

            if let Err(e) = ws_tx.send(message).await {
                log::error!("Failed to send WebSocket message: {}", e);
                break;
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = receive_task => {},
        _ = broadcast_task => {},
    }
}
