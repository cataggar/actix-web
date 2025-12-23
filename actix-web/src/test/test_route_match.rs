use actix_http::Request;

use crate::{
    body::MessageBody,
    dev::Service,
    http::Method,
    service::ServiceResponse,
    test::TestRequest,
    Error,
};

/// Returns the name of the route that matches the given path and method in an initialized service.
///
/// This function does not execute the handler; it only performs route matching to determine
/// which named route would handle the request.
///
/// # Examples
/// ```
/// use actix_web::{test, web, App, http::Method};
///
/// #[actix_web::test]
/// async fn test_match_route_name() {
///     let app = test::init_service(
///         App::new()
///             .service(web::resource("/api/users").name("users").to(|| async { "users" }))
///     ).await;
///
///     assert_eq!(
///         test::match_route_name(&app, "/api/users", Method::GET).await,
///         Some("users".to_string())
///     );
///     assert_eq!(
///         test::match_route_name(&app, "/api/unknown", Method::GET).await,
///         None
///     );
/// }
/// ```
pub async fn match_route_name<S, B>(
    service: &S,
    path: &str,
    method: Method,
) -> Option<String>
where
    S: Service<Request, Response = ServiceResponse<B>, Error = Error>,
    B: MessageBody,
{
    let req = TestRequest::default()
        .uri(path)
        .method(method)
        .to_request();

    // Call the service to get a properly initialized ServiceRequest with resource map
    let res = service.call(req).await.ok()?;
    let req = res.request();
    req.match_name().map(|s| s.to_string())
}

/// Returns the route pattern that matches the given path and method in an initialized service.
///
/// This function does not execute the handler; it only performs route matching to determine
/// which route pattern would handle the request. The returned pattern includes parameter
/// placeholders, e.g., `/users/{id}`.
///
/// # Examples
/// ```
/// use actix_web::{test, web, App, http::Method};
///
/// #[actix_web::test]
/// async fn test_match_route_pattern() {
///     let app = test::init_service(
///         App::new()
///             .service(web::resource("/api/users/{id}").to(|| async { "user" }))
///     ).await;
///
///     assert_eq!(
///         test::match_route_pattern(&app, "/api/users/123", Method::GET).await,
///         Some("/api/users/{id}".to_string())
///     );
/// }
/// ```
pub async fn match_route_pattern<S, B>(
    service: &S,
    path: &str,
    method: Method,
) -> Option<String>
where
    S: Service<Request, Response = ServiceResponse<B>, Error = Error>,
    B: MessageBody,
{
    let req = TestRequest::default()
        .uri(path)
        .method(method)
        .to_request();

    // Call the service to get a properly initialized ServiceRequest with resource map
    let res = service.call(req).await.ok()?;
    let req = res.request();
    req.match_pattern()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{test, web, App};

    #[actix_rt::test]
    async fn test_match_route_name_basic() {
        let app = test::init_service(
            App::new()
                .service(web::resource("/users/{id}").name("user").to(|| async { "user" }))
                .service(web::resource("/posts/{id}").name("post").to(|| async { "post" })),
        )
        .await;

        assert_eq!(
            match_route_name(&app, "/users/123", Method::GET).await,
            Some("user".to_string())
        );
        assert_eq!(
            match_route_name(&app, "/posts/456", Method::GET).await,
            Some("post".to_string())
        );
        assert_eq!(
            match_route_name(&app, "/unknown", Method::GET).await,
            None
        );
    }

    #[actix_rt::test]
    async fn test_match_route_pattern_basic() {
        let app = test::init_service(
            App::new()
                .service(web::resource("/users/{id}").to(|| async { "user" }))
                .service(web::resource("/posts/{id}").to(|| async { "post" })),
        )
        .await;

        assert_eq!(
            match_route_pattern(&app, "/users/123", Method::GET).await,
            Some("/users/{id}".to_string())
        );
        assert_eq!(
            match_route_pattern(&app, "/posts/456", Method::GET).await,
            Some("/posts/{id}".to_string())
        );
        assert_eq!(
            match_route_pattern(&app, "/unknown", Method::GET).await,
            None
        );
    }

    #[actix_rt::test]
    async fn test_match_with_method() {
        let app = test::init_service(
            App::new().service(
                web::resource("/users")
                    .name("users")
                    .route(web::get().to(|| async { "get users" }))
                    .route(web::post().to(|| async { "create user" })),
            ),
        )
        .await;

        assert_eq!(
            match_route_name(&app, "/users", Method::GET).await,
            Some("users".to_string())
        );
        assert_eq!(
            match_route_name(&app, "/users", Method::POST).await,
            Some("users".to_string())
        );
        assert_eq!(
            match_route_pattern(&app, "/users", Method::GET).await,
            Some("/users".to_string())
        );
    }

    #[actix_rt::test]
    async fn test_scoped_routes() {
        let app = test::init_service(
            App::new().service(
                web::scope("/api")
                    .service(web::resource("/users/{id}").name("user").to(|| async { "user" }))
                    .service(web::resource("/posts/{id}").name("post").to(|| async { "post" })),
            ),
        )
        .await;

        assert_eq!(
            match_route_name(&app, "/api/users/123", Method::GET).await,
            Some("user".to_string())
        );
        assert_eq!(
            match_route_name(&app, "/api/posts/456", Method::GET).await,
            Some("post".to_string())
        );
        assert_eq!(
            match_route_pattern(&app, "/api/users/123", Method::GET).await,
            Some("/api/users/{id}".to_string())
        );
    }

    #[actix_rt::test]
    async fn test_route_with_different_paths() {
        let app = test::init_service(
            App::new()
                .service(
                    web::resource("/admin")
                        .name("admin")
                        .to(|| async { "admin" }),
                )
                .service(
                    web::resource("/public")
                        .name("public")
                        .to(|| async { "public" }),
                ),
        )
        .await;

        // /admin should work
        assert_eq!(
            match_route_name(&app, "/admin", Method::GET).await,
            Some("admin".to_string())
        );

        // /public should work
        assert_eq!(
            match_route_name(&app, "/public", Method::GET).await,
            Some("public".to_string())
        );

        // Unknown path should return None
        assert_eq!(
            match_route_name(&app, "/unknown", Method::GET).await,
            None
        );
    }

    #[actix_rt::test]
    async fn test_method_not_allowed() {
        let app = test::init_service(
            App::new().service(
                web::resource("/users")
                    .name("users")
                    .route(web::get().to(|| async { "get users" })),
            ),
        )
        .await;

        // GET should work
        assert_eq!(
            match_route_name(&app, "/users", Method::GET).await,
            Some("users".to_string())
        );

        // POST not allowed but should still match the resource
        assert_eq!(
            match_route_name(&app, "/users", Method::POST).await,
            Some("users".to_string())
        );
    }
}
