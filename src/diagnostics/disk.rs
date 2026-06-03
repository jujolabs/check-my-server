use anyhow::{Context, Result};
use serde::Serialize;
use std::{ffi::CString, fs};

#[derive(Serialize)]
pub struct DiskEntry {
    pub mount: String,
    pub total_kb: u64,
    pub used_kb: u64,
    pub available_kb: u64,
    pub used_percent: f64,
}

#[derive(Serialize)]
pub struct DiskReport {
    pub filesystems: Vec<DiskEntry>,
}

const SKIP_FS: &[&str] = &[
    "tmpfs", "devtmpfs", "proc", "sysfs", "devpts", "cgroup", "cgroup2",
    "pstore", "bpf", "tracefs", "hugetlbfs", "mqueue", "debugfs", "fusectl",
    "securityfs", "efivarfs", "autofs", "configfs", "squashfs",
];

pub fn collect() -> Result<DiskReport> {
    let content = fs::read_to_string("/proc/mounts").context("read /proc/mounts")?;

    let mut seen = std::collections::HashSet::new();
    let mut filesystems = Vec::new();

    for line in content.lines() {
        let mut fields = line.split_whitespace();
        let _device = fields.next().unwrap_or("");
        let mount = fields.next().unwrap_or("");
        let fs_type = fields.next().unwrap_or("");

        if SKIP_FS.contains(&fs_type) || !seen.insert(mount.to_string()) {
            continue;
        }

        if let Ok(entry) = statvfs_entry(mount)
            && entry.total_kb > 0 {
                filesystems.push(entry);
            }
    }

    Ok(DiskReport { filesystems })
}

fn statvfs_entry(mount: &str) -> Result<DiskEntry> {
    let path = CString::new(mount).context("invalid mount path")?;
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };

    let ret = unsafe { libc::statvfs(path.as_ptr(), &mut stat) };
    anyhow::ensure!(ret == 0, "statvfs failed for {mount}");

    let block = stat.f_frsize as u64;
    let total_kb = stat.f_blocks * block / 1024;
    let available_kb = stat.f_bavail * block / 1024;
    let used_kb = total_kb.saturating_sub(available_kb);
    let used_percent = if total_kb > 0 {
        (used_kb as f64 / total_kb as f64) * 100.0
    } else {
        0.0
    };

    Ok(DiskEntry {
        mount: mount.to_string(),
        total_kb,
        used_kb,
        available_kb,
        used_percent,
    })
}
