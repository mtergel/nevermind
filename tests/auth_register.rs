use reqwest::StatusCode;

pub mod common;
use common::helpers::{spawn_app, TestUser};

#[tokio::test]
async fn register_works() {
    let app = spawn_app().await;

    let new_user = TestUser::generate();

    let register_body = serde_json::json!({
        "email": &new_user.email,
        "username": &new_user.username,
        "password": &new_user.password
    });

    let res = app
        .api_client
        .post(&format!("{}/auth/users", &app.address))
        .json(&register_body)
        .send()
        .await
        .expect("failed to execute request");

    assert!(res.status().is_success());

    let login_body = serde_json::json!({
        "grant_type": "password",
        "email": &new_user.email,
        "password": &new_user.password
    });

    let login_res = app.post_login(&login_body).await;
    assert!(login_res.status().is_success());
}

#[tokio::test]
async fn register_fails_for_already_registered_user() {
    let app = spawn_app().await;

    let new_user = TestUser::generate();

    let register_body = serde_json::json!({
        "email": &app.test_user.email,
        "username": &new_user.username,
        "password": &new_user.password
    });

    let res = app
        .api_client
        .post(&format!("{}/auth/users", &app.address))
        .json(&register_body)
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
