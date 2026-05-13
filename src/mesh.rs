use crate::config::Config;
use crate::protocol::{CapabilityAnnouncement, JsonCodec, Ping, Pong, TaskRequest, TaskResponse};
use crate::relay::RelayClient;
use anyhow::{Context, Result};
use libp2p::gossipsub::{self, MessageAuthenticity, MessageId, TopicHash};
use libp2p::identify;
use libp2p::mdns;
use libp2p::request_response::{self, ProtocolSupport};
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::{identity::Keypair, Multiaddr, StreamProtocol, Swarm, SwarmBuilder};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use tracing::{info, trace, warn};

/// JSON codec type aliases for task dispatch and ping/pong.
type TaskCodec = JsonCodec<TaskRequest, TaskResponse>;
type PingCodec = JsonCodec<Ping, Pong>;

/// The P2P mesh node behaviour.
#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "Event")]
pub struct MeshBehaviour {
    /// mDNS for local peer discovery
    mdns: mdns::Behaviour<mdns::Tokio>,

    /// Identify protocol for exchanging peer info
    identify: identify::Behaviour,

    /// Gossipsub for capability announcements
    gossipsub: gossipsub::Behaviour,

    /// Request-response for task dispatch (JSON codec)
    task_rr: request_response::Behaviour<TaskCodec>,

    /// Ping/pong for health checks (JSON codec)
    ping_rr: request_response::Behaviour<PingCodec>,
}

#[allow(clippy::large_enum_variant)]
pub enum Event {
    Mdns(mdns::Event<mdns::Tokio>),
    Identify(identify::Event),
    Gossipsub(gossipsub::Event),
    TaskRR(request_response::Event<TaskRequest, TaskResponse>),
    PingRR(request_response::Event<Ping, Pong>),
}

impl From<mdns::Event<mdns::Tokio>> for Event {
    fn from(e: mdns::Event<mdns::Tokio>) -> Self { Event::Mdns(e) }
}
impl From<identify::Event> for Event {
    fn from(e: identify::Event) -> Self { Event::Identify(e) }
}
impl From<gossipsub::Event> for Event {
    fn from(e: gossipsub::Event) -> Self { Event::Gossipsub(e) }
}
impl From<request_response::Event<TaskRequest, TaskResponse>> for Event {
    fn from(e: request_response::Event<TaskRequest, TaskResponse>) -> Self { Event::TaskRR(e) }
}
impl From<request_response::Event<Ping, Pong>> for Event {
    fn from(e: request_response::Event<Ping, Pong>) -> Self { Event::PingRR(e) }
}

/// The P2P mesh node.
pub struct MeshNode {
    swarm: Swarm<MeshBehaviour>,
    config: Config,
    keypair: Keypair,
    task_count: u32,
    start_time: std::time::Instant,
    relay_client: Option<RelayClient>,
    announce_topic: TopicHash,
}

