use zero2prod_axum::{configuration::Settings, startup::Application};

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::fmt()
        .with_timer(tracing_subscriber::fmt::time::ChronoLocal::default())
        .with_max_level(tracing::Level::INFO)
        .compact()
        .with_level(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set up tracing");

    let config = if let Ok(_) = std::env::var("RUSTEST") {
        Settings::load_configuration().unwrap()
    } else {
        Settings::load_configuration_from_env().unwrap()
    };

    if let Err(e) = Application::build(config)
        .await
        .expect("Failed to build application")
        .run_until_stopped()
        .await
    {
        eprintln!("Error: {}", e);
    }
}
