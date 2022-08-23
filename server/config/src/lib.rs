use anyhow::Result;
use envy::Error;
use serde::Deserialize;

pub trait FromEnv: Sized {
    fn from_env() -> Result<Self, Error>;
}

trait FromEnvLikeKeyValuePairs: Sized {
    fn from_iter(iter: impl Iterator<Item = (String, String)> + Clone) -> Result<Self, Error>;
}

impl<T: FromEnvLikeKeyValuePairs> FromEnv for T {
    fn from_env() -> Result<Self, Error> {
        // std::env::Vars is not Clone
        Self::from_iter(std::env::vars().collect::<Vec<_>>().into_iter())
    }
}

#[derive(Deserialize, Eq, PartialEq, Debug)]
pub struct Port(pub u16);

#[derive(Deserialize, Debug)]
pub struct AppConfig {
    pub game_data_server_config: GameDataServerConfig,
    pub target_database_config: ConiferDatabaseConfig,
}

impl FromEnvLikeKeyValuePairs for AppConfig {
    fn from_iter(iter: impl Iterator<Item = (String, String)> + Clone) -> Result<Self, Error> {
        Ok(Self {
            game_data_server_config: GameDataServerConfig::from_iter(iter.clone())?,
            target_database_config: ConiferDatabaseConfig::from_iter(iter)?,
        })
    }
}

#[derive(Deserialize, Debug)]
pub struct GameDataServerConfig {
    pub grpc_endpoint_url: String,
}

#[derive(Deserialize, Debug)]
pub struct ConiferDatabaseConfig {
    pub host: String,
    pub port: Port,
    pub database_name: String,
    pub user: String,
    pub password: String,
}

impl FromEnvLikeKeyValuePairs for GameDataServerConfig {
    fn from_iter(iter: impl Iterator<Item = (String, String)>) -> Result<Self, Error> {
        envy::prefixed("GAME_DATA_SERVER_").from_iter(iter)
    }
}

impl FromEnvLikeKeyValuePairs for ConiferDatabaseConfig {
    fn from_iter(iter: impl Iterator<Item = (String, String)>) -> Result<Self, Error> {
        envy::prefixed("CONIFERS_DB_").from_iter(iter)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn read_config_from_iterator() {
        let setting = [
            (
                "GAME_DATA_SERVER_GRPC_ENDPOINT_URL".to_string(),
                "https://game-data-server-url.local/grpc".to_string(),
            ),
            (
                "CONIFERS_DB_HOST".to_string(),
                "example-db-host.local".to_string(),
            ),
            ("CONIFERS_DB_PORT".to_string(), "3307".to_string()),
            (
                "CONIFERS_DB_DATABASE_NAME".to_string(),
                "conifers".to_string(),
            ),
            ("CONIFERS_DB_USER".to_string(), "mysql".to_string()),
            (
                "CONIFERS_DB_PASSWORD".to_string(),
                "$tr0ngpAssw0rd".to_string(),
            ),
        ];

        AppConfig::from_iter(setting.into_iter()).unwrap();
    }
}
