use std::net::TcpListener;
use std::path::PathBuf;

use axum::routing;
use axum::routing::IntoMakeService;
use axum::Router;
use axum::Server;

use hyper::server::conn::AddrIncoming;

use bb8_postgres::PostgresConnectionManager;
use hyperlocal::SocketIncoming;
use hyperlocal::UnixServerExt;
use secrecy::ExposeSecret;

use crate::configuration::Settings;
use crate::connection_pool::ConnectionPool;
use crate::email_client::EmailClient;
use crate::routes::confirm;
use crate::routes::get_hello;
use crate::routes::health_check;
use crate::routes::subscribe_handler;

mod db_migration;

pub enum ServerType {
    TcpSocket(Server<AddrIncoming, IntoMakeService<Router>>),
    UnixSocket(Server<SocketIncoming, IntoMakeService<Router>>),
}

#[derive(Clone)]
pub enum PortType {
    Tcp(u16),
    Unix(PathBuf),
}

/// This is a central type of our codebase. `Application` type builds server
/// for both production and testing purposes.
pub struct Application {
    server: ServerType,
    #[allow(unused)]
    port: PortType,
    unix_socket_file: Option<PathBuf>,
}

/// Shareable type, we insert it to the main `Router` as state,
/// at the launch stage.
#[derive(Clone)]
pub struct AppState {
    pub base_url: String,
    pub pool: ConnectionPool,
    pub email_client: EmailClient,
}

impl Application {
    /// Build a new server.
    ///
    /// This functions builds a new `Application` with given configuration.
    /// It also configures a pool of connections to the PostgreSQL database.
    pub async fn build(
        configuration: Settings,
    ) -> Result<Application, std::io::Error> {
        let postgres_connection =
            get_postgres_connection_pool(&configuration).await;

        db_migration::run_migration(&postgres_connection).await;

        let timeout = configuration.email_client.timeout_millis();

        let sender_email =
            configuration.email_client.sender().map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
            })?;

        let email_client = EmailClient::new(
            configuration.email_client.base_url,
            sender_email,
            configuration.email_client.authorization_token,
            timeout,
            configuration.email_delivery_service,
        )
        .map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
        })?;

        let address =
            format!("{}:{}", configuration.app_addr, configuration.app_port);
        let listener = TcpListener::bind(address)?;

        let (server, unix_socket_path) = Self::build_server(
            &configuration.unix_socket,
            &configuration.app_base_url,
            listener,
            postgres_connection,
            email_client,
        );

        let port = match server {
            ServerType::TcpSocket(ref server) => {
                PortType::Tcp(server.local_addr().port())
            }
            ServerType::UnixSocket(_) => {
                PortType::Unix(unix_socket_path.clone().unwrap())
            }
        };

        Ok(Self {
            server,
            port,
            unix_socket_file: unix_socket_path,
        })
    }

    /// Get port on which current application is ran.
    pub fn port(&self) -> PortType {
        self.port.clone()
    }

    /// This function only returns when the application is stopped.
    pub async fn run_until_stopped(self) -> Result<(), hyper::Error> {
        match self.server {
            ServerType::TcpSocket(server) => {
                let graceful = server.with_graceful_shutdown(async move {
                    let _ = tokio::signal::ctrl_c().await;
                    tracing::info!("Was serving on tcp socket!");
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                });
                graceful.await?;
                Ok(())
            }
            ServerType::UnixSocket(server) => {
                // When future in this function is resolved, application
                // shutdowns. Also we could use tx/rx to shutdown from the
                // inside.
                let graceful = server.with_graceful_shutdown(async move {
                    let _ = tokio::signal::ctrl_c().await;
                    let path = self.unix_socket_file.unwrap();
                    delete_socket_file(path);
                    tracing::info!("Shutdown!");
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                });
                graceful.await?;
                Ok(())
            }
        }
    }

    /// Configure `Server`.
    fn build_server(
        unix_socket: &str,
        base_url: &str,
        listener: TcpListener,
        pool: ConnectionPool,
        email_client: EmailClient,
    ) -> (ServerType, Option<PathBuf>) {
        // We do not wrap pool into arc because internally it alreaday has an
        // `Arc`, and copying is cheap.
        let app_state = AppState {
            pool,
            email_client,
            base_url: base_url.to_string(),
        };
        let app =
            Router::new()
                .route("/health_check", routing::get(health_check))
                .route("/hello", routing::get(get_hello))
                .route("/subscriptions", routing::post(subscribe_handler))
                .route("/subscriptions/confirm", routing::get(confirm))
                // DEBUG:
                .fallback(routing::get(
                    |uri: axum::http::Uri,
                     orig_uri: axum::extract::OriginalUri| async move {
                        let uri = uri.path();
                        let orig_uri = orig_uri.path();
                        format!("uri: {}\norig_uri: {}", uri, orig_uri)
                    },
                ))
                .with_state(app_state);

        if unix_socket.is_empty() {
            tracing::info!("Running on tcp socket!");
            (
                ServerType::TcpSocket(
                    axum::Server::from_tcp(listener)
                        .expect("Cant create server from tcp listener.")
                        .serve(app.into_make_service()),
                ),
                None,
            )
        } else {
            tracing::info!("Running on unix socket: {}", unix_socket);
            (
                // FIXME: set socket file permissions from rust code
                ServerType::UnixSocket(
                    axum::Server::bind_unix(unix_socket)
                        .expect("Cant create server from unix socket.")
                        .serve(app.into_make_service()),
                ),
                // We need to store path to socket, for clean up it on shutdown
                Some(PathBuf::from(unix_socket)),
            )
        }
    }
}

/// Returns a connection pool to the PostgreSQL database.
async fn get_postgres_connection_pool(
    configuration: &Settings,
) -> ConnectionPool {
    let manager = PostgresConnectionManager::new_from_stringlike(
        configuration.database.connection_string().expose_secret(),
        tokio_postgres::NoTls,
    )
    .unwrap();
    bb8::Pool::builder().build(manager).await.unwrap()
}

fn delete_socket_file(path: PathBuf) {
    tracing::info!("Was serving on unix socket: {}", path.display());
    match std::fs::remove_file(path) {
        Ok(_) => {
            tracing::info!("Socket file was successfully deleted!")
        }
        Err(e) => {
            tracing::error!("Failed to delete socket file: {}", e)
        }
    }
}
