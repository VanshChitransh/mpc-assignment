mod middleware;
mod routes;
mod services;
use store::Store;
use tracing_actix_web::TracingLogger;
use tracing_subscriber::{fmt, EnvFilter};

use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer, Result};
use dotenv::dotenv;
use middleware::{AuthMiddleware, JwtAuth};
use routes::{solana, user};
use services::{create_default_mpc_client, create_jupiter_client, create_solana_client};
use sqlx::PgPool;
use std::env;
use tracing::{info, error};

pub struct AppState {
    pub db: PgPool,
    pub jwt_auth: JwtAuth,
    pub mpc_client: services::MpcClient,
    pub jupiter_client: services::JupiterClient,
    pub solana_client: services::SolanaClient,
}

async fn health() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "service": "solana-wallet-backend",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
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

    // Initialize database connection pool
    info!("Connecting to database...");
    let pool = match Store::new_pool(&database_url).await {
        Ok(pool) => {
            info!("Database connection established");
            pool
        }
        Err(e) => {
            error!("Failed to connect to database: {}", e);
            std::process::exit(1);
        }
    };

    // Run database migrations
    info!("Running database migrations...");
    if let Err(e) = sqlx::migrate!("../migrations").run(&pool).await {
        error!("Failed to run migrations: {}", e);
        std::process::exit(1);
    }
    info!("Database migrations completed");

    // Initialize services
    let jwt_auth = JwtAuth::new(jwt_secret);
    let mpc_client = create_default_mpc_client();
    let jupiter_client = create_jupiter_client();
    let solana_client = create_solana_client();
    
    info!("Initialized all services successfully");
    
    // Create application state
    let app_state = web::Data::new(AppState {
        db: pool,
        jwt_auth: jwt_auth.clone(),
        mpc_client,
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
            .wrap(cors)
            .wrap(AuthMiddleware::new(jwt_auth.clone()))
            .wrap(TracingLogger::default())
            .route("/health", web::get().to(health))
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
            )
    })
    .bind(bind_address)?
    .run()
    .await
}