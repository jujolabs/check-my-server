mod diagnostics;
mod metrics;

use anyhow::Result;
use axum::{Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};
use std::sync::{Arc, RwLock};
use tokio::{signal, time::{Duration, interval}};

enum CheckResult<T> {
    Success(T),
    Failure { error: String },
}

impl<T> From<Result<T>> for CheckResult<T> {
    fn from(r: Result<T>) -> Self {
        match r {
            Ok(v) => CheckResult::Success(v),
            Err(e) => CheckResult::Failure { error: format!("{:#}", e) },
        }
    }
}

struct Report {
    host:      CheckResult<diagnostics::host::HostReport>,
    system:    CheckResult<diagnostics::system::SystemReport>,
    memory:    CheckResult<diagnostics::memory::MemoryReport>,
    cpu:       CheckResult<diagnostics::cpu::CpuReport>,
    disk:      CheckResult<diagnostics::disk::DiskReport>,
    diskstats: CheckResult<diagnostics::diskstats::DiskStatsReport>,
    network:   CheckResult<diagnostics::network::NetworkReport>,
    pressure:  CheckResult<diagnostics::pressure::PressureReport>,
}

fn join<T: Send + 'static>(handle: std::thread::JoinHandle<Result<T>>) -> CheckResult<T> {
    handle
        .join()
        .unwrap_or_else(|_| Err(anyhow::anyhow!("thread panicked")))
        .into()
}

fn collect_report() -> Report {
    let host      = std::thread::spawn(diagnostics::host::collect);
    let system    = std::thread::spawn(diagnostics::system::collect);
    let memory    = std::thread::spawn(diagnostics::memory::collect);
    let cpu       = std::thread::spawn(diagnostics::cpu::collect);
    let disk      = std::thread::spawn(diagnostics::disk::collect);
    let diskstats = std::thread::spawn(diagnostics::diskstats::collect);
    let network   = std::thread::spawn(diagnostics::network::collect);
    let pressure  = std::thread::spawn(diagnostics::pressure::collect);

    Report {
        host:      join(host),
        system:    join(system),
        memory:    join(memory),
        cpu:       join(cpu),
        disk:      join(disk),
        diskstats: join(diskstats),
        network:   join(network),
        pressure:  join(pressure),
    }
}

pub fn collect_metrics() -> String {
    metrics::format(collect_report())
}

type Cache = Arc<RwLock<String>>;

async fn poll_loop(cache: Cache, interval_secs: u64) {
    let mut ticker = interval(Duration::from_secs(interval_secs));
    ticker.tick().await; // first tick immediate — skip, startup already collected
    loop {
        ticker.tick().await;
        match tokio::task::spawn_blocking(|| metrics::format(collect_report())).await {
            Ok(body) => *cache.write().unwrap_or_else(|e| e.into_inner()) = body,
            Err(e) => tracing::warn!("poll collection failed: {e}"),
        }
    }
}

async fn metrics_handler(State(cache): State<Cache>) -> impl IntoResponse {
    let body = cache.read().unwrap_or_else(|e| e.into_inner()).clone();
    (
        StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
        body,
    )
}

async fn health_handler() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("ctrl-c handler");
    };
    #[cfg(unix)]
    let sigterm = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("sigterm handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let sigterm = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = sigterm => {} }
}

pub async fn serve(addr: &str, interval_secs: u64) -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let initial = tokio::task::spawn_blocking(|| metrics::format(collect_report())).await?;
    let cache: Cache = Arc::new(RwLock::new(initial));

    tokio::spawn(poll_loop(cache.clone(), interval_secs));

    let app = Router::new()
        .route("/metrics", get(metrics_handler))
        .route("/health", get(health_handler))
        .with_state(cache);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("listening — metrics http://{addr}/metrics  health http://{addr}/health  interval {interval_secs}s");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}
