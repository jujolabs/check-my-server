use anyhow::{Context, Result};
use serde::Serialize;
use std::fs;

#[derive(Serialize)]
pub struct MemoryReport {
    pub total_kb: u64,
    pub available_kb: u64,
    pub used_kb: u64,
    pub used_percent: f64,
    pub swap_total_kb: u64,
    pub swap_free_kb: u64,
    pub swap_used_kb: u64,
    pub swap_used_percent: f64,
}

pub fn collect() -> Result<MemoryReport> {
    let content = fs::read_to_string("/proc/meminfo").context("read /proc/meminfo")?;

    let mut total_kb = 0u64;
    let mut available_kb = 0u64;
    let mut swap_total_kb = 0u64;
    let mut swap_free_kb = 0u64;

    for line in content.lines() {
        if let Some(val) = parse_kb(line, "MemTotal:") {
            total_kb = val;
        } else if let Some(val) = parse_kb(line, "MemAvailable:") {
            available_kb = val;
        } else if let Some(val) = parse_kb(line, "SwapTotal:") {
            swap_total_kb = val;
        } else if let Some(val) = parse_kb(line, "SwapFree:") {
            swap_free_kb = val;
        }
    }

    let used_kb = total_kb.saturating_sub(available_kb);
    let used_percent = percent(used_kb, total_kb);

    let swap_used_kb = swap_total_kb.saturating_sub(swap_free_kb);
    let swap_used_percent = percent(swap_used_kb, swap_total_kb);

    Ok(MemoryReport {
        total_kb,
        available_kb,
        used_kb,
        used_percent,
        swap_total_kb,
        swap_free_kb,
        swap_used_kb,
        swap_used_percent,
    })
}

fn parse_kb(line: &str, key: &str) -> Option<u64> {
    let rest = line.strip_prefix(key)?.trim();
    rest.strip_suffix("kB")?.trim().parse().ok()
}

fn percent(used: u64, total: u64) -> f64 {
    if total > 0 { (used as f64 / total as f64) * 100.0 } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_kb_valid() {
        assert_eq!(parse_kb("MemTotal:       16255592 kB", "MemTotal:"), Some(16255592));
    }

    #[test]
    fn test_parse_kb_wrong_key() {
        assert_eq!(parse_kb("MemTotal:       16255592 kB", "MemFree:"), None);
    }

    #[test]
    fn test_parse_kb_malformed() {
        assert_eq!(parse_kb("MemTotal:       abc kB", "MemTotal:"), None);
    }

    #[test]
    fn test_percent_zero_total() {
        assert_eq!(percent(100, 0), 0.0);
    }

    #[test]
    fn test_percent_half() {
        assert!((percent(50, 100) - 50.0).abs() < 0.001);
    }
}
