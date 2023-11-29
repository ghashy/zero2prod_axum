use bb8_postgres::PostgresConnectionManager;
use refinery::embed_migrations;

embed_migrations!("./migrations");

pub(super) async fn run_migration(
    postgres_connection: &bb8::Pool<
        PostgresConnectionManager<tokio_postgres::NoTls>,
    >,
) {
    let report = match migrations::runner()
        .run_async(
            &mut postgres_connection.dedicated_connection().await.unwrap(),
        )
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Can't run migration on db: {}", e);
            return;
        }
    };

    if report.applied_migrations().is_empty() {
        tracing::info!("No migrations applied");
    }

    for migration in report.applied_migrations() {
        tracing::info!("Migration: {}", migration);
    }
}
