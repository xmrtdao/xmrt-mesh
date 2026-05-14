use anyhow::{Context, Result};
use libp2p::identity::{self, Keypair};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::Cli;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Agent name
    pub agent_name: String,

    /// Listening port for P2P
    pub port: u16,

    /// P2P keypair (auto-generated if not provided)
    pub key_file: Option<String>,

    /// Bootstrap peers to connect to on startup
    pub bootstrap_peers: Vec<String>,

    /// Capabilities this agent provides
    pub capabilities: Vec<String>,

    /// Go relay base URL
    pub relay_url: Option<String>,

    /// mDNS discovery enabled
    pub mdns_enabled: bool,

    /// Gossipsub topic for capability announcements
    pub announce_topic: String,

    /// Request-response protocol name
    pub task_protocol: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            agent_name: "unknown".into(),
            port: 9000,
            key_file: None,
            bootstrap_peers: vec![],
            capabilities: vec!["bash".into()],
            relay_url: None,
            mdns_enabled: true,
            announce_topic: "/xmrt/mesh/agents/0.1.0".into(),
            task_protocol: "/xmrt/mesh/tasks/0.1.0".into(),
        }
    }
}

impl Config {
    /// Load config from file, overlay CLI args.
    pub fn load(path: &Path, cli: &Cli) -> Result<(Config, Keypair)> {
        let mut cfg: Config = if path.exists() {
            let data = std::fs::read_to_string(path)
                .with_context(|| format!("reading config: {}", path.display()))?;
            toml::from_str(&data).with_context(|| format!("parsing config: {}", path.display()))?
        } else {
            Config::default()
        };

        // CLI overrides
        if let Some(name) = &cli.name {
            cfg.agent_name = name.clone();
        }
        if let Some(port) = cli.port {
            cfg.port = port;
        }
        if !cli.peers.is_empty() {
            cfg.bootstrap_peers = cli.peers.clone();
        }
        if let Some(relay) = &cli.relay {
            cfg.relay_url = Some(relay.clone());
        }

        // Load or generate keypair
        let keypair = load_or_generate_key(&cfg.key_file)?;

        Ok((cfg, keypair))
    }

    pub fn peer_id(&self, keypair: &Keypair) -> String {
        keypair.public().to_peer_id().to_string()
    }
}

fn load_or_generate_key(key_file: &Option<String>) -> Result<Keypair> {
    if let Some(path) = key_file {
        let data =
            std::fs::read_to_string(path).with_context(|| format!("reading key file: {}", path))?;
        let keypair = identity::Keypair::from_protobuf_encoding(&hex::decode(data.trim())?)
            .context("decoding keypair")?;
        return Ok(keypair);
    }

    let keypair = identity::Keypair::generate_ed25519();

    // Save generated key
    let encoded = hex::encode(keypair.to_protobuf_encoding().unwrap());
    let key_path = "mesh-key.txt";
    std::fs::write(key_path, &encoded).context("saving generated key")?;
    tracing::info!("Generated new keypair, saved to {}", key_path);

    Ok(keypair)
}
