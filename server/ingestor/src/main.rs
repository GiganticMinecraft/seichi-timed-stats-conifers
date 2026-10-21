#![deny(clippy::all, clippy::cargo)]
#![warn(clippy::nursery, clippy::pedantic)]
#![allow(clippy::cargo_common_metadata, clippy::multiple_crate_versions)]

use std::time::{Duration, Instant};

use tokio::signal::unix::{signal, Signal, SignalKind};

use init_tracing_opentelemetry::otlp::OtelGuard;
use init_tracing_opentelemetry::resource::DetectResource;
use init_tracing_opentelemetry::{FmtTraceContext, TracingConfig};
use opentelemetry::metrics::{Counter, Gauge, Histogram};
use opentelemetry::KeyValue;
use pyroscope::backend::{pprof_backend, BackendConfig, PprofConfig};
use pyroscope::pyroscope::{PyroscopeAgentBuilder, PyroscopeAgentRunning};
use pyroscope::PyroscopeAgent;
use tracing::field::Empty;
use tracing::Instrument;

use domain::models::{BreakCount, BuildCount, PlayTicks, VoteCount};
use domain::repositories::{PlayerStatsRepository, PlayerTimedStatsRepository};

/// OpenTelemetry の `service.name` と Pyroscope のアプリケーション名の既定値
const SERVICE_NAME: &str = "seichi-timed-stats-conifers-ingestor";

/// 統計の種類。span の属性とメトリクスの `stat_kind` に使う
trait StatKind {
    const NAME: &'static str;
}

impl StatKind for BreakCount {
    const NAME: &'static str = "break_count";
}

impl StatKind for BuildCount {
    const NAME: &'static str = "build_count";
}

impl StatKind for PlayTicks {
    const NAME: &'static str = "play_ticks";
}

impl StatKind for VoteCount {
    const NAME: &'static str = "vote_count";
}

/// 統計の種類を問わない、実行全体についての値に付ける `stat_kind`
const STAT_KIND_ALL: &str = "all";

/// ingest のメトリクス。OpenTelemetry が無効なときは no-op の instrument になる。
///
/// `CronJob` の 1 回の実行ごとにプロセスが終わるため、cumulative の counter は
/// 毎回 0 から始まる。Prometheus 側で `rate` / `increase` を使うと実行をまたいだ
/// 増加を取りこぼすので、`max_over_time` / `count_over_time` などで見ること
struct IngestMetrics {
    /// 実行全体 (`stat_kind="all"`) と統計の種類ごとの所要時間 (`outcome`: success / failure / aborted)。
    /// 両方が同じ instrument に入るので、`stat_kind` をまたいで合計しないこと
    duration: Histogram<f64>,
    /// 上流から取得したプレーヤー数 (統計の種類ごと)
    players: Gauge<u64>,
    /// 失敗の回数。失敗した実行 1 回につき 1 だけ数える (失敗した時点で ingest を止めるため)。
    /// fetch / record の失敗はその統計の種類、setup (接続の準備) と aborted (シグナルで中断) は
    /// `stat_kind="all"` に付く。失敗した実行の数は `stat_kind` を問わず合計すればよい
    failures: Counter<u64>,
}

impl IngestMetrics {
    /// global の meter provider から作るため、telemetry の初期化後に呼ぶこと
    fn new() -> Self {
        let meter = opentelemetry::global::meter(SERVICE_NAME);
        Self {
            duration: meter
                .f64_histogram("conifers.ingest.duration")
                .with_unit("s")
                .with_description("ingest にかかった時間")
                // 既定の境界 (0〜10000) は秒単位の短いバッチには粗すぎるため、
                // 通常 (数秒〜十数秒) から Job の実行期限 (240 秒) までを刻む
                .with_boundaries(vec![
                    0.5, 1.0, 2.0, 5.0, 10.0, 15.0, 20.0, 30.0, 60.0, 120.0, 240.0,
                ])
                .build(),
            players: meter
                .u64_gauge("conifers.ingest.players")
                .with_unit("{player}")
                .with_description("上流から取得したプレーヤー数")
                .build(),
            failures: meter
                .u64_counter("conifers.ingest.failures")
                .with_description("ingest の失敗回数")
                .build(),
        }
    }

    fn record_duration(&self, stat_kind: &'static str, outcome: &'static str, elapsed: Duration) {
        self.duration.record(
            elapsed.as_secs_f64(),
            &[
                KeyValue::new("stat_kind", stat_kind),
                KeyValue::new("outcome", outcome),
            ],
        );
    }

