use crate::common::helpers::spawn_app;
use fake::{faker::internet::en::SafeEmail, Fake};
use reqwest::StatusCode;
use serde::Deserialize;

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

#[tokio::test]
async fn make_email_primary_works() {
    let app = spawn_app().await;

    // insert new email
    let new_email: String = SafeEmail().fake();
    let _ = sqlx::query!(
        r#"
            insert into email(user_id, email)
            values ($1, $2)
        "#,
        app.test_user.user_id,
        new_email
    )
    .execute(&app.db_pool)
    .await
    .unwrap();

    let token = app.login_and_get_token().await;
    let update_email_to_primary_body = serde_json::json!({
        "email": new_email,
    });

    let res = app
        .api_client
        .post(&format!("{}/auth/emails/primary", &app.address))
        .header("Authorization", "Bearer ".to_owned() + &token)
        .json(&update_email_to_primary_body)
        .send()
        .await
        .expect("failed to execute request");

    assert!(res.status().is_success());

    let is_primary = sqlx::query_scalar!(
        r#"
            select is_primary
            from email
            where email = $1
        "#,
        new_email
    )
    .fetch_one(&app.db_pool)
    .await
    .unwrap();

    assert_eq!(is_primary, true)
}

#[tokio::test]
async fn list_email_works() {
    let app = spawn_app().await;
    let token = app.login_and_get_token().await;

    let res = app
        .api_client
        .get(&format!("{}/auth/emails", &app.address))
        .header("Authorization", "Bearer ".to_owned() + &token)
        .send()
        .await
        .expect("failed to execute request");

    #[derive(Deserialize)]
    struct EmailResponse {
        #[allow(dead_code)]
        pub email_id: String,
    }

    assert!(res.status().is_success());

    let emails = res.json::<Vec<EmailResponse>>().await.unwrap();
    assert_eq!(emails.len(), 1);

    // insert new email
    let new_email: String = SafeEmail().fake();
    let _ = sqlx::query!(
        r#"
            insert into email(user_id, email)
            values ($1, $2)
        "#,
        app.test_user.user_id,
        new_email
    )
    .execute(&app.db_pool)
    .await
    .unwrap();

    let res = app
        .api_client
        .get(&format!("{}/auth/emails", &app.address))
        .header("Authorization", "Bearer ".to_owned() + &token)
        .send()
        .await
        .expect("failed to execute request");

    assert!(res.status().is_success());

    let emails = res.json::<Vec<EmailResponse>>().await.unwrap();
    assert_eq!(emails.len(), 2);
}
