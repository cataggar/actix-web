use std::{mem, rc::Rc};

use actix_http::{body::MessageBody, Method};
use actix_service::{
    apply,
    boxed::{self, BoxService},
    fn_service, Service, ServiceFactory, ServiceFactoryExt, Transform,
};
use futures_core::future::LocalBoxFuture;

use crate::{
    guard::{self, Guard},
    handler::{handler_service, Handler},
    middleware::Compat,
    service::{BoxedHttpServiceFactory, ServiceRequest, ServiceResponse},
    Error, FromRequest, HttpResponse, Responder,
};

/// Marker type for storing the matched route name in request extensions.
/// This is set during route matching and can be retrieved for testing purposes.
#[derive(Debug, Clone)]
pub struct MatchedRouteName(pub String);

/// A request handler with [guards](guard).
///
/// Route uses a builder-like pattern for configuration. If handler is not set, a `404 Not Found`
/// handler is used.
pub struct Route {
    service: BoxedHttpServiceFactory,
    guards: Rc<Vec<Box<dyn Guard>>>,
    name: Option<String>,
}

impl Route {
    /// Create new route which matches any request.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Route {
        Route {
            service: boxed::factory(fn_service(|req: ServiceRequest| async {
                Ok(req.into_response(HttpResponse::NotFound()))
            })),
            guards: Rc::new(Vec::new()),
            name: None,
        }
    }

    /// Registers a route middleware.
    ///
    /// `mw` is a middleware component (type), that can modify the requests and responses handled by
    /// this `Route`.
    ///
    /// See [`App::wrap`](crate::App::wrap) for more details.
    #[doc(alias = "middleware")]
    #[doc(alias = "use")] // nodejs terminology
    pub fn wrap<M, B>(self, mw: M) -> Route
    where
        M: Transform<
                BoxService<ServiceRequest, ServiceResponse, Error>,
                ServiceRequest,
                Response = ServiceResponse<B>,
                Error = Error,
                InitError = (),
            > + 'static,
        B: MessageBody + 'static,
    {
        Route {
            service: boxed::factory(apply(Compat::new(mw), self.service)),
            guards: self.guards,
            name: self.name,
        }
    }

    pub(crate) fn take_guards(&mut self) -> Vec<Box<dyn Guard>> {
        mem::take(Rc::get_mut(&mut self.guards).unwrap())
    }
}

impl ServiceFactory<ServiceRequest> for Route {
    type Response = ServiceResponse;
    type Error = Error;
    type Config = ();
    type Service = RouteService;
    type InitError = ();
    type Future = LocalBoxFuture<'static, Result<Self::Service, Self::InitError>>;

    fn new_service(&self, _: ()) -> Self::Future {
        let fut = self.service.new_service(());
        let guards = Rc::clone(&self.guards);
        let name = self.name.clone();

        Box::pin(async move {
            let service = fut.await?;
            Ok(RouteService { service, guards, name })
        })
    }
}

pub struct RouteService {
    service: BoxService<ServiceRequest, ServiceResponse, Error>,
    guards: Rc<Vec<Box<dyn Guard>>>,
    name: Option<String>,
}

impl RouteService {
    // TODO(breaking): remove pass by ref mut
    #[allow(clippy::needless_pass_by_ref_mut)]
    pub fn check(&self, req: &mut ServiceRequest) -> bool {
        let guard_ctx = req.guard_ctx();

        for guard in self.guards.iter() {
            if !guard.check(&guard_ctx) {
                return false;
            }
        }
        true
    }

    /// Get the route name if one has been set.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

impl Service<ServiceRequest> for RouteService {
    type Response = ServiceResponse;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_service::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        self.service.call(req)
    }
}

