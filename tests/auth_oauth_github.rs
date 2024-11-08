use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use wiremock::{
    matchers::{bearer_token, method, path},
    Mock, MockServer, ResponseTemplate,
};

pub mod common;
use common::helpers::{spawn_app, TestUser};

#[derive(Debug, PartialEq, Deserialize, Serialize, sqlx::Type)]
#[sqlx(type_name = "social_provider", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum AssertionProvider {
    Github,
    Google,
    Facebook,
    Discord,
}

#[tokio::test]
async fn github_oauth_for_new_user_works() {
    let app = spawn_app().await;

    // Front end get's code
    let code = generate_random_code(20);

    let login_body = serde_json::json!({
        "grant_type": "assertion",
        "code": code,
        "provider": "github"
    });

    let new_user = TestUser::generate();

    setup_oauth_mock(&app.oauth_mock_server, &new_user.email).await;
    let res = app.post_login(&login_body).await;
    assert!(res.status().is_success());

    let db_email = sqlx::query!(
        r#"
            select e.email_id, e.email, e.verified, 
            p.provider as "provider!: AssertionProvider"
            from email e
            inner join social_login p using (email_id)
            where email = $1
        "#,
        new_user.email
    )
    .fetch_one(&app.db_pool)
    .await
    .unwrap();

    assert_eq!(new_user.email, db_email.email);
    assert_eq!(db_email.provider, AssertionProvider::Github);
}

#[tokio::test]
async fn github_oauth_for_existing_user_works() {
    let app = spawn_app().await;

    // Front end get's code
    let code = generate_random_code(20);

    let login_body = serde_json::json!({
        "grant_type": "assertion",
        "code": code,
        "provider": "github"
    });

    setup_oauth_mock(&app.oauth_mock_server, &app.test_user.email).await;
    let res = app.post_login(&login_body).await;
    assert!(res.status().is_success());

    let db_email = sqlx::query!(
        r#"
            select e.email_id, e.email, e.verified, 
            p.provider as "provider!: AssertionProvider"
            from email e
            inner join social_login p using (email_id)
            where email = $1
        "#,
        app.test_user.email
    )
    .fetch_one(&app.db_pool)
    .await
    .unwrap();

    assert_eq!(app.test_user.email, db_email.email);
    assert_eq!(db_email.provider, AssertionProvider::Github);
}

#[tokio::test]
async fn github_oauth_for_existing_social_github_user_works() {
    let app = spawn_app().await;

    // Add social login for user
    let email_id = sqlx::query_scalar!(
        r#"
            select email_id
            from email
            where email = $1
        "#,
        app.test_user.email
    )
    .fetch_one(&app.db_pool)
    .await
    .unwrap();

    let _ = sqlx::query!(
        r#"
            insert into social_login (email_id, user_id, provider, provider_user_id)
            values ($1, $2, $3, $4)
        "#,
        email_id,
        app.test_user.user_id,
        "github" as _,
        "123456"
    )
    .execute(&app.db_pool)
    .await;

    // Front end get's code
    let code = generate_random_code(20);

    let login_body = serde_json::json!({
        "grant_type": "assertion",
        "code": code,
        "provider": "github"
    });

    setup_oauth_mock(&app.oauth_mock_server, &app.test_user.email).await;
    let res = app.post_login(&login_body).await;
    assert!(res.status().is_success());

    let db_email = sqlx::query!(
        r#"
            select e.email_id, e.email, e.verified, 
            p.provider as "provider!: AssertionProvider"
            from email e
            inner join social_login p using (email_id)
            where email = $1
        "#,
        app.test_user.email
    )
    .fetch_one(&app.db_pool)
    .await
    .unwrap();

    assert_eq!(app.test_user.email, db_email.email);
    assert_eq!(db_email.provider, AssertionProvider::Github);
}

fn generate_random_code(length: usize) -> String {
    let mut rng = thread_rng();
    let mut hex_string = String::with_capacity(length);

    // Generate random bytes and convert them to hex.
    for _ in 0..length {
        let random_byte: u8 = rng.gen();
        write!(&mut hex_string, "{:02x}", random_byte).expect("Failed to write to string");
    }

    hex_string
}

async fn setup_oauth_mock(server: &MockServer, email: &str) {
    // Mock Github OAuth
    let b_token = "gho_16C7e42F292c6912E7710c838347Ae178B4a";
    let token_body = serde_json::json!({
        "access_token": b_token,
        "scope": "user",
        "token_type": "bearer"
    });

    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        // this doesn't work idk
        // .and(query_param("client_id", &config.app_github_id))
        // .and(query_param("code", code))
        .respond_with(ResponseTemplate::new(200).set_body_json(token_body))
        .expect(1)
        .mount(server)
        .await;

    // Mock Github API
    let user_response = serde_json::json!({
        "login": "octocat",
        "id": 1,
        "node_id": "MDQ6VXNlcjE=",
        "avatar_url": "https://github.com/images/error/octocat_happy.gif",
        "url": "https://api.github.com/users/octocat",
        "name": "monalisa octocat",
        "email": email,
        "bio": "There once was...",
    });

    Mock::given(method("GET"))
        .and(path("/user"))
        .and(bearer_token(&b_token))
        .respond_with(ResponseTemplate::new(200).set_body_json(user_response))
        .expect(1)
        .mount(server)
        .await;
}
