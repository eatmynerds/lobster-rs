use crate::{
    flixhq::html::FlixHQHTML,
    providers::{
        vidcloud::{Source, Track, VidCloud},
        VideoExtractor,
    },
    MediaType, Provider, BASE_URL, CLIENT,
};
use anyhow::anyhow;
use serde::Deserialize;
use tracing::{debug, error};

#[derive(Debug)]
pub enum FlixHQInfo {
    Tv(FlixHQShow),
    Movie(FlixHQMovie),
}

#[derive(Debug)]
pub struct FlixHQMovie {
    pub title: String,
    pub year: String,
    pub media_type: MediaType,
    pub duration: String,
    pub image: String,
    pub id: String,
}

#[derive(Debug)]
pub struct FlixHQShow {
    pub title: String,
    pub media_type: MediaType,
    pub image: String,
    pub id: String,
    pub seasons: FlixHQSeason,
    pub episodes: usize,
}

#[derive(Debug)]
pub struct FlixHQSeason {
    pub total_seasons: usize,
    pub episodes: Vec<Vec<FlixHQEpisode>>,
}

#[derive(Debug)]
pub struct FlixHQResult {
    pub id: String,
    pub title: String,
    pub year: String,
    pub image: String,
    pub duration: String,
    pub media_type: Option<MediaType>,
}

#[derive(Debug)]
pub struct FlixHQEpisode {
    pub id: String,
    pub title: String,
}

#[derive(Debug)]
pub struct FlixHQServers {
    pub servers: Vec<FlixHQServer>,
}

#[derive(Debug)]
pub struct FlixHQServer {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct FlixHQServerInfo {
    link: String,
}

#[derive(Debug)]
pub struct FlixHQSources {
    pub subtitles: FlixHQSubtitles,
    pub sources: FlixHQSourceType,
}

#[derive(Debug)]
pub enum FlixHQSourceType {
    VidCloud(Vec<Source>),
}

#[derive(Debug)]
pub enum FlixHQSubtitles {
    VidCloud(Vec<Track>),
}

pub struct FlixHQ;

impl FlixHQ {
    pub async fn search(&self, query: &str) -> anyhow::Result<Vec<FlixHQInfo>> {
        debug!("Starting search for query: {}", query);
        let parsed_query = query.replace(" ", "-");

        debug!("Formatted query: {}", parsed_query);

        let page_html = CLIENT
            .get(&format!("{}/search/{}", BASE_URL, parsed_query))
            .send()
            .await?
            .text()
            .await?;

        debug!("Received HTML for search results");
        let results = self.parse_search(&page_html);

        debug!("Search completed with {} results", results.len());
        Ok(results)
    }

