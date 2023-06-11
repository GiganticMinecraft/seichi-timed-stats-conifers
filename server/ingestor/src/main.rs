#![deny(clippy::all, clippy::cargo)]
#![warn(clippy::nursery, clippy::pedantic)]
#![allow(clippy::cargo_common_metadata)]

use domain::models::{BreakCount, BuildCount, PlayTicks, VoteCount};
use domain::repositories::{PlayerStatsRepository, PlayerTimedStatsRepository};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

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

use crate::config::SENTRY_CONFIG;

mod config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // setup sentry
    // only send sentry events when it's not running locally
    if SENTRY_CONFIG.environment_name != "local" {
        let _guard = sentry::init((
            "https://20ce98e4b5304846be70f3bd78a6a588@sentry.onp.admin.seichi.click/9",
            sentry::ClientOptions {
                release: sentry::release_name!(),
                traces_sample_rate: 0.1,
                environment: Some(SENTRY_CONFIG.environment_name.to_owned().into()),
                ..Default::default()
            },
        ));

        sentry::configure_scope(|scope| scope.set_level(Some(sentry::Level::Warning)));
    }

    // initialize tracing
    // see https://github.com/tokio-rs/axum/blob/79a0a54bc9f0f585c974b5e6793541baff980662/examples/tracing-aka-logging/src/main.rs
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let stats_repository = stats_repository_impl().await?;
    let timed_stats_repository = timed_stats_repository_impl().await?;

    fetch_and_record::<BreakCount>(&stats_repository, &timed_stats_repository).await?;
    fetch_and_record::<BuildCount>(&stats_repository, &timed_stats_repository).await?;
    fetch_and_record::<PlayTicks>(&stats_repository, &timed_stats_repository).await?;
    fetch_and_record::<VoteCount>(&stats_repository, &timed_stats_repository).await?;

    Ok(())
}
