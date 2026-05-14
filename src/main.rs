use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

mod config;
mod mesh;
mod protocol;
mod relay;

/// XMRT DAO P2P Mesh Node — libp2p-based agent meshnet
#[derive(Parser)]
#[command(
    name = "xmrt-mesh",
    version = "0.1.0",
    about = "XMRT DAO P2P Mesh Node"
)]
struct Cli {
    /// Config file path
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,

    /// Agent name for capability announcements
    #[arg(short, long)]
    name: Option<String>,

    /// Listening port
    #[arg(short, long)]
    port: Option<u16>,

    /// Peers to dial on startup
    #[arg(short, long)]
    peers: Vec<String>,

    /// Go relay base URL (e.g., http://localhost:8081)
    #[arg(short, long)]
    relay: Option<String>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(&cli.log_level))
        .init();

    // Load config (returns config + generated/loaded keypair)
    let (cfg, keypair) = config::Config::load(&cli.config, &cli)?;
    let peer_id = cfg.peer_id(&keypair);

    tracing::info!("XMRT Mesh v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("Agent: {}", cfg.agent_name);
    tracing::info!("Listening on: /ip4/0.0.0.0/tcp/{}", cfg.port);
    tracing::info!("Peer ID: {}", peer_id);

    // Initialize P2P mesh
    let mut mesh_node = mesh::MeshNode::new(&cfg, keypair).await?;

    // Connect to Go relay if configured
    if let Some(relay_url) = &cfg.relay_url {
        let relay_client =
            relay::RelayClient::new(relay_url.clone(), &cfg.agent_name, &cfg.capabilities);
        relay_client.register().await?;
        mesh_node.set_relay_client(relay_client);
        tracing::info!("Connected to Go relay: {}", relay_url);
    }

    // Dial initial peers
    for addr in &cfg.bootstrap_peers {
        tracing::info!("Dialing peer: {}", addr);
        mesh_node.dial(addr).await?;
    }

    // Start the mesh event loop
    mesh_node.run().await?;

    Ok(())
}
