use reqwest::StatusCode;

pub mod common;
use common::helpers::spawn_app;
use serde::Deserialize;

#[tokio::test]
async fn permission_check_fails_for_no_access() {
    let app = spawn_app().await;
    let token = app.login_and_get_token().await;

    let res = app
        .api_client
        .post(&format!("{}/admin/users", &app.address))
        .header("Authorization", "Bearer ".to_owned() + &token)
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn permission_check_works_for_root() {
    let app = spawn_app().await;

    // adding permission
    let _ = sqlx::query!(
        r#"
            insert into user_role (user_id, role)
            values ($1, $2::app_role)
        "#,
        app.test_user.user_id,
        "root" as _
    )
    .execute(&app.db_pool)
    .await
    .unwrap();

    let token = app.login_and_get_token().await;

    let res = app
        .api_client
        .get(&format!("{}/admin/users", &app.address))
        .header("Authorization", "Bearer ".to_owned() + &token)
        .send()
        .await
        .expect("failed to execute request");

    assert!(res.status().is_success());
}

#[tokio::test]
async fn permission_check_works_for_moderator() {
    let app = spawn_app().await;

    // adding permission
    let _ = sqlx::query!(
        r#"
            insert into user_role (user_id, role)
            values ($1, $2::app_role)
        "#,
        app.test_user.user_id,
        "moderator" as _
    )
    .execute(&app.db_pool)
    .await
    .unwrap();

    let token = app.login_and_get_token().await;

    let res = app
        .api_client
        .get(&format!("{}/admin/users", &app.address))
        .header("Authorization", "Bearer ".to_owned() + &token)
        .send()
        .await
        .expect("failed to execute request");

    assert!(res.status().is_success());
}

#[tokio::test]
async fn scope_is_added_to_jwt() {
    let app = spawn_app().await;

    // adding permission
    let _ = sqlx::query!(
        r#"
            insert into user_role (user_id, role)
            values ($1, $2::app_role)
        "#,
        app.test_user.user_id,
        "root" as _
    )
    .execute(&app.db_pool)
    .await
    .unwrap();

    let login_body = serde_json::json!({
        "grant_type": "password",
        "email": &app.test_user.email,
        "password": &app.test_user.password
    });

    let res = app
        .api_client
        .post(&format!("{}/oauth/token", &app.address))
        .json(&login_body)
        .send()
        .await
        .expect("failed to execute request");

    #[derive(Debug, Deserialize)]
    pub struct GrantResponse {
        pub scope: String, // space separeted scope tokens
    }

    let user_res = res.json::<GrantResponse>().await.unwrap();
    let base_root_scopes = vec!["user.view"];

    assert!(base_root_scopes.iter().all(|&s| user_res.scope.contains(s)))
}
