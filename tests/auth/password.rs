use crate::common::helpers::{spawn_app, TestApp};
use fake::{faker::internet::en::Password, Fake};
use redis::AsyncCommands;
use reqwest::StatusCode;

#[tokio::test]
async fn forgot_password_works() {
    let app = spawn_app().await;

    reset_password_send(&app).await;
}

#[tokio::test]
async fn reset_password_works() {
    let app = spawn_app().await;

    let reset_res = reset_password_send(&app).await;
    let new_password: String = Password(6..12).fake();

    let body = serde_json::json!({
        "token": reset_res.otp,
        "new_password": new_password
    });

    let res = app
        .api_client
        .post(&format!("{}/auth/reset-password", &app.address))
        .json(&body)
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    let login_body = serde_json::json!({
        "grant_type": "password",
        "email": &app.test_user.email,
        "password": new_password
    });

    let res = app.post_login(&login_body).await;
    assert!(res.status().is_success());
}

struct ResetPasswordRes {
    otp: String,
}

async fn reset_password_send(app: &TestApp) -> ResetPasswordRes {
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

    let mut conn = app
        .redis_client
        .get_multiplexed_tokio_connection()
        .await
        .unwrap();

    let key = "reset:*";
    let otps: Vec<String> = conn
        .keys(key)
        .await
        .expect("not error when connecting to redis");

    let current_otp = get_first_otp(&otps);
    assert!(current_otp.is_some(), "Expected a otp but found None");

    let current_otp = current_otp.unwrap();

    ResetPasswordRes { otp: current_otp }
}

fn get_first_otp(vec: &[String]) -> Option<String> {
    if let Some(first) = vec.get(0) {
        let parts: Vec<&str> = first.split(':').collect();
        if !parts.is_empty() {
            return Some(parts.last().unwrap().to_string());
        }
    }
    None
}
