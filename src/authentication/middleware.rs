use actix_web::{dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform}, Error, HttpMessage, HttpResponse, body::{BoxBody, EitherBody}, HttpRequest};
use futures::future::{ready, LocalBoxFuture, Ready};
use sqlx::PgPool;
use std::rc::Rc;
use uuid::Uuid;

use crate::models::User;
use crate::authentication::JwtConfig;

// Constants for header and query param names
const AUTHORIZATION_HEADER: &str = "Authorization";
const BEARER_PREFIX: &str = "Bearer ";
const API_KEY_PARAM: &str = "apiKey";

pub struct AuthMiddleware {
    pub pool: PgPool,
    pub jwt_config: JwtConfig,
}

impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = Error;
    type Transform = AuthMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddlewareService {
            service: Rc::new(service),
            pool: self.pool.clone(),
            jwt_config: self.jwt_config.clone(),
        }))
    }
}

pub struct AuthMiddlewareService<S> {
    service: Rc<S>,
    pool: PgPool,
    jwt_config: JwtConfig,
}

impl<S, B> Service<ServiceRequest> for AuthMiddlewareService<S>
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
        let jwt_config = self.jwt_config.clone();
        let service = self.service.clone();

        Box::pin(async move {

            // First check for JWT token in Authorization header
            if let Some(auth_header) = req.headers().get(AUTHORIZATION_HEADER) {
                if let Ok(auth_str) = auth_header.to_str() {
                    if auth_str.starts_with(BEARER_PREFIX) {
                        let token = auth_str.trim_start_matches(BEARER_PREFIX);

                        // Validate JWT token
                        if let Ok(claims) = jwt_config.validate_token(token) {
                            // Extract user ID from token claims
                            if let Ok(user_id) = Uuid::parse_str(&claims.sub) {
                                // Store user ID in request extensions
                                req.extensions_mut().insert(user_id);
                                let res = service.call(req).await?;
                                // Important: Convert the response to the expected type
                                return Ok(res.map_into_left_body());
                            }
                        }
                    }
                }
            }

            // If no valid JWT, fall back to API key (backward compatibility)
            let query_string = req.query_string();
            let api_key = query_string
                .split('&')
                .find_map(|param| {
                    let parts: Vec<&str> = param.split('=').collect();
                    if parts.len() == 2 && parts[0] == API_KEY_PARAM {
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
                    let res = service.call(req).await?;
                    // Important: Convert the response to the expected type
                    return Ok(res.map_into_left_body());
                }
            }

            // Neither JWT nor API key is valid
            // Create the unauthorized response
            let response = HttpResponse::Unauthorized()
                .json(serde_json::json!({
                    "error": "Invalid authentication"
                }));

            // Convert request into a service response with our error response
            let service_response = ServiceResponse::new(
                req.into_parts().0,
                response
            );

            // Important: Convert the error response to the expected type
            Ok(service_response.map_into_right_body())
        })
    }
}

pub fn get_user_id_from_request(req: &HttpRequest) -> Option<Uuid> {
    req.extensions().get::<Uuid>().copied()
}