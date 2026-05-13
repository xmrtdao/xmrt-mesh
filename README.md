# XMRT Mesh — Rust P2P Agent Meshnet

libp2p-based distributed mesh networking for the XMRT DAO agent fleet.

## Architecture

```
┌──────────────┐     libp2p P2P Mesh     ┌──────────────┐
│  Eliza-Dev   │◀══════════════════════▶│   Hermes     │
│  (laptop)    │     Noise + Yamux      │   (phone)    │
│  port 9000   │     ──────────────     │   port 9000  │
└──────┬───────┘                        └──────┬───────┘
       │                                       │
       │         ┌──────────────┐              │
       └────────▶│  Go Relay    │◀─────────────┘
                 │  (port 8081) │
                 └──────┬───────┘
                        │
                 ┌──────┴───────┐
                 │   Supabase   │
                 └──────────────┘
```

## Features

- **mDNS discovery** — automatic local peer discovery
- **Gossipsub** — capability announcements across the mesh
- **Request-response** — direct task dispatch to specific agents
- **Noise encryption** — all traffic encrypted via Noise protocol
- **Yamux multiplexing** — multiple streams over one connection
- **Go relay integration** — reports task results back to the Go relay
- **Custom protocol** — `/xmrt/mesh/agents/0.1.0` for announcements, `/xmrt/mesh/tasks/0.1.0` for tasks

## Prerequisites

- Rust 1.85+ (edition 2024)
- For Windows: Visual Studio Build Tools 2022 (C++ workload) or MinGW-w64
- For Linux: `build-essential`, `pkg-config`, `libssl-dev`
- For ARM64 (Hermes): cross-compilation toolchain

## Quick Start

```bash
# Build
cargo build --release

# Run with config
./target/release/xmrt-mesh

# Run with explicit args
./target/release/xmrt-mesh \
  --name eliza-dev \
  --port 9000 \
  --relay http://localhost:8081 \
  --peers /ip4/192.168.14.115/tcp/9000

# Enable debug logging
RUST_LOG=debug ./target/release/xmrt-mesh
```

## Configuration

Config via `config.toml` (default) or CLI arguments:

```toml
agent_name = "eliza-dev"
port = 9000
mdns_enabled = true
announce_topic = "/xmrt/mesh/agents/0.1.0"
task_protocol = "/xmrt/mesh/tasks/0.1.0"
capabilities = ["bash", "node", "python", "curl", "git"]
relay_url = "http://localhost:8081"
bootstrap_peers = []
```

CLI flags override config file values.

## Cross-Compile

```bash
# Linux x86_64
cargo build --release --target x86_64-unknown-linux-gnu

# ARM64 (Hermes phone)
cargo build --release --target aarch64-unknown-linux-gnu

# ARMv7 (Raspberry Pi)
cargo build --release --target armv7-unknown-linux-gnueabihf
```

## Protocol

### Capability Announcement (Gossipsub)
Every 120 seconds, each node publishes its capabilities to `/xmrt/mesh/agents/0.1.0`:
```json
{
  "agent_id": "eliza-dev",
  "agent_name": "Eliza-Dev",
  "peer_id": "12D3KooW...",
  "capabilities": ["bash", "node", "python"],
  "version": "0.1.0",
  "listen_addrs": ["/ip4/192.168.1.100/tcp/9000"]
}
```

### Task Dispatch (Request-Response)
Tasks are dispatched via the `/xmrt/mesh/tasks/0.1.0` protocol:
- Request: `{ "task_id", "title", "description", "capability", "priority", "payload" }`
- Response: `{ "task_id", "status", "result", "error" }`

### Health Check (Request-Response)
Ping/pong via `/xmrt/mesh/ping/0.1.0`:
- Request: empty `Ping`
- Response: `{ "agent_name", "peer_id", "uptime_secs", "task_count" }`

## Module Structure

```
src/
├── main.rs      # Entry point, CLI, event loop
├── config.rs    # Configuration loading (TOML + CLI overrides)
├── mesh.rs      # libp2p swarm setup, event handling, task processing
├── protocol.rs  # Message types (capability announcement, task dispatch, ping)
└── relay.rs     # Go Relay HTTP client integration
```

## Related

- Go Relay Daemon: `../relay-go/`
- Architecture Issue: https://github.com/xmrtdao/mobilemonero/issues/9
