use crate::common::helpers::spawn_app;
use fake::{faker::internet::en::SafeEmail, Fake};
use reqwest::StatusCode;

#[tokio::test]
async fn add_email_works() {
    let app = spawn_app().await;
    let token = app.login_and_get_token().await;

    let new_email: String = SafeEmail().fake();
    let new_email_body = serde_json::json!({
        "new_email": new_email,
        "password": app.test_user.password,
    });

    let res = app
        .api_client
        .post(&format!("{}/auth/emails", &app.address))
        .header("Authorization", "Bearer ".to_owned() + &token)
        .json(&new_email_body)
        .send()
        .await
        .expect("failed to execute request");

    assert!(res.status().is_success());

    let row = sqlx::query!(
        r#"
            select email, verified, is_primary
            from email
            where email = $1
        "#,
        new_email
    )
    .fetch_optional(&app.db_pool)
    .await
    .unwrap();

    assert!(row.is_some());

    let row_data = row.unwrap();

    assert_eq!(row_data.is_primary, false);
    assert_eq!(row_data.verified, false);
}

#[tokio::test]
async fn add_email_fails_when_existing() {
    let app = spawn_app().await;
    let token = app.login_and_get_token().await;

    let new_email: String = SafeEmail().fake();

    let user_id = sqlx::query_scalar!(
        r#"
            select user_id
            from "user"
            where username = $1
        "#,
        app.test_user.username
    )
    .fetch_one(&app.db_pool)
    .await
    .unwrap();

    let _ = sqlx::query!(
        r#"
            insert into email (email, user_id)
            values ($1, $2)
        "#,
        new_email,
        user_id
    )
    .execute(&app.db_pool)
    .await
    .unwrap();

    let new_email_body = serde_json::json!({
        "new_email": new_email,
        "password": app.test_user.password,
    });

    let res = app
        .api_client
        .post(&format!("{}/auth/emails", &app.address))
        .header("Authorization", "Bearer ".to_owned() + &token)
        .json(&new_email_body)
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY)
}
