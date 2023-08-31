use std::net::TcpListener;

use axum::routing;
use axum::routing::IntoMakeService;
use axum::Router;
use axum::Server;

use hyper::server::conn::AddrIncoming;

// ───── Current Crate Imports ────────────────────────────────────────────── //

use crate::connection_pool::ConnectionPool;
use crate::routes::health_check;
use crate::routes::subscribe_handler;

// ───── Body ─────────────────────────────────────────────────────────────── //

pub fn run(
    listener: TcpListener,
    pool: ConnectionPool,
) -> Server<AddrIncoming, IntoMakeService<Router>> {
    let app = Router::new()
        .route("/health_check", routing::get(health_check))
        .route("/subscriptions", routing::post(subscribe_handler))
        .with_state(pool);

    let server = axum::Server::from_tcp(listener)
        .expect("Cant create server from tcp listener.")
        .serve(app.into_make_service());
    server
}
