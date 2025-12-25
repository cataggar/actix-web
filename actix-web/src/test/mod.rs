//! Various helpers for Actix applications to use during testing.
//!
//! # Initializing A Test Service
//! - [`init_service`]
//!
//! # Off-The-Shelf Test Services
//! - [`ok_service`]
//! - [`status_service`]
//!
//! # Calling Test Service
//! - [`TestRequest`]
//! - [`call_service`]
//! - [`try_call_service`]
//! - [`call_and_read_body`]
//! - [`call_and_read_body_json`]
//! - [`try_call_and_read_body_json`]
//!
//! # Reading Response Payloads
//! - [`read_body`]
//! - [`try_read_body`]
//! - [`read_body_json`]
//! - [`try_read_body_json`]
//!
//! # Testing Route Matching
//! - [`matched_route_name`]: Extract the matched route name from a request for testing

// TODO: more docs on generally how testing works with these parts

pub use actix_http::test::TestBuffer;

mod test_request;
mod test_services;
mod test_utils;

#[allow(deprecated)]
pub use self::test_services::{default_service, ok_service, simple_service, status_service};
#[cfg(test)]
pub(crate) use self::test_utils::try_init_service;
#[allow(deprecated)]
pub use self::test_utils::{read_response, read_response_json};
pub use self::{
    test_request::TestRequest,
    test_utils::{
        call_and_read_body, call_and_read_body_json, call_service, init_service, read_body,
        read_body_json, try_call_and_read_body_json, try_call_service, try_read_body,
        try_read_body_json,
    },
};

use crate::{route::MatchedRouteName, HttpMessage};

/// Extracts the matched route name from a service request.
///
/// Returns the name of the route that matched this request, if any route with a name was matched.
/// This is primarily useful for testing route matching logic without executing handlers.
///
/// # Examples
/// ```
/// use actix_web::{test, web, App, HttpResponse};
///
/// #[actix_web::test]
/// async fn test_route_matching() {
///     let app = test::init_service(
///         App::new().service(
///             web::resource("/test")
///                 .route(web::get().name("get-test").to(|| HttpResponse::Ok()))
///                 .route(web::post().name("post-test").to(|| HttpResponse::Created()))
///         )
///     ).await;
///
///     let req = test::TestRequest::get().uri("/test").to_request();
///     let resp = test::call_service(&app, req).await;
///
///     assert_eq!(test::matched_route_name(&resp).as_deref(), Some("get-test"));
/// }
/// ```
pub fn matched_route_name<B>(res: &crate::service::ServiceResponse<B>) -> Option<String> {
    res.request()
        .extensions()
        .get::<MatchedRouteName>()
        .map(|m| m.0.clone())
}

/// Reduces boilerplate code when testing expected response payloads.
///
/// Must be used inside an async test. Works for both `ServiceRequest` and `HttpRequest`.
///
/// # Examples
///
/// ```
/// use actix_web::{http::StatusCode, HttpResponse};
///
/// let res = HttpResponse::with_body(StatusCode::OK, "http response");
/// assert_body_eq!(res, b"http response");
/// ```
#[cfg(test)]
macro_rules! assert_body_eq {
    ($res:ident, $expected:expr) => {
        assert_eq!(
            ::actix_http::body::to_bytes($res.into_body())
                .await
                .expect("error reading test response body"),
            ::bytes::Bytes::from_static($expected),
        )
    };
}

#[cfg(test)]
pub(crate) use assert_body_eq;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{http::StatusCode, service::ServiceResponse, HttpResponse};

    #[actix_rt::test]
    async fn assert_body_works_for_service_and_regular_response() {
        let res = HttpResponse::with_body(StatusCode::OK, "http response");
        assert_body_eq!(res, b"http response");

        let req = TestRequest::default().to_http_request();
        let res = HttpResponse::with_body(StatusCode::OK, "service response");
        let res = ServiceResponse::new(req, res);
        assert_body_eq!(res, b"service response");
    }
}
