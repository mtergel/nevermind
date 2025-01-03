use reqwest::StatusCode;
use secrecy::ExposeSecret;
use serde::Deserialize;

pub mod common;
use common::helpers::spawn_app;

#[derive(Deserialize)]
struct SessionData {
    pub session_id: String,
}

#[tokio::test]
async fn list_session_works() {
    let app = spawn_app().await;
    let token = app.login_and_get_token().await;

    let res = app
        .api_client
        .get(&format!("{}/auth/sessions", &app.address))
        .header("Authorization", "Bearer ".to_owned() + &token)
        .send()
        .await
        .expect("failed to execute request");

    assert!(res.status().is_success());

    let data = res.json::<Vec<SessionData>>().await.unwrap();
    assert_eq!(data.len(), 1);

    // Add new session again
    let token = app.login_and_get_token().await;

    let res = app
        .api_client
        .get(&format!("{}/auth/sessions", &app.address))
        .header("Authorization", "Bearer ".to_owned() + &token)
        .send()
        .await
        .expect("failed to execute request");

    let data = res.json::<Vec<SessionData>>().await.unwrap();
    assert_eq!(data.len(), 2);
}

#[tokio::test]
async fn revoke_session_works() {
    let app = spawn_app().await;
    let token = app.login_and_get_token().await;

    let res = app
        .api_client
        .get(&format!("{}/auth/sessions", &app.address))
        .header("Authorization", "Bearer ".to_owned() + &token)
        .send()
        .await
        .expect("failed to execute request");

    assert!(res.status().is_success());

    let data = res.json::<Vec<SessionData>>().await.unwrap();
    assert_eq!(data.len(), 1);

    let revoke_body = serde_json::json!({
        "session_id": &data[0].session_id,
        "password": &app.test_user.password
    });

    app.api_client
        .delete(&format!("{}/auth/sessions/revoke", &app.address))
        .header("Authorization", "Bearer ".to_owned() + &token)
        .json(&revoke_body)
        .send()
        .await
        .expect("failed to execute request");

    let res = app
        .api_client
        .get(&format!("{}/auth/sessions", &app.address))
        .header("Authorization", "Bearer ".to_owned() + &token)
        .send()
        .await
        .expect("failed to execute request");

    let data = res.json::<Vec<SessionData>>().await.unwrap();
    assert_eq!(data.len(), 0);
}

#[tokio::test]
async fn revoke_session_by_id_works() {
    let app = spawn_app().await;
    let token = app.login_and_get_token().await;

    let res = app
        .api_client
        .get(&format!("{}/auth/sessions", &app.address))
        .header("Authorization", "Bearer ".to_owned() + &token)
        .send()
        .await
        .expect("failed to execute request");

    assert!(res.status().is_success());

    let data = res.json::<Vec<SessionData>>().await.unwrap();
    assert_eq!(data.len(), 1);

    let revoke_body = serde_json::json!({
        "user_id": &app.test_user.user_id
    });

    // send request as machine
    let res = app
        .api_client
        .delete(&format!(
            "{}/auth/sessions/{}/revoke",
            &app.address, &data[0].session_id
        ))
        .header("X-Api-Key", app.config.api_key.expose_secret())
        .json(&revoke_body)
        .send()
        .await
        .expect("failed to execute request");

    dbg!(&res);
    assert!(res.status().is_success());

    // check again
    let res = app
        .api_client
        .get(&format!("{}/auth/sessions", &app.address))
        .header("Authorization", "Bearer ".to_owned() + &token)
        .send()
        .await
        .expect("failed to execute request");

    let data = res.json::<Vec<SessionData>>().await.unwrap();
    assert_eq!(data.len(), 0);
}

#[tokio::test]
async fn revoke_session_by_id_fails_if_api_key_is_wrong() {
    let app = spawn_app().await;

    let revoke_body = serde_json::json!({
        "user_id": &app.test_user.user_id
    });

    // send request as machine
    let res = app
        .api_client
        .delete(&format!(
            "{}/auth/sessions/{}/revoke",
            &app.address, "some-id"
        ))
        .header("X-Api-Key", "some-random-key")
        .json(&revoke_body)
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn revoke_session_by_id_fails_if_api_key_is_missing() {
    let app = spawn_app().await;

    let revoke_body = serde_json::json!({
        "user_id": &app.test_user.user_id
    });

    // send request as machine
    let res = app
        .api_client
        .delete(&format!(
            "{}/auth/sessions/{}/revoke",
            &app.address, "some-id"
        ))
        .json(&revoke_body)
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
