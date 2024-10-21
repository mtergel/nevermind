use reqwest::StatusCode;

use crate::common::helpers::spawn_app;

#[tokio::test]
async fn forgot_password_works() {
    let app = spawn_app().await;

    let body = serde_json::json!({
        "email": &app.test_user.email,
    });

    let res = app
        .api_client
        .post(&format!("{}/auth/forgot-password", &app.address))
        .json(&body)
        .send()
        .await
        .expect("failed to execute request");

    assert!(res.status().is_success());
}
