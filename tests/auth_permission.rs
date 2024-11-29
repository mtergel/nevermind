use reqwest::StatusCode;

pub mod common;
use common::helpers::spawn_app;

#[tokio::test]
async fn permission_check_fails_for_no_access() {
    let app = spawn_app().await;
    let token = app.login_and_get_token().await;

    let res = app
        .api_client
        .post(&format!("{}/users", &app.address))
        .header("Authorization", "Bearer ".to_owned() + &token)
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn permission_check_works_for_root() {
    let app = spawn_app().await;

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
        .get(&format!("{}/users", &app.address))
        .header("Authorization", "Bearer ".to_owned() + &token)
        .send()
        .await
        .expect("failed to execute request");

    assert!(res.status().is_success());
}
