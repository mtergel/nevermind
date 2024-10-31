use reqwest::StatusCode;

use crate::common::helpers::{register_new_user, spawn_app};

#[tokio::test]
async fn verify_email_works() {
    let app = spawn_app().await;

    let register_res = register_new_user(&app).await;

    let res = app
        .api_client
        .post(&format!(
            "{}/auth/emails/verify/{}",
            &app.address, &register_res.otp
        ))
        .header(
            "Authorization",
            "Bearer ".to_owned() + &register_res.access_token,
        )
        .send()
        .await
        .expect("failed to execute request");

    assert!(res.status().is_success());
}

#[tokio::test]
async fn verify_missing_auth_header() {
    let app = spawn_app().await;

    let res = app
        .api_client
        .post(&format!("{}/auth/emails/verify/some-token", &app.address))
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn verify_invalid_auth_token() {
    let app = spawn_app().await;
    let random_jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

    let res = app
        .api_client
        .post(&format!("{}/auth/emails/verify/some-token", &app.address))
        .header("Authorization", "Bearer ".to_owned() + random_jwt)
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn verify_invalid_otp() {
    let app = spawn_app().await;
    let register_res = register_new_user(&app).await;

    let res = app
        .api_client
        .post(&format!("{}/auth/emails/verify/some-token", &app.address))
        .header(
            "Authorization",
            "Bearer ".to_owned() + &register_res.access_token,
        )
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn verify_empty_otp() {
    let app = spawn_app().await;
    let register_res = register_new_user(&app).await;

    let res = app
        .api_client
        .post(&format!("{}/auth/emails/verify/", &app.address))
        .header(
            "Authorization",
            "Bearer ".to_owned() + &register_res.access_token,
        )
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
