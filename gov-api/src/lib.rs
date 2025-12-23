//! # gov-api
//!
//! REST API for compliance platform integrations.
//!
//! This crate provides the HTTP API layer for the compliance platform,
//! exposing endpoints for frameworks, controls, assets, evidence, and reports.
//!
//! ## Features
//!
//! - RESTful API with versioned endpoints
//! - Standardized request/response format
//! - Pagination support
//! - Health check endpoints
//! - JWT authentication context
//!
//! ## Example
//!
//! ```rust,no_run
//! use gov_api::{create_router, ApiState};
//!
//! #[tokio::main]
//! async fn main() {
//!     let state = ApiState::new("1.0.0");
//!     let app = create_router(state);
//!
//!     // Bind and serve
//!     let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
//!     axum::serve(listener, app).await.unwrap();
//! }
//! ```
//!
//! ## API Endpoints
//!
//! ### Health
//! - `GET /health` - Health check with component status
//! - `GET /health/ready` - Readiness check
//! - `GET /health/live` - Liveness check
//!
//! ### Frameworks
//! - `GET /api/v1/frameworks` - List all frameworks
//! - `GET /api/v1/frameworks/:id` - Get framework details
//!
//! ### Controls
//! - `GET /api/v1/controls` - List all controls
//! - `GET /api/v1/controls/:id` - Get control details
//!
//! ### Assets
//! - `GET /api/v1/assets` - List all assets
//! - `GET /api/v1/assets/:id` - Get asset details
//!
//! ### Evidence
//! - `GET /api/v1/evidence` - List evidence
//! - `POST /api/v1/evidence` - Create evidence
//! - `GET /api/v1/evidence/:id` - Get evidence details
//!
//! ### Reports
//! - `GET /api/v1/reports` - List reports
//! - `POST /api/v1/reports` - Generate report
//! - `GET /api/v1/reports/:id` - Get report details
//!
//! ### Compliance
//! - `GET /api/v1/compliance/score` - Get compliance scores
//! - `GET /api/v1/compliance/drift` - Get drift status

pub mod error;
pub mod routes;
pub mod types;

// Re-export main types
pub use error::{ApiError, ErrorResponse, Result};
pub use routes::{create_router, ApiState};
pub use types::{
    ApiResponse, AuthContext, ComponentHealth, HealthResponse, PaginatedResponse, Pagination,
};
