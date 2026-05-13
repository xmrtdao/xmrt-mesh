use libp2p::request_response::Codec;
use libp2p::StreamProtocol;
use serde::{Deserialize, Serialize};
use std::io;
use std::marker::PhantomData;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequest {
    pub task_id: String,
    pub title: String,
    pub description: String,
    pub capability: String,
    pub priority: i32,
    pub payload: serde_json::Value,
}

/// Task result sent back.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResponse {
    pub task_id: String,
    pub status: String,
    pub result: Option<String>,
    pub error: Option<String>,
}

/// Health check ping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ping;

/// Health check pong.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pong {
    pub agent_name: String,
    pub peer_id: String,
    pub uptime_secs: u64,
    pub task_count: u32,
}

/// Generic JSON codec for any request/response pair.
/// Serializes to/from JSON using async I/O with length-delimited framing.
#[derive(Debug, Clone, Default)]
pub struct JsonCodec<Req, Res> {
    _marker: PhantomData<(Req, Res)>,
}

impl<Req, Res> JsonCodec<Req, Res> {
    pub fn new() -> Self {
        Self { _marker: PhantomData }
    }
}

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
        T: tokio::io::AsyncRead + Unpin + Send,
    {
        let mut buf = Vec::new();
        io.read_to_end(&mut buf).await?;
        serde_json::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn read_response<T>(&mut self, _protocol: &Self::Protocol, io: &mut T) -> io::Result<Self::Response>
    where
        T: tokio::io::AsyncRead + Unpin + Send,
    {
        let mut buf = Vec::new();
        io.read_to_end(&mut buf).await?;
        serde_json::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn write_request<T>(&mut self, _protocol: &Self::Protocol, io: &mut T, req: Self::Request) -> io::Result<()>
    where
        T: tokio::io::AsyncWrite + Unpin + Send,
    {
        let data = serde_json::to_vec(&req).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        io.write_all(&data).await
    }

    async fn write_response<T>(&mut self, _protocol: &Self::Protocol, io: &mut T, res: Self::Response) -> io::Result<()>
    where
        T: tokio::io::AsyncWrite + Unpin + Send,
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
