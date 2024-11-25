use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use sqlx::postgres::{PgConnectOptions, PgSslMode};

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub enum Stage {
    Dev,
    Prod,
}

impl Stage {
    pub fn as_str(&self) -> &'static str {
        match self {
            Stage::Dev => "dev",
            Stage::Prod => "prod",
        }
    }
}

impl TryFrom<String> for Stage {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "dev" => Ok(Self::Dev),
            "prod" => Ok(Self::Prod),
            other => Err(format!(
                "{} is not supported stage, Use either `dev` or `prod`",
                other
            )),
        }
    }
}

#[derive(Deserialize, Clone)]
pub struct AppConfig {
    pub stage: Stage,
    pub port: u16,
    pub host: String,
    pub hmac: SecretString,
    pub api_key: SecretString,

    pub frontend: FrontConfig,
    pub email: EmailConfig,
    pub github: GithubOAuthConfig,
    pub discord: DiscordOAuthConfig,
    pub db: DatabaseConfig,
    pub redis: RedisConfig,
    pub aws: AWSConfig,
}

#[derive(Deserialize, Clone)]
pub struct FrontConfig {
    pub url: String,
}

#[derive(Deserialize, Clone)]
pub struct EmailConfig {
    pub from_mail: String,
    pub account_email_limit: u8,
}

#[derive(Deserialize, Clone)]
pub struct GithubOAuthConfig {
    pub id: String,
    pub secret: SecretString,
    pub token_url: String,
    pub api_base_url: String,
}

#[derive(Deserialize, Clone)]
pub struct DiscordOAuthConfig {
    pub id: String,
    pub secret: SecretString,
    pub token_url: String,
    pub api_base_url: String,
}

#[derive(Deserialize, Clone)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: SecretString,
    pub name: String,
    pub require_ssl: bool,
}

#[derive(Deserialize, Clone)]
pub struct RedisConfig {
    pub uri: String,
}

#[derive(Deserialize, Clone)]
pub struct AWSConfig {
    pub s3: String,
    pub cdn: String,
}

impl DatabaseConfig {
    pub fn db_connect_options(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };

        PgConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(self.password.expose_secret())
            .port(self.port)
            .ssl_mode(ssl_mode)
            .database(&self.name)
    }
}

pub fn get_configuration() -> Result<AppConfig, config::ConfigError> {
    let base_path = std::env::current_dir().expect("Failed to determine the current directory");
    let configuration_dir = base_path.join("config");

    // Detect the running environment.
    // Default to `dev` if unspecified.
    let stage: Stage = std::env::var("STAGE")
        .unwrap_or_else(|_| "dev".into())
        .try_into()
        .expect("Failed to parse STAGE");

    let stage_config_filename = format!("{}.toml", stage.as_str());
    let app_config = config::Config::builder()
        .add_source(config::File::from(configuration_dir.join("base.toml")))
        .add_source(config::File::from(
            configuration_dir.join(stage_config_filename),
        ))
        // Add in settings from environment variables (with a prefix of APP and '__' as separator)
        // E.g. `APP_APPLICATION__PORT=5001 would set `Settings.application.port`
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
        // load stage env to config
        .set_override("stage", stage.as_str())?
        .build()?;

    // deserialize and freeze config
    app_config.try_deserialize::<AppConfig>()
}
