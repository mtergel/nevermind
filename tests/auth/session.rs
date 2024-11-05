use serde::Deserialize;

use crate::common::helpers::spawn_app;

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

    #[derive(Deserialize)]
    struct SessionData {
        #[allow(dead_code)]
        pub session_id: String,
    }

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