    fn record_failure(&self, stat_kind: &'static str, reason: &'static str) {
        self.failures.add(
            1,
            &[
                KeyValue::new("stat_kind", stat_kind),
                KeyValue::new("reason", reason),
            ],
        );
    }
}

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

/// 統計 1 種類分の処理の結果 (outcome と所要時間、失敗の理由) を span とメトリクスに記録する。
/// 結果を記録しないまま drop された (シグナルで ingest ごと中断された) ときは aborted として記録する。
/// 中断の failures は実行全体 (`stat_kind="all"`) の側で 1 回だけ数える
struct StatRun<'a> {
    stat_kind: &'static str,
    span: tracing::Span,
    metrics: &'a IngestMetrics,
    started: Instant,
    finished: bool,
}

impl<'a> StatRun<'a> {
    fn start(stat_kind: &'static str, metrics: &'a IngestMetrics) -> Self {
        Self {
            stat_kind,
            span: tracing::Span::current(),
            metrics,
            started: Instant::now(),
            finished: false,
        }
    }

    fn finish(&mut self, outcome: &'static str) {
        if std::mem::replace(&mut self.finished, true) {
            return;
        }
        self.span.record("outcome", outcome);
        if outcome != "success" {
            self.span.record("otel.status_code", "error");
        }
        self.metrics
            .record_duration(self.stat_kind, outcome, self.started.elapsed());
    }

    fn fail(&mut self, reason: &'static str) {
        self.metrics.record_failure(self.stat_kind, reason);
        self.finish("failure");
    }
}

impl Drop for StatRun<'_> {
    fn drop(&mut self) {
        self.finish("aborted");
    }
}

/// 統計 1 種類分を上流から取得して記録する。
/// span には取得件数・記録件数・記録形式 (full / diff) を属性として付ける
#[tracing::instrument(
    skip_all,
    fields(
        stat_kind = Stats::NAME,
        fetched_players = Empty,
        recorded_entries = Empty,
        snapshot_kind = Empty,
        outcome = Empty,
        otel.status_code = Empty,
    )
)]
async fn fetch_and_record<Stats>(
    stats_repository: &(impl PlayerStatsRepository<Stats> + Sync),
    timed_stats_repository: &(impl PlayerTimedStatsRepository<Stats> + Sync),
    metrics: &IngestMetrics,
) -> anyhow::Result<()>
where
    Stats: StatKind + Send + 'static,
{
    let mut run = StatRun::start(Stats::NAME, metrics);

    let snapshot = stats_repository
        .fetch_stats_snapshot_of_all_players()
        .await
        .inspect_err(|_| run.fail("fetch"))?;
    let fetched_players = snapshot.len();
    run.span.record("fetched_players", fetched_players);
    metrics.players.record(
        u64::try_from(fetched_players).unwrap_or(u64::MAX),
        &[KeyValue::new("stat_kind", Stats::NAME)],
    );

    let recorded = timed_stats_repository
        .record_snapshot(snapshot)
        .await
        .inspect_err(|_| run.fail("record"))?;
    run.span
        .record("recorded_entries", recorded.recorded_entries);
    run.span.record("snapshot_kind", recorded.kind.as_str());
    run.finish("success");

    tracing::info!(
        fetched_players,
        recorded_entries = recorded.recorded_entries,
        snapshot_kind = recorded.kind.as_str(),
        "recorded snapshot"
    );
    Ok(())
}

