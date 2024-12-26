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
    pub image: String,
    pub id: String,
}

#[derive(Debug)]
pub struct FlixHQShow {
    pub title: String,
    pub media_type: MediaType,
    pub image: String,
    pub year: String,
    pub id: String,
    pub seasons: usize,
    pub episodes: usize,
}

#[derive(Clone, Debug)]
pub struct FlixHQResult {
    pub id: String,
    pub title: String,
    pub year: String,
    pub image: String,
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
        let page_html = CLIENT
            .get(&format!("{}/search/{}", BASE_URL, query))
            .send()
            .await?
            .text()
            .await?;

        let ids = self.parse_search(&page_html);

        let urls: Arc<Vec<String>> = Arc::new(
            ids.iter()
                .flatten()
                .map(|id| format!("{}/{}", BASE_URL, id))
                .collect(),
        );

        let bodies = urls
            .iter()
            .enumerate()
            .map(|(index, url)| {
                let client = &CLIENT;
                async move {
                    let resp = client.get(url).send().await;
                    match resp {
                        Ok(response) => {
                            let text = response.text().await;
                            text.map(|body| (index, body))
                                .map_err(|e| format!("Failed to fetch body: {}", e))
                        }
                        Err(e) => Err(format!("Failed to fetch URL: {}", e)),
                    }
                }
            })
            .collect::<FuturesUnordered<_>>();

        let search_results: Arc<Mutex<Vec<FlixHQResult>>> = Arc::new(Mutex::new(Vec::new()));

        bodies
            .for_each(|result| {
                let urls = Arc::clone(&urls);
                let results = Arc::clone(&search_results);
                async move {
                    match result {
                        Ok((index, text)) => {
                            let url = &urls[index];
                            let id = url.splitn(4, '/').collect::<Vec<&str>>()[3];
                            let search_result = self.single_page(text, id);
                            results
                                .lock()
                                .expect("Failed to lock mutex")
                                .push(search_result);
                        }
                        Err(err) => {
                            eprintln!("Error processing URL: {}", err);
                        }
                    }
                }
            })
            .await;

        let search_results = Arc::try_unwrap(search_results)
            .expect("Arc still has multiple owners")
            .into_inner()
            .expect("Failed to acquire lock");

        let mut results: Vec<FlixHQInfo> = vec![];

        for search_result in search_results.iter() {
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

                    let season_ids = self.info_season(season_html);

                    let mut seasons_and_episodes = vec![];

                    for season in season_ids {
                        let episode_html = CLIENT
                            .get(format!("{}/ajax/v2/season/episodes/{}", BASE_URL, &season))
                            .send()
                            .await?
                            .text()
                            .await?;

                        let episodes = self.info_episode(episode_html);
                        seasons_and_episodes.push(episodes);
                    }

                    results.push(FlixHQInfo::Tv(FlixHQShow {
                        episodes: seasons_and_episodes
                            .last()
                            .map(|x| x.len())
                            .expect("Failed to map episodes"),
                        seasons: seasons_and_episodes.len(),
                        id: search_result
                            .id
                            .clone()
                            .split('-')
                            .last()
                            .unwrap_or_default()
                            .to_owned(),
                        title: search_result.title.clone(),
                        year: search_result
                            .year
                            .clone()
                            .split('-')
                            .nth(0)
                            .unwrap_or_default()
                            .to_owned(),
                        image: search_result.image.clone(),
                        media_type: search_result.media_type.clone().unwrap_or(MediaType::Tv),
                    }));
                }

                Some(MediaType::Movie) => {
                    results.push(FlixHQInfo::Movie(FlixHQMovie {
                        id: search_result
                            .id
                            .clone()
                            .split('-')
                            .last()
                            .unwrap_or_default()
                            .to_owned(),
                        title: search_result.title.clone(),
                        image: search_result.image.clone(),
                        year: search_result
                            .year
                            .clone()
                            .split('-')
                            .nth(0)
                            .unwrap_or_default()
                            .to_owned(),
                        media_type: search_result.media_type.clone().unwrap_or(MediaType::Movie),
                    }));
                }
                None => return Err(anyhow!("No results found")),
            }
        }

        Ok(results)
    }
}
