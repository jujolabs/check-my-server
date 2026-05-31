use super::{CheckResult, Report};

pub fn format(report: Report) -> String {
    let mut out = String::new();

    match report.host {
        CheckResult::Success(h) => {
            g(&mut out, "node_uname_info", "System information",
                &[("hostname", &h.hostname), ("os_type", &h.os_type),
                  ("kernel_release", &h.kernel_release), ("arch", &h.arch)],
                1.0);
        }
        CheckResult::Failure { error } => {
            out.push_str(&format!("# collector error: host: {error}\n"));
        }
    }

    if let CheckResult::Success(s) = report.system {
        g(&mut out, "node_uptime_seconds", "System uptime in seconds", &[], s.uptime_seconds);
        g(&mut out, "node_load1", "1 minute load average", &[], s.load_avg_1);
        g(&mut out, "node_load5", "5 minute load average", &[], s.load_avg_5);
        g(&mut out, "node_load15", "15 minute load average", &[], s.load_avg_15);
        g(&mut out, "node_procs_running", "Running processes", &[], s.running_procs as f64);
        g(&mut out, "node_procs_total", "Total processes", &[], s.total_procs as f64);
        g(&mut out, "node_filefd_allocated", "Open file descriptors", &[], s.open_fds as f64);
        g(&mut out, "node_filefd_maximum", "Maximum file descriptors", &[], s.max_fds as f64);
    }

    if let CheckResult::Success(m) = report.memory {
        g(&mut out, "node_memory_total_bytes", "Total memory bytes", &[], kb(m.total_kb));
        g(&mut out, "node_memory_available_bytes", "Available memory bytes", &[], kb(m.available_kb));
        g(&mut out, "node_memory_used_bytes", "Used memory bytes", &[], kb(m.used_kb));
        g(&mut out, "node_swap_total_bytes", "Total swap bytes", &[], kb(m.swap_total_kb));
        g(&mut out, "node_swap_free_bytes", "Free swap bytes", &[], kb(m.swap_free_kb));
        g(&mut out, "node_swap_used_bytes", "Used swap bytes", &[], kb(m.swap_used_kb));
    }

    if let CheckResult::Success(c) = report.cpu {
        g(&mut out, "node_cpu_usage_percent", "CPU usage percent", &[], c.usage_percent);
        g(&mut out, "node_cpu_user_percent", "CPU user time percent", &[], c.user_percent);
        g(&mut out, "node_cpu_system_percent", "CPU system time percent", &[], c.system_percent);
        g(&mut out, "node_cpu_iowait_percent", "CPU iowait percent", &[], c.iowait_percent);
        g(&mut out, "node_cpu_idle_percent", "CPU idle percent", &[], c.idle_percent);
    }

    if let CheckResult::Success(d) = report.disk {
        hdr(&mut out, "node_filesystem_size_bytes", "Filesystem size in bytes");
        for f in &d.filesystems {
            line(&mut out, "node_filesystem_size_bytes", &[("mountpoint", &f.mount)], kb(f.total_kb));
        }
        hdr(&mut out, "node_filesystem_used_bytes", "Filesystem used bytes");
        for f in &d.filesystems {
            line(&mut out, "node_filesystem_used_bytes", &[("mountpoint", &f.mount)], kb(f.used_kb));
        }
        hdr(&mut out, "node_filesystem_avail_bytes", "Filesystem available bytes");
        for f in &d.filesystems {
            line(&mut out, "node_filesystem_avail_bytes", &[("mountpoint", &f.mount)], kb(f.available_kb));
        }
    }

    if let CheckResult::Success(n) = report.network {
        macro_rules! net_family {
            ($name:expr, $help:expr, $field:ident) => {
                hdr(&mut out, $name, $help);
                for i in &n.interfaces {
                    line(&mut out, $name, &[("device", &i.interface)], i.$field as f64);
                }
            };
        }
        net_family!("node_network_receive_bytes_total",    "Bytes received",         rx_bytes);
        net_family!("node_network_receive_packets_total",  "Packets received",       rx_packets);
        net_family!("node_network_receive_errors_total",   "Receive errors",         rx_errors);
        net_family!("node_network_receive_drop_total",     "Receive drops",          rx_dropped);
        net_family!("node_network_transmit_bytes_total",   "Bytes transmitted",      tx_bytes);
        net_family!("node_network_transmit_packets_total", "Packets transmitted",    tx_packets);
        net_family!("node_network_transmit_errors_total",  "Transmit errors",        tx_errors);
        net_family!("node_network_transmit_drop_total",    "Transmit drops",         tx_dropped);
    }

    out
}

fn hdr(out: &mut String, name: &str, help: &str) {
    out.push_str(&format!("# HELP {name} {help}\n# TYPE {name} gauge\n"));
}

fn line(out: &mut String, name: &str, labels: &[(&str, &str)], value: f64) {
    if labels.is_empty() {
        out.push_str(&format!("{name} {value}\n"));
    } else {
        let ls: String = labels.iter().map(|(k, v)| format!("{k}=\"{v}\"")).collect::<Vec<_>>().join(",");
        out.push_str(&format!("{name}{{{ls}}} {value}\n"));
    }
}

fn g(out: &mut String, name: &str, help: &str, labels: &[(&str, &str)], value: f64) {
    hdr(out, name, help);
    line(out, name, labels, value);
}

fn kb(v: u64) -> f64 {
    (v * 1024) as f64
}
