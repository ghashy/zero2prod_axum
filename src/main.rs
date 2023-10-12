// ───── Current Crate Imports ────────────────────────────────────────────── //

use zero2prod_axum::{configuration::Settings, startup::Application};

// ───── Body ─────────────────────────────────────────────────────────────── //

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_level(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set up tracing");

    // Panic if we can't read configuration
    let config =
        Settings::load_configuration().expect("Failed to read configuration.");

    if let Err(e) = Application::build(config)
        .await
        .expect("Failed to build application")
        .run_until_stopped()
        .await
    {
        eprintln!("Error: {}", e);
    }
}
