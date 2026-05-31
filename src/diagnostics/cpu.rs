use anyhow::{Context, Result};
use serde::Serialize;
use std::{fs, thread, time::Duration};

#[derive(Serialize)]
pub struct CpuReport {
    pub usage_percent: f64,
    pub user_percent: f64,
    pub system_percent: f64,
    pub iowait_percent: f64,
    pub idle_percent: f64,
}

pub fn collect() -> Result<CpuReport> {
    let a = read_stat()?;
    thread::sleep(Duration::from_millis(200));
    let b = read_stat()?;

    let total = (b.total - a.total) as f64;
    if total == 0.0 {
        anyhow::bail!("cpu stat delta is zero");
    }

    let pct = |delta: u64| (delta as f64 / total) * 100.0;

    let idle_percent = pct(b.idle - a.idle);
    Ok(CpuReport {
        usage_percent: 100.0 - idle_percent,
        user_percent: pct(b.user - a.user),
        system_percent: pct(b.system - a.system),
        iowait_percent: pct(b.iowait - a.iowait),
        idle_percent,
    })
}

struct StatSnapshot {
    user: u64,
    system: u64,
    idle: u64,
    iowait: u64,
    total: u64,
}

fn read_stat() -> Result<StatSnapshot> {
    let content = fs::read_to_string("/proc/stat").context("read /proc/stat")?;
    let line = content.lines().next().context("empty /proc/stat")?;
    parse_stat_line(line)
}

fn parse_stat_line(line: &str) -> Result<StatSnapshot> {
    // cpu  user nice system idle iowait irq softirq steal ...
    let mut fields = line.split_whitespace();
    fields.next(); // skip "cpu" label

    let vals: Vec<u64> = fields
        .map(|s| s.parse::<u64>().unwrap_or(0))
        .collect();

    let user   = vals.first().copied().unwrap_or(0);
    let nice   = vals.get(1).copied().unwrap_or(0);
    let system = vals.get(2).copied().unwrap_or(0);
    let idle   = vals.get(3).copied().unwrap_or(0);
    let iowait = vals.get(4).copied().unwrap_or(0);
    let total: u64 = vals.iter().sum();

    Ok(StatSnapshot { user: user + nice, system, idle, iowait, total })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_stat_line() {
        let line = "cpu  100 20 50 800 10 0 0 0 0 0";
        let s = parse_stat_line(line).unwrap();
        assert_eq!(s.user, 120);   // user + nice
        assert_eq!(s.system, 50);
        assert_eq!(s.idle, 800);
        assert_eq!(s.iowait, 10);
        assert_eq!(s.total, 980);  // sum of all fields
    }

    #[test]
    fn test_parse_stat_line_zeros() {
        let s = parse_stat_line("cpu  0 0 0 0 0 0 0 0 0 0").unwrap();
        assert_eq!(s.total, 0);
    }
}