impl Route {
    /// Add method guard to the route.
    ///
    /// # Examples
    /// ```
    /// # use actix_web::*;
    /// # fn main() {
    /// App::new().service(web::resource("/path").route(
    ///     web::get()
    ///         .method(http::Method::CONNECT)
    ///         .guard(guard::Header("content-type", "text/plain"))
    ///         .to(|req: HttpRequest| HttpResponse::Ok()))
    /// );
    /// # }
    /// ```
    pub fn method(mut self, method: Method) -> Self {
        Rc::get_mut(&mut self.guards)
            .unwrap()
            .push(Box::new(guard::Method(method)));
        self
    }

    /// Add guard to the route.
    ///
    /// # Examples
    /// ```
    /// # use actix_web::*;
    /// # fn main() {
    /// App::new().service(web::resource("/path").route(
    ///     web::route()
    ///         .guard(guard::Get())
    ///         .guard(guard::Header("content-type", "text/plain"))
    ///         .to(|req: HttpRequest| HttpResponse::Ok()))
    /// );
    /// # }
    /// ```
    pub fn guard<F: Guard + 'static>(mut self, f: F) -> Self {
        Rc::get_mut(&mut self.guards).unwrap().push(Box::new(f));
        self
    }

    /// Set route name.
    ///
    /// Name can be used for URL generation within resource.
    ///
    /// # Examples
    /// ```
    /// use actix_web::{web, App, HttpResponse};
    ///
    /// let app = App::new().service(
    ///     web::resource("/test")
    ///         .route(web::get().name("get-handler").to(|| HttpResponse::Ok()))
    /// );
    /// ```
    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    /// Get route name if it has been set.
    pub fn get_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Set handler function, use request extractors for parameters.
    ///
    /// # Examples
    /// ```
    /// use actix_web::{web, http, App};
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize)]
    /// struct Info {
    ///     username: String,
    /// }
    ///
    /// /// extract path info using serde
    /// async fn index(info: web::Path<Info>) -> String {
    ///     format!("Welcome {}!", info.username)
    /// }
    ///
    /// let app = App::new().service(
    ///     web::resource("/{username}/index.html") // <- define path parameters
    ///         .route(web::get().to(index))        // <- register handler
    /// );
    /// ```
    ///
    /// It is possible to use multiple extractors for one handler function.
    /// ```
    /// # use std::collections::HashMap;
    /// # use serde::Deserialize;
    /// use actix_web::{web, App};
    ///
    /// #[derive(Deserialize)]
    /// struct Info {
    ///     username: String,
    /// }
    ///
    /// /// extract path info using serde
    /// async fn index(
    ///     path: web::Path<Info>,
    ///     query: web::Query<HashMap<String, String>>,
    ///     body: web::Json<Info>
    /// ) -> String {
    ///     format!("Welcome {}!", path.username)
    /// }
    ///
    /// let app = App::new().service(
    ///     web::resource("/{username}/index.html") // <- define path parameters
    ///         .route(web::get().to(index))
    /// );
    /// ```
    pub fn to<F, Args>(mut self, handler: F) -> Self
    where
        F: Handler<Args>,
        Args: FromRequest + 'static,
        F::Output: Responder + 'static,
    {
        self.service = handler_service(handler);
        self
    }

    /// Set raw service to be constructed and called as the request handler.
    ///
    /// # Examples
    /// ```
    /// # use std::convert::Infallible;
    /// # use futures_util::future::LocalBoxFuture;
    /// # use actix_web::{*, dev::*, http::header};
    /// struct HelloWorld;
    ///
    /// impl Service<ServiceRequest> for HelloWorld {
    ///     type Response = ServiceResponse;
    ///     type Error = Infallible;
    ///     type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;
    ///
    ///     dev::always_ready!();
    ///
    ///     fn call(&self, req: ServiceRequest) -> Self::Future {
    ///         let (req, _) = req.into_parts();
    ///
    ///         let res = HttpResponse::Ok()
    ///             .insert_header(header::ContentType::plaintext())
    ///             .body("Hello world!");
    ///
    ///         Box::pin(async move { Ok(ServiceResponse::new(req, res)) })
    ///     }
    /// }
    ///
    /// App::new().route(
    ///     "/",
    ///     web::get().service(fn_factory(|| async { Ok(HelloWorld) })),
    /// );
    /// ```
    pub fn service<S, E>(mut self, service_factory: S) -> Self
    where
        S: ServiceFactory<
                ServiceRequest,
                Response = ServiceResponse,
                Error = E,
                InitError = (),
                Config = (),
            > + 'static,
        E: Into<Error> + 'static,
    {
        self.service = boxed::factory(service_factory.map_err(Into::into));
        self
    }
}

