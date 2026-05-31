#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr = std::env::var("CMS_ADDR").unwrap_or_else(|_| "0.0.0.0:9100".into());
    let interval_secs: u64 = std::env::var("CMS_INTERVAL")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(15);
    check_my_server::serve(&addr, interval_secs).await
}