async fn fetch_and_record_all(metrics: &IngestMetrics) -> anyhow::Result<()> {
    let repositories = async {
        anyhow::Ok((
            stats_repository_impl().await?,
            timed_stats_repository_impl().await?,
        ))
    }
    .await;
    let (stats_repository, timed_stats_repository) =
        repositories.inspect_err(|_| metrics.record_failure(STAT_KIND_ALL, "setup"))?;

    fetch_and_record::<BreakCount>(&stats_repository, &timed_stats_repository, metrics).await?;
    fetch_and_record::<BuildCount>(&stats_repository, &timed_stats_repository, metrics).await?;
    fetch_and_record::<PlayTicks>(&stats_repository, &timed_stats_repository, metrics).await?;
    fetch_and_record::<VoteCount>(&stats_repository, &timed_stats_repository, metrics).await?;

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

/// OpenTelemetry (traces / metrics) の flush を待つ上限。OTLP の送信は 1 回あたり既定 10 秒で
/// 打ち切られ、`OtelGuard` の drop は traces → metrics の順に送る。送信先に届かないときでも
/// traces の 1 回分 (10 秒) を待った後に metrics を送り切れるよう 25 秒取る。
/// Pyroscope の flush (上限 20 秒) と並行して待つので、SIGTERM 後の猶予 (30 秒) に収まる
const OTEL_FLUSH_TIMEOUT: Duration = Duration::from_secs(25);

/// ブロッキングな停止処理を専用スレッドで動かし、上限を超えたら待たずに戻る
/// (`spawn_blocking` だとランタイムの終了時にそのスレッドの完了を待ってしまうため使わない)
async fn run_blocking_with_timeout(
    what: &'static str,
    timeout: Duration,
    f: impl FnOnce() + Send + 'static,
) {
    let (done_tx, done_rx) = tokio::sync::oneshot::channel();
    std::thread::spawn(move || {
        f();
        let _ = done_tx.send(());
    });
    if tokio::time::timeout(timeout, done_rx).await.is_err() {
        tracing::warn!(
            what,
            "停止処理が時間内に終わらなかったため、待たずに終了します"
        );
    }
}

/// 短命な `CronJob` のため、プロセス終了前にテレメトリを flush する。
///
/// - OpenTelemetry: metrics は `PeriodicReader` の周期 (既定 60 秒) より実行時間が短いので、
///   ここで flush しないと 1 度も送られない。span も batch で送るため同様
/// - Pyroscope: agent を止めると手元のプロファイルが送られる
///
/// `std::process::exit` は drop を走らせないので、終了コードを返す前に必ずここを通すこと
async fn flush_telemetry(
    otel_guard: Option<OtelGuard>,
    pyroscope_agent: Option<PyroscopeAgent<PyroscopeAgentRunning>>,
) {
    let otel = async {
        if let Some(otel_guard) = otel_guard {
            // OtelGuard の drop が traces / metrics の force_flush と shutdown を行う
            run_blocking_with_timeout("OpenTelemetry", OTEL_FLUSH_TIMEOUT, move || {
                drop(otel_guard);
            })
            .await;
        }
    };
    let pyroscope = async {
        if let Some(agent) = pyroscope_agent {
            run_blocking_with_timeout("Pyroscope", PYROSCOPE_FLUSH_TIMEOUT, move || {
                match agent.stop() {
                    Ok(agent) => agent.shutdown(),
                    Err(error) => tracing::warn!(%error, "Pyroscope agent の停止に失敗しました"),
                }
            })
            .await;
        }
    };
    tokio::join!(otel, pyroscope);
}

/// ingest を最後まで実行したか、シグナルで中断したか
enum Outcome {
    Completed(anyhow::Result<()>),
    Aborted {
        signal: &'static str,
        exit_code: i32,
    },
}

/// stdout への JSON ログと、OpenTelemetry (traces + metrics) を初期化する。
///
/// OpenTelemetry の設定は `OTEL_*` 環境変数 (`OTEL_SERVICE_NAME`, `OTEL_EXPORTER_OTLP_ENDPOINT`,
/// `OTEL_EXPORTER_OTLP_PROTOCOL`, `OTEL_TRACES_SAMPLER`, `OTEL_RESOURCE_ATTRIBUTES` など) から読む。
/// 送り先が無い (ローカル実行など) ときは OpenTelemetry を無効にし、
/// 送り先の無い exporter の警告を出さずに stdout ログだけにする ([`otel_endpoint_configured`])。
/// `service.version` は `SERVICE_VERSION` (イメージのビルド時に git の commit SHA を入れる) から読む
fn init_telemetry(
    otel_enabled: bool,
) -> Result<init_tracing_opentelemetry::Guard, init_tracing_opentelemetry::Error> {
    // ログレベルは RUST_LOG (無ければ info)。
    // ログは stdout の JSON を Alloy が拾って Loki に送る経路を正とし、OTLP logs は使わない。
    // trace_id / span_id を各行に付け、Loki から Tempo の trace へ辿れるようにする
    TracingConfig::default()
        .with_json_format()
        .with_stdout()
        .with_fmt_trace_context(FmtTraceContext::TraceIdSpanId)
        .with_otel(otel_enabled)
        .with_metrics(otel_enabled)
        .with_resource_config(
            DetectResource::default()
                .with_fallback_service_name(SERVICE_NAME)
                .with_fallback_service_version(env!("CARGO_PKG_VERSION")),
        )
        .init_subscriber()
}

/// Pyroscope に付けるバージョン。OpenTelemetry の `service.version` と揃える
/// (どちらもイメージのビルド時に入れる `SERVICE_VERSION` を優先する)
fn service_version() -> String {
    std::env::var("SERVICE_VERSION")
        .ok()
        .filter(|version| !version.is_empty())
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_owned())
}

