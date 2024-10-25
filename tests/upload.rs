pub mod common;
use fake::{faker::filesystem::en::FileName, Fake};
use nevermind::app::storage::path::S3Path;
use reqwest::StatusCode;

use crate::common::helpers::spawn_app;

#[tokio::test]
async fn upload_profile_works() {
    let app = spawn_app().await;
    let random_file_name: String = FileName().fake();
    let token = app.login_and_get_token().await;

    let img_body = serde_json::json!({
        "path": S3Path::Profile,
        "file_name": random_file_name,
        "file_type": "image/jpeg",
        "file_size": 250_000
    });

    let res = app
        .api_client
        .post(&format!("{}/upload", &app.address))
        .header("Authorization", "Bearer ".to_owned() + &token)
        .json(&img_body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(res.status().is_success());
}

#[tokio::test]
async fn upload_profile_fails_for_wrong_file_type() {
    let app = spawn_app().await;
    let random_file_name: String = FileName().fake();
    let token = app.login_and_get_token().await;

    let img_body = serde_json::json!({
        "path": S3Path::Profile,
        "file_name": random_file_name,
        "file_type": "image/avif",
        "file_size": 250_000
    });

    let res = app
        .api_client
        .post(&format!("{}/upload", &app.address))
        .header("Authorization", "Bearer ".to_owned() + &token)
        .json(&img_body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn upload_profile_fails_for_wrong_size() {
    let app = spawn_app().await;
    let random_file_name: String = FileName().fake();
    let token = app.login_and_get_token().await;

    let img_body = serde_json::json!({
        "path": S3Path::Profile,
        "file_name": random_file_name,
        "file_type": "image/jpeg",
        "file_size": 1_000_000
    });

    let res = app
        .api_client
        .post(&format!("{}/upload", &app.address))
        .header("Authorization", "Bearer ".to_owned() + &token)
        .json(&img_body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
