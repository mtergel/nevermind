use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::common::helpers::spawn_app;

#[derive(Debug, Serialize, Deserialize)]
pub struct GrantResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

#[tokio::test]
async fn refresh_token_flow_works() {
    let app = spawn_app().await;

    let login_body = serde_json::json!({
        "grant_type": "password",
        "email": &app.test_user.email,
        "password": &app.test_user.password
    });
    let login_res = app.post_login(&login_body).await;
    assert!(login_res.status().is_success());
    let login_res_body = login_res.json::<GrantResponse>().await.unwrap();

    let input = serde_json::json!({
        "grant_type": "refresh_token",
        "refresh_token": login_res_body.refresh_token
    });

    let res = app.post_login(&input).await;
    let body = res.json::<GrantResponse>().await.unwrap();
    assert_eq!(
        body.expires_in,
        time::Duration::hours(1).whole_seconds() as u64
    );
}

#[tokio::test]
async fn refresh_token_missing() {
    let app = spawn_app().await;

    let input = serde_json::json!({
        "grant_type": "refresh_token",
    });

    let res = app.post_login(&input).await;
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn refresh_token_invalid() {
    let app = spawn_app().await;

    let input = serde_json::json!({
        "grant_type": "refresh_token",
        "refresh_token": "refresh-token-invalid"
    });

    let res = app.post_login(&input).await;
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn refresh_token_expired() {
    let app = spawn_app().await;

    let login_body = serde_json::json!({
        "grant_type": "password",
        "email": &app.test_user.email,
        "password": &app.test_user.password
    });
    let login_res = app.post_login(&login_body).await;
    assert!(login_res.status().is_success());
    let login_res_body = login_res.json::<GrantResponse>().await.unwrap();

    // remove session from redis
    let mut conn = app
        .redis_client
        .get_multiplexed_tokio_connection()
        .await
        .unwrap();

    let keys: Vec<String> = redis::cmd("KEYS")
        .arg(format!("user:{}:session_id:*", app.test_user.user_id))
        .query_async(&mut conn)
        .await
        .unwrap();

    let _: () = redis::cmd("JSON.DEL")
        .arg(&keys[0])
        .arg("$")
        .query_async(&mut conn)
        .await
        .unwrap();

    let input = serde_json::json!({
        "grant_type": "refresh_token",
        "refresh_token": login_res_body.refresh_token
    });

    let res = app.post_login(&input).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