/// OTLP の送り先が環境変数で指定されているか。signal ごとの指定 (`OTEL_EXPORTER_OTLP_TRACES_ENDPOINT` など)
/// だけでも有効にする
fn otel_endpoint_configured() -> bool {
    [
        "OTEL_EXPORTER_OTLP_ENDPOINT",
        "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT",
        "OTEL_EXPORTER_OTLP_METRICS_ENDPOINT",
    ]
    .iter()
    .any(|key| std::env::var_os(key).is_some_and(|endpoint| !endpoint.is_empty()))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 旧 Sentry の撤去 (GiganticMinecraft/seichi_infra#5613) に伴い SDK を除去した。
    // エラーの検知は stdout ログと Kubernetes 側の Job 失敗アラートに委ねる
    // OpenTelemetry の初期化に失敗しても ingest 本体は止めず、stdout ログだけで続ける
    // (exporter の構築は subscriber の登録より前に失敗するので、やり直せる)
    let mut telemetry_guard = match init_telemetry(otel_endpoint_configured()) {
        Ok(guard) => guard,
        Err(otel_error) => {
            let guard = init_telemetry(false)?;
            tracing::warn!(error = %otel_error, "OpenTelemetry の初期化に失敗したため、無効にして続行します");
            guard
        }
    };
    let metrics = IngestMetrics::new();

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
                SERVICE_NAME,
                100,
                "pyroscope-rs",
                service_version(),
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

    // 1 回の実行全体を表す root span。統計の種類ごとの span と上流 gRPC の client span は
    // この子になり、上流 (seichi-game-data-server) へは traceparent で伝播する
    let ingest_span = tracing::info_span!("ingest", outcome = Empty, otel.status_code = Empty);
    let started = Instant::now();

    // シグナルを受けたら進行中の ingest を drop して中断する。統計の種類ごとに
    // 別トランザクションで記録するため、中断時点で記録済みの種類はそのまま残り、
    // 実行中のトランザクションだけがロールバックされる。
    // biased: ingest の完了とシグナルが同時に準備できた場合は完了を優先し、
    // 記録し終えた回を「中断」として再試行させない
    let outcome = tokio::select! {
        biased;
        result = fetch_and_record_all(&metrics).instrument(ingest_span.clone()) => {
            Outcome::Completed(result)
        }
        (signal, exit_code) = shutdown_signals.recv() => Outcome::Aborted { signal, exit_code },
    };

    // root span に結果を記録して閉じる (span は閉じた時点で export 対象になる)
    let elapsed = started.elapsed();
    ingest_span.in_scope(|| match &outcome {
        Outcome::Completed(Ok(())) => {
            ingest_span.record("outcome", "success");
            metrics.record_duration(STAT_KIND_ALL, "success", elapsed);
            tracing::info!(elapsed_secs = elapsed.as_secs_f64(), "ingest completed");
        }
        Outcome::Completed(Err(error)) => {
            // ERROR のイベントで span の status も error になる
            ingest_span.record("outcome", "failure");
            metrics.record_duration(STAT_KIND_ALL, "failure", elapsed);
            tracing::error!(error = format!("{error:#}"), "ingest failed");
        }
        Outcome::Aborted { signal, .. } => {
            ingest_span.record("outcome", "aborted");
            ingest_span.record("otel.status_code", "error");
            metrics.record_duration(STAT_KIND_ALL, "aborted", elapsed);
            metrics.record_failure(STAT_KIND_ALL, "aborted");
            tracing::warn!(signal, "shutdown signal received, aborting ingestion");
        }
    });
    drop(ingest_span);

    // ingest が失敗・中断した場合も flush してから終了する
    flush_telemetry(telemetry_guard.otel_guard.take(), pyroscope_agent).await;

    match outcome {
        Outcome::Completed(Ok(())) => {}
        // 失敗は上で JSON の ERROR ログ (原因の連鎖を含む) に出したので、main から Err を返して
        // JSON でない "Error: ..." を stderr に重ねて出さないよう、終了コード 1 で直接終わる
        Outcome::Completed(Err(_)) => std::process::exit(1),
        // 中断時は非 0 で終了する。0 で終了すると、ノードの drain などで Pod だけが
        // 止められた場合に Job が「成功」として完了し、取り込めなかった回が
        // 再試行されず、成功の途絶を見る ConifersIngestorStalled も検知できなくなるため
        Outcome::Aborted { exit_code, .. } => std::process::exit(exit_code),
    }

    Ok(())
}
