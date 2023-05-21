#![deny(clippy::all, clippy::cargo)]
#![warn(clippy::nursery, clippy::pedantic)]
#![allow(clippy::cargo_common_metadata)]

use config::{AppConfig, FromEnv};
use tokio::signal::unix::{signal, SignalKind};
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // initialize tracing
    // see https://github.com/tokio-rs/axum/blob/79a0a54bc9f0f585c974b5e6793541baff980662/examples/tracing-aka-logging/src/main.rs
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    println!("Reading config...");
    let _config = AppConfig::from_env()?;

    // begin piping process in background
    let scheduler = scheduled_snapshots_piping_process().await;
    scheduler.start().await.unwrap();

    // wait for SIGTERM
    let mut signal_stream = signal(SignalKind::terminate())?;
    signal_stream.recv().await;

    Ok(())
}

async fn scheduled_snapshots_piping_process() -> JobScheduler {
    let scheduler = JobScheduler::new().await.unwrap();

    scheduler
        .add(
            Job::new_async("0 5 * * * *", |_uuid, _l| {
                Box::pin(async {
                    // TODO: run piping process here
                })
            })
            .unwrap(),
        )
        .await
        .unwrap();

    scheduler
}
