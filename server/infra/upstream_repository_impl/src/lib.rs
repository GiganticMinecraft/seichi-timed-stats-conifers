#[allow(dead_code)]
#[allow(clippy::nursery, clippy::pedantic, clippy::all)]
mod buf_generated {
    include!("gen/mod.rs");
}

pub mod config {
    #[derive(serde::Deserialize, Debug, Clone)]
    pub struct GrpcClient {
        pub game_data_server_grpc_endpoint_url: String,
    }

    impl GrpcClient {
        pub fn from_env() -> anyhow::Result<Self> {
            Ok(envy::from_env::<Self>()?)
        }
    }
}

use anyhow::anyhow;
use buf_generated::gigantic_minecraft::seichi_game_data::v1::read_service_client::ReadServiceClient;
use std::collections::HashMap;
type GameDataGrpcClient = ReadServiceClient<tonic::transport::Channel>;

#[derive(Debug)]
pub struct GrpcUpstreamRepository {
    client: GameDataGrpcClient,
}

impl GrpcUpstreamRepository {
    pub async fn try_new(config: config::GrpcClient) -> anyhow::Result<Self> {
        let client = ReadServiceClient::connect(config.game_data_server_grpc_endpoint_url).await?;
        Ok(Self { client })
    }

    pub(crate) fn client(&self) -> GameDataGrpcClient {
        self.client.clone()
    }
}

fn empty_request() -> tonic::Request<pbjson_types::Empty> {
    tonic::Request::new(pbjson_types::Empty::default())
}

use domain::models::{
    BreakCount, BuildCount, PlayTicks, Player, PlayerUuidString, StatsSnapshot, VoteCount,
};
use domain::repositories::PlayerStatsRepository;

#[async_trait::async_trait]
impl PlayerStatsRepository<BreakCount> for GrpcUpstreamRepository {
    async fn fetch_stats_snapshot_of_all_players(
        &self,
    ) -> anyhow::Result<StatsSnapshot<BreakCount>> {
        let request_time = chrono::Utc::now();
        let map = self
            .client()
            .break_counts(empty_request())
            .await?
            .into_inner()
            .results
            .into_iter()
            .map(|count| {
                anyhow::Ok((
                    Player {
                        uuid: PlayerUuidString::from_string(
                            &count.player.ok_or(anyhow!("player_uuid is missing"))?.uuid,
                        )?,
                    },
                    BreakCount(count.break_count as u64),
                ))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;

        Ok(StatsSnapshot {
            player_stats: map,
            utc_timestamp: request_time,
        })
    }
}

#[async_trait::async_trait]
impl PlayerStatsRepository<BuildCount> for GrpcUpstreamRepository {
    async fn fetch_stats_snapshot_of_all_players(
        &self,
    ) -> anyhow::Result<StatsSnapshot<BuildCount>> {
        let request_time = chrono::Utc::now();
        let map = self
            .client()
            .build_counts(empty_request())
            .await?
            .into_inner()
            .results
            .into_iter()
            .map(|count| {
                anyhow::Ok((
                    Player {
                        uuid: PlayerUuidString::from_string(
                            &count.player.ok_or(anyhow!("player_uuid is missing"))?.uuid,
                        )?,
                    },
                    BuildCount(count.build_count as u64),
                ))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;

        Ok(StatsSnapshot {
            player_stats: map,
            utc_timestamp: request_time,
        })
    }
}

#[async_trait::async_trait]
impl PlayerStatsRepository<PlayTicks> for GrpcUpstreamRepository {
    async fn fetch_stats_snapshot_of_all_players(
        &self,
    ) -> anyhow::Result<StatsSnapshot<PlayTicks>> {
        let request_time = chrono::Utc::now();
        let map = self
            .client()
            .play_ticks(empty_request())
            .await?
            .into_inner()
            .results
            .into_iter()
            .map(|count| {
                anyhow::Ok((
                    Player {
                        uuid: PlayerUuidString::from_string(
                            &count.player.ok_or(anyhow!("player_uuid is missing"))?.uuid,
                        )?,
                    },
                    PlayTicks(count.play_ticks as u64),
                ))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;

        Ok(StatsSnapshot {
            player_stats: map,
            utc_timestamp: request_time,
        })
    }
}

#[async_trait::async_trait]
impl PlayerStatsRepository<VoteCount> for GrpcUpstreamRepository {
    async fn fetch_stats_snapshot_of_all_players(
        &self,
    ) -> anyhow::Result<StatsSnapshot<VoteCount>> {
        let request_time = chrono::Utc::now();
        let map = self
            .client()
            .vote_counts(empty_request())
            .await?
            .into_inner()
            .results
            .into_iter()
            .map(|count| {
                anyhow::Ok((
                    Player {
                        uuid: PlayerUuidString::from_string(
                            &count.player.ok_or(anyhow!("player_uuid is missing"))?.uuid,
                        )?,
                    },
                    VoteCount(count.vote_count as u64),
                ))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;

        Ok(StatsSnapshot {
            player_stats: map,
            utc_timestamp: request_time,
        })
    }
}
