use std::str::FromStr;

use crate::{providers::VideoExtractor, CLIENT, flixhq::flixhq::BASE_URL, utils::decrypt::decrypt_url};
use anyhow::anyhow;
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
    pub server: u32,
}

impl VidCloud {
    pub fn new() -> Self {
        debug!("Initializing VidCloud instance.");
        Self {
            sources: vec![],
            tracks: vec![],
            server: 0,
        }
    }
}


/// Sources Enum for when its being decrypted
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum File {
    EncryptedURL(String),
    DecryptedURL(Vec<Video>),
}

/// Contains the Subtitles for the Sources
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tracks {
    pub file: String,
    pub label: String,
    pub kind: String,
    pub default: Option<bool>,
}

/// Contains the Encrypted Sources
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Sources {
    pub sources: Option<serde_json::Value>,
    pub tracks: Option<Vec<Tracks>>,
    pub encrypted: bool,
    pub server: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Video {
    pub file: String,
    pub r#type: String,
}

impl VideoExtractor for VidCloud {
    async fn extract(&mut self, server_url: &str) -> anyhow::Result<()> {
        // https://cloudvidz.net/embed-1/v2/e-1/AMchghPhubH2?z=
        println!("{:#?}", server_url);
        let url = url::Url::from_str(server_url)?;
        let host = url.host().ok_or(anyhow!("Invalid server url"))?;
        let server_id = url.path_segments().ok_or(anyhow!("asdklfjasdlkf"))?.into_iter().last().ok_or(anyhow!("aksjdfhaskdjf"))?;

        let sources_text = reqwest::Client::new()
            .get(format!("https://{}/embed-1/v2/e-1/getSources?id={}", host, server_id))
            .header("X-Requested-With", "XMLHttpRequest")
            .send()
            .await?
            .text()
            .await?;

        let encrypted_sources: Sources =
            serde_json::from_str(&sources_text).expect("Failed to deserialize json");

        let url = match encrypted_sources.sources {
            Some(serde_json::Value::String(sources)) => File::EncryptedURL(sources),
            Some(serde_json::Value::Array(sources)) => {
                let sources = sources
                    .into_iter()
                    .map(|x| serde_json::from_value::<Video>(x).unwrap())
                    .collect::<Vec<_>>();
                File::DecryptedURL(sources)
            }
            _ => {
                panic!("Please fix this") 
            }
        };

        let sources = match url {
            File::DecryptedURL(decrypted_url) => decrypted_url,
            File::EncryptedURL(encrypted_url) => {
                let key: String = reqwest::Client::new()
                    .get("https://raw.githubusercontent.com/eatmynerds/key/refs/heads/e1/key.txt")
                    .send()
                    .await?
                    .text()
                    .await?;

                let decrypted_str = decrypt_url(&encrypted_url, &key.into_bytes())
                    .expect("Unable to decrypt URL");

                let decrypted: Vec<Video> =
                    serde_json::from_str(&decrypted_str).expect("Failed to deserialize json");

                decrypted
            }
        };

        match serde_json::from_str::<Self>(&sources_text) {
            Ok(sources) => {
                self.sources = sources.sources.into_iter().map(|s| Source {
                    file: s.file,
                    r#type: s.r#type,
                }).collect();
                self.tracks = sources.tracks.into_iter().map(|t| Track {
                    file: t.file,
                    label: t.label,
                    kind: t.kind,
                    default: t.default,
                }).collect();
                self.server = sources.server as u32;
            }
            Err(e) => {
                error!("Failed to parse sources: {}", e);
                return Err(anyhow!("Failed to parse sources"));
            }
        }

        Ok(())
    }
}