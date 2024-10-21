use secrecy::{ExposeSecret, SecretString};
use sqlx::postgres::{PgConnectOptions, PgSslMode};

#[derive(clap::Parser, Clone)]
pub struct AppConfig {
    #[clap(long, env)]
    pub stage: Stage,

    // App configs
    #[clap(long, env, default_value_t = 8080)]
    pub app_application_port: u16,

    #[clap(long, env, default_value = "0.0.0.0")]
    pub app_application_host: String,

    #[clap(long, env)]
    pub app_application_hmac: SecretString,

    #[clap(long, env, default_value = "http://localhost:3000")]
    pub app_frontend_url: String,

    #[clap(long, env)]
    pub app_from_mail: String,

    #[clap(long, env, default_value_t = 2)]
    pub app_application_account_email_limit: u8,

    #[clap(long, env, default_value = "127.0.0.0")]
    pub db_host: String,

    #[clap(long, env, default_value_t = 5432)]
    pub db_port: u16,

    #[clap(long, env)]
    pub db_username: String,

    #[clap(long, env)]
    pub db_password: SecretString,

    #[clap(long, env, default_value = "nevermind")]
    pub db_name: String,

    #[clap(long, env, default_value_t = false)]
    pub db_require_ssl: bool,

    #[clap(long, env)]
    pub redis_uri: SecretString,
}

#[derive(clap::ValueEnum, Debug, Clone, PartialEq)]
#[clap(rename_all = "kebab_case")]
pub enum Stage {
    Dev,
    Prod,
}

impl AppConfig {
    pub fn db_connect_options(&self) -> PgConnectOptions {
        let ssl_mode = if self.db_require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };

        PgConnectOptions::new()
            .host(&self.db_host)
            .username(&self.db_username)
            .password(self.db_password.expose_secret())
            .port(self.db_port)
            .ssl_mode(ssl_mode)
            .database(&self.db_name)
    }

    pub fn redis_connection_string(&self) -> &str {
        self.redis_uri.expose_secret()
    }
}
