pub mod health;
pub mod user;
pub mod solana;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg
        .service(web::scope("/api")
            .service(web::scope("/user")
                .route("/signup", web::post().to(user::sign_up))
                .route("/signin", web::post().to(user::sign_in))
                .route("/profile", web::get().to(user::get_profile)))
            .service(web::scope("/solana")
                .route("/balance", web::get().to(solana::get_balance))
                .route("/quote", web::post().to(solana::get_quote))
                .route("/swap", web::post().to(solana::swap))
                .route("/send", web::post().to(solana::send))))
        .service(health::health_check);
}