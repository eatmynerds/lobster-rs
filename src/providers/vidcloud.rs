use crate::{providers::VideoExtractor, BASE_URL, CLIENT};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Source {
    pub file: String,
    pub r#type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Track {
    pub file: String,
    pub label: String,
    pub kind: String,
    pub default: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VidCloud {
    pub sources: Vec<Source>,
    pub tracks: Vec<Track>,
    pub t: u32,
    pub server: u32,
}

impl VidCloud {
    pub fn new() -> Self {
        Self {
            sources: vec![],
            tracks: vec![],
            t: 0,
            server: 0,
        }
    }
}

impl VideoExtractor for VidCloud {
    async fn extract(&mut self, server_url: &str) -> anyhow::Result<()> {
        let response = CLIENT
            .get(format!(
            "https://testing-embed-decrypt.harc6r.easypanel.host/embed?embed_url={}&referrer={}",
            server_url, BASE_URL
        ))
            .send()
            .await?
            .text()
            .await?;

        let sources: Self = serde_json::from_str(&response).expect("Failed to serialize sources!");

        self.sources = sources.sources;
        self.tracks = sources.tracks;
        self.t = sources.t;
        self.server = sources.server;

        Ok(())
    }
}
