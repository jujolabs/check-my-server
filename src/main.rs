#[tokio::main]
async fn main() -> anyhow::Result<()> {
    check_my_server::serve("0.0.0.0:9100").await
}
