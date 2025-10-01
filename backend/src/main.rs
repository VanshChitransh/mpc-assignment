use actix_web::{web, App, HttpServer, middleware::Logger};
use actix_web::middleware::NormalizePath;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use tracing_actix_web::TracingLogger;
use dotenv::dotenv;
use prometheus::Registry;
use std::sync::Arc;
use store::Store;

mod middleware;
mod routes;
mod services;
mod blockchain;
mod error;

use middleware::auth::JwtAuth;
use middleware::AuthMiddleware;
use routes::{user_config, solana_config, health_config};
use services::{create_mpc_client, create_jupiter_client};
use blockchain::create_solana_blockchain;

#[derive(Clone)]
pub struct AppState {
    pub db: Pool<Postgres>,
    pub store: Store,
    pub jwt_auth: JwtAuth,
    pub mpc_client: Arc<services::MpcClient>,
    pub solana_blockchain: blockchain::solana::SolanaBlockchain,
    pub jupiter_client: services::JupiterClient,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    
    // Initialize logging
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info,actix_web=info,sqlx=warn");
    }
    tracing_subscriber::fmt::init();
    
    // Initialize Prometheus registry
    let registry = Registry::new();
    
    // Get configuration from environment
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/solana_wallet".to_string());
    
    let bind_address = std::env::var("BIND_ADDRESS")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    
    // Create database connection pool
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .expect("Failed to connect to PostgreSQL");
    
    let store = Store::new(pool.clone());
    
    // Initialize JWT authentication
    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "your-256-bit-secret".to_string());
    
    let jwt_auth = JwtAuth::new(jwt_secret);
    
    // Initialize MPC client
    let mpc_client = Arc::new(create_mpc_client());
    
    // Initialize Solana blockchain
    let solana_blockchain = create_solana_blockchain();
    
    // Initialize Jupiter client
    let jupiter_client = create_jupiter_client();
    
    // Create app state
    let app_state = web::Data::new(AppState {
        db: pool.clone(),
        store,
        jwt_auth: jwt_auth.clone(),
        mpc_client,
        solana_blockchain,
        jupiter_client,
    });
    
    println!("🚀 Starting server at http://{}", bind_address);
    
    // Start HTTP server
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .wrap(TracingLogger::default())
            .wrap(NormalizePath::trim())
            .wrap(AuthMiddleware::new(jwt_auth.clone()))
            .app_data(app_state.clone())
            .configure(health_config)
            .configure(user_config)
            .configure(solana_config)
    })
    .bind(bind_address)?
    .run()
    .await
}