use anyhow::{Context, Result};
use serde::Serialize;
use std::fs;

#[derive(Serialize)]
pub struct PsiMetric {
    pub avg10: f64,
    pub avg60: f64,
    pub avg300: f64,
}

#[derive(Serialize)]
pub struct PressureReport {
    pub cpu_some: PsiMetric,
    pub memory_some: PsiMetric,
    pub memory_full: PsiMetric,
    pub io_some: PsiMetric,
    pub io_full: PsiMetric,
}

pub fn collect() -> Result<PressureReport> {
    let cpu = fs::read_to_string("/proc/pressure/cpu").context("read /proc/pressure/cpu")?;
    let memory =
        fs::read_to_string("/proc/pressure/memory").context("read /proc/pressure/memory")?;
    let io = fs::read_to_string("/proc/pressure/io").context("read /proc/pressure/io")?;

    let cpu_some = parse_line(&cpu, "some").context("parse cpu some")?;
    let memory_some = parse_line(&memory, "some").context("parse memory some")?;
    let memory_full = parse_line(&memory, "full").context("parse memory full")?;
    let io_some = parse_line(&io, "some").context("parse io some")?;
    let io_full = parse_line(&io, "full").context("parse io full")?;

    Ok(PressureReport {
        cpu_some,
        memory_some,
        memory_full,
        io_some,
        io_full,
    })
}

fn parse_line(content: &str, prefix: &str) -> Result<PsiMetric> {
    let line = content
        .lines()
        .find(|l| l.starts_with(prefix))
        .with_context(|| format!("missing '{prefix}' line"))?;

    Ok(PsiMetric {
        avg10: extract_f64(line, "avg10=")?,
        avg60: extract_f64(line, "avg60=")?,
        avg300: extract_f64(line, "avg300=")?,
    })
}

fn extract_f64(line: &str, key: &str) -> Result<f64> {
    let start = line
        .find(key)
        .with_context(|| format!("key '{key}' not found"))?
        + key.len();
    let rest = &line[start..];
    let end = rest.find(' ').unwrap_or(rest.len());
    rest[..end]
        .parse::<f64>()
        .with_context(|| format!("parse f64 for '{key}'"))
}
