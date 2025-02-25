use actix_web::{dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform}, Error, HttpMessage, HttpResponse, body::{BoxBody, EitherBody}, HttpRequest};
use futures::future::{ready, LocalBoxFuture, Ready};
use sqlx::PgPool;
use std::rc::Rc;
use uuid::Uuid;

use crate::models::User;

pub struct ApiKeyMiddleware {
    pub pool: PgPool,
}

impl<S, B> Transform<S, ServiceRequest> for ApiKeyMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = Error;
    type Transform = ApiKeyMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ApiKeyMiddlewareService {
            service: Rc::new(service),
            pool: self.pool.clone(),
        }))
    }
}

pub struct ApiKeyMiddlewareService<S> {
    service: Rc<S>,
    pool: PgPool,
}

impl<S, B> Service<ServiceRequest> for ApiKeyMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let pool = self.pool.clone();
        let service = self.service.clone();

        Box::pin(async move {
            // Extract API key from query parameters
            let query_string = req.query_string();
            let api_key = query_string
                .split('&')
                .find_map(|param| {
                    let parts: Vec<&str> = param.split('=').collect();
                    if parts.len() == 2 && parts[0] == "apiKey" {
                        Some(parts[1].to_string())
                    } else {
                        None
                    }
                });

            // If API key is found, verify it
            if let Some(api_key) = api_key {
                if let Ok(Some(user)) = User::find_by_api_key(&pool, &api_key).await {
                    // Store user ID in request extensions
                    req.extensions_mut().insert(user.id);
                    // Process the request with the service and map the response
                    let res = service.call(req).await?;
                    return Ok(res.map_into_left_body());
                }
            }

            // API key is missing or invalid
            let (request, _) = req.into_parts();
            let response = HttpResponse::Unauthorized()
                .json(serde_json::json!({
                    "error": "Invalid or missing API key"
                }));

            // Create a ServiceResponse with the correct body type
            let service_response = ServiceResponse::new(request, response);

            // Convert to the expected response type
            Ok(service_response.map_into_right_body())
        })
    }
}

pub fn get_user_id_from_request(req: &HttpRequest) -> Option<Uuid> {
    req.extensions().get::<Uuid>().copied()
}