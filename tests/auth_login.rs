use reqwest::StatusCode;

pub mod common;
use common::helpers::spawn_app;

#[tokio::test]
async fn login_works() {
    let app = spawn_app().await;

    let login_body = serde_json::json!({
        "grant_type": "password",
        "email": &app.test_user.email,
        "password": &app.test_user.password
    });

    let res = app.post_login(&login_body).await;
    assert!(res.status().is_success());
}

#[tokio::test]
async fn login_user_not_found() {
    let app = spawn_app().await;

    let login_body = serde_json::json!({
        "grant_type": "password",
        "email": "nonexistent@example.com",
        "password": "somepassword"
    });

    let res = app.post_login(&login_body).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_invalid_email_format() {
    let app = spawn_app().await;

    let login_body = serde_json::json!({
        "grant_type": "password",
        "email": "invalid-email",
        "password": &app.test_user.password
    });

    let res = app.post_login(&login_body).await;
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn login_missing_email() {
    let app = spawn_app().await;

    let login_body = serde_json::json!({
        "grant_type": "password",
        "password": &app.test_user.password
    });

    let res = app.post_login(&login_body).await;
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn login_missing_password() {
    let app = spawn_app().await;

    let login_body = serde_json::json!({
        "grant_type": "password",
        "email": &app.test_user.email
    });

    let res = app.post_login(&login_body).await;
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn login_missing_grant() {
    let app = spawn_app().await;

    let login_body = serde_json::json!({
        "email": &app.test_user.email,
        "password": &app.test_user.password
    });

    let res = app.post_login(&login_body).await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn login_invalid_grant() {
    let app = spawn_app().await;

    let login_body = serde_json::json!({
        "grant_type": "invalid-grant",
        "email": &app.test_user.email,
        "password": &app.test_user.password
    });

    let res = app.post_login(&login_body).await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn login_fails_for_reset_password() {
    let app = spawn_app().await;

    let _ = sqlx::query!(
        r#"
            update "user"
            set reset_password = true
            where user_id = $1
        "#,
        app.test_user.user_id
    )
    .execute(&app.db_pool)
    .await
    .unwrap();

    let login_body = serde_json::json!({
        "grant_type": "password",
        "email": &app.test_user.email,
        "password": &app.test_user.password
    });

    let res = app.post_login(&login_body).await;
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}
