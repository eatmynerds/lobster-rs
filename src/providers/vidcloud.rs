use crate::{providers::VideoExtractor, CLIENT};
use log::{debug, error};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Source {
    pub file: String,
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
}

impl VidCloud {
    pub fn new() -> Self {
        debug!("Initializing VidCloud instance.");
        Self {
            sources: vec![],
            tracks: vec![],
        }
    }
}

impl VideoExtractor for VidCloud {
    async fn extract(&mut self, server_url: &str) -> anyhow::Result<()> {
        let request_url = format!("https://dec.eatmynerds.live?url={}", server_url);

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
