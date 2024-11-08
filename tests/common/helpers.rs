use argon2::{password_hash::SaltString, Argon2, PasswordHash};
use clap::Parser;
use fake::{
    faker::internet::en::{Password, SafeEmail, Username},
    Fake,
};
use nevermind::{
    app::{get_db_connection_pool, get_redis_client, Application},
    config::AppConfig,
    telemetry::{build_telemetry, register_telemetry},
};
use redis::AsyncCommands;
use serde::Deserialize;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::sync::LazyLock;
use uuid::Uuid;
use wiremock::MockServer;

static TELEMETRY: LazyLock<()> = LazyLock::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let telemetry = build_telemetry(subscriber_name, default_filter_level, std::io::stdout);
        register_telemetry(telemetry);
    } else {
        let null_telemetry = build_telemetry(subscriber_name, default_filter_level, std::io::sink);
        register_telemetry(null_telemetry);
    };
});

#[derive(Debug, Deserialize)]
pub struct GrantResponse {
    pub access_token: String,
}

pub struct TestApp {
    pub address: String,
    pub api_client: reqwest::Client,
    pub db_pool: PgPool,
    pub redis_client: redis::Client,
    pub test_user: TestUser,
    pub config: AppConfig,
    pub oauth_mock_server: MockServer,
}

impl TestApp {
    pub async fn post_login<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(&format!("{}/oauth/token", &self.address))
            .json(body)
            .send()
            .await
            .expect("failed to execute request")
    }

    pub async fn login_and_get_token(&self) -> String {
        let login_body = serde_json::json!({
            "grant_type": "password",
            "email": &self.test_user.email,
            "password": &self.test_user.password
        });

        let res = self
            .api_client
            .post(&format!("{}/oauth/token", &self.address))
            .json(&login_body)
            .send()
            .await
            .expect("failed to execute request");

        let user_tokens = res.json::<GrantResponse>().await.unwrap();

        user_tokens.access_token
    }
}

pub struct TestUser {
    pub user_id: Uuid,
    pub username: String,
    pub email: String,
    pub password: String,
}

impl TestUser {
    pub fn generate() -> Self {
        TestUser {
            user_id: Uuid::new_v4(),
            username: Username().fake(),
            email: SafeEmail().fake(),
            password: Password(6..12).fake(),
        }
    }

    async fn store(&self, pool: &PgPool) {
        let salt = SaltString::generate(rand::thread_rng());
        let password_hash =
            PasswordHash::generate(Argon2::default(), &self.password, salt.as_salt())
                .unwrap()
                .to_string();

        sqlx::query!(
            r#"
                insert into "user" (user_id, username, password_hash)
                values ($1, $2, $3)
            "#,
            self.user_id,
            self.username,
            password_hash
        )
        .execute(pool)
        .await
        .expect("failed to store test user");

        sqlx::query!(
            r#"
                insert into email (user_id, email, verified, is_primary)
                values ($1, $2, $3, $4)
            "#,
            self.user_id,
            self.email,
            true,
            true
        )
        .execute(pool)
        .await
        .expect("failed to store test user");
    }
}

pub async fn spawn_app() -> TestApp {
    dotenvy::dotenv().ok();

    LazyLock::force(&TELEMETRY);

    // lauch github oauth mock
    let oauth_mock_server = MockServer::start().await;

    // Randomise configuration to ensure test isolation
    let app_config = {
        let mut c = AppConfig::parse();

        // Use a different database for each test case
        c.db_name = Uuid::new_v4().to_string();

        // Use a random OS port
        c.app_application_port = 0;

        c.app_github_api_base_uri = oauth_mock_server.uri();
        c.app_github_token_url = format!("{}/login/oauth/access_token", oauth_mock_server.uri());

        c.app_discord_api_base_uri = oauth_mock_server.uri();
        c.app_discord_token_url = format!("{}/api/v10/oauth2/token", oauth_mock_server.uri());

        c
    };

    tracing::info!("App config: {:?}", &app_config);

    setup_database(&app_config).await;

    let api_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let db_pool = get_db_connection_pool(&app_config);
    let redis_client = get_redis_client(&app_config);
    let mut test_app = TestApp {
        address: "".to_string(),
        api_client,
        db_pool,
        redis_client,
        test_user: TestUser::generate(),
        config: app_config.clone(),
        oauth_mock_server,
    };

    let app = Application::build(app_config).await.unwrap();
    test_app.address = format!("http://localhost:{}", &app.port);

    _ = tokio::spawn(app.run_until_stopped());

    test_app.test_user.store(&test_app.db_pool).await;

    test_app
}

async fn setup_database(config: &AppConfig) -> PgPool {
    // Connect to postgres instance and create new database
    let mut maintenance_config = config.clone();
    maintenance_config.db_name = "postgres".to_string();
    maintenance_config.db_username = "postgres".to_string();

    // Create database
    let mut connection = PgConnection::connect_with(&maintenance_config.db_connect_options())
        .await
        .expect("Failed to connect to Postgres");

    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.db_name).as_str())
        .await
        .expect("Failed to create database.");

    // Migrate database
    let connection_pool = PgPool::connect_with(config.db_connect_options())
        .await
        .expect("Failed to connect to Postgres.");

    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");

    connection_pool
}

pub struct RegisterNewUserRes {
    pub access_token: String,
    pub otp: String,
    pub new_user: TestUser,
}

pub async fn register_new_user(app: &TestApp) -> RegisterNewUserRes {
    let new_user = TestUser::generate();

    let register_body = serde_json::json!({
        "email": &new_user.email,
        "username": &new_user.username,
        "password": &new_user.password
    });

    let res = app
        .api_client
        .post(&format!("{}/auth/users", &app.address))
        .json(&register_body)
        .send()
        .await
        .expect("failed to execute request");

    assert!(res.status().is_success());

    let login_body = serde_json::json!({
        "grant_type": "password",
        "email": &new_user.email,
        "password": &new_user.password
    });

    let login_res = app.post_login(&login_body).await;
    assert!(login_res.status().is_success());

    let user_tokens = login_res.json::<GrantResponse>().await.unwrap();

    let user_id = sqlx::query_scalar!(
        r#"
            select user_id
            from "user"
            where username = $1
        "#,
        new_user.username
    )
    .fetch_one(&app.db_pool)
    .await
    .unwrap();

    let mut conn = app
        .redis_client
        .get_multiplexed_tokio_connection()
        .await
        .unwrap();
    let key = format!("user:{}:email:*", user_id);
    let otps: Vec<String> = conn
        .keys(key)
        .await
        .expect("not error when connecting to redis");

    let current_otp = get_first_otp(&otps);
    assert!(current_otp.is_some(), "Expected a otp but found None");

    let current_otp = current_otp.unwrap();

    RegisterNewUserRes {
        access_token: user_tokens.access_token,
        otp: current_otp,
        new_user,
    }
}

fn get_first_otp(vec: &[String]) -> Option<String> {
    if let Some(first) = vec.get(0) {
        let parts: Vec<&str> = first.split(':').collect();
        if !parts.is_empty() {
            return Some(parts.last().unwrap().to_string());
        }
    }
    None
}
