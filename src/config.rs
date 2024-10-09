use secrecy::SecretString;

#[derive(clap::Parser)]
pub struct AppConfig {
    #[clap(long, env)]
    pub stage: Stage,

    // App configs
    #[clap(long, env)]
    pub app_application_port: u16,

    #[clap(long, env)]
    pub app_application_host: String,

    #[clap(long, env)]
    pub app_application_hmac: SecretString,
}

#[derive(clap::ValueEnum, Debug, Clone)]
#[clap(rename_all = "kebab_case")]
pub enum Stage {
    Dev,
    Prod,
}
