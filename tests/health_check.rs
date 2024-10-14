pub mod common;
use crate::common::helpers::spawn_app;

#[tokio::test]
async fn health_check_works() {
    let app = spawn_app().await;

    let res = app
        .api_client
        .get(&format!("{}/health_check", &app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(res.status().is_success());
}
