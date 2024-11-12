use serde::Deserialize;

pub mod common;
use common::helpers::{spawn_app, TestUser};

#[tokio::test]
async fn get_user_profile_works() {
    let app = spawn_app().await;
    let token = app.login_and_get_token().await;

    let res = app
        .api_client
        .get(&format!("{}/auth/me", &app.address))
        .header("Authorization", "Bearer ".to_owned() + &token)
        .send()
        .await
        .expect("failed to execute request");

    assert!(res.status().is_success());

    #[derive(Deserialize)]
    struct MeResponse {
        pub username: String,
        pub email: String,
        pub email_verified: bool,
    }

    let data = res.json::<MeResponse>().await.unwrap();

    assert_eq!(data.username, app.test_user.username);
    assert_eq!(data.email, app.test_user.email);
    assert_eq!(data.email_verified, true);
}

#[tokio::test]
async fn complete_user_profile_works() {
    let app = spawn_app().await;

    // update profile to mark as incomplete
    let _ = sqlx::query!(
        r#"
            update "user"
            set reset_username = true
            where user_id = $1

        "#,
        app.test_user.user_id
    )
    .execute(&app.db_pool)
    .await
    .unwrap();

    let token = app.login_and_get_token().await;

    let new_user = TestUser::generate();
    let update_body = serde_json::json!({
        "username": &new_user.username,
    });

    let update_res = app
        .api_client
        .post(&format!("{}/auth/me/complete", &app.address))
        .header("Authorization", "Bearer ".to_owned() + &token)
        .json(&update_body)
        .send()
        .await
        .expect("failed to execute request");

    assert!(update_res.status().is_success());

    // check profile
    let res = app
        .api_client
        .get(&format!("{}/auth/me", &app.address))
        .header("Authorization", "Bearer ".to_owned() + &token)
        .send()
        .await
        .expect("failed to execute request");

    assert!(res.status().is_success());

    #[derive(Deserialize)]
    struct MeResponse {
        pub username: String,
    }

    let data = res.json::<MeResponse>().await.unwrap();

    assert_eq!(data.username, new_user.username);
}

#[tokio::test]
async fn complete_user_profile_does_not_update_if_not_set() {
    let app = spawn_app().await;

    // remove mark
    let _ = sqlx::query!(
        r#"
            update "user"
            set reset_username = null
            where user_id = $1

        "#,
        app.test_user.user_id
    )
    .execute(&app.db_pool)
    .await
    .unwrap();

    let token = app.login_and_get_token().await;

    // new username
    let new_user = TestUser::generate();
    let update_body = serde_json::json!({
        "username": &new_user.username,
    });

    let update_res = app
        .api_client
        .post(&format!("{}/auth/me/complete", &app.address))
        .header("Authorization", "Bearer ".to_owned() + &token)
        .json(&update_body)
        .send()
        .await
        .expect("failed to execute request");

    assert!(update_res.status().is_success());

    // check profile
    let res = app
        .api_client
        .get(&format!("{}/auth/me", &app.address))
        .header("Authorization", "Bearer ".to_owned() + &token)
        .send()
        .await
        .expect("failed to execute request");

    assert!(res.status().is_success());

    #[derive(Deserialize)]
    struct MeResponse {
        pub username: String,
    }

    let data = res.json::<MeResponse>().await.unwrap();

    assert_eq!(data.username, app.test_user.username);
}
