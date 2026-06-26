//! WebSocket fallback communication implementation

/// WebSocket fallback handler (stub implementation)
pub struct WebSocketFallback;

impl Default for WebSocketFallback {
    fn default() -> Self {
        Self::new()
    }
}

impl WebSocketFallback {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_fallback_construction() {
        let ws = WebSocketFallback::new();
        let _ = ws; // zero-sized, confirm it compiles
    }

    #[test]
    fn test_websocket_fallback_default() {
        let ws = WebSocketFallback::default();
        let _ = ws;
    }
}
