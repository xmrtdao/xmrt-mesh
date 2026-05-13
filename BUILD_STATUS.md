# Build Status

## ✅ Go Relay (relay-go/) — DEPLOYED on port 8081
- 1,636+ lines, 11 Go files
- Builds cleanly with `go build ./cmd/relayd/`
- Cross-compiled: Windows, Linux, ARM64 (Hermes)
- Running alongside TS relay (port 8080)

## 🟡 Rust P2P Mesh (xmrt-mesh/) — CODE COMPLETE, needs build verification
- 734+ lines, 5 Rust modules
- Architecture: mDNS + Gossipsub + request-response + Identify + Go relay integration
- **Build environment**: VS Build Tools 2022 installed ✅
- **JsonCodec<Req, Res>** — Generic JSON codec implemented in `protocol.rs` ✅
  - Handles `TaskRequest`/`TaskResponse` and `Ping`/`Pong` pairs via `PhantomData`
  - Implements libp2p's `Codec` trait with async read/write
  - `#[derive(Clone, Default)]` for codec construction

### Known issues
1. **libp2p 0.54 API compatibility** — verify against actual libp2p 0.54 crate APIs
2. **Windows build** — requires MSVC toolchain (`build.cmd` or `build.ps1`)
3. **ARM64 cross-compile** — needs `aarch64-unknown-linux-gnu` target

### CI
- GitHub Actions workflow added (`.github/workflows/ci.yml`)
- Builds on ubuntu-latest + windows-latest
- ARM64 cross-compilation artifact

### Quick build
```bash
# Linux/macOS
cargo build --release

# Windows (with MSVC)
build.cmd --release

# ARM64 cross-compile
cargo build --release --target aarch64-unknown-linux-gnu
```
