# check-my-server

![Rust](https://img.shields.io/badge/language-Rust-orange?logo=rust)
![Platform](https://img.shields.io/badge/platform-Linux-lightgrey?logo=linux)
![Prometheus](https://img.shields.io/badge/metrics-Prometheus-orange?logo=prometheus)
![Version](https://img.shields.io/badge/version-0.4.0-blue)

Lightweight Prometheus exporter for Linux server metrics. Reads from `/proc` directly — no node_exporter required.

## Getting Started

### Run directly

Supports **x86_64** and **aarch64** (ARM64). Download the binary matching your architecture:

```bash
# x86_64
curl -fsSL https://github.com/jujolabs/check-my-server/releases/latest/download/check-my-server-x86_64-linux \
  -o check-my-server && chmod +x check-my-server

# aarch64
curl -fsSL https://github.com/jujolabs/check-my-server/releases/latest/download/check-my-server-aarch64-linux \
  -o check-my-server && chmod +x check-my-server

# defaults: listen on 0.0.0.0:9100, collect every 15s
./check-my-server

# custom config
CMS_ADDR=0.0.0.0:9100 CMS_INTERVAL=30 RUST_LOG=debug ./check-my-server
```

### systemd service (recommended for servers)

```bash
curl -fsSL https://raw.githubusercontent.com/jujolabs/check-my-server/main/contrib/install.sh | sudo bash
```

Downloads the latest binary, installs the systemd unit, and starts the service on boot.
To override config, edit `/etc/systemd/system/check-my-server.service` and run `systemctl daemon-reload`.

<details>
<summary>Manual steps</summary>

```bash
sudo cp check-my-server /usr/local/bin/
sudo chmod +x /usr/local/bin/check-my-server
sudo cp contrib/check-my-server.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now check-my-server
```

</details>

## Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /metrics` | Prometheus text format metrics |
| `GET /health` | Returns `200 ok` — use for liveness probes |

## Configuration

| Env var | Default | Description |
|---------|---------|-------------|
| `CMS_ADDR` | `0.0.0.0:9100` | Listen address |
| `CMS_INTERVAL` | `15` | Collection interval in seconds |
| `RUST_LOG` | `info` | Log level (`error`, `warn`, `info`, `debug`, `trace`) |

```bash
CMS_ADDR=0.0.0.0:9200 CMS_INTERVAL=30 RUST_LOG=debug ./target/release/check-my-server
```

## How it works

- Collects all metrics once at startup, then every `CMS_INTERVAL` seconds in the background
- Scrapes are served instantly from memory — zero collection work per request
- If a module fails (e.g. unreadable `/proc` file), its metrics are omitted and a `# collector error:` comment appears in the output; all other modules still respond
- Shuts down cleanly on SIGTERM or Ctrl+C

## Prometheus config

```yaml
scrape_configs:
  - job_name: check-my-server
    static_configs:
      - targets: ['<host>:9100']
```

## Metrics

### Host
| Metric | Description |
|--------|-------------|
| `node_uname_info{hostname, os_type, kernel_release, arch}` | System info (value always 1) |

### System
| Metric | Description |
|--------|-------------|
| `node_uptime_seconds` | System uptime |
| `node_load1` | 1-minute load average |
| `node_load5` | 5-minute load average |
| `node_load15` | 15-minute load average |
| `node_procs_running` | Currently running processes |
| `node_procs_total` | Total processes |
| `node_filefd_allocated` | Open file descriptors |
| `node_filefd_maximum` | File descriptor limit |

### Memory
| Metric | Description |
|--------|-------------|
| `node_memory_total_bytes` | Total RAM |
| `node_memory_available_bytes` | Available RAM |
| `node_memory_used_bytes` | Used RAM |
| `node_swap_total_bytes` | Total swap |
| `node_swap_free_bytes` | Free swap |
| `node_swap_used_bytes` | Used swap |

### CPU
| Metric | Description |
|--------|-------------|
| `node_cpu_usage_percent` | Overall CPU usage |
| `node_cpu_user_percent` | User-space CPU time |
| `node_cpu_system_percent` | Kernel CPU time |
| `node_cpu_iowait_percent` | I/O wait time |
| `node_cpu_idle_percent` | Idle time |

### Disk
| Metric | Labels | Description |
|--------|--------|-------------|
| `node_filesystem_size_bytes` | `mountpoint` | Total filesystem size |
| `node_filesystem_used_bytes` | `mountpoint` | Used space |
| `node_filesystem_avail_bytes` | `mountpoint` | Available space |

### Network
| Metric | Labels | Description |
|--------|--------|-------------|
| `node_network_receive_bytes_total` | `device` | Bytes received |
| `node_network_receive_packets_total` | `device` | Packets received |
| `node_network_receive_errors_total` | `device` | Receive errors |
| `node_network_receive_drop_total` | `device` | Receive drops |
| `node_network_transmit_bytes_total` | `device` | Bytes transmitted |
| `node_network_transmit_packets_total` | `device` | Packets transmitted |
| `node_network_transmit_errors_total` | `device` | Transmit errors |
| `node_network_transmit_drop_total` | `device` | Transmit drops |
