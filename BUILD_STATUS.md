# Build Status

## ✅ Go Relay (relay-go/) — DEPLOYED on port 8081
- 1,636+ lines, 11 Go files
- Builds cleanly with `go build ./cmd/relayd/`
- Cross-compiled: Windows, Linux, ARM64 (Hermes)
- Running alongside TS relay (port 8080)

## 🟢 Rust P2P Mesh (xmrt-mesh/) — BUILD SUCCESSFUL ✅
- 734+ lines, 5 Rust modules
- Architecture: mDNS + Gossipsub + request-response + Identify + Go relay integration
- libp2p 0.54.1 — compatible with installed crate versions ✅
- **Windows build (MSVC):** `cargo build --release` — compiles clean ✅
- **Binary:** `target/release/xmrt-mesh.exe` (7.2MB)
- **Config:** `config.toml` — agent name, port, capabilities, bootstrap peers
- **Hermes (ARM64 Termux):** `cargo build --release` on-device via `pkg install rust`

### API migration fixes applied
- `mdns::Behaviour` → `mdns::tokio::Behaviour` (Tokio provider generic)
- `request_response::Behaviour::new` — takes 2 args (protocols + config), codec via generic
- `JsonCodec` uses `#[async_trait]` with `futures::io::AsyncRead/Write` (not tokio)
- `StreamProtocol::try_from_owned()` for dynamic protocol strings
- `with_tcp` — security closure signature: `FnOnce(&Keypair) -> Result<_, _>`
- `gossipsub::subscribe` takes `&Topic<H>`, not `&TopicHash`
- `identify::Event` patterns need `..` for optional fields
