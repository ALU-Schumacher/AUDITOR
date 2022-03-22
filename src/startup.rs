use crate::routes::{add, get, get_since, health_check, update};
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::PgPool;
use std::net::TcpListener;

pub fn run(listener: TcpListener, db_pool: PgPool) -> Result<Server, std::io::Error> {
    let db_pool = web::Data::new(db_pool);
    let server = HttpServer::new(move || {
        App::new()
            .route("/health_check", web::get().to(health_check))
            .route("/add", web::post().to(add))
            .route("/update", web::post().to(update))
            .route("/get", web::get().to(get))
            .route("/get/{startstop}/since/{date}", web::get().to(get_since))
            .app_data(db_pool.clone())
    })
    .listen(listener)?
    .run();
    Ok(server)
}
