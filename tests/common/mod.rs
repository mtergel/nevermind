use clap::Parser;
use nevermind::{
    app::{get_db_connection_pool, Application},
    config::AppConfig,
    telemetry::register_telemetry,
};
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::sync::LazyLock;
use uuid::Uuid;

static TELEMETRY: LazyLock<()> = LazyLock::new(|| {
    register_telemetry();
});

pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub api_client: reqwest::Client,
    pub db_pool: PgPool,
}

impl TestApp {}

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
    let app = Application::build(app_config).await.unwrap();

    let test_app = TestApp {
        address: format!("http://localhost:{}", &app.port),
        port: app.port,
        api_client,
        db_pool,
    };

    _ = tokio::spawn(app.run_until_stopped());

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
