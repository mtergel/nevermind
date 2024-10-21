use redis::AsyncCommands;
use reqwest::StatusCode;
use serde::Deserialize;

use crate::common::helpers::{spawn_app, TestApp, TestUser};

#[derive(Debug, Deserialize)]
struct GrantResponse {
    pub access_token: String,
}

#[tokio::test]
async fn verify_email_works() {
    let app = spawn_app().await;

    let register_res = register_new_user(&app).await;

    let verify_body = serde_json::json!({
        "token": register_res.otp
    });

    let res = app
        .api_client
        .post(&format!("{}/auth/emails/verify", &app.address))
        .header(
            "Authorization",
            "Bearer ".to_owned() + &register_res.access_token,
        )
        .json(&verify_body)
        .send()
        .await
        .expect("failed to execute request");

    assert!(res.status().is_success());
}

#[tokio::test]
async fn verify_missing_auth_header() {
    let app = spawn_app().await;

    let verify_body = serde_json::json!({
        "token": "some-token"
    });

    let res = app
        .api_client
        .post(&format!("{}/auth/emails/verify", &app.address))
        .json(&verify_body)
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn verify_invalid_auth_token() {
    let app = spawn_app().await;

    let verify_body = serde_json::json!({
        "token": "some-token"
    });

    let res = app
        .api_client
        .post(&format!("{}/auth/emails/verify", &app.address))
        .header("Authorization", "Bearer ".to_owned() + "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c")
        .json(&verify_body)
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn verify_invalid_otp() {
    let app = spawn_app().await;
    let register_res = register_new_user(&app).await;

    let verify_body = serde_json::json!({
        "token": "some-token"
    });

    let res = app
        .api_client
        .post(&format!("{}/auth/emails/verify", &app.address))
        .header(
            "Authorization",
            "Bearer ".to_owned() + &register_res.access_token,
        )
        .json(&verify_body)
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn verify_missing_token_field_from_body() {
    let app = spawn_app().await;
    let register_res = register_new_user(&app).await;

    let verify_body = serde_json::json!({
        "some-other-field": "other"
    });

    let res = app
        .api_client
        .post(&format!("{}/auth/emails/verify", &app.address))
        .header(
            "Authorization",
            "Bearer ".to_owned() + &register_res.access_token,
        )
        .json(&verify_body)
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

struct RegisterNewUserRes {
    access_token: String,
    otp: String,
}

async fn register_new_user(app: &TestApp) -> RegisterNewUserRes {
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

    let user_tokens = login_res.json::<GrantResponse>().await.unwrap();

    let user_id = sqlx::query_scalar!(
        r#"
            select user_id
            from "user"
            where username = $1
        "#,
        new_user.username
    )
    .fetch_one(&app.db_pool)
    .await
    .unwrap();

    let mut conn = app
        .redis_client
        .get_multiplexed_tokio_connection()
        .await
        .unwrap();
    let key = format!("user:{}:email:*", user_id);
    let otps: Vec<String> = conn
        .keys(key)
        .await
        .expect("not error when connecting to redis");

    let current_otp = get_first_otp(&otps);
    assert!(current_otp.is_some(), "Expected a otp but found None");

    let current_otp = current_otp.unwrap();

    RegisterNewUserRes {
        access_token: user_tokens.access_token,
        otp: current_otp,
    }
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
