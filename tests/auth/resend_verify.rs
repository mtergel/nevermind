use crate::common::helpers::{register_new_user, spawn_app};
use redis::AsyncCommands;
use reqwest::StatusCode;

#[tokio::test]
async fn verify_email_works_uses_previous_token() {
    let app = spawn_app().await;
    let register_res = register_new_user(&app).await;
    let body = serde_json::json!({
        "email": &register_res.new_user.email,
    });

    let res = app
        .api_client
        .post(&format!("{}/auth/emails/resend", &app.address))
        .json(&body)
        .header(
            "Authorization",
            "Bearer ".to_owned() + &register_res.access_token,
        )
        .send()
        .await
        .expect("failed to execute request");

    assert!(res.status().is_success());

    let user_id = sqlx::query_scalar!(
        r#"
            select user_id
            from "user"
            where username = $1
        "#,
        register_res.new_user.username
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

    assert_eq!(otps.len(), 1);
}

#[tokio::test]
async fn verify_email_works_create_new_token() {
    let app = spawn_app().await;
    let register_res = register_new_user(&app).await;
    let body = serde_json::json!({
        "email": &register_res.new_user.email,
    });

    let user_id = sqlx::query_scalar!(
        r#"
            select user_id
            from "user"
            where username = $1
        "#,
        register_res.new_user.username
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
    let _: () = conn.del(&key).await.expect("failed to delete keys");

    let res = app
        .api_client
        .post(&format!("{}/auth/emails/resend", &app.address))
        .json(&body)
        .header(
            "Authorization",
            "Bearer ".to_owned() + &register_res.access_token,
        )
        .send()
        .await
        .expect("failed to execute request");

    assert!(res.status().is_success());

    let otps: Vec<String> = conn
        .keys(&key)
        .await
        .expect("not error when connecting to redis");

    assert_eq!(otps.len(), 1);
}

#[tokio::test]
async fn verify_email_fails_for_verified() {
    let app = spawn_app().await;
    let token = app.login_and_get_token().await;
    let body = serde_json::json!({
        "email": &app.test_user.email,
    });

    let res = app
        .api_client
        .post(&format!("{}/auth/emails/resend", &app.address))
        .json(&body)
        .header("Authorization", "Bearer ".to_owned() + &token)
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
