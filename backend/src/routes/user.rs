use crate::middleware::auth::{JwtAuth, get_user_id};
use crate::services::mpc::MpcClient;

use actix_web::{get, post, web, HttpResponse, Result, HttpRequest};
use sqlx::PgPool;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::Utc;
use tracing::{info, error};
use std::sync::Arc;

/// =======================
/// Request / Response Types
/// =======================

#[derive(Deserialize)]
pub struct SignUpRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct SignInRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub public_key: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Serialize)]
pub struct UserAuthResponse {
    pub success: bool,
    pub token: String,
    pub user: UserResponse,
}

#[derive(Serialize)]
pub struct UserErrorResponse {
    pub success: bool,
    pub error: String,
}

/// =======================
/// Signup Endpoint
/// =======================
#[post("/signup")]
pub async fn sign_up(
    pool: web::Data<PgPool>,
    jwt_auth: web::Data<JwtAuth>,
    mpc_client: web::Data<Arc<MpcClient>>,
    req_body: web::Json<SignUpRequest>,
) -> Result<HttpResponse> {
    let signup_req = req_body.into_inner();
    info!("User signup attempt: {}", signup_req.email);

    // Validate email format
    if !signup_req.email.contains('@') {
        return Ok(HttpResponse::BadRequest().json(UserErrorResponse {
            success: false,
            error: "Invalid email format".to_string(),
        }));
    }

    // Validate password length
    if signup_req.password.len() < 8 {
        return Ok(HttpResponse::BadRequest().json(UserErrorResponse {
            success: false,
            error: "Password must be at least 8 characters".to_string(),
        }));
    }

    // Check if user already exists
    match sqlx::query!(
        "SELECT email FROM users WHERE email = $1",
        signup_req.email.to_lowercase()
    )
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(_)) => {
            return Ok(HttpResponse::BadRequest().json(UserErrorResponse {
                success: false,
                error: "User already exists".to_string(),
            }));
        }
        Ok(None) => {}
        Err(e) => {
            error!("Database error during user check: {}", e);
            return Ok(HttpResponse::InternalServerError().json(UserErrorResponse {
                success: false,
                error: "Database error".to_string(),
            }));
        }
    }

    // Hash password
    let password_hash = match hash(signup_req.password, DEFAULT_COST) {
        Ok(hash) => hash,
        Err(e) => {
            error!("Password hashing error: {}", e);
            return Ok(HttpResponse::InternalServerError().json(UserErrorResponse {
                success: false,
                error: "Password processing error".to_string(),
            }));
        }
    };

    // Create user
    let user_id = Uuid::new_v4();
    let now = Utc::now();

    match sqlx::query!(
        r#"
        INSERT INTO users (id, email, password_hash, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        user_id,
        signup_req.email.to_lowercase(),
        password_hash,
        now,
        now
    )
    .execute(pool.as_ref())
    .await
    {
        Ok(_) => {
            info!("Successfully created user {}: {}", user_id, signup_req.email);
        }
        Err(e) => {
            error!("Failed to create user: {}", e);
            return Ok(HttpResponse::InternalServerError().json(UserErrorResponse {
                success: false,
                error: "Failed to create user".to_string(),
            }));
        }
    }

    // ===== Trigger MPC key generation =====
    let public_key = match mpc_client.generate_key(&user_id.to_string()).await {
        Ok(pk) => {
            info!("MPC key generated successfully for user {}: {}", user_id, pk);
            // Save public key to DB
            match sqlx::query!(
                "UPDATE users SET public_key = $1, updated_at = $2 WHERE id = $3",
                pk,
                Utc::now(),
                user_id
            )
            .execute(pool.as_ref())
            .await
            {
                Ok(_) => Some(pk),
                Err(e) => {
                    error!("Failed to save public key for user {}: {}", user_id, e);
                    None
                }
            }
        }
        Err(e) => {
            error!("MPC key generation failed for user {}: {}", user_id, e);
            None
        }
    };

    // Generate JWT
    let token = jwt_auth.generate_token(&user_id, &signup_req.email).map_err(|e| {
        error!("JWT encoding error: {}", e);
        actix_web::error::ErrorInternalServerError("Token generation error")
    })?;

    let user_response = UserResponse {
        id: user_id.to_string(),
        email: signup_req.email,
        public_key,
        created_at: now,
    };

    Ok(HttpResponse::Created().json(UserAuthResponse {
        success: true,
        token,
        user: user_response,
    }))
}

/// =======================
/// Signin Endpoint
/// =======================
#[post("/signin")]
pub async fn sign_in(
    pool: web::Data<PgPool>,
    jwt_auth: web::Data<JwtAuth>,
    req_body: web::Json<SignInRequest>,
) -> Result<HttpResponse> {
    let signin_req = req_body.into_inner();
    info!("User signin attempt: {}", signin_req.email);

    // Get user from DB
    let user = match sqlx::query!(
        "SELECT id, email, password_hash, public_key, created_at FROM users WHERE email = $1",
        signin_req.email.to_lowercase()
    )
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Ok(HttpResponse::Unauthorized().json(UserErrorResponse {
                success: false,
                error: "Invalid credentials".to_string(),
            }));
        }
        Err(e) => {
            error!("Database error during signin: {}", e);
            return Ok(HttpResponse::InternalServerError().json(UserErrorResponse {
                success: false,
                error: "Database error".to_string(),
            }));
        }
    };

    // Verify password
    match verify(&signin_req.password, &user.password_hash) {
        Ok(true) => info!("Successful signin for user {}: {}", user.id, user.email),
        Ok(false) => {
            return Ok(HttpResponse::Unauthorized().json(UserErrorResponse {
                success: false,
                error: "Invalid credentials".to_string(),
            }));
        }
        Err(e) => {
            error!("Password verification error: {}", e);
            return Ok(HttpResponse::InternalServerError().json(UserErrorResponse {
                success: false,
                error: "Authentication error".to_string(),
            }));
        }
    }

    // Generate JWT
    let token = jwt_auth.generate_token(&user.id, &user.email).map_err(|e| {
        error!("JWT encoding error: {}", e);
        actix_web::error::ErrorInternalServerError("Token generation error")
    })?;

    let user_response = UserResponse {
        id: user.id.to_string(),
        email: user.email,
        public_key: user.public_key,
        created_at: user.created_at,
    };

    Ok(HttpResponse::Ok().json(UserAuthResponse {
        success: true,
        token,
        user: user_response,
    }))
}

/// =======================
/// Profile Endpoint
/// =======================
#[get("/profile")]
pub async fn get_profile(
    pool: web::Data<PgPool>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let user_id = match get_user_id(&req) {
        Ok(id) => id,
        Err(err_response) => return Ok(err_response),
    };

    info!("Fetching profile for user {}", user_id);

    let user = match sqlx::query!(
        "SELECT id, email, public_key, created_at FROM users WHERE id = $1",
        user_id
    )
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Ok(HttpResponse::NotFound().json(UserErrorResponse {
                success: false,
                error: "User not found".to_string(),
            }));
        }
        Err(e) => {
            error!("Database error fetching profile: {}", e);
            return Ok(HttpResponse::InternalServerError().json(UserErrorResponse {
                success: false,
                error: "Database error".to_string(),
            }));
        }
    };

    let user_response = UserResponse {
        id: user.id.to_string(),
        email: user.email,
        public_key: user.public_key,
        created_at: user.created_at,
    };

    Ok(HttpResponse::Ok().json(user_response))
}
