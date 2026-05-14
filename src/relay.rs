use crate::protocol::{RelayTask, TaskResponse};
use anyhow::Result;
use serde_json::json;
use std::time::Duration;

/// Client for communicating with the Go Relay Daemon.
pub struct RelayClient {
    base_url: String,
    agent_name: String,
    capabilities: Vec<String>,
    client: reqwest::Client,
}

impl RelayClient {
    pub fn new(base_url: String, agent_name: &str, capabilities: &[String]) -> Self {
        Self {
            base_url,
            agent_name: agent_name.into(),
            capabilities: capabilities.to_vec(),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap(),
        }
    }

    /// Register with the Go relay via its REST API.
    pub async fn register(&self) -> Result<()> {
        let url = format!("{}/api/v1/agents", self.base_url);
        let resp = self.client.get(&url).send().await?;

        tracing::info!("Relay registration check: {}", resp.status());
        Ok(())
    }

    /// Report task completion back to the Go relay.
    pub async fn report_result(&self, response: &TaskResponse) {
        let url = format!("{}/api/v1/tasks/{}", self.base_url, response.task_id);

        let status = match response.status.as_str() {
            "completed" => "completed",
            _ => "failed",
        };

        let body = json!({
            "status": status,
            "result": response.result,
            "error": response.error,
        });

        match self.client.patch(&url).json(&body).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    tracing::info!("Reported task {} as {}", response.task_id, status);
                } else {
                    tracing::warn!("Failed to report task: {}", resp.status());
                }
            }
            Err(e) => {
                tracing::error!("Failed to report task to relay: {e}");
            }
        }
    }

    /// Fetch pending tasks from the relay.
    pub async fn fetch_pending_tasks(&self) -> Result<Vec<RelayTask>> {
        let url = format!("{}/api/v1/tasks?status=pending", self.base_url);
        let resp = self.client.get(&url).send().await?;
        let data: serde_json::Value = resp.json().await?;
        let tasks: Vec<RelayTask> = serde_json::from_value(data["tasks"].clone())?;
        Ok(tasks)
    }

    /// Send a health heartbeat to the relay.
    pub async fn heartbeat(&self) -> Result<()> {
        let url = format!("{}/health", self.base_url);
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            tracing::warn!("Relay heartbeat failed: {}", resp.status());
        }
        Ok(())
    }
}
