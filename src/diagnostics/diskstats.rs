use anyhow::{Context, Result};
use serde::Serialize;
use std::fs;

#[derive(Serialize)]
pub struct DiskStatsEntry {
    pub device: String,
    pub reads_completed: u64,
    pub writes_completed: u64,
    pub read_bytes: u64,
    pub written_bytes: u64,
    pub read_ms: u64,
    pub write_ms: u64,
}

#[derive(Serialize)]
pub struct DiskStatsReport {
    pub devices: Vec<DiskStatsEntry>,
}

pub fn collect() -> Result<DiskStatsReport> {
    let content = fs::read_to_string("/proc/diskstats").context("read /proc/diskstats")?;
    let mut devices = Vec::new();

    for line in content.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 11 {
            continue;
        }
        let name = fields[2];
        if name.starts_with("loop") || name.starts_with("ram") {
            continue;
        }

        let reads_completed  = fields[3].parse::<u64>().unwrap_or(0);
        let sectors_read     = fields[5].parse::<u64>().unwrap_or(0);
        let read_ms          = fields[6].parse::<u64>().unwrap_or(0);
        let writes_completed = fields[7].parse::<u64>().unwrap_or(0);
        let sectors_written  = fields[9].parse::<u64>().unwrap_or(0);
        let write_ms         = fields[10].parse::<u64>().unwrap_or(0);

        devices.push(DiskStatsEntry {
            device: name.to_string(),
            reads_completed,
            writes_completed,
            read_bytes:    sectors_read    * 512,
            written_bytes: sectors_written * 512,
            read_ms,
            write_ms,
        });
    }

    Ok(DiskStatsReport { devices })
}
