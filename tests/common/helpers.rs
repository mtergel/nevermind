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
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::sync::LazyLock;
use uuid::Uuid;

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

pub struct TestApp {
    pub address: String,
    pub api_client: reqwest::Client,
    pub db_pool: PgPool,
    pub redis_client: redis::Client,
    pub test_user: TestUser,
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
    // Config setup
    dotenvy::dotenv().ok();

    LazyLock::force(&TELEMETRY);

    // Randomise configuration to ensure test isolation
    let app_config = {
        let mut c = AppConfig::parse();

        // Use a different database for each test case
        c.db_name = Uuid::new_v4().to_string();

        // Use a random OS port
        c.app_application_port = 0;

        c
    };

    setup_database(&app_config).await;

    let api_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let db_pool = get_db_connection_pool(&app_config);
    let redis_client = get_redis_client(&app_config);
    let app = Application::build(app_config).await.unwrap();

    let test_app = TestApp {
        address: format!("http://localhost:{}", &app.port),
        api_client,
        db_pool,
        redis_client,
        test_user: TestUser::generate(),
    };

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
