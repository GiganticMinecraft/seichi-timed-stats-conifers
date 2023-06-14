#![deny(clippy::all, clippy::cargo)]
#![warn(clippy::nursery, clippy::pedantic)]
#![allow(clippy::cargo_common_metadata)]

use std::time::{Duration, SystemTime};

use pprof::ProfilerGuardBuilder;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

use domain::models::{BreakCount, BuildCount, PlayTicks, VoteCount};
use domain::repositories::{PlayerStatsRepository, PlayerTimedStatsRepository};

use crate::config::SENTRY_CONFIG;

mod config;

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
    tracing_subscriber::registry()
        .with(sentry::integrations::tracing::layer())
        .with(
            tracing_subscriber::fmt::layer().with_filter(tracing_subscriber::EnvFilter::new(
                std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
            )),
        )
        .init();

    // setup sentry
    // only send sentry events when we are not running locally
    let _sentry_client_guard = if SENTRY_CONFIG.environment_name != "local" {
        Some(sentry::init((
            "https://20ce98e4b5304846be70f3bd78a6a588:2cfe5fb8288c4635bb84630b41d21bf2@sentry.onp.admin.seichi.click/9",
            sentry::ClientOptions {
                release: sentry::release_name!(),
                traces_sample_rate: 1.0,
                enable_profiling: true,
                profiles_sample_rate: 1.0,
                environment: Some(SENTRY_CONFIG.environment_name.clone().into()),
                shutdown_timeout: Duration::from_secs(10),
                ..Default::default()
            },
        )))
    } else {
        None
    };

    // hack: spin up profiler, or else the profiler takes around 2 seconds to start
    //       at the beginning of a profiled span
    drop(ProfilerGuardBuilder::default().build());

    fetch_and_record_all().await?;

    Ok(())
}
