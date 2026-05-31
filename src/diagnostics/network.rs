use anyhow::{Context, Result};
use serde::Serialize;
use std::fs;

#[derive(Serialize)]
pub struct InterfaceEntry {
    pub interface: String,
    pub rx_bytes: u64,
    pub rx_packets: u64,
    pub rx_errors: u64,
    pub rx_dropped: u64,
    pub tx_bytes: u64,
    pub tx_packets: u64,
    pub tx_errors: u64,
    pub tx_dropped: u64,
}

#[derive(Serialize)]
pub struct NetworkReport {
    pub interfaces: Vec<InterfaceEntry>,
}

pub fn collect() -> Result<NetworkReport> {
    let content = fs::read_to_string("/proc/net/dev").context("read /proc/net/dev")?;

    let interfaces = content
        .lines()
        .skip(2) // skip two header lines
        .filter_map(parse_line)
        .filter(|e| e.interface != "lo")
        .collect();

    Ok(NetworkReport { interfaces })
}

pub(crate) fn parse_line(line: &str) -> Option<InterfaceEntry> {
    let (iface, rest) = line.split_once(':')?;
    let interface = iface.trim().to_string();

    let vals: Vec<u64> = rest
        .split_whitespace()
        .map(|s| s.parse().unwrap_or(0))
        .collect();

    // /proc/net/dev columns:
    // rx: bytes packets errs drop fifo frame compressed multicast
    // tx: bytes packets errs drop fifo colls carrier compressed
    Some(InterfaceEntry {
        interface,
        rx_bytes:   *vals.get(0)?,
        rx_packets: *vals.get(1)?,
        rx_errors:  *vals.get(2)?,
        rx_dropped: *vals.get(3)?,
        tx_bytes:   *vals.get(8)?,
        tx_packets: *vals.get(9)?,
        tx_errors:  *vals.get(10)?,
        tx_dropped: *vals.get(11)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_line_valid() {
        let line = " wlan0: 2055925546 1568050    0    0    0     0          0         0 95610723  214919    0    0    0     0       0          0";
        let entry = parse_line(line).unwrap();
        assert_eq!(entry.interface, "wlan0");
        assert_eq!(entry.rx_bytes, 2055925546);
        assert_eq!(entry.rx_packets, 1568050);
        assert_eq!(entry.rx_errors, 0);
        assert_eq!(entry.tx_bytes, 95610723);
        assert_eq!(entry.tx_packets, 214919);
    }

    #[test]
    fn test_parse_line_no_colon() {
        assert!(parse_line("no colon here").is_none());
    }

    #[test]
    fn test_parse_line_too_short() {
        assert!(parse_line("eth0: 1 2 3").is_none());
    }
}
