use core::panic;

use fake::{faker::internet::en::Password, Fake};
use redis::AsyncCommands;
use reqwest::StatusCode;

pub mod common;
use common::helpers::{spawn_app, TestApp, TestUser};

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

#[tokio::test]
async fn change_password_works() {
    let app = spawn_app().await;
    let token = app.login_and_get_token().await;

    let new_user = TestUser::generate();
    let body = serde_json::json!({
        "password": app.test_user.password,
        "new_password": new_user.password
    });

    let res = app
        .api_client
        .post(&format!("{}/auth/change-password", &app.address))
        .header("Authorization", "Bearer ".to_owned() + &token)
        .json(&body)
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    let login_body = serde_json::json!({
        "grant_type": "password",
        "email": &app.test_user.email,
        "password": new_user.password
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

    let pattern = "reset:*";
    let mut iter: redis::AsyncIter<String> = conn
        .scan_match(pattern)
        .await
        .expect("failed to scan iterate to redis");

    let mut current_otp: Option<String> = None; // Make this mutable
    let mut otps: Vec<String> = Vec::new();

    while let Some(otp) = iter.next_item().await {
        otps.push(otp);
    }

    drop(iter);

    for otp in otps {
        let value: String = conn.get(&otp).await.expect("failed to get email using key");
        if value == app.test_user.email {
            current_otp = Some(otp);
            break;
        }
    }

    if let Some(otp) = current_otp {
        let token = extract_token(&otp);
        assert!(token.is_some(), "Expected a otp but found None");

        ResetPasswordRes {
            otp: token.unwrap(),
        }
    } else {
        panic!("could not find otp")
    }
}

fn extract_token(first: &str) -> Option<String> {
    let parts: Vec<&str> = first.split(':').collect();
    if !parts.is_empty() {
        return Some(parts.last().unwrap().to_string());
    }

    None
}