impl MeshNode {
    /// Create a new mesh node.
    pub async fn new(config: &Config, keypair: Keypair) -> Result<Self> {
        let peer_id = keypair.public().to_peer_id();

        // --- mDNS ---
        let mdns = mdns::Behaviour::new(mdns::Config::default(), peer_id)?;

        // --- Identify ---
        let identify = identify::Behaviour::new(
            identify::Config::new("/xmrt/mesh/0.1.0".into(), keypair.public())
                .with_interval(Duration::from_secs(60)),
        );

        // --- Gossipsub ---
        let message_id_fn = |message: &gossipsub::Message| {
            let mut s = DefaultHasher::new();
            message.data.hash(&mut s);
            MessageId::from(s.finish().to_string())
        };

        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10))
            .message_id_fn(message_id_fn)
            .build()
            .map_err(|e| anyhow::anyhow!("gossipsub config: {e}"))?;

        let gossipsub = gossipsub::Behaviour::new(
            MessageAuthenticity::Signed(keypair.clone()),
            gossipsub_config,
        )?;

        // --- Request-Response (Tasks) ---
        let task_rr = request_response::Behaviour::new(
            vec![(
                StreamProtocol::new(&config.task_protocol),
                ProtocolSupport::Full,
            )],
            request_response::Config::default()
                .with_request_timeout(Duration::from_secs(60)),
            JsonCodec::new(),
        );

        // --- Request-Response (Ping) ---
        let ping_rr = request_response::Behaviour::new(
            vec![(
                StreamProtocol::new("/xmrt/mesh/ping/0.1.0"),
                ProtocolSupport::Full,
            )],
            request_response::Config::default()
                .with_request_timeout(Duration::from_secs(10)),
            JsonCodec::new(),
        );

        // Build swarm
        let behaviour = MeshBehaviour {
            mdns,
            identify,
            gossipsub,
            task_rr,
            ping_rr,
        };

        let swarm = SwarmBuilder::with_existing_identity(keypair.clone())
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default(),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default(),
            )?
            .with_behaviour(|_| behaviour)?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(120)))
            .build();

        let announce_topic = gossipsub::IdentTopic::new(&config.announce_topic).hash();

        let mut node = Self {
            swarm,
            config: config.clone(),
            keypair,
            task_count: 0,
            start_time: std::time::Instant::now(),
            relay_client: None,
            announce_topic,
        };

        node.swarm.behaviour_mut().gossipsub.subscribe(&node.announce_topic)?;

        Ok(node)
    }

    pub fn set_relay_client(&mut self, client: RelayClient) {
        self.relay_client = Some(client);
    }

    pub async fn dial(&mut self, addr: &str) -> Result<()> {
        let multiaddr: Multiaddr = addr.parse()?;
        self.swarm.dial(multiaddr)?;
        Ok(())
    }

    pub fn listen(&mut self) -> Result<()> {
        let addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", self.config.port).parse()?;
        self.swarm.listen_on(addr)?;
        Ok(())
    }

    pub fn announce(&mut self) {
        let announcement = CapabilityAnnouncement {
            agent_id: self.config.agent_name.clone(),
            agent_name: self.config.agent_name.clone(),
            peer_id: self.keypair.public().to_peer_id().to_string(),
            capabilities: self.config.capabilities.clone(),
            version: env!("CARGO_PKG_VERSION").into(),
            listen_addrs: self.swarm.listeners().map(|a| a.to_string()).collect(),
        };

        let data = serde_json::to_vec(&announcement).unwrap();
        if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(
            self.announce_topic.clone(),
            data,
        ) {
            warn!("Failed to publish announcement: {e}");
        } else {
            info!("Announced capabilities: {:?}", self.config.capabilities);
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        self.listen()?;
        self.announce();
        let mut announce_interval = tokio::time::interval(Duration::from_secs(120));

        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => {
                    self.handle_event(event).await?;
                }
                _ = announce_interval.tick() => {
                    self.announce();
                    self.ping_random_peer();
                }
            }
        }
    }

    async fn handle_event(&mut self, event: SwarmEvent<Event>) -> Result<()> {
        match event {
            SwarmEvent::Behaviour(behaviour_event) => {
                match behaviour_event {
                    Event::Mdns(e) => self.handle_mdns(e).await,
                    Event::Identify(e) => self.handle_identify(e).await,
                    Event::Gossipsub(e) => self.handle_gossipsub(e).await?,
                    Event::TaskRR(e) => self.handle_task_rr(e).await?,
                    Event::PingRR(e) => self.handle_ping_rr(e).await,
                }
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("Listening on: {address}");
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                info!("Connected to peer: {peer_id}");
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                info!("Disconnected from peer: {peer_id}");
            }
            SwarmEvent::IncomingConnection { .. } => {}
            _ => {}
        }
        Ok(())
    }

    async fn handle_mdns(&mut self, event: mdns::Event<mdns::Tokio>) {
        match event {
            mdns::Event::Discovered(list) => {
                for (peer_id, addr) in list {
                    info!("mDNS discovered: {peer_id} at {addr}");
                }
            }
            mdns::Event::Expired(list) => {
                for (peer_id, _) in list {
                    info!("mDNS expired: {peer_id}");
                }
            }
        }
    }

    async fn handle_identify(&mut self, event: identify::Event) {
        match event {
            identify::Event::Received { peer_id, info } => {
                info!("Identified peer {peer_id}: {:?}", info.protocol_version);
            }
            identify::Event::Sent { peer_id } => {
                trace!("Identify sent to {peer_id}");
            }
            _ => {}
        }
    }

    async fn handle_gossipsub(&mut self, event: gossipsub::Event) -> Result<()> {
        match event {
            gossipsub::Event::Message { message, .. } => {
                if let Ok(ann) = serde_json::from_slice::<CapabilityAnnouncement>(&message.data) {
                    info!(
                        "Agent discovered: {} ({}) — capabilities: {:?}",
                        ann.agent_name, ann.peer_id, ann.capabilities
                    );
                }
            }
            gossipsub::Event::Subscribed { peer_id, topic } => {
                info!("Peer {peer_id} subscribed to {topic}");
            }
            gossipsub::Event::Unsubscribed { peer_id, topic } => {
                info!("Peer {peer_id} unsubscribed from {topic}");
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_task_rr(
        &mut self,
        event: request_response::Event<TaskRequest, TaskResponse>,
    ) -> Result<()> {
        match event {
            request_response::Event::Message { peer, message } => {
                match message {
                    request_response::Message::Request {
                        request, channel, ..
                    } => {
                        info!(
                            "Task request from {peer}: {} — {}",
                            request.task_id, request.title
                        );
                        self.task_count += 1;
                        let response = self.process_task(&request).await;
                        self.swarm.behaviour_mut().task_rr.send_response(channel, response)?;
                    }
                    request_response::Message::Response { response, .. } => {
                        info!("Task response: {} — {}", response.task_id, response.status);
                        if let Some(relay) = &self.relay_client {
                            relay.report_result(&response).await;
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_ping_rr(&mut self, event: request_response::Event<Ping, Pong>) {
        match event {
            request_response::Event::Message { peer, message } => {
                match message {
                    request_response::Message::Request { channel, .. } => {
                        let pong = Pong {
                            agent_name: self.config.agent_name.clone(),
                            peer_id: self.keypair.public().to_peer_id().to_string(),
                            uptime_secs: self.start_time.elapsed().as_secs(),
                            task_count: self.task_count,
                        };
                        if let Err(e) = self.swarm.behaviour_mut().ping_rr.send_response(channel, pong) {
                            warn!("Failed to send pong to {peer}: {e}");
                        }
                    }
                    request_response::Message::Response { response, .. } => {
                        trace!("Pong from {peer}: {:?}", response);
                    }
                }
            }
            _ => {}
        }
    }

    async fn process_task(&self, request: &TaskRequest) -> TaskResponse {
        match request.capability.as_str() {
            "bash" | "shell" => {
                match execute_shell(&request.payload) {
                    Ok(output) => TaskResponse {
                        task_id: request.task_id.clone(),
                        status: "completed".into(),
                        result: Some(output),
                        error: None,
                    },
                    Err(e) => TaskResponse {
                        task_id: request.task_id.clone(),
                        status: "failed".into(),
                        result: None,
                        error: Some(e.to_string()),
                    },
                }
            }
            _ => TaskResponse {
                task_id: request.task_id.clone(),
                status: "rejected".into(),
                result: None,
                error: Some(format!("unsupported capability: {}", request.capability)),
            },
        }
    }

    fn ping_random_peer(&mut self) {
        let peers: Vec<_> = self.swarm.connected_peers().copied().collect();
        if !peers.is_empty() {
            let peer = peers[0];
            self.swarm.behaviour_mut().ping_rr.send_request(&peer, Ping);
        }
    }
}

fn execute_shell(payload: &serde_json::Value) -> Result<String, std::io::Error> {
    let command = payload.get("command").and_then(|v| v.as_str()).unwrap_or("echo hello");
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
