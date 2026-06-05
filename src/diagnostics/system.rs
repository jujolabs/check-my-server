use anyhow::{Context, Result};
use serde::Serialize;
use std::fs;

#[derive(Serialize)]
pub struct SystemReport {
    pub uptime_seconds: f64,
    pub load_avg_1: f64,
    pub load_avg_5: f64,
    pub load_avg_15: f64,
    pub running_procs: u32,
    pub total_procs: u32,
    pub open_fds: u64,
    pub max_fds: u64,
}

pub fn collect() -> Result<SystemReport> {
    let uptime_seconds =
        parse_uptime(&fs::read_to_string("/proc/uptime").context("read /proc/uptime")?)?;
    let (load_avg_1, load_avg_5, load_avg_15, running_procs, total_procs) =
        parse_loadavg(&fs::read_to_string("/proc/loadavg").context("read /proc/loadavg")?)?;
    let (open_fds, max_fds) = parse_file_nr(
        &fs::read_to_string("/proc/sys/fs/file-nr").context("read /proc/sys/fs/file-nr")?,
    )?;

    Ok(SystemReport {
        uptime_seconds,
        load_avg_1,
        load_avg_5,
        load_avg_15,
        running_procs,
        total_procs,
        open_fds,
        max_fds,
    })
}

fn parse_uptime(content: &str) -> Result<f64> {
    content
        .split_whitespace()
        .next()
        .context("empty /proc/uptime")?
        .parse::<f64>()
        .context("parse uptime")
}

fn parse_loadavg(content: &str) -> Result<(f64, f64, f64, u32, u32)> {
    let mut fields = content.split_whitespace();

    let load1 = fields
        .next()
        .context("load1")?
        .parse::<f64>()
        .context("parse load1")?;
    let load5 = fields
        .next()
        .context("load5")?
        .parse::<f64>()
        .context("parse load5")?;
    let load15 = fields
        .next()
        .context("load15")?
        .parse::<f64>()
        .context("parse load15")?;

    let procs = fields.next().context("procs")?;
    let (running, total) = procs.split_once('/').context("parse procs")?;
    let running_procs = running.parse::<u32>().context("parse running")?;
    let total_procs = total.parse::<u32>().context("parse total")?;

    Ok((load1, load5, load15, running_procs, total_procs))
}

fn parse_file_nr(content: &str) -> Result<(u64, u64)> {
    let mut fields = content.split_whitespace();

    let open_fds = fields
        .next()
        .context("open_fds")?
        .parse::<u64>()
        .context("parse open_fds")?;
    fields.next(); // unused (always 0 on Linux 2.6+)
    let max_fds = fields
        .next()
        .context("max_fds")?
        .parse::<u64>()
        .context("parse max_fds")?;

    Ok((open_fds, max_fds))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_uptime() {
        let (secs, _) = (parse_uptime("97585.07 59181.27").unwrap(), ());
        assert!((secs - 97585.07).abs() < 0.01);
    }

    #[test]
    fn test_parse_uptime_empty() {
        assert!(parse_uptime("").is_err());
    }

    #[test]
    fn test_parse_loadavg() {
        let (l1, l5, l15, run, total) = parse_loadavg("1.38 1.04 1.07 2/847 12345").unwrap();
        assert!((l1 - 1.38).abs() < 0.001);
        assert!((l5 - 1.04).abs() < 0.001);
        assert!((l15 - 1.07).abs() < 0.001);
        assert_eq!(run, 2);
        assert_eq!(total, 847);
    }

    #[test]
    fn test_parse_loadavg_malformed() {
        assert!(parse_loadavg("1.0 2.0 3.0 bad").is_err());
    }

    #[test]
    fn test_parse_file_nr() {
        let (open, max) = parse_file_nr("11686\t0\t9223372036854775807").unwrap();
        assert_eq!(open, 11686);
        assert_eq!(max, 9223372036854775807);
    }
}
