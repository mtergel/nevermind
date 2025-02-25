use auth::token::TokenManager;
use aws_config::{meta::region::RegionProviderChain, BehaviorVersion, SdkConfig};
use axum::{middleware::from_fn_with_state, Router};
use email::client::EmailClient;
use middleware::{api_key_required, login_required};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::sync::Arc;
use storage::client::S3Storage;
use tokio::net::TcpListener;
use uuid::Uuid;

use axum::{extract::MatchedPath, http::Request};
use tower_http::trace::TraceLayer;
use tracing::info_span;

pub mod auth;
pub mod email;
pub mod error;
pub mod extrator;
pub mod middleware;
pub mod oauth;
pub mod otp;
pub mod storage;
pub mod utils;

use crate::{
    config::{AppConfig, Stage},
    routes::{
        admin,
        auth::{self as auth_route},
        docs, health_check, oauth as oauth_route, upload,
    },
};

pub struct Application {
    listener: TcpListener,
    pub port: u16,
    app: Router,
}

#[derive(Clone)]
pub struct ApiContext {
    pub config: Arc<AppConfig>,
    pub db_pool: Arc<PgPool>,
    pub redis_client: Arc<redis::Client>,
    pub token_manager: Arc<TokenManager>,
    pub email_client: Arc<EmailClient>,
    pub storage_client: Arc<S3Storage>,
    pub http_client: reqwest::Client,
}

impl Application {
    pub async fn build(config: AppConfig) -> Result<Self, anyhow::Error> {
        // Connection
        let addr = format!("{}:{}", config.host, config.port);
        let listener = TcpListener::bind(addr).await?;
        let port = listener.local_addr().unwrap().port();

        // Database
        let db_pool = get_db_connection_pool(&config);
        let redis_client = get_redis_client(&config);

        let token_manager = TokenManager::new(&config.hmac);

        let aws_config = get_aws_config().await;
        let email_client = EmailClient::new(
            &aws_config,
            &config.email.from_mail,
            &config.frontend.url,
            config.stage == Stage::Dev,
        );

        let storage_client = S3Storage::new(&aws_config, &config.aws.s3, &config.aws.cdn);

        // it uses arc internally
        let http_client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();

        let api_context = ApiContext {
            config: Arc::new(config),
            db_pool: Arc::new(db_pool),
            redis_client: Arc::new(redis_client),
            token_manager: Arc::new(token_manager),
            email_client: Arc::new(email_client),
            storage_client: Arc::new(storage_client),
            http_client,
        };

        let app = build_routes(api_context);

        Ok(Self {
            port,
            listener,
            app,
        })
    }

    /// Used in main, run the app
    pub async fn run_gracefully(self, close_rx: tokio::sync::oneshot::Receiver<()>) {
        axum::serve(self.listener, self.app)
            .with_graceful_shutdown(async move {
                _ = close_rx.await;
            })
            .await
            .unwrap();
    }

    /// Useful for tests
    /// Don't use in main
    pub async fn run_until_stopped(self) {
        axum::serve(self.listener, self.app).await.unwrap();
    }
}

fn build_routes(api_context: ApiContext) -> Router {
    let protected = Router::new()
        .merge(auth_route::router())
        .merge(upload::router())
        .merge(admin::router()) // Has extra permission route_layer inside
        .route_layer(from_fn_with_state(api_context.clone(), login_required));

    let api_key_protected = Router::new()
        .merge(auth_route::api_key_protected())
        .route_layer(from_fn_with_state(api_context.clone(), api_key_required));

    // Incoming request goes through middleware from bottom to top
    // and outgoing request goes through middleware from top to bottom

    Router::new()
        .merge(health_check::router())
        .merge(docs::router())
        .merge(oauth_route::router())
        .merge(auth_route::public_router())
        .merge(protected)
        .merge(api_key_protected)
        .with_state(api_context)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|req: &Request<_>| {
                    let request_id = Uuid::new_v4();

                    let matched_path = req
                        .extensions()
                        .get::<MatchedPath>()
                        .map(MatchedPath::as_str);

                    info_span!(
                        "http_request",
                        method = ?req.method(),
                        matched_path,
                        request_id = ?request_id,
                        user_id = tracing::field::Empty
                    )
                })
                .on_failure(()),
        )
}

pub fn get_db_connection_pool(config: &AppConfig) -> PgPool {
    PgPoolOptions::new().connect_lazy_with(config.db.db_connect_options())
}

pub fn get_redis_client(config: &AppConfig) -> redis::Client {
    redis::Client::open(config.redis.uri.clone()).unwrap()
}

async fn get_aws_config() -> SdkConfig {
    let region_provider = RegionProviderChain::default_provider().or_else("ap-southeast-1");

    aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await
}
