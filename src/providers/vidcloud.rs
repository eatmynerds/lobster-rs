use crate::{providers::VideoExtractor, BASE_URL, CLIENT};
use log::{debug, error};
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
    #[serde(skip_serializing_if = "Option::is_none")]
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
        debug!("Initializing VidCloud instance.");
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
        let request_url = format!(
            "https://testing-embed-decrypt.harc6r.easypanel.host/embed?embed_url={}&referrer={}",
            server_url, BASE_URL
        );

        debug!("Starting extraction process for URL: {}", server_url);
        debug!("Constructed request URL: {}", request_url);

        let response = match CLIENT.get(&request_url).send().await {
            Ok(resp) => {
                debug!("Received response from server.");
                match resp.text().await {
                    Ok(text) => text,
                    Err(e) => {
                        error!("Failed to read response text: {}", e);
                        return Err(e.into());
                    }
                }
            }
            Err(e) => {
                error!("HTTP request failed: {}", e);
                return Err(e.into());
            }
        };

        match serde_json::from_str::<Self>(&response) {
            Ok(sources) => {
                self.sources = sources.sources;
                self.tracks = sources.tracks;
                self.t = sources.t;
                self.server = sources.server;
                debug!("Successfully deserialized response into VidCloud.");
            }
            Err(e) => {
                error!("Failed to deserialize response: {}", e);
                return Err(e.into());
            }
        }

        Ok(())
    }
}
