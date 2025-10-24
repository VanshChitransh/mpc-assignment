// backend/src/main.rs
use actix_web::{web, App, HttpServer, middleware::Logger};
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod blockchain;
mod error;
mod middleware;
mod routes;
mod services;
mod store;

use blockchain::solana;
use middleware::auth::JwtAuth;
use routes::{health, solana as solana_routes, user};
use services::mpc::{MpcClient, create_mpc_client};
use services::jupiter::{JupiterClient, create_jupiter_client};
use store::Store;

#[derive(Clone)]
pub struct AppState {
    store: Store,
    mpc_client: Arc<MpcClient>,
    jupiter_client: Arc<JupiterClient>,
    solana_client: Arc<solana::SolanaClient>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load environment variables
    dotenv::dotenv().ok();
    
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/solana_wallet".into());
    
    let jwt_secret = env::var("JWT_SECRET")
        .unwrap_or_else(|_| "your_jwt_secret".into());

    let mpc_node_urls = vec![
        env::var("MPC_NODE_1_URL").unwrap_or_else(|_| "http://localhost:8001".into()),
        env::var("MPC_NODE_2_URL").unwrap_or_else(|_| "http://localhost:8002".into()),
        env::var("MPC_NODE_3_URL").unwrap_or_else(|_| "http://localhost:8003".into()),
    ];

    let solana_rpc_url = env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://api.devnet.solana.com".into());

    let jupiter_api_url = env::var("JUPITER_API_URL")
        .unwrap_or_else(|_| "https://quote-api.jup.ag/v6".into());

    // Database connection pool
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .expect("Failed to connect to database");

    // Initialize store
    let store = Store::new(pool.clone());
    
    // Initialize MPC client
    let mpc_client = Arc::new(create_mpc_client(mpc_node_urls, true)); // Mock mode for testing
    
    // Initialize Jupiter client
    let jupiter_client = Arc::new(create_jupiter_client(jupiter_api_url, true)); // Mock mode for testing
    
    // Initialize Solana client
    let solana_client = Arc::new(solana::create_solana_client(solana_rpc_url, true)); // Mock mode for testing
    
    // Initialize JWT auth middleware
    let jwt_auth = JwtAuth::new(jwt_secret);

    // Create application state
    let app_state = AppState {
        store,
        mpc_client,
        jupiter_client,
        solana_client,
    };

    // Start HTTP server
    println!("🚀 Starting server at http://127.0.0.1:8080");
    
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(jwt_auth.clone()))
            .app_data(web::Data::new(app_state.mpc_client.clone()))
            .app_data(web::Data::new(app_state.jupiter_client.clone()))
            .app_data(web::Data::new(app_state.solana_client.clone()))
            .app_data(web::Data::new(app_state.store.clone()))
            .app_data(web::Data::new(app_state.clone()))
            .service(health::health_check)
            .service(
                web::scope("/api/user")
                    .service(user::sign_up)
                    .service(user::sign_in)
                    .service(user::get_profile)
            )
            .service(
                web::scope("/api/solana")
                    .route("/balance", web::get().to(solana_routes::get_balance))
                    .route("/quote", web::post().to(solana_routes::get_quote))
                    .route("/swap", web::post().to(solana_routes::swap))
                    .route("/send", web::post().to(solana_routes::send))
            )
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}