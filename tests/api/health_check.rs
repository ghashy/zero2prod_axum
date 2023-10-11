//! Health integration test.
//!
//! IMPORTANT: Runnig PostgreSQL server is required for testing!

use zero2prod_axum::configuration::Settings;

// ───── Current Crate Imports ────────────────────────────────────────────── //

use crate::helpers::{spawn_app_locally, TestApp};

// ───── Body ─────────────────────────────────────────────────────────────── //

#[tokio::test]
async fn health_check_test() {
    let config = Settings::load_configuration().unwrap();

    let TestApp { address, .. } = spawn_app_locally(config).await;

    // We need to bring in `reqwest`
    // to perform HTTP requests against our application.
    let client = reqwest::Client::new();

    // Act
    let response = client
        .get(&format!("{}/health_check", address))
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}