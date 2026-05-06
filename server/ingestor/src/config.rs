use std::sync::LazyLock;

#[derive(serde::Deserialize, Debug)]
pub struct Sentry {
    pub environment_name: String,
}

pub static SENTRY_CONFIG: LazyLock<Sentry> =
    LazyLock::new(|| envy::prefixed("SENTRY_").from_env::<Sentry>().unwrap());
