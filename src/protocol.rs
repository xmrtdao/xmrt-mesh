use async_trait::async_trait;
use libp2p::futures::prelude::*;
use libp2p::request_response::Codec;
use libp2p::StreamProtocol;
use serde::{Deserialize, Serialize};
use std::io;
use std::marker::PhantomData;

/// Agent capability announcement sent over Gossipsub.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityAnnouncement {
    pub agent_id: String,
    pub agent_name: String,
    pub peer_id: String,
    pub capabilities: Vec<String>,
    pub version: String,
    pub listen_addrs: Vec<String>,
}

/// Task dispatch sent over request-response.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskRequest {
    pub task_id: String,
    pub title: String,
    pub description: String,
    pub capability: String,
    pub priority: i32,
    pub payload: serde_json::Value,
}

/// Task result sent back.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskResponse {
    pub task_id: String,
    pub status: String,
    pub result: Option<String>,
    pub error: Option<String>,
}

impl std::fmt::Display for TaskResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TaskResponse {{ id: {}, status: {} }}", self.task_id, self.status)
    }
}

/// Health check ping.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Ping;

/// Health check pong.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Pong {
    pub agent_name: String,
    pub peer_id: String,
    pub uptime_secs: u64,
    pub task_count: u32,
}

impl std::fmt::Display for Pong {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Pong {{ agent: {}, peer: {}, uptime: {}s, tasks: {} }}",
            self.agent_name, self.peer_id, self.uptime_secs, self.task_count
        )
    }
}

/// Generic JSON codec for any request/response pair.
/// Serializes to/from JSON using async I/O.
#[derive(Debug, Clone, Default)]
pub struct JsonCodec<Req, Res> {
    _marker: PhantomData<(Req, Res)>,
}

impl<Req, Res> JsonCodec<Req, Res> {
    pub fn new() -> Self {
        Self { _marker: PhantomData }
    }
}

#[async_trait]
impl<Req, Res> Codec for JsonCodec<Req, Res>
where
    Req: for<'de> Deserialize<'de> + Serialize + Send + 'static,
    Res: for<'de> Deserialize<'de> + Serialize + Send + 'static,
{
    type Protocol = StreamProtocol;
    type Request = Req;
    type Response = Res;

    async fn read_request<T>(&mut self, _protocol: &Self::Protocol, io: &mut T) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut buf = Vec::new();
        io.read_to_end(&mut buf).await?;
        serde_json::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn read_response<T>(&mut self, _protocol: &Self::Protocol, io: &mut T) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut buf = Vec::new();
        io.read_to_end(&mut buf).await?;
        serde_json::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn write_request<T>(&mut self, _protocol: &Self::Protocol, io: &mut T, req: Self::Request) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data = serde_json::to_vec(&req).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        io.write_all(&data).await
    }

    async fn write_response<T>(&mut self, _protocol: &Self::Protocol, io: &mut T, res: Self::Response) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data = serde_json::to_vec(&res).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        io.write_all(&data).await
    }
}

/// Task dispatched from Go relay via HTTP callback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayTask {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub capability: Option<String>,
    pub priority: i32,
    pub payload: Option<serde_json::Value>,
}
