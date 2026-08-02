#![deny(clippy::all, clippy::cargo)]
#![warn(clippy::nursery, clippy::pedantic)]
#![allow(clippy::cargo_common_metadata, clippy::multiple_crate_versions)]

use pyroscope::backend::{pprof_backend, BackendConfig, PprofConfig};
use pyroscope::pyroscope::PyroscopeAgentBuilder;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

use domain::models::{BreakCount, BuildCount, PlayTicks, VoteCount};
use domain::repositories::{PlayerStatsRepository, PlayerTimedStatsRepository};

async fn stats_repository_impl() -> anyhow::Result<
    impl PlayerStatsRepository<BreakCount>
        + PlayerStatsRepository<BuildCount>
        + PlayerStatsRepository<PlayTicks>
        + PlayerStatsRepository<VoteCount>,
> {
    use infra_upstream_repository_impl::{config::GrpcClient, GrpcUpstreamRepository};
    GrpcUpstreamRepository::try_new(GrpcClient::from_env()?).await
}

async fn timed_stats_repository_impl() -> anyhow::Result<
    impl PlayerTimedStatsRepository<BreakCount>
        + PlayerTimedStatsRepository<BuildCount>
        + PlayerTimedStatsRepository<PlayTicks>
        + PlayerTimedStatsRepository<VoteCount>,
> {
    use infra_db_repository_impl::{config::Database, DatabaseConnector};
    DatabaseConnector::try_new(Database::from_env()?).await
}

#[tracing::instrument(skip(stats_repository, timed_stats_repository))]
async fn fetch_and_record<Stats>(
    stats_repository: &(impl PlayerStatsRepository<Stats> + Sync),
    timed_stats_repository: &(impl PlayerTimedStatsRepository<Stats> + Sync),
) -> anyhow::Result<()>
where
    Stats: Send + 'static,
{
    let snapshot = stats_repository
        .fetch_stats_snapshot_of_all_players()
        .await?;

    timed_stats_repository.record_snapshot(snapshot).await?;
    Ok(())
}

#[tracing::instrument]
async fn fetch_and_record_all() -> anyhow::Result<()> {
    let stats_repository = stats_repository_impl().await?;
    let timed_stats_repository = timed_stats_repository_impl().await?;

    fetch_and_record::<BreakCount>(&stats_repository, &timed_stats_repository).await?;
    fetch_and_record::<BuildCount>(&stats_repository, &timed_stats_repository).await?;
    fetch_and_record::<PlayTicks>(&stats_repository, &timed_stats_repository).await?;
    fetch_and_record::<VoteCount>(&stats_repository, &timed_stats_repository).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // initialize tracing
    // see https://github.com/tokio-rs/axum/blob/79a0a54bc9f0f585c974b5e6793541baff980662/examples/tracing-aka-logging/src/main.rs
    // 旧 Sentry の撤去 (GiganticMinecraft/seichi_infra#5613) に伴い SDK を除去した。
    // エラーの検知は stdout ログと Kubernetes 側の Job 失敗アラートに委ねる
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer().with_filter(tracing_subscriber::EnvFilter::new(
                std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
            )),
        )
        .init();

    // 継続プロファイリング (Grafana Pyroscope への push)。
    // PYROSCOPE_SERVER_ADDRESS 未設定 (ローカル実行など) の場合は何もしない。
    // プロファイル取得の失敗で ingest 本体を止めない
    let pyroscope_agent = std::env::var("PYROSCOPE_SERVER_ADDRESS")
        .ok()
        .and_then(|server_address| {
            let started = PyroscopeAgentBuilder::new(
                &server_address,
                "seichi-timed-stats-conifers-ingestor",
                100,
                "pyroscope-rs",
                env!("CARGO_PKG_VERSION"),
                pprof_backend(PprofConfig::default(), BackendConfig::default()),
            )
            .build()
            .and_then(pyroscope::PyroscopeAgent::start);

            match started {
                Ok(agent) => Some(agent),
                Err(error) => {
                    tracing::warn!(%error, "Pyroscope agent の起動に失敗したため、プロファイルなしで続行します");
                    None
                }
            }
        });

    let result = fetch_and_record_all().await;

    // 短命な CronJob のため、プロセス終了前にプロファイルを flush する
    // (ingest が失敗した場合も flush してから終了する)
    if let Some(agent) = pyroscope_agent {
        match agent.stop() {
            Ok(agent) => agent.shutdown(),
            Err(error) => tracing::warn!(%error, "Pyroscope agent の停止に失敗しました"),
        }
    }

    result?;

    Ok(())
}