#[cfg(test)]
mod tests {
    use std::{convert::Infallible, time::Duration};

    use actix_rt::time::sleep;
    use bytes::Bytes;
    use futures_core::future::LocalBoxFuture;
    use serde::Serialize;

    use crate::{
        dev::{always_ready, fn_factory, fn_service, Service},
        error,
        guard,
        http::{header, Method, StatusCode},
        middleware::{DefaultHeaders, Logger},
        service::{ServiceRequest, ServiceResponse},
        test::{call_service, init_service, read_body, TestRequest},
        web, App, HttpResponse,
    };

    #[derive(Serialize, PartialEq, Debug)]
    struct MyObject {
        name: String,
    }

    #[actix_rt::test]
    async fn test_route() {
        let srv =
            init_service(
                App::new()
                    .service(
                        web::resource("/test")
                            .route(web::get().to(HttpResponse::Ok))
                            .route(web::put().to(|| async {
                                Err::<HttpResponse, _>(error::ErrorBadRequest("err"))
                            }))
                            .route(web::post().to(|| async {
                                sleep(Duration::from_millis(100)).await;
                                Ok::<_, Infallible>(HttpResponse::Created())
                            }))
                            .route(web::delete().to(|| async {
                                sleep(Duration::from_millis(100)).await;
                                Err::<HttpResponse, _>(error::ErrorBadRequest("err"))
                            })),
                    )
                    .service(web::resource("/json").route(web::get().to(|| async {
                        sleep(Duration::from_millis(25)).await;
                        web::Json(MyObject {
                            name: "test".to_string(),
                        })
                    }))),
            )
            .await;

        let req = TestRequest::with_uri("/test")
            .method(Method::GET)
            .to_request();
        let resp = call_service(&srv, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let req = TestRequest::with_uri("/test")
            .method(Method::POST)
            .to_request();
        let resp = call_service(&srv, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);

        let req = TestRequest::with_uri("/test")
            .method(Method::PUT)
            .to_request();
        let resp = call_service(&srv, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let req = TestRequest::with_uri("/test")
            .method(Method::DELETE)
            .to_request();
        let resp = call_service(&srv, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let req = TestRequest::with_uri("/test")
            .method(Method::HEAD)
            .to_request();
        let resp = call_service(&srv, req).await;
        assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);

        let req = TestRequest::with_uri("/json").to_request();
        let resp = call_service(&srv, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body = read_body(resp).await;
        assert_eq!(body, Bytes::from_static(b"{\"name\":\"test\"}"));
    }

    #[actix_rt::test]
    async fn route_middleware() {
        let srv = init_service(
            App::new()
                .route("/", web::get().to(HttpResponse::Ok).wrap(Logger::default()))
                .service(
                    web::resource("/test")
                        .route(web::get().to(HttpResponse::Ok))
                        .route(
                            web::post()
                                .to(HttpResponse::Created)
                                .wrap(DefaultHeaders::new().add(("x-test", "x-posted"))),
                        )
                        .route(
                            web::delete()
                                .to(HttpResponse::Accepted)
                                // logger changes body type, proving Compat is not needed
                                .wrap(Logger::default()),
                        ),
                ),
        )
        .await;

        let req = TestRequest::get().uri("/test").to_request();
        let res = call_service(&srv, req).await;
        assert_eq!(res.status(), StatusCode::OK);
        assert!(!res.headers().contains_key("x-test"));

        let req = TestRequest::post().uri("/test").to_request();
        let res = call_service(&srv, req).await;
        assert_eq!(res.status(), StatusCode::CREATED);
        assert_eq!(res.headers().get("x-test").unwrap(), "x-posted");

        let req = TestRequest::delete().uri("/test").to_request();
        let res = call_service(&srv, req).await;
        assert_eq!(res.status(), StatusCode::ACCEPTED);
    }

    #[actix_rt::test]
    async fn test_service_handler() {
        struct HelloWorld;

        impl Service<ServiceRequest> for HelloWorld {
            type Response = ServiceResponse;
            type Error = crate::Error;
            type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

            always_ready!();

            fn call(&self, req: ServiceRequest) -> Self::Future {
                let (req, _) = req.into_parts();

                let res = HttpResponse::Ok()
                    .insert_header(header::ContentType::plaintext())
                    .body("Hello world!");

                Box::pin(async move { Ok(ServiceResponse::new(req, res)) })
            }
        }

        let srv = init_service(
            App::new()
                .route(
                    "/hello",
                    web::get().service(fn_factory(|| async { Ok(HelloWorld) })),
                )
                .route(
                    "/bye",
                    web::get().service(fn_factory(|| async {
                        Ok::<_, ()>(fn_service(|req: ServiceRequest| async {
                            let (req, _) = req.into_parts();

                            let res = HttpResponse::Ok()
                                .insert_header(header::ContentType::plaintext())
                                .body("Goodbye, and thanks for all the fish!");

                            Ok::<_, Infallible>(ServiceResponse::new(req, res))
                        }))
                    })),
                ),
        )
        .await;

        let req = TestRequest::get().uri("/hello").to_request();
        let resp = call_service(&srv, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = read_body(resp).await;
        assert_eq!(body, Bytes::from_static(b"Hello world!"));

        let req = TestRequest::get().uri("/bye").to_request();
        let resp = call_service(&srv, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = read_body(resp).await;
        assert_eq!(
            body,
            Bytes::from_static(b"Goodbye, and thanks for all the fish!")
        );
    }

    #[actix_rt::test]
    async fn test_named_routes() {
        let srv = init_service(
            App::new().service(
                web::resource("/test")
                    .route(web::get().name("get-test").to(|| async { "GET" }))
                    .route(web::post().name("post-test").to(|| async { "POST" }))
                    .route(web::put().to(|| async { "PUT" })), // unnamed route
            ),
        )
        .await;

        // Test GET route with name
        let req = TestRequest::get().uri("/test").to_request();
        let resp = call_service(&srv, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            crate::test::matched_route_name(&resp).as_deref(),
            Some("get-test")
        );

        // Test POST route with name
        let req = TestRequest::post().uri("/test").to_request();
        let resp = call_service(&srv, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            crate::test::matched_route_name(&resp).as_deref(),
            Some("post-test")
        );

        // Test PUT route without name
        let req = TestRequest::put().uri("/test").to_request();
        let resp = call_service(&srv, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(crate::test::matched_route_name(&resp), None);
    }

    #[actix_rt::test]
    async fn test_named_route_with_guards() {
        let srv = init_service(
            App::new().service(
                web::resource("/test")
                    .route(
                        web::get()
                            .name("json-get")
                            .guard(guard::Header("content-type", "application/json"))
                            .to(|| async { "JSON GET" }),
                    )
                    .route(
                        web::get()
                            .name("plain-get")
                            .guard(guard::Header("content-type", "text/plain"))
                            .to(|| async { "PLAIN GET" }),
                    ),
            ),
        )
        .await;

        // Test with JSON content-type
        let req = TestRequest::get()
            .uri("/test")
            .insert_header((header::CONTENT_TYPE, "application/json"))
            .to_request();
        let resp = call_service(&srv, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            crate::test::matched_route_name(&resp).as_deref(),
            Some("json-get")
        );

        // Test with plain text content-type
        let req = TestRequest::get()
            .uri("/test")
            .insert_header((header::CONTENT_TYPE, "text/plain"))
            .to_request();
        let resp = call_service(&srv, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            crate::test::matched_route_name(&resp).as_deref(),
            Some("plain-get")
        );
    }
}
