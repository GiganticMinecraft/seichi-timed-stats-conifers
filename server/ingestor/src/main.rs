#![deny(clippy::all, clippy::cargo)]
#![warn(clippy::nursery, clippy::pedantic)]
#![allow(clippy::cargo_common_metadata, clippy::multiple_crate_versions)]

use std::time::Duration;

use tokio::signal::unix::{signal, Signal, SignalKind};

use pyroscope::backend::{pprof_backend, BackendConfig, PprofConfig};
use pyroscope::pyroscope::{PyroscopeAgentBuilder, PyroscopeAgentRunning};
use pyroscope::PyroscopeAgent;
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

/// SIGTERM (Kubernetes による Pod 停止) と SIGINT (Ctrl-C) のハンドラ。
/// ingest を始める前に両方を登録し、それ以降に届いたシグナルを取りこぼさない
struct ShutdownSignals {
    sigterm: Signal,
    sigint: Signal,
}

impl ShutdownSignals {
    fn register() -> std::io::Result<Self> {
        Ok(Self {
            sigterm: signal(SignalKind::terminate())?,
            sigint: signal(SignalKind::interrupt())?,
        })
    }

    /// どちらかのシグナルを待つ。戻り値は慣例に従った終了コード (128 + シグナル番号)
    async fn recv(&mut self) -> (&'static str, i32) {
        tokio::select! {
            _ = self.sigterm.recv() => ("SIGTERM", 128 + 15),
            _ = self.sigint.recv() => ("SIGINT", 128 + 2),
        }
    }
}

/// Pyroscope の flush (停止処理) を待つ上限。
/// agent の停止は内部タイマーの周期 (10 秒) の区切りまで待つため数秒かかる。
/// 依存クレート内部のエラー経路で無期限に待つことがあるため上限を設け、
/// SIGTERM 後の猶予 (terminationGracePeriodSeconds 既定 30 秒) 内に終了できるようにする
const PYROSCOPE_FLUSH_TIMEOUT: Duration = Duration::from_secs(20);

/// 短命な `CronJob` のため、プロセス終了前にプロファイルを flush する。
/// 停止処理はブロッキングなので専用スレッドで動かし、上限を超えたら待たずに戻る
/// (`spawn_blocking` だとランタイムの終了時にそのスレッドの完了を待ってしまうため使わない)
async fn flush_pyroscope(agent: PyroscopeAgent<PyroscopeAgentRunning>) {
    let (done_tx, done_rx) = tokio::sync::oneshot::channel();
    std::thread::spawn(move || {
        match agent.stop() {
            Ok(agent) => agent.shutdown(),
            Err(error) => tracing::warn!(%error, "Pyroscope agent の停止に失敗しました"),
        }
        let _ = done_tx.send(());
    });
    if tokio::time::timeout(PYROSCOPE_FLUSH_TIMEOUT, done_rx)
        .await
        .is_err()
    {
        tracing::warn!("Pyroscope agent の停止が時間内に終わらなかったため、待たずに終了します");
    }
}

/// ingest を最後まで実行したか、シグナルで中断したか
enum Outcome {
    Completed(anyhow::Result<()>),
    Aborted { exit_code: i32 },
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

    // concurrencyPolicy: Replace による置き換えや activeDeadlineSeconds 超過、
    // ノードの drain などで Pod が止められるときに SIGTERM を受ける。
    // ハンドラの登録は ingest を始める前に済ませる
    let mut shutdown_signals = ShutdownSignals::register()?;

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
            .and_then(PyroscopeAgent::start);

            match started {
                Ok(agent) => Some(agent),
                Err(error) => {
                    tracing::warn!(%error, "Pyroscope agent の起動に失敗したため、プロファイルなしで続行します");
                    None
                }
            }
        });

    // シグナルを受けたら進行中の ingest を drop して中断する。統計の種類ごとに
    // 別トランザクションで記録するため、中断時点で記録済みの種類はそのまま残り、
    // 実行中のトランザクションだけがロールバックされる。
    // biased: ingest の完了とシグナルが同時に準備できた場合は完了を優先し、
    // 記録し終えた回を「中断」として再試行させない
    let outcome = tokio::select! {
        biased;
        result = fetch_and_record_all() => Outcome::Completed(result),
        (signal, exit_code) = shutdown_signals.recv() => {
            tracing::warn!(signal, "shutdown signal received, aborting ingestion");
            Outcome::Aborted { exit_code }
        }
    };

    // ingest が失敗・中断した場合も flush してから終了する
    if let Some(agent) = pyroscope_agent {
        flush_pyroscope(agent).await;
    }

    match outcome {
        Outcome::Completed(result) => result?,
        // 中断時は非 0 で終了する。0 で終了すると、ノードの drain などで Pod だけが
        // 止められた場合に Job が「成功」として完了し、取り込めなかった回が
        // 再試行されず、成功の途絶を見る ConifersIngestorStalled も検知できなくなるため
        Outcome::Aborted { exit_code } => std::process::exit(exit_code),
    }

    Ok(())
}
