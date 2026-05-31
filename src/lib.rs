mod diagnostics;
mod metrics;

use anyhow::Result;
use axum::{Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};
use std::sync::{Arc, RwLock};
use tokio::time::{Duration, interval};

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
    host: CheckResult<diagnostics::host::HostReport>,
    system: CheckResult<diagnostics::system::SystemReport>,
    memory: CheckResult<diagnostics::memory::MemoryReport>,
    cpu: CheckResult<diagnostics::cpu::CpuReport>,
    disk: CheckResult<diagnostics::disk::DiskReport>,
    network: CheckResult<diagnostics::network::NetworkReport>,
}

fn join<T: Send + 'static>(handle: std::thread::JoinHandle<Result<T>>) -> CheckResult<T> {
    handle
        .join()
        .unwrap_or_else(|_| Err(anyhow::anyhow!("thread panicked")))
        .into()
}

fn collect_report() -> Report {
    let host    = std::thread::spawn(diagnostics::host::collect);
    let system  = std::thread::spawn(diagnostics::system::collect);
    let memory  = std::thread::spawn(diagnostics::memory::collect);
    let cpu     = std::thread::spawn(diagnostics::cpu::collect);
    let disk    = std::thread::spawn(diagnostics::disk::collect);
    let network = std::thread::spawn(diagnostics::network::collect);

    Report {
        host:    join(host),
        system:  join(system),
        memory:  join(memory),
        cpu:     join(cpu),
        disk:    join(disk),
        network: join(network),
    }
}

pub fn collect_metrics() -> String {
    metrics::format(collect_report())
}

type Cache = Arc<RwLock<String>>;

async fn poll_loop(cache: Cache) {
    let mut ticker = interval(Duration::from_secs(15));
    ticker.tick().await; // first tick immediate — skip, startup already collected
    loop {
        ticker.tick().await;
        if let Ok(body) = tokio::task::spawn_blocking(|| metrics::format(collect_report())).await {
            *cache.write().unwrap() = body;
        }
    }
}

async fn metrics_handler(State(cache): State<Cache>) -> impl IntoResponse {
    let body = cache.read().unwrap().clone();
    (
        StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
        body,
    )
}

pub async fn serve(addr: &str) -> Result<()> {
    let initial = tokio::task::spawn_blocking(|| metrics::format(collect_report())).await?;
    let cache: Cache = Arc::new(RwLock::new(initial));

    tokio::spawn(poll_loop(cache.clone()));

    let app = Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(cache);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("Listening on http://{addr}/metrics");
    axum::serve(listener, app).await?;
    Ok(())
}
