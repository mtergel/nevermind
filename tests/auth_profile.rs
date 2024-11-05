use serde::Deserialize;

pub mod common;
use common::helpers::spawn_app;

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
