use super::{html::FlixHQHTML, FlixHQ};
use crate::{MediaType, BASE_URL, CLIENT};
use anyhow::anyhow;
use futures::{stream::FuturesUnordered, StreamExt};
use std::sync::{Arc, Mutex};

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
    pub seasons: usize,
    pub episodes: usize,
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
    pub url: String,
}

impl FlixHQ {
    pub async fn search(&self, query: &str) -> anyhow::Result<Vec<FlixHQInfo>> {
        let parsed_query = query.replace(" ", "-");

        let page_html = CLIENT
            .get(&format!("{}/search/{}", BASE_URL, parsed_query))
            .send()
            .await?
            .text()
            .await?;

        let results = self.parse_search(&page_html);

        Ok(results)
    }

    pub async fn info(&self, media_id: &str) -> anyhow::Result<FlixHQInfo> {
        let info_html = CLIENT
            .get(&format!("{}/{}", BASE_URL, media_id))
            .send()
            .await?
            .text()
            .await?;

        let search_result = self.single_page(&info_html, media_id);

        match &search_result.media_type {
            Some(MediaType::Tv) => {
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

                for season in season_ids {
                    let episode_html = CLIENT
                        .get(format!("{}/ajax/v2/season/episodes/{}", BASE_URL, &season))
                        .send()
                        .await?
                        .text()
                        .await?;

                    let episodes = self.episode_info(&episode_html);
                    seasons_and_episodes.push(episodes);
                }

                return Ok(FlixHQInfo::Tv(FlixHQShow {
                    episodes: seasons_and_episodes
                        .last()
                        .map(|x| x.len())
                        .expect("Failed to map episodes"),
                    seasons: seasons_and_episodes.len(),
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
            None => return Err(anyhow!("No results found")),
        }
    }
}
