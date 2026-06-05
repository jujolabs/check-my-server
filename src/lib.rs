mod diagnostics;
mod metrics;

use anyhow::Result;
use axum::{Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};
use std::sync::{Arc, RwLock};
use tokio::{
    signal,
    time::{Duration, interval},
};

#[derive(serde::Serialize)]
#[serde(untagged)]
enum StatusEntry<T: serde::Serialize> {
    Ok(T),
    Err { error: String },
}

#[derive(serde::Serialize)]
struct StatusReport {
    version: &'static str,
    host: StatusEntry<diagnostics::host::HostReport>,
    system: StatusEntry<diagnostics::system::SystemReport>,
    memory: StatusEntry<diagnostics::memory::MemoryReport>,
    cpu: StatusEntry<diagnostics::cpu::CpuReport>,
    disk: StatusEntry<diagnostics::disk::DiskReport>,
    diskstats: StatusEntry<diagnostics::diskstats::DiskStatsReport>,
    network: StatusEntry<diagnostics::network::NetworkReport>,
    pressure: StatusEntry<diagnostics::pressure::PressureReport>,
}

enum CheckResult<T> {
    Success(T),
    Failure { error: String },
}

impl<T> From<Result<T>> for CheckResult<T> {
    fn from(r: Result<T>) -> Self {
        match r {
            Ok(v) => CheckResult::Success(v),
            Err(e) => CheckResult::Failure {
                error: format!("{:#}", e),
            },
        }
    }
}

impl<T: serde::Serialize> From<CheckResult<T>> for StatusEntry<T> {
    fn from(r: CheckResult<T>) -> Self {
        match r {
            CheckResult::Success(v) => StatusEntry::Ok(v),
            CheckResult::Failure { error } => StatusEntry::Err { error },
        }
    }
}

struct Report {
    host: CheckResult<diagnostics::host::HostReport>,
    system: CheckResult<diagnostics::system::SystemReport>,
    memory: CheckResult<diagnostics::memory::MemoryReport>,
    cpu: CheckResult<diagnostics::cpu::CpuReport>,
    disk: CheckResult<diagnostics::disk::DiskReport>,
    diskstats: CheckResult<diagnostics::diskstats::DiskStatsReport>,
    network: CheckResult<diagnostics::network::NetworkReport>,
    pressure: CheckResult<diagnostics::pressure::PressureReport>,
}

fn join<T: Send + 'static>(handle: std::thread::JoinHandle<Result<T>>) -> CheckResult<T> {
    handle
        .join()
        .unwrap_or_else(|_| Err(anyhow::anyhow!("thread panicked")))
        .into()
}

fn collect_report() -> Report {
    let host = std::thread::spawn(diagnostics::host::collect);
    let system = std::thread::spawn(diagnostics::system::collect);
    let memory = std::thread::spawn(diagnostics::memory::collect);
    let cpu = std::thread::spawn(diagnostics::cpu::collect);
    let disk = std::thread::spawn(diagnostics::disk::collect);
    let diskstats = std::thread::spawn(diagnostics::diskstats::collect);
    let network = std::thread::spawn(diagnostics::network::collect);
    let pressure = std::thread::spawn(diagnostics::pressure::collect);

    Report {
        host: join(host),
        system: join(system),
        memory: join(memory),
        cpu: join(cpu),
        disk: join(disk),
        diskstats: join(diskstats),
        network: join(network),
        pressure: join(pressure),
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

async fn version_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        concat!("check-my-server ", env!("CARGO_PKG_VERSION"), "\n"),
    )
}

async fn status_handler() -> impl IntoResponse {
    let report = match tokio::task::spawn_blocking(collect_report).await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [("content-type", "text/plain")],
                format!("collection panicked: {e}"),
            )
                .into_response();
        }
    };
    let status = StatusReport {
        version: env!("CARGO_PKG_VERSION"),
        host: report.host.into(),
        system: report.system.into(),
        memory: report.memory.into(),
        cpu: report.cpu.into(),
        disk: report.disk.into(),
        diskstats: report.diskstats.into(),
        network: report.network.into(),
        pressure: report.pressure.into(),
    };
    match serde_json::to_string_pretty(&status) {
        Ok(json) => (StatusCode::OK, [("content-type", "application/json")], json).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [("content-type", "text/plain")],
            e.to_string(),
        )
            .into_response(),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_metrics_endpoint() {
        let cache: Cache = Arc::new(RwLock::new("test_metric 42\n".to_string()));
        let app = Router::new()
            .route("/metrics", get(metrics_handler))
            .with_state(cache);
        let req = axum::http::Request::builder()
            .uri("/metrics")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let ct = resp
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(ct.starts_with("text/plain"), "content-type: {ct}");
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], b"test_metric 42\n");
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let app = Router::new().route("/health", get(health_handler));
        let req = axum::http::Request::builder()
            .uri("/health")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_version_endpoint() {
        let app = Router::new().route("/version", get(version_handler));
        let req = axum::http::Request::builder()
            .uri("/version")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let text = std::str::from_utf8(&body).unwrap();
        assert!(text.starts_with("check-my-server "), "got: {text:?}");
        assert!(text.contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn test_format_per_core_metrics() {
        use diagnostics::cpu::{CpuCoreReport, CpuReport};

        let cpu_report = CpuReport {
            usage_percent: 30.0,
            user_percent: 20.0,
            system_percent: 10.0,
            iowait_percent: 0.0,
            idle_percent: 70.0,
            cores: vec![
                CpuCoreReport {
                    core: 0,
                    usage_percent: 25.0,
                    idle_percent: 75.0,
                },
                CpuCoreReport {
                    core: 1,
                    usage_percent: 35.0,
                    idle_percent: 65.0,
                },
            ],
        };
        let report = Report {
            host: CheckResult::Failure {
                error: "skip".into(),
            },
            system: CheckResult::Failure {
                error: "skip".into(),
            },
            memory: CheckResult::Failure {
                error: "skip".into(),
            },
            cpu: CheckResult::Success(cpu_report),
            disk: CheckResult::Failure {
                error: "skip".into(),
            },
            diskstats: CheckResult::Failure {
                error: "skip".into(),
            },
            network: CheckResult::Failure {
                error: "skip".into(),
            },
            pressure: CheckResult::Failure {
                error: "skip".into(),
            },
        };
        let out = metrics::format(report);
        assert!(
            out.contains(r#"node_cpu_core_usage_percent{cpu="0"} 25"#),
            "got: {out}"
        );
        assert!(
            out.contains(r#"node_cpu_core_usage_percent{cpu="1"} 35"#),
            "got: {out}"
        );
        assert!(
            out.contains(r#"node_cpu_core_idle_percent{cpu="0"} 75"#),
            "got: {out}"
        );
        assert!(
            out.contains(r#"node_cpu_core_idle_percent{cpu="1"} 65"#),
            "got: {out}"
        );
    }
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
        .route("/version", get(version_handler))
        .route("/status", get(status_handler))
        .with_state(cache);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(
        "listening — metrics http://{addr}/metrics  health http://{addr}/health  version http://{addr}/version  status http://{addr}/status  interval {interval_secs}s"
    );
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}
