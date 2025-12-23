use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Standardized API response wrapper
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// Whether the request was successful
    pub success: bool,
    /// Response data
    pub data: Option<T>,
    /// Optional message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Request ID for tracing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl<T> ApiResponse<T> {
    /// Create a successful response
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
            request_id: None,
        }
    }

    /// Create a success response with message
    pub fn success_with_message(data: T, message: &str) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: Some(message.to_string()),
            request_id: None,
        }
    }
}

impl ApiResponse<()> {
    /// Create a success response without data
    pub fn ok() -> Self {
        Self {
            success: true,
            data: None,
            message: None,
            request_id: None,
        }
    }

    /// Create a success response with message only
    pub fn message(message: &str) -> Self {
        Self {
            success: true,
            data: None,
            message: Some(message.to_string()),
            request_id: None,
        }
    }
}

/// Pagination parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    pub page: u32,
    /// Items per page
    #[serde(default = "default_per_page")]
    pub per_page: u32,
    /// Sort field
    pub sort_by: Option<String>,
    /// Sort direction
    #[serde(default)]
    pub sort_desc: bool,
}

fn default_page() -> u32 {
    1
}

fn default_per_page() -> u32 {
    25
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            page: 1,
            per_page: 25,
            sort_by: None,
            sort_desc: false,
        }
    }
}

/// Paginated response wrapper
#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    /// Items for current page
    pub items: Vec<T>,
    /// Total number of items
    pub total: u64,
    /// Current page
    pub page: u32,
    /// Items per page
    pub per_page: u32,
    /// Total number of pages
    pub total_pages: u32,
    /// Whether there's a next page
    pub has_next: bool,
    /// Whether there's a previous page
    pub has_prev: bool,
}

impl<T> PaginatedResponse<T> {
    /// Create a new paginated response
    pub fn new(items: Vec<T>, total: u64, pagination: &Pagination) -> Self {
        let total_pages = ((total as f64) / (pagination.per_page as f64)).ceil() as u32;
        Self {
            items,
            total,
            page: pagination.page,
            per_page: pagination.per_page,
            total_pages,
            has_next: pagination.page < total_pages,
            has_prev: pagination.page > 1,
        }
    }
}

/// Authentication context from JWT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
    /// User ID
    pub user_id: String,
    /// Tenant ID
    pub tenant_id: Uuid,
    /// User email
    pub email: Option<String>,
    /// User roles
    pub roles: Vec<String>,
    /// Token expiration timestamp
    pub exp: i64,
}

impl AuthContext {
    /// Check if user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// Check if user is admin
    pub fn is_admin(&self) -> bool {
        self.has_role("admin") || self.has_role("owner")
    }
}

/// Health check response
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Service status
    pub status: String,
    /// Service version
    pub version: String,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Component health
    pub components: std::collections::HashMap<String, ComponentHealth>,
}

/// Component health status
#[derive(Debug, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Component status
    pub status: String,
    /// Optional message
    pub message: Option<String>,
    /// Response time in ms
    pub response_time_ms: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response() {
        let response = ApiResponse::success(vec![1, 2, 3]);
        assert!(response.success);
        assert!(response.data.is_some());
    }

    #[test]
    fn test_pagination() {
        let pagination = Pagination::default();
        assert_eq!(pagination.page, 1);
        assert_eq!(pagination.per_page, 25);
    }

    #[test]
    fn test_paginated_response() {
        let items = vec!["a", "b", "c"];
        let pagination = Pagination::default();
        let response = PaginatedResponse::new(items, 100, &pagination);

        assert_eq!(response.total, 100);
        assert_eq!(response.total_pages, 4);
        assert!(response.has_next);
        assert!(!response.has_prev);
    }

    #[test]
    fn test_auth_context() {
        let auth = AuthContext {
            user_id: "user123".to_string(),
            tenant_id: Uuid::new_v4(),
            email: Some("user@example.com".to_string()),
            roles: vec!["admin".to_string(), "viewer".to_string()],
            exp: 0,
        };

        assert!(auth.is_admin());
        assert!(auth.has_role("viewer"));
        assert!(!auth.has_role("editor"));
    }
}
