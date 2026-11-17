mod app;
mod auth;
mod browser;
mod db;
mod docs;
mod handlers;
mod models;
mod providers;
mod repositories;
mod utils;

use actix_web::{App, HttpServer, web};
use std::env;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();

    let db_pool = db::neon::create_db_pool()
        .await
        .expect("Failed to connect to Neon PostgreSQL");

    println!("Connected to Neon PostgreSQL!");

    let port = env::var("PORT")
        .unwrap_or_else(|_| "10000".to_string())
        .parse::<u16>()
        .expect("Invalid PORT");

    println!("Web Reader running on http://0.0.0.0:{}", port);

    HttpServer::new(move || {
        App::new()
            .wrap(app::routes::cors())
            .app_data(web::Data::new(db_pool.clone()))
            .configure(app::routes::configure)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
