use clap::Parser;
use nevermind::{
    app::Application,
    config::AppConfig,
    telemetry::{build_telemetry, register_telemetry},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Config setup
    dotenvy::dotenv().ok();

    let app_config = AppConfig::parse();
    let app = Application::build(app_config).await?;

    // Setup telemetry
    let telemetry = build_telemetry(
        env!("CARGO_CRATE_NAME").into(),
        "info".into(),
        std::io::stdout,
    );
    register_telemetry(telemetry);

    // Run on tokio multi-thread
    let (close_tx, close_rx) = tokio::sync::oneshot::channel();
    let server_handle = tokio::spawn(async move { app.run_gracefully(close_rx).await });

    // Listen for shutdown command
    shutdown_signal().await;

    // Send shutdown signal to server
    _ = close_tx.send(());

    // Wait for server to gracefully shutdown
    _ = server_handle.await;

    Ok(())
}

async fn shutdown_signal() {
    use tokio::signal;

    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
