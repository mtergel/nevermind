use clap::Parser;
use nevermind::{app::Application, config::AppConfig};

pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub api_client: reqwest::Client,
}

impl TestApp {}

pub async fn spawn_app() -> TestApp {
    // Config setup
    dotenvy::dotenv().ok();

    // Randomise configuration to ensure test isolation
    let app_config = {
        let mut c = AppConfig::parse();

        // Use a random OS port
        c.app_application_port = 0;

        c
    };

    let api_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let app = Application::build(app_config).await.unwrap();

    let test_app = TestApp {
        address: format!("http://localhost:{}", &app.port),
        port: app.port,
        api_client,
    };

    _ = tokio::spawn(app.run_until_stopped());

    test_app
}
