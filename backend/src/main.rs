use actix_web::{web, App, HttpServer, middleware::Logger};
use dotenv::dotenv;
use env_logger::Env;
use sqlx::PgPool;
use std::{env, sync::Arc};

mod routes;
mod middleware;
mod services;
mod error;

use middleware::auth::{JwtAuth, AuthMiddleware};
use services::mpc::MpcClient;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    
    let jwt_secret = env::var("JWT_SECRET")
        .unwrap_or_else(|_| "your-secret-key-change-in-production-min-32-chars-long".to_string());
    
    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to create database pool");

    // Initialize JWT auth
    let jwt_auth = JwtAuth::new(jwt_secret);

    // Initialize MPC client
    let mpc_node_urls = vec![
        env::var("MPC_NODE_1_URL").unwrap_or_else(|_| "http://localhost:8001".to_string()),
        env::var("MPC_NODE_2_URL").unwrap_or_else(|_| "http://localhost:8002".to_string()),
        env::var("MPC_NODE_3_URL").unwrap_or_else(|_| "http://localhost:8003".to_string()),
    ];

    let mpc_config = services::mpc::MpcConfig {
        node_urls: mpc_node_urls,
        request_timeout: std::time::Duration::from_secs(30),
        max_retries: 3,
        initial_backoff: std::time::Duration::from_millis(100),
        max_backoff: std::time::Duration::from_secs(5),
        load_balancing: services::mpc::LoadBalancingStrategy::HealthBased,
        signing_threshold: 2,
        circuit_breaker_threshold: 5,
        circuit_breaker_timeout: std::time::Duration::from_secs(30),
    };

    let mpc_client = Arc::new(MpcClient::new(mpc_config));

    let bind_address = "127.0.0.1:8080";
    println!("🚀 Starting server at http://{}", bind_address);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(jwt_auth.clone()))
            .app_data(web::Data::new(mpc_client.clone())) // Add MPC client
            .wrap(AuthMiddleware::new(jwt_auth.clone()))
            .wrap(Logger::default())
            .service(
                web::scope("/api")
                    .service(
                        web::scope("/user")
                            .service(routes::user::sign_up)
                            .service(routes::user::sign_in)
                            .service(routes::user::get_profile)
                    )
                    .service(
                        web::scope("/solana")
                            .service(routes::solana::get_balance)
                            .service(routes::solana::get_quote)
                            .service(routes::solana::execute_swap)
                            .service(routes::solana::send_tokens)
                    )
            )
            .service(routes::health::health_check)
    })
    .bind(bind_address)?
    .run()
    .await
}