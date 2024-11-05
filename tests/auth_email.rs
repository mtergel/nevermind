use fake::{faker::internet::en::SafeEmail, Fake};
use reqwest::StatusCode;
use serde::Deserialize;
use uuid::Uuid;

pub mod common;
use common::helpers::spawn_app;

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
    let new_email_id = sqlx::query_scalar!(
        r#"
            insert into email(user_id, email)
            values ($1, $2)
            returning email_id
        "#,
        app.test_user.user_id,
        new_email
    )
    .fetch_one(&app.db_pool)
    .await
    .unwrap();

    let token = app.login_and_get_token().await;

    let res = app
        .api_client
        .patch(&format!(
            "{}/auth/emails/{}/primary",
            &app.address, &new_email_id
        ))
        .header("Authorization", "Bearer ".to_owned() + &token)
        .send()
        .await
        .expect("failed to execute request");

    assert!(res.status().is_success());

    // new email should be primary
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

    assert_eq!(is_primary, true);

    // previous email should be not primary
    let is_primary = sqlx::query_scalar!(
        r#"
            select is_primary
            from email
            where email = $1
        "#,
        &app.test_user.email
    )
    .fetch_one(&app.db_pool)
    .await
    .unwrap();

    assert_eq!(is_primary, false)
}

#[tokio::test]
async fn make_email_primary_fails_for_invalid_email() {
    let app = spawn_app().await;

    let token = app.login_and_get_token().await;
    let new_uuid = Uuid::new_v4().to_string();

    let res = app
        .api_client
        .patch(&format!(
            "{}/auth/emails/{}/primary",
            &app.address, &new_uuid
        ))
        .header("Authorization", "Bearer ".to_owned() + &token)
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(res.status(), StatusCode::NOT_FOUND);
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

#[tokio::test]
async fn delete_email_works() {
    let app = spawn_app().await;
    let token = app.login_and_get_token().await;

    let new_email: String = SafeEmail().fake();
    let email_id = sqlx::query_scalar!(
        r#"
            insert into email(user_id, email)
            values ($1, $2)
            returning email_id
        "#,
        app.test_user.user_id,
        new_email
    )
    .fetch_one(&app.db_pool)
    .await
    .unwrap();

    let res = app
        .api_client
        .delete(&format!("{}/auth/emails/{}", &app.address, &email_id))
        .header("Authorization", "Bearer ".to_owned() + &token)
        .send()
        .await
        .expect("failed to execute request");

    assert!(res.status().is_success());

    let row = sqlx::query_scalar!(
        r#"
            select email_id 
            from email
            where email_id = $1
        "#,
        email_id
    )
    .fetch_optional(&app.db_pool)
    .await
    .unwrap();

    assert!(row.is_none());
}

#[tokio::test]
async fn delete_email_fails_for_invalid_email() {
    let app = spawn_app().await;
    let token = app.login_and_get_token().await;

    let email_id: String = Uuid::new_v4().to_string();

    let res = app
        .api_client
        .delete(&format!("{}/auth/emails/{}", &app.address, &email_id))
        .header("Authorization", "Bearer ".to_owned() + &token)
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
