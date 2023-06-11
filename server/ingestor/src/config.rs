use once_cell::sync::Lazy;

#[derive(serde::Deserialize, Debug)]
pub struct Env {
    pub environment_name: String,
}

pub static SENTRY_CONFIG: Lazy<Sentry> = Lazy::new(|| envy::prefixed("SENTRY_").from_env::<Sentry>().unwrap());
