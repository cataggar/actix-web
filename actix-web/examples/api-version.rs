//! Example demonstrating API version routing with guards.
//!
//! This example shows how to use the `ApiVersion` guard to route requests
//! to different handlers based on the API version specified in the path.
//!
//! The example implements three API versions:
//! - `2024-09-01`: Legacy API version
//! - `2025-09-01-preview`: Preview of new API (shares implementation with 2025-09-01)
//! - `2025-09-01`: Current stable API (shares implementation with preview)
//!
//! Run with:
//! ```sh
//! cargo run --package actix-web --example api-version
//! ```
//!
//! Test with:
//! ```sh
//! curl http://localhost:8080/api/2024-09-01/users
//! curl http://localhost:8080/api/2025-09-01/users
//! curl http://localhost:8080/api/2025-09-01-preview/users
//! curl http://localhost:8080/api/2024-09-01/items
//! curl http://localhost:8080/api/2025-09-01/items
//! curl http://localhost:8080/api/invalid-version/users  # Should return 404
//! ```

use actix_web::{guard, web, App, HttpResponse, HttpServer, Result};

// Handler for the 2024-09-01 API version (legacy)
async fn users_v2024() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "version": "2024-09-01",
        "users": [
            {"id": 1, "name": "Alice"},
            {"id": 2, "name": "Bob"}
        ]
    })))
}

// Handler for the 2025-09-01 and 2025-09-01-preview API versions (current)
async fn users_v2025() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "version": "2025-09-01",
        "users": [
            {"id": 1, "name": "Alice", "email": "alice@example.com"},
            {"id": 2, "name": "Bob", "email": "bob@example.com"},
            {"id": 3, "name": "Charlie", "email": "charlie@example.com"}
        ],
        "note": "This version includes email addresses"
    })))
}

// Handler for the 2024-09-01 API version items endpoint (legacy)
async fn items_v2024() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "version": "2024-09-01",
        "items": [
            {"id": 1, "name": "Item A"},
            {"id": 2, "name": "Item B"}
        ]
    })))
}

// Handler for the 2025-09-01 and 2025-09-01-preview API versions items endpoint (current)
async fn items_v2025() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "version": "2025-09-01",
        "items": [
            {"id": 1, "name": "Item A", "category": "Electronics", "price": 99.99},
            {"id": 2, "name": "Item B", "category": "Books", "price": 19.99},
            {"id": 3, "name": "Item C", "category": "Clothing", "price": 49.99}
        ],
        "note": "This version includes category and price information"
    })))
}

// Handler for version information
async fn version_info() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "supported_versions": [
            "2024-09-01",
            "2025-09-01-preview",
            "2025-09-01"
        ],
        "latest_stable": "2025-09-01",
        "latest_preview": "2025-09-01-preview",
        "legacy": ["2024-09-01"]
    })))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    log::info!("Starting API version routing example");
    log::info!("Server running at http://localhost:8080");
    log::info!("");
    log::info!("Try these endpoints:");
    log::info!("  GET /api/2024-09-01/users");
    log::info!("  GET /api/2025-09-01/users");
    log::info!("  GET /api/2025-09-01-preview/users");
    log::info!("  GET /api/2024-09-01/items");
    log::info!("  GET /api/2025-09-01/items");
    log::info!("  GET /versions");

    HttpServer::new(|| {
        App::new()
            // Version information endpoint (no version in path)
            .route("/versions", web::get().to(version_info))
            // Routes for the 2024-09-01 API version (legacy)
            .service(
                web::resource("/api/{api_version}/users")
                    .guard(guard::ApiVersion("2024-09-01"))
                    .route(web::get().to(users_v2024)),
            )
            .service(
                web::resource("/api/{api_version}/items")
                    .guard(guard::ApiVersion("2024-09-01"))
                    .route(web::get().to(items_v2024)),
            )
            // Routes for the 2025-09-01 and 2025-09-01-preview API versions (current)
            // These two versions share the same implementation
            .service(
                web::resource("/api/{api_version}/users")
                    .guard(
                        guard::ApiVersion("2025-09-01")
                            .or_version("2025-09-01-preview")
                    )
                    .route(web::get().to(users_v2025)),
            )
            .service(
                web::resource("/api/{api_version}/items")
                    .guard(
                        guard::ApiVersion("2025-09-01")
                            .or_version("2025-09-01-preview")
                    )
                    .route(web::get().to(items_v2025)),
            )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
