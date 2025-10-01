use actix_web::web;

pub mod user;
pub mod solana;
pub mod solana_v1;
pub mod health;

// Export route configuration functions
pub use solana::config as solana_config;
pub use solana_v1::config as solana_v1_config;
pub use health::config as health_config;

// Temporary solution - create a user config function
pub fn user_config(cfg: &mut web::ServiceConfig) {
    cfg.route("/api/user/signup", web::post().to(user::sign_up));
}