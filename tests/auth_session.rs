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
