// backend/src/middleware/auth.rs
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse, body::EitherBody, http::header,
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
    pub sub: String,      // user_id (subject)
    pub username: String, // email
    pub exp: usize,       // expiration timestamp
    pub iat: usize,       // issued at timestamp
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
    type Transform = AuthMiddlewareService<S>;
    type InitError = ();
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
        let jwt_auth = self.jwt_auth.clone();
        let service = self.service.clone();

        Box::pin(async move {
            let path = req.path();
            
            // Public endpoints
            if is_public_endpoint(path) {
                let res = service.call(req).await?;
                return Ok(res.map_into_left_body());
            }

            // Extract and validate token
            let token = match extract_token_from_request(&req) {
                Ok(token) => token,
                Err(err_response) => {
                    return Ok(req.into_response(err_response).map_into_right_body());
                }
            };

            match jwt_auth.validate_token(&token) {
                Ok(claims) => {
                    req.extensions_mut().insert(claims.clone());
                    
                    if let Ok(user_id) = Uuid::parse_str(&claims.sub) {
                        req.extensions_mut().insert(user_id);
                    }

                    let res = service.call(req).await?;
                    Ok(res.map_into_left_body())
                }
                Err(err) => {
                    let error_response = create_auth_error_response(&err);
                    Ok(req.into_response(error_response).map_into_right_body())
                }
            }
        })
    }
}

fn is_public_endpoint(path: &str) -> bool {
    let public_paths = vec![
        "/health",
        "/api/user/signup",
        "/api/user/signin",
    ];

    public_paths.iter().any(|public_path| path.starts_with(public_path))
}

fn extract_token_from_request(req: &ServiceRequest) -> Result<String, HttpResponse> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .ok_or_else(|| {
            HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Missing Authorization header",
                "message": "Please provide a valid JWT token in the Authorization header"
            }))
        })?;

    let auth_str = auth_header
        .to_str()
        .map_err(|_| {
            HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Invalid Authorization header format",
                "message": "Authorization header contains invalid characters"
            }))
        })?;

    if !auth_str.starts_with("Bearer ") {
        return Err(HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "Invalid Authorization format",
            "message": "Authorization header must use Bearer token format: 'Bearer <token>'"
        })));
    }

    let token = auth_str[7..].to_string();
    
    if token.is_empty() {
        return Err(HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "Empty token",
            "message": "JWT token is empty"
        })));
    }

    Ok(token)
}

fn create_auth_error_response(err: &jsonwebtoken::errors::Error) -> HttpResponse {
    use jsonwebtoken::errors::ErrorKind;

    let (error_type, message) = match err.kind() {
        ErrorKind::ExpiredSignature => (
            "Token expired",
            "Your session has expired. Please sign in again."
        ),
        ErrorKind::InvalidToken => (
            "Invalid token",
            "The provided token is invalid. Please sign in again."
        ),
        ErrorKind::InvalidSignature => (
            "Invalid signature",
            "Token signature verification failed. Please sign in again."
        ),
        _ => (
            "Authentication failed",
            "Token validation failed. Please sign in again."
        ),
    };

    HttpResponse::Unauthorized().json(serde_json::json!({
        "error": error_type,
        "message": message,
        "details": err.to_string()
    }))
}

pub fn get_user_id(req: &actix_web::HttpRequest) -> Result<Uuid, HttpResponse> {
    req.extensions()
        .get::<Uuid>()
        .cloned()
        .ok_or_else(|| {
            HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Authentication required",
                "message": "User ID not found in request. Please authenticate."
            }))
        })
}

pub fn get_claims(req: &actix_web::HttpRequest) -> Result<Claims, HttpResponse> {
    req.extensions()
        .get::<Claims>()
        .cloned()
        .ok_or_else(|| {
            HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Authentication required",
                "message": "Claims not found in request. Please authenticate."
            }))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_token_generation_and_validation() {
        let jwt_auth = JwtAuth::new("test_secret_key".to_string());
        let user_id = Uuid::new_v4();
        let username = "test@example.com";

        let token = jwt_auth.generate_token(&user_id, username).unwrap();
        assert!(!token.is_empty());

        let claims = jwt_auth.validate_token(&token).unwrap();
        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.username, username);
    }

    #[test]
    fn test_invalid_token() {
        let jwt_auth = JwtAuth::new("test_secret_key".to_string());
        let result = jwt_auth.validate_token("invalid.token.here");
        assert!(result.is_err());
    }

    #[test]
    fn test_public_endpoint_detection() {
        assert!(is_public_endpoint("/health"));
        assert!(is_public_endpoint("/api/user/signup"));
        assert!(is_public_endpoint("/api/user/signin"));
        assert!(!is_public_endpoint("/api/user/profile"));
        assert!(!is_public_endpoint("/api/solana/balance"));
    }
}