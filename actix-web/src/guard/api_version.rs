use super::{Guard, GuardContext};

/// A guard that matches requests based on an API version in the URL path.
///
/// This guard checks if the API version segment in the path matches any of the specified
/// accepted versions. It's designed for API versioning patterns where the version is part
/// of the URL path.
///
/// # Path Format
/// This guard expects paths in the format `/api/{version}/...` where the version appears
/// as the second path segment after `/api/`. For example:
/// - `/api/2024-09-01/users`
/// - `/api/2025-09-01-preview/items`
///
/// # Examples
/// ```
/// use actix_web::{web, guard, HttpResponse};
///
/// // Route for a specific API version
/// web::resource("/api/{api_version}/users")
///     .guard(guard::ApiVersion("2024-09-01"))
///     .route(web::get().to(|| async { HttpResponse::Ok().body("2024-09-01 users") }));
///
/// // Route that accepts multiple API versions
/// web::resource("/api/{api_version}/items")
///     .guard(guard::ApiVersion("2025-09-01").or_version("2025-09-01-preview"))
///     .route(web::get().to(|| async { HttpResponse::Ok().body("2025-09-01 items") }));
/// ```
///
/// # Note
/// The guard extracts the version directly from the URI path during routing,
/// before path parameters are populated. This means it checks the literal path segment
/// rather than using the route's path parameter extraction.
#[allow(non_snake_case)]
pub fn ApiVersion(version: impl Into<String>) -> ApiVersionGuard {
    ApiVersionGuard {
        versions: vec![version.into()],
    }
}

/// A guard that checks if the API version in the path matches one of the accepted versions.
///
/// This guard is designed for use with paths in the format `/api/{version}/...`
/// where the version is the second path segment.
///
/// Construct an `ApiVersionGuard` using [`ApiVersion`].
#[derive(Debug, Clone)]
pub struct ApiVersionGuard {
    versions: Vec<String>,
}

impl ApiVersionGuard {
    /// Adds another accepted API version to the guard.
    ///
    /// # Examples
    /// ```
    /// use actix_web::{guard, web, HttpResponse};
    ///
    /// web::resource("/api/{api_version}/resource")
    ///     .guard(
    ///         guard::ApiVersion("2025-09-01")
    ///             .or_version("2025-09-01-preview")
    ///     )
    ///     .route(web::get().to(|| async { HttpResponse::Ok() }));
    /// ```
    pub fn or_version(mut self, version: impl Into<String>) -> Self {
        self.versions.push(version.into());
        self
    }
}

impl Guard for ApiVersionGuard {
    fn check(&self, ctx: &GuardContext<'_>) -> bool {
        // Get the path from the request URI
        let path = ctx.head().uri.path();
        
        // Extract the version from paths matching /api/{version}/... pattern
        // Split the path and look for the segment after "/api/"
        let segments: Vec<&str> = path.split('/').collect();
        
        // Expected path format: /api/{version}/... (at least 4 segments including empty first one)
        if segments.len() >= 4 && segments.get(1) == Some(&"api") {
            if let Some(&version) = segments.get(2) {
                // Check if the version matches any of the accepted versions
                return self.versions.iter().any(|v| v == version);
            }
        }
        
        // No matching version found
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::TestRequest;

    #[test]
    fn api_version_match() {
        // Create a test request with api_version parameter
        let req = TestRequest::default()
            .uri("/api/2024-09-01/users")
            .param("api_version", "2024-09-01")
            .to_srv_request();

        // Guard should match the exact version
        let guard = ApiVersion("2024-09-01");
        assert!(guard.check(&req.guard_ctx()));

        // Guard should not match different version
        let guard = ApiVersion("2025-09-01");
        assert!(!guard.check(&req.guard_ctx()));
    }

    #[test]
    fn api_version_multiple() {
        // Test request with 2025-09-01 version
        let req = TestRequest::default()
            .uri("/api/2025-09-01/items")
            .param("api_version", "2025-09-01")
            .to_srv_request();

        // Guard should match when version is in the list
        let guard = ApiVersion("2025-09-01").or_version("2025-09-01-preview");
        assert!(guard.check(&req.guard_ctx()));

        // Test request with 2025-09-01-preview version
        let req = TestRequest::default()
            .uri("/api/2025-09-01-preview/items")
            .param("api_version", "2025-09-01-preview")
            .to_srv_request();

        // Guard should match the preview version
        let guard = ApiVersion("2025-09-01").or_version("2025-09-01-preview");
        assert!(guard.check(&req.guard_ctx()));

        // Guard should not match version not in the list
        let req = TestRequest::default()
            .uri("/api/2024-09-01/items")
            .param("api_version", "2024-09-01")
            .to_srv_request();

        let guard = ApiVersion("2025-09-01").or_version("2025-09-01-preview");
        assert!(!guard.check(&req.guard_ctx()));
    }

    #[test]
    fn api_version_missing() {
        // Test request without api_version parameter
        let req = TestRequest::default()
            .uri("/api/users")
            .to_srv_request();

        // Guard should not match when api_version parameter is missing
        let guard = ApiVersion("2024-09-01");
        assert!(!guard.check(&req.guard_ctx()));
    }

    #[test]
    fn api_version_empty() {
        // Test request with empty api_version parameter
        let req = TestRequest::default()
            .uri("/api//users")
            .param("api_version", "")
            .to_srv_request();

        // Guard should not match empty version
        let guard = ApiVersion("2024-09-01");
        assert!(!guard.check(&req.guard_ctx()));
    }

    #[actix_rt::test]
    async fn test_api_version_integration() {
        use crate::{test, web, App, HttpResponse};

        let app = test::init_service(
            App::new()
                .service(
                    web::resource("/api/{api_version}/users")
                        .guard(ApiVersion("2024-09-01"))
                        .route(web::get().to(|| async { HttpResponse::Ok().body("2024") })),
                )
                .service(
                    web::resource("/api/{api_version}/users")
                        .guard(ApiVersion("2025-09-01").or_version("2025-09-01-preview"))
                        .route(web::get().to(|| async { HttpResponse::Ok().body("2025") })),
                ),
        )
        .await;

        // Test 2024-09-01 version
        let req = test::TestRequest::get()
            .uri("/api/2024-09-01/users")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        let body = test::read_body(resp).await;
        assert_eq!(body, "2024");

        // Test 2025-09-01 version
        let req = test::TestRequest::get()
            .uri("/api/2025-09-01/users")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        let body = test::read_body(resp).await;
        assert_eq!(body, "2025");

        // Test 2025-09-01-preview version
        let req = test::TestRequest::get()
            .uri("/api/2025-09-01-preview/users")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        let body = test::read_body(resp).await;
        assert_eq!(body, "2025");

        // Test invalid version
        let req = test::TestRequest::get()
            .uri("/api/invalid/users")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_client_error());
    }
}
