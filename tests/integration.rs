/// Smoke tests — verify each diagnostic can collect from the live system.
/// These require Linux /proc and /sys to be present.

#[test]
fn collect_metrics_returns_output() {
    let out = check_my_server::collect_metrics();
    assert!(!out.is_empty(), "metrics output should not be empty");
    assert!(out.contains("node_uptime_seconds"), "expected uptime metric");
    assert!(out.contains("node_memory_total_bytes"), "expected memory metric");
}
