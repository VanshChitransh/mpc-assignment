use actix_web::{web, App, HttpServer, middleware::Logger};
use dotenv::dotenv;
use env_logger::Env;
use sqlx::PgPool;
use std::env;

mod config;
mod models;
mod routes;
mod services;
mod middleware;
mod error;

use config::Config;
use middleware::{auth::AuthMiddleware, cors::cors_config};
use crate::middleware::auth::{JwtAuth, AuthMiddleware};
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables
    dotenv().ok();

    // Initialize logger
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    // Load configuration
    let config = Config::from_env().expect("Failed to load configuration");

    // Create database connection pool
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    
    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to create database pool");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run database migrations");

    let bind_address = format!("{}:{}", config.host, config.port);
    println!("🚀 Starting server at http://{}", bind_address);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(config.clone()))
            .wrap(Logger::default())
            .wrap(cors_config())
            .service(
                web::scope("/api")
                    .service(
                        web::scope("/user")
                            .service(routes::user::sign_up)
                            .service(routes::user::sign_in)
                            .service(
                                web::scope("")
                                    .wrap(AuthMiddleware)
                                    .service(routes::user::get_profile)
                            )
                    )
                    .service(
                        web::scope("/solana")
                            .wrap(AuthMiddleware)
                            .service(routes::solana::get_balance)
                            .service(routes::solana::send_sol)
                            .service(routes::solana::send_token)
                    )
                    .service(
                        web::scope("/swap")
                            .wrap(AuthMiddleware)
                            .service(routes::solana::get_quote)
                            .service(routes::solana::execute_swap)
                    )
            )
            .service(
                web::scope("/health")
                    .service(routes::health::health_check)
            )
    })
    .bind(&bind_address)?
    .run()
    .await
}