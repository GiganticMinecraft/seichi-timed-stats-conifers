use once_cell::sync::Lazy;

#[derive(serde::Deserialize, Debug)]
pub struct Env {
    pub environment_name: String,
}

pub static ENV: Lazy<Env> = Lazy::new(|| envy::prefixed("ENV_").from_env::<Env>().unwrap());
