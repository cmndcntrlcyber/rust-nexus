//! HTTP fallback communication implementation

use crate::*;

#[allow(dead_code)] // config read in upcoming exfiltration wiring
pub struct HttpExfiltration {
    config: WebCommsConfig,
}

impl HttpExfiltration {
    pub fn new(config: &WebCommsConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    pub async fn exfiltrate_data(
        &self,
        _endpoint: &str,
        _message: &WebCommsMessage,
        _method: &str,
    ) -> Result<String> {
        // Stub implementation
        Ok("HTTP exfiltration not yet implemented".to_string())
    }
}

#[allow(dead_code)] // config read in upcoming HTTP server wiring
pub struct HttpServer {
    config: WebCommsConfig,
}

impl HttpServer {
    pub fn new(config: WebCommsConfig) -> Self {
        Self { config }
    }

    pub async fn start(&self, _bind_address: &str, _port: u16) -> Result<()> {
        // Stub implementation
        Err(NexusError::NetworkError(
            "HTTP server not yet implemented".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_exfiltration_construction() {
        let config = WebCommsConfig::default();
        let exfil = HttpExfiltration::new(&config);
        assert_eq!(exfil.config.request_timeout, config.request_timeout);
    }

    #[tokio::test]
    async fn test_http_exfiltration_stub_returns_ok() {
        let config = WebCommsConfig::default();
        let exfil = HttpExfiltration::new(&config);
        let msg = WebCommsMessage::TaskRequest("agent-001".to_string());
        let result = exfil.exfiltrate_data("https://example.com", &msg, "POST").await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_http_server_construction() {
        let config = WebCommsConfig::default();
        let timeout = config.request_timeout;
        let server = HttpServer::new(config);
        assert_eq!(server.config.request_timeout, timeout);
    }

    #[tokio::test]
    async fn test_http_server_start_returns_err_stub() {
        let config = WebCommsConfig::default();
        let server = HttpServer::new(config);
        let result = server.start("127.0.0.1", 8080).await;
        assert!(result.is_err());
    }
}
