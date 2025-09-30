mod middleware;
mod routes;
mod services;
mod models;
mod blockchain;
use store::Store;
use tracing_actix_web::TracingLogger;
use tracing_subscriber::{fmt, EnvFilter};

use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer, Result};
use dotenv::dotenv;
use middleware::{AuthMiddleware, JwtAuth, RateLimitMiddleware, ApiMetrics};
use routes::{solana, user, wallet, api, solana_v1};
use blockchain::{SolanaBlockchain, create_solana_blockchain};
use services::{create_default_mpc_client, create_jupiter_client, create_solana_client};
use sqlx::PgPool;
use std::env;
use std::sync::Arc;
use tracing::{info, error};
use prometheus::{Registry, Encoder, TextEncoder};
use utoipa_swagger_ui::SwaggerUi;
use utoipa::OpenApi;

pub struct AppState {
    pub db: PgPool,
    pub store: Store,
    pub jwt_auth: JwtAuth,
    pub mpc_client: services::MpcClient,
    pub jupiter_client: services::JupiterClient,
    pub solana_blockchain: SolanaBlockchain,
    pub solana_client: services::SolanaClient,
}

async fn health() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "service": "solana-wallet-backend",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

async fn metrics() -> Result<HttpResponse> {
    let registry = Registry::new();
    let encoder = TextEncoder::new();
    let metric_families = registry.gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    let response = String::from_utf8(buffer).unwrap();
    
    Ok(HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4; charset=utf-8")
        .body(response))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize environment variables
    dotenv().ok();
    
    // Initialize tracing
    fmt()
        .with_env_filter(EnvFilter::new("backend=info,sqlx=warn"))
        .init();

    info!("Starting Solana Wallet Backend...");

    // Get configuration from environment
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let jwt_secret = env::var("JWT_SECRET")
        .expect("JWT_SECRET must be set");
    let bind_address = env::var("BIND_ADDRESS")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string());

    // Initialize database connection pool and store
    info!("Connecting to database...");
    let store = match Store::from_url(&database_url).await {
        Ok(store) => {
            info!("Database connection established");
            store
        }
        Err(e) => {
            error!("Failed to connect to database: {}", e);
            std::process::exit(1);
        }
    };

    // Initialize services
    let jwt_auth = JwtAuth::new(jwt_secret);
    let mpc_client = create_default_mpc_client();
    let solana_blockchain = create_solana_blockchain();
    let jupiter_client = create_jupiter_client();
    let solana_client = create_solana_client();
    
    // Initialize Prometheus metrics
    let registry = Registry::new();
    let api_metrics = Arc::new(ApiMetrics::new(&registry).expect("Failed to create API metrics"));
    
    // Initialize rate limiter
    let rate_limiter = RateLimitMiddleware::default();
    
    info!("Initialized all services successfully");
    
    // Create application state
    let app_state = web::Data::new(AppState {
        db: store.pool.clone(),
        store,
        jwt_auth: jwt_auth.clone(),
        mpc_client,
        solana_blockchain,
        jupiter_client,
        solana_client,
    });

    info!("Starting HTTP server on {}", bind_address);

    // Start HTTP server
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .app_data(app_state.clone())
            .app_data(web::Data::new(rate_limiter.clone()))
            .app_data(web::Data::new(api_metrics.clone()))
            .wrap(cors)
            .wrap(AuthMiddleware::new(jwt_auth.clone()))
            .wrap(TracingLogger::default())
            .service(
                SwaggerUi::new("/api/docs/{_:.*}")
                    .url("/api-docs/openapi.json", api::ApiDoc::openapi())
            )
            .route("/health", web::get().to(health))
            .route("/metrics", web::get().to(metrics))
            .service(
                web::scope("/api")
                    .service(
                        web::scope("/user")
                            .route("/signup", web::post().to(user::sign_up))
                            .route("/signin", web::post().to(user::sign_in))
                            .route("/profile", web::get().to(user::get_user))
                    )
                    .service(
                        web::scope("/solana")
                            .route("/balance", web::get().to(solana::get_balance))
                            .route("/quote", web::post().to(solana::get_quote))
                            .route("/swap", web::post().to(solana::execute_swap))
                            .route("/send", web::post().to(solana::send_tokens))
                    )
                    .configure(wallet::config)
                    .configure(api::config)
                    .configure(solana_v1::config)
            )
    })
    .bind(bind_address)?
    .run()
    .await
}
