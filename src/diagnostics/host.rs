use anyhow::Result;
use serde::Serialize;
use std::{ffi::CStr, fs, path::Path};

#[derive(Serialize)]
pub struct HostReport {
    pub hostname: String,
    pub os_type: String,
    pub kernel_release: String,
    pub kernel_version: String,
    pub arch: String,
    pub packages: Option<u64>,
    pub package_manager: Option<String>,
}

pub fn collect() -> Result<HostReport> {
    let uts = uname()?;
    let (packages, package_manager) = count_packages();

    Ok(HostReport {
        hostname: uts.nodename,
        os_type: uts.sysname,
        kernel_release: uts.release,
        kernel_version: uts.version,
        arch: uts.machine,
        packages,
        package_manager,
    })
}

struct Utsname {
    sysname: String,
    nodename: String,
    release: String,
    version: String,
    machine: String,
}

fn uname() -> Result<Utsname> {
    let mut buf: libc::utsname = unsafe { std::mem::zeroed() };
    let ret = unsafe { libc::uname(&mut buf) };
    anyhow::ensure!(ret == 0, "uname syscall failed");

    let to_string = |field: &[libc::c_char]| -> String {
        let ptr = field.as_ptr();
        unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned()
    };

    Ok(Utsname {
        sysname: to_string(&buf.sysname),
        nodename: to_string(&buf.nodename),
        release: to_string(&buf.release),
        version: to_string(&buf.version),
        machine: to_string(&buf.machine),
    })
}

fn count_packages() -> (Option<u64>, Option<String>) {
    // Arch Linux
    if Path::new("/var/lib/pacman/local").is_dir() {
        let count = fs::read_dir("/var/lib/pacman/local")
            .ok()
            .map(|entries| entries.filter_map(|e| e.ok()).count() as u64);
        return (count, Some("pacman".into()));
    }

    // Debian / Ubuntu
    if Path::new("/var/lib/dpkg/info").is_dir() {
        let count = fs::read_dir("/var/lib/dpkg/info").ok().map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().map(|x| x == "list").unwrap_or(false))
                .count() as u64
        });
        return (count, Some("dpkg".into()));
    }

    // Alpine
    let apk_db = Path::new("/lib/apk/db/installed");
    if apk_db.exists() {
        let count = fs::read_to_string(apk_db)
            .ok()
            .map(|s| s.lines().filter(|l| l.starts_with("P:")).count() as u64);
        return (count, Some("apk".into()));
    }

    (None, None)
}
