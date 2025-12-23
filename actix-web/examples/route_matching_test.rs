//! Example demonstrating route matching test helpers
//!
//! This example shows how to use the new route matching functions to
//! verify named route configuration without executing handlers.

use actix_web::{http::Method, test, web, App};

#[actix_web::test]
async fn test_route_verification() {
    // Initialize an application with named routes
    let app = test::init_service(
        App::new()
            .service(
                web::resource("/api/users")
                    .name("list_users")
                    .route(web::get().to(|| async { "get users" }))
                    .route(web::post().to(|| async { "create user" })),
            )
            .service(
                web::resource("/api/users/{id}")
                    .name("user_detail")
                    .route(web::get().to(|| async { "get user" }))
                    .route(web::put().to(|| async { "update user" }))
                    .route(web::delete().to(|| async { "delete user" })),
            )
            .service(
                web::scope("/admin")
                    .service(
                        web::resource("/dashboard")
                            .name("admin_dashboard")
                            .to(|| async { "admin dashboard" }),
                    )
                    .service(
                        web::resource("/settings")
                            .name("admin_settings")
                            .to(|| async { "admin settings" }),
                    ),
            ),
    )
    .await;

    // Test route names - verify that paths map to expected named routes
    assert_eq!(
        test::match_route_name(&app, "/api/users", Method::GET).await,
        Some("list_users".to_string()),
        "GET /api/users should route to list_users"
    );

    assert_eq!(
        test::match_route_name(&app, "/api/users", Method::POST).await,
        Some("list_users".to_string()),
        "POST /api/users should route to list_users"
    );

    assert_eq!(
        test::match_route_name(&app, "/api/users/123", Method::GET).await,
        Some("user_detail".to_string()),
        "GET /api/users/123 should route to user_detail"
    );

    assert_eq!(
        test::match_route_name(&app, "/admin/dashboard", Method::GET).await,
        Some("admin_dashboard".to_string()),
        "GET /admin/dashboard should route to admin_dashboard"
    );

    // Test route patterns - verify that parameter placeholders are correct
    assert_eq!(
        test::match_route_pattern(&app, "/api/users/456", Method::PUT).await,
        Some("/api/users/{id}".to_string()),
        "PUT /api/users/456 should match pattern /api/users/{{id}}"
    );

    assert_eq!(
        test::match_route_pattern(&app, "/admin/settings", Method::GET).await,
        Some("/admin/settings".to_string()),
        "GET /admin/settings should match pattern /admin/settings"
    );

    // Test non-existent routes
    assert_eq!(
        test::match_route_name(&app, "/api/unknown", Method::GET).await,
        None,
        "Non-existent route should return None"
    );

    assert_eq!(
        test::match_route_pattern(&app, "/api/unknown", Method::GET).await,
        None,
        "Non-existent route pattern should return None"
    );

    println!("✓ All route verification tests passed!");
}

#[actix_web::test]
async fn test_route_method_specificity() {
    // Example showing how to verify method-specific routing
    let app = test::init_service(
        App::new()
            .service(
                web::resource("/resource")
                    .name("my_resource")
                    .route(web::get().to(|| async { "GET" }))
                    .route(web::post().to(|| async { "POST" })),
            )
            .service(
                web::resource("/get_only")
                    .name("get_only_resource")
                    .route(web::get().to(|| async { "GET only" })),
            ),
    )
    .await;

    // Both GET and POST should route to the same named resource
    assert_eq!(
        test::match_route_name(&app, "/resource", Method::GET).await,
        Some("my_resource".to_string())
    );
    assert_eq!(
        test::match_route_name(&app, "/resource", Method::POST).await,
        Some("my_resource".to_string())
    );

    // Only GET is defined for this resource
    assert_eq!(
        test::match_route_name(&app, "/get_only", Method::GET).await,
        Some("get_only_resource".to_string())
    );

    // POST would still match the resource name, but would return 405 Method Not Allowed
    // when actually called (not tested here since we're only checking route matching)
    assert_eq!(
        test::match_route_name(&app, "/get_only", Method::POST).await,
        Some("get_only_resource".to_string())
    );

    println!("✓ Method specificity tests passed!");
}

#[actix_web::test]
async fn test_nested_scopes() {
    // Example showing route matching with nested scopes
    let app = test::init_service(
        App::new().service(
            web::scope("/api")
                .service(web::scope("/v1").service(
                    web::resource("/users/{id}")
                        .name("v1_user")
                        .to(|| async { "v1 user" }),
                ))
                .service(web::scope("/v2").service(
                    web::resource("/users/{id}")
                        .name("v2_user")
                        .to(|| async { "v2 user" }),
                )),
        ),
    )
    .await;

    // Different versions should route to different named routes
    assert_eq!(
        test::match_route_name(&app, "/api/v1/users/123", Method::GET).await,
        Some("v1_user".to_string())
    );
    assert_eq!(
        test::match_route_name(&app, "/api/v2/users/123", Method::GET).await,
        Some("v2_user".to_string())
    );

    // Pattern should include the full path
    assert_eq!(
        test::match_route_pattern(&app, "/api/v1/users/123", Method::GET).await,
        Some("/api/v1/users/{id}".to_string())
    );
    assert_eq!(
        test::match_route_pattern(&app, "/api/v2/users/456", Method::GET).await,
        Some("/api/v2/users/{id}".to_string())
    );

    println!("✓ Nested scope tests passed!");
}
