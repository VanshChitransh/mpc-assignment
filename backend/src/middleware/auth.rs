use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse, body::EitherBody,
};
use futures_util::future::{ready, Ready};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::{
    future::Future,
    pin::Pin,
    rc::Rc,
};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String, // user_id
    pub username: String,
    pub exp: usize, // expiration timestamp
    pub iat: usize, // issued at timestamp
}

#[derive(Clone)]
pub struct JwtAuth {
    secret: String,
    expiration_hours: i64,
}

impl JwtAuth {
    pub fn new(secret: String) -> Self {
        Self { 
            secret,
            expiration_hours: 24,
        }
    }

    pub fn generate_token(&self, user_id: &Uuid, username: &str) -> Result<String, jsonwebtoken::errors::Error> {
        let now = chrono::Utc::now().timestamp() as usize;
        let expiration = now + (self.expiration_hours * 60 * 60) as usize;

        let claims = Claims {
            sub: user_id.to_string(),
            username: username.to_string(),
            exp: expiration,
            iat: now,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_ref()),
        )
    }

    pub fn validate_token(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        let validation = Validation::new(Algorithm::HS256);
        
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_ref()),
            &validation,
        ).map(|data| data.claims)
    }
}

pub struct AuthMiddleware {
    jwt_auth: Rc<JwtAuth>,
}

impl AuthMiddleware {
    pub fn new(jwt_auth: JwtAuth) -> Self {
        Self {
            jwt_auth: Rc::new(jwt_auth),
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddlewareService {
            service: Rc::new(service),
            jwt_auth: self.jwt_auth.clone(),
        }))
    }
}

pub struct AuthMiddlewareService<S> {
    service: Rc<S>,
    jwt_auth: Rc<JwtAuth>,
}

impl<S, B> Service<ServiceRequest> for AuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let jwt_auth = self.jwt_auth.clone();

        Box::pin(async move {
            // Skip auth for public endpoints
            let path = req.path();
            if path == "/api/user/signup" || path == "/api/user/signin" || path.starts_with("/health") {
                let res = service.call(req).await?;
                return Ok(res.map_into_left_body());
            }

            // Extract token from Authorization header
            let auth_header = req
                .headers()
                .get("Authorization")
                .and_then(|h| h.to_str().ok())
                .and_then(|h| {
                    if h.starts_with("Bearer ") {
                        Some(&h[7..])
                    } else {
                        None
                    }
                });

            let token = match auth_header {
                Some(token) => token,
                None => {
                    let (req, _) = req.into_parts();
                    let response = HttpResponse::Unauthorized()
                        .json(serde_json::json!({
                            "error": "Missing authorization token"
                        }))
                        .map_into_right_body();
                    return Ok(ServiceResponse::new(req, response));
                }
            };

            // Validate token
            match jwt_auth.validate_token(token) {
                Ok(claims) => {
                    // Add claims to request extensions
                    req.extensions_mut().insert(claims);
                    let res = service.call(req).await?;
                    Ok(res.map_into_left_body())
                }
                Err(_) => {
                    let (req, _) = req.into_parts();
                    let response = HttpResponse::Unauthorized()
                        .json(serde_json::json!({
                            "error": "Invalid or expired token"
                        }))
                        .map_into_right_body();
                    Ok(ServiceResponse::new(req, response))
                }
            }
        })
    }
}

// Helper trait to extract user info from request
pub trait AuthExtensions {
    fn get_user_claims(&self) -> Option<Claims>;
    fn get_user_id(&self) -> Option<Uuid>;
}

impl AuthExtensions for actix_web::HttpRequest {
    fn get_user_claims(&self) -> Option<Claims> {
        self.extensions().get::<Claims>().cloned()
    }

    fn get_user_id(&self) -> Option<Uuid> {
        self.get_user_claims()
            .and_then(|claims| Uuid::parse_str(&claims.sub).ok())
    }
}