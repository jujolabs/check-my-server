use anyhow::{Context, Result};
use serde::Serialize;
use std::{fs, sync::Mutex, thread, time::Duration};

#[derive(Serialize)]
pub struct CpuCoreReport {
    pub core: usize,
    pub usage_percent: f64,
    pub idle_percent: f64,
}

#[derive(Serialize)]
pub struct CpuReport {
    pub usage_percent: f64,
    pub user_percent: f64,
    pub system_percent: f64,
    pub iowait_percent: f64,
    pub idle_percent: f64,
    pub cores: Vec<CpuCoreReport>,
}

static PREV: Mutex<Option<FullSnapshot>> = Mutex::new(None);

pub fn collect() -> Result<CpuReport> {
    let current = read_stat()?;
    let mut guard = PREV.lock().unwrap_or_else(|e| e.into_inner());

    if let Some(prev) = guard.take() {
        let report = compute_report(&prev, &current);
        *guard = Some(current);
        report
    } else {
        // first call — no previous snapshot, fall back to 200ms sample
        drop(guard);
        let a = read_stat()?;
        thread::sleep(Duration::from_millis(200));
        let b = read_stat()?;
        let report = compute_report(&a, &b);
        *PREV.lock().unwrap_or_else(|e| e.into_inner()) = Some(b);
        report
    }
}

fn compute_report(a: &FullSnapshot, b: &FullSnapshot) -> Result<CpuReport> {
    let total = (b.agg.total - a.agg.total) as f64;
    anyhow::ensure!(total > 0.0, "cpu stat delta is zero");
    let pct = |delta: u64| (delta as f64 / total) * 100.0;
    let idle_percent = pct(b.agg.idle - a.agg.idle);

    let cores = a.cores.iter().zip(b.cores.iter()).enumerate()
        .map(|(i, (ca, cb))| {
            let core_total = (cb.total - ca.total) as f64;
            let (core_usage, core_idle) = if core_total > 0.0 {
                let idle = ((cb.idle - ca.idle) as f64 / core_total) * 100.0;
                (100.0 - idle, idle)
            } else {
                (0.0, 100.0)
            };
            CpuCoreReport { core: i, usage_percent: core_usage, idle_percent: core_idle }
        })
        .collect();

    Ok(CpuReport {
        usage_percent:  100.0 - idle_percent,
        user_percent:   pct(b.agg.user   - a.agg.user),
        system_percent: pct(b.agg.system - a.agg.system),
        iowait_percent: pct(b.agg.iowait - a.agg.iowait),
        idle_percent,
        cores,
    })
}

struct StatSnapshot {
    user: u64,
    system: u64,
    idle: u64,
    iowait: u64,
    total: u64,
}

struct FullSnapshot {
    agg: StatSnapshot,
    cores: Vec<StatSnapshot>,
}

fn read_stat() -> Result<FullSnapshot> {
    let content = fs::read_to_string("/proc/stat").context("read /proc/stat")?;
    let mut lines = content.lines();
    let agg_line = lines.next().context("empty /proc/stat")?;
    let agg = parse_stat_line(agg_line)?;
    let mut cores = Vec::new();
    for line in lines {
        // per-core lines start with "cpu0", "cpu1", etc.
        if line.starts_with("cpu") && line.len() > 3 && line.chars().nth(3).is_some_and(|c| c.is_ascii_digit()) {
            cores.push(parse_stat_line(line)?);
        } else {
            break;
        }
    }
    Ok(FullSnapshot { agg, cores })
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

    #[test]
    fn test_compute_report_with_cores() {
        let a = FullSnapshot {
            agg: StatSnapshot { user: 100, system: 50, idle: 800, iowait: 10, total: 980 },
            cores: vec![
                StatSnapshot { user: 50, system: 25, idle: 400, iowait: 5, total: 490 },
                StatSnapshot { user: 50, system: 25, idle: 400, iowait: 5, total: 490 },
            ],
        };
        let b = FullSnapshot {
            agg: StatSnapshot { user: 200, system: 100, idle: 1600, iowait: 20, total: 1960 },
            cores: vec![
                StatSnapshot { user: 100, system: 50, idle: 800, iowait: 10, total: 980 },
                StatSnapshot { user: 100, system: 50, idle: 800, iowait: 10, total: 980 },
            ],
        };
        let r = compute_report(&a, &b).unwrap();
        assert_eq!(r.cores.len(), 2);
        assert_eq!(r.cores[0].core, 0);
        assert_eq!(r.cores[1].core, 1);
        let expected_idle = (400.0_f64 / 490.0) * 100.0;
        assert!((r.cores[0].idle_percent  - expected_idle).abs() < 0.001);
        assert!((r.cores[0].usage_percent - (100.0 - expected_idle)).abs() < 0.001);
    }

    #[test]
    fn test_compute_report_no_cores() {
        let a = FullSnapshot {
            agg: StatSnapshot { user: 100, system: 50, idle: 800, iowait: 10, total: 980 },
            cores: vec![],
        };
        let b = FullSnapshot {
            agg: StatSnapshot { user: 200, system: 100, idle: 1600, iowait: 20, total: 1960 },
            cores: vec![],
        };
        let r = compute_report(&a, &b).unwrap();
        assert!(r.cores.is_empty());
    }

    #[test]
    fn test_read_stat_parses_cores() {
        let snap = read_stat().unwrap();
        assert!(!snap.cores.is_empty(), "expected per-core cpu lines in /proc/stat");
    }
}
