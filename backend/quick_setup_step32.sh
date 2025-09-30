#!/bin/bash

echo "🚀 Quick Setup for Step 3.2"
echo "==========================="

# 1. Update Cargo.toml
echo "Updating Cargo.toml..."
cat > Cargo.toml << 'EOF'
[package]
name = "backend"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "backend"
path = "src/main.rs"

[dependencies]
actix-web = "4"
actix-rt = "2"
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "postgres", "uuid", "chrono"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
reqwest = { version = "0.11", features = ["json"] }
bcrypt = "0.15"
jsonwebtoken = "9"
validator = { version = "0.16", features = ["derive"] }
uuid = { version = "1.0", features = ["serde", "v4"] }
chrono = { version = "0.4", features = ["serde"] }
env_logger = "0.11"
log = "0.4"
futures-util = "0.3"
anyhow = "1.0"
thiserror = "1.0"
dotenv = "0.15"

[dev-dependencies]
actix-http-test = "3"
EOF

# 2. Create directory structure
mkdir -p src/routes src/middleware tests

# 3. Update lib.rs
cat > src/lib.rs << 'EOF'
pub mod services;
pub mod routes;
pub mod middleware;
pub mod error;

use actix_web::{web, App, HttpServer, middleware::Logger};
use sqlx::PgPool;
use std::sync::Arc;

pub use error::{AppError, Result};

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub mpc_client: Arc<services::mpc::MPCClient>,
    pub jwt_secret: String,
}

pub async fn create_app(
    db: PgPool,
    mpc_nodes: Vec<String>,
    jwt_secret: String,
) -> std::io::Result<actix_web::dev::Server> {
    let mpc_client = Arc::new(services::mpc::MPCClient::new(mpc_nodes));
    
    let app_state = AppState {
        db,
        mpc_client,
        jwt_secret: jwt_secret.clone(),
    };

    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .wrap(Logger::default())
            .configure(routes::configure)
    })
    .bind(("0.0.0.0", 8080))?
    .run();

    Ok(server)
}
EOF

echo "✅ Basic structure created"
echo ""
echo "📝 Now you need to:"
echo "1. Copy the error.rs, middleware/auth.rs, middleware/mod.rs, routes/mod.rs, and routes/user.rs files"
echo "2. I'll provide them in separate messages due to length"
echo ""
echo "Ready for the next files?"