    pub async fn info(&self, media_id: &str) -> anyhow::Result<FlixHQInfo> {
        debug!("Fetching info for media_id: {}", media_id);
        let info_html = CLIENT
            .get(&format!("{}/{}", BASE_URL, media_id))
            .send()
            .await?
            .text()
            .await?;

        debug!("Received HTML for media info");
        let search_result = self.single_page(&info_html, media_id);

        match &search_result.media_type {
            Some(MediaType::Tv) => {
                debug!("Media type is Tv. Processing seasons and episodes");
                let id = search_result
                    .id
                    .split('-')
                    .last()
                    .unwrap_or_default()
                    .to_owned();

                let season_html = CLIENT
                    .get(format!("{}/ajax/v2/tv/seasons/{}", BASE_URL, id))
                    .send()
                    .await?
                    .text()
                    .await?;

                let season_ids = self.season_info(&season_html);

                let mut seasons_and_episodes = vec![];
                for season in &season_ids {
                    let episode_html = CLIENT
                        .get(format!("{}/ajax/v2/season/episodes/{}", BASE_URL, &season))
                        .send()
                        .await?
                        .text()
                        .await?;

                    let episodes = self.episode_info(&episode_html);
                    seasons_and_episodes.push(episodes);
                }

                debug!(
                    "Fetched {} seasons with {} episodes",
                    season_ids.len(),
                    seasons_and_episodes.last().map(|x| x.len()).unwrap_or(0)
                );

                return Ok(FlixHQInfo::Tv(FlixHQShow {
                    episodes: seasons_and_episodes.last().map(|x| x.len()).unwrap_or(0),
                    seasons: FlixHQSeason {
                        total_seasons: season_ids.len(),
                        episodes: seasons_and_episodes,
                    },
                    id: search_result
                        .id
                        .split('-')
                        .last()
                        .unwrap_or_default()
                        .to_owned(),
                    title: search_result.title,
                    image: search_result.image,
                    media_type: MediaType::Tv,
                }));
            }

            Some(MediaType::Movie) => {
                debug!("Media type is Movie");
                return Ok(FlixHQInfo::Movie(FlixHQMovie {
                    id: search_result
                        .id
                        .split('-')
                        .last()
                        .unwrap_or_default()
                        .to_owned(),
                    title: search_result.title,
                    image: search_result.image,
                    year: search_result
                        .year
                        .split('-')
                        .nth(0)
                        .unwrap_or_default()
                        .to_owned(),
                    duration: search_result.duration,
                    media_type: MediaType::Movie,
                }));
            }
            None => {
                error!("No results found for media_id: {}", media_id);
                return Err(anyhow!("No results found"));
            }
        }
    }

    pub async fn servers(&self, episode_id: &str, media_id: &str) -> anyhow::Result<FlixHQServers> {
        debug!(
            "Fetching servers for episode_id: {} and media_id: {}",
            episode_id, media_id
        );
        let episode_id = format!(
            "{}/ajax/{}",
            BASE_URL,
            if !episode_id.starts_with(&format!("{}/ajax", BASE_URL)) && !media_id.contains("movie")
            {
                format!("v2/episode/servers/{}", episode_id)
            } else {
                format!("movie/episodes/{}", episode_id)
            }
        );

        let server_html = CLIENT.get(episode_id).send().await?.text().await?;

        debug!("Received HTML for servers");
        let servers = self.info_server(server_html, media_id);

        debug!("Found {} servers", servers.len());
        Ok(FlixHQServers { servers })
    }

    pub async fn sources(
        &self,
        episode_id: &str,
        media_id: &str,
        server: Provider,
    ) -> anyhow::Result<FlixHQSources> {
        debug!(
            "Fetching sources for episode_id: {}, media_id: {}, server: {}",
            episode_id, media_id, server
        );
        let servers = self.servers(episode_id, media_id).await?;

        let i = match servers
            .servers
            .iter()
            .position(|s| s.name == server.to_string())
        {
            Some(index) => index,
            None => {
                error!("Server {} not found!", server);
                std::process::exit(1);
            }
        };

        let parts = &servers.servers[i].url;

        debug!("Selected server URL: {}", parts);
        let server_id: &str = parts
            .split('.')
            .collect::<Vec<_>>()
            .last()
            .copied()
            .unwrap_or_default();

        let server_json = CLIENT
            .get(format!("{}/ajax/episode/sources/{}", BASE_URL, server_id))
            .send()
            .await?
            .text()
            .await?;

        let server_info: FlixHQServerInfo = serde_json::from_str(&server_json)?;

        match server {
            Provider::Vidcloud | Provider::Upcloud => {
                debug!("Processing VidCloud or UpCloud sources");
                let mut vidcloud = VidCloud::new();
                vidcloud.extract(&server_info.link).await?;

                debug!("Sources and subtitles extracted successfully");
                return Ok(FlixHQSources {
                    sources: FlixHQSourceType::VidCloud(vidcloud.sources),
                    subtitles: FlixHQSubtitles::VidCloud(vidcloud.tracks),
                });
            }
        }
    }
}
