use super::flixhq::{
    FlixHQ, FlixHQEpisode, FlixHQInfo, FlixHQMovie, FlixHQResult, FlixHQSeason, FlixHQServer,
    FlixHQShow,
};
use crate::{MediaType, BASE_URL};
use log::{debug, warn};
use visdom::types::Elements;
use visdom::Vis;

fn create_html_fragment(html: &str) -> Elements<'_> {
    Vis::load(html).expect("Failed to load HTML")
}

pub(super) trait FlixHQHTML {
    fn parse_recent_shows(&self, html: &str) -> Vec<FlixHQInfo>;
    fn parse_recent_movies(&self, html: &str) -> Vec<FlixHQInfo>;
    fn parse_trending_movies(&self, html: &str) -> Vec<FlixHQInfo>;
    fn parse_trending_shows(&self, html: &str) -> Vec<FlixHQInfo>;
    fn parse_search(&self, html: &str) -> Vec<FlixHQInfo>;
    fn single_page(&self, html: &str, id: &str) -> FlixHQResult;
    fn season_info(&self, html: &str) -> Vec<String>;
    fn episode_info(&self, html: &str) -> Vec<FlixHQEpisode>;
    fn info_server(&self, html: String, media_id: &str) -> Vec<FlixHQServer>;
}

struct PageElement {
    id: String,
    image: String,
    title: String,
    release_date: String,
    episode: String,
}

fn page_elements<'a>(page_parser: &'a Page) -> impl Iterator<Item = PageElement> + use<'a> {
    let ids = page_parser.page_ids();
    let images = page_parser.page_images();
    let titles = page_parser.page_titles();
    let release_dates = page_parser.page_release_dates();
    let episodes = page_parser.page_episodes();

    ids.zip(images)
        .zip(titles)
        .zip(release_dates)
        .zip(episodes)
        .map(
            |((((id, image), title), release_date), episode)| PageElement {
                id,
                image,
                title,
                release_date,
                episode,
            },
        )
}

struct TrendingMovieElement {
    id: String,
    image: String,
    title: String,
    release_date: String,
    duration: String,
}

fn trending_movies<'a>(
    trending_parser: &'a Trending,
) -> impl Iterator<Item = TrendingMovieElement> + use<'a> {
    let ids = trending_parser.trending_movie_ids();
    let images = trending_parser.trending_movie_images();
    let titles = trending_parser.trending_movie_titles();
    let release_dates = trending_parser.trending_movie_release_dates();
    let durations = trending_parser.trending_movie_duration();

    ids.zip(images)
        .zip(titles)
        .zip(release_dates)
        .zip(durations)
        .map(
            |((((id, image), title), release_date), duration)| TrendingMovieElement {
                id,
                image,
                title,
                release_date,
                duration,
            },
        )
}

struct TrendingShowElement {
    id: String,
    image: String,
    title: String,
    season: String,
    episode: String,
}

fn trending_shows<'a>(
    trending_parser: &'a Trending,
) -> impl Iterator<Item = TrendingShowElement> + use<'a> {
    let ids = trending_parser.trending_show_ids();
    let images = trending_parser.trending_show_images();
    let titles = trending_parser.trending_show_titles();
    let seasons = trending_parser.trending_show_seasons();
    let episodes = trending_parser.trending_show_episodes();

    ids.zip(images).zip(titles).zip(seasons).zip(episodes).map(
        |((((id, image), title), season), episode)| TrendingShowElement {
            id,
            image,
            title,
            season,
            episode,
        },
    )
}

struct RecentMovieElement {
    id: String,
    image: String,
    title: String,
    release_date: String,
    duration: String,
}

fn recent_movies<'a>(
    recent_parser: &'a Recent,
) -> impl Iterator<Item = RecentMovieElement> + use<'a> {
    let ids = recent_parser.recent_movie_ids();
    let images = recent_parser.recent_movie_images();
    let titles = recent_parser.recent_movie_titles();
    let release_dates = recent_parser.recent_movie_release_dates();
    let durations = recent_parser.recent_movie_durations();

    ids.zip(images)
        .zip(titles)
        .zip(release_dates)
        .zip(durations)
        .map(
            |((((id, image), title), release_date), duration)| RecentMovieElement {
                id,
                image,
                title,
                release_date,
                duration,
            },
        )
}

struct RecentShowElement {
    id: String,
    image: String,
    title: String,
    season: String,
    episode: String,
}

fn recent_shows<'a>(
    recent_parser: &'a Recent,
) -> impl Iterator<Item = RecentShowElement> + use<'a> {
    let ids = recent_parser.recent_show_ids();
    let titles = recent_parser.recent_show_titles();
    let images = recent_parser.recent_show_images();
    let seasons = recent_parser.recent_show_seasons();
    let episodes = recent_parser.recent_show_episodes();

    ids.zip(images).zip(titles).zip(seasons).zip(episodes).map(
        |((((id, image), title), season), episode)| RecentShowElement {
            id,
            image,
            title,
            season,
            episode,
        },
    )
}

impl FlixHQHTML for FlixHQ {
    fn parse_recent_shows(&self, html: &str) -> Vec<FlixHQInfo> {
        let recent_parser = Recent::new(html);

        let mut results: Vec<FlixHQInfo> = vec![];
        for RecentShowElement {
            id,
            image,
            title,
            season,
            episode,
        } in recent_shows(&recent_parser)
        {
            results.push(FlixHQInfo::Tv(FlixHQShow {
                id,
                title,
                image,
                seasons: FlixHQSeason {
                    total_seasons: season.replace("SS ", "").parse().unwrap_or(0),
                    episodes: vec![],
                },
                episodes: episode.replace("EPS ", "").parse().unwrap_or(0),
                media_type: MediaType::Tv,
            }));
        }

        results
    }

    fn parse_recent_movies(&self, html: &str) -> Vec<FlixHQInfo> {
        let recent_parser = Recent::new(html);

        let mut results: Vec<FlixHQInfo> = vec![];
        for RecentMovieElement {
            id,
            image,
            title,
            release_date,
            duration,
        } in recent_movies(&recent_parser)
        {
            results.push(FlixHQInfo::Movie(FlixHQMovie {
                id,
                title,
                year: release_date,
                image,
                duration,
                media_type: MediaType::Movie,
            }));
        }

        results
    }

    fn parse_trending_movies(&self, html: &str) -> Vec<FlixHQInfo> {
        let trending_parser = Trending::new(html);

        let mut results: Vec<FlixHQInfo> = vec![];
        for TrendingMovieElement {
            id,
            image,
            title,
            release_date,
            duration,
        } in trending_movies(&trending_parser)
        {
            results.push(FlixHQInfo::Movie(FlixHQMovie {
                id,
                title,
                year: release_date,
                image,
                duration,
                media_type: MediaType::Movie,
            }));
        }

        results
    }

    fn parse_trending_shows(&self, html: &str) -> Vec<FlixHQInfo> {
        let trending_parser = Trending::new(html);

        let mut results: Vec<FlixHQInfo> = vec![];
        for TrendingShowElement {
            id,
            image,
            title,
            season,
            episode,
        } in trending_shows(&trending_parser)
        {
            results.push(FlixHQInfo::Tv(FlixHQShow {
                id,
                title,
                image,
                seasons: FlixHQSeason {
                    total_seasons: season.replace("SS ", "").parse().unwrap_or(0),
                    episodes: vec![],
                },
                episodes: episode.replace("EPS ", "").parse().unwrap_or(0),
                media_type: MediaType::Tv,
            }));
        }

        results
    }

    fn parse_search(&self, html: &str) -> Vec<FlixHQInfo> {
        debug!("Parsing search results from HTML.");
        let page_parser = Page::new(html);

        let mut results: Vec<FlixHQInfo> = vec![];
        for PageElement {
            id,
            image,
            title,
            release_date,
            episode,
        } in page_elements(&page_parser)
        {
            debug!("Processing media item: ID = {}, Title = {}", id, title);
            let media_type = page_parser.media_type(&id);

            match media_type {
                Some(MediaType::Tv) => {
                    debug!("Identified as TV show.");
                    results.push(FlixHQInfo::Tv(FlixHQShow {
                        id,
                        title,
                        image,
                        seasons: FlixHQSeason {
                            total_seasons: release_date.replace("SS ", "").parse().unwrap_or(0),
                            episodes: vec![],
                        },
                        episodes: episode.replace("EPS ", "").parse().unwrap_or(0),
                        media_type: MediaType::Tv,
                    }));
                }
                Some(MediaType::Movie) => {
                    debug!("Identified as Movie.");
                    results.push(FlixHQInfo::Movie(FlixHQMovie {
                        id,
                        title,
                        year: release_date,
                        image,
                        duration: episode,
                        media_type: MediaType::Movie,
                    }));
                }
                None => warn!("Unknown media type for ID = {}", id),
            }
        }

        debug!("Parsed {} results.", results.len());
        results
    }

    fn single_page(&self, html: &str, id: &str) -> FlixHQResult {
        debug!("Parsing single page for ID = {}", id);
        let elements = create_html_fragment(html);
        let search_parser = Search::new(&elements);
        let info_parser = Info::new(&elements);

        let result = FlixHQResult {
            title: search_parser.title(),
            image: search_parser.image(),
            year: info_parser.label(3, "Released:").join(""),
            duration: info_parser.duration(),
            media_type: Some(MediaType::Tv),
            id: id.to_string(),
        };

        debug!("Parsed single page result: {:?}", result);
        result
    }

    fn season_info(&self, html: &str) -> Vec<String> {
        debug!("Extracting season information.");
        let season_parser = Season::new(html);

        let seasons: Vec<String> = season_parser
            .season_results()
            .into_iter()
            .flatten()
            .collect();

        debug!("Extracted {} seasons.", seasons.len());
        seasons
    }

    fn episode_info(&self, html: &str) -> Vec<FlixHQEpisode> {
        debug!("Extracting episode information.");
        let episode_parser = Episode::new(html);

        let episodes = episode_parser.episode_results();
        debug!("Extracted {} episodes.", episodes.len());
        episodes
    }

    fn info_server(&self, html: String, media_id: &str) -> Vec<FlixHQServer> {
        debug!("Extracting server information for media ID = {}", media_id);
        let server_parser = Server::new(&html);
        let servers = server_parser.parse_server_html(media_id);

        debug!("Extracted {} servers.", servers.len());
        servers
    }
}

struct Page<'a> {
    elements: Elements<'a>,
}

impl<'a> Page<'a> {
    fn new(html: &'a str) -> Self {
        let elements = create_html_fragment(html);
        Self { elements }
    }

    fn page_ids(&self) -> impl Iterator<Item = String> + use<'a> {
        self.elements
            .find("div.film-poster > a")
            .into_iter()
            .filter_map(|element| {
                element
                    .get_attribute("href")
                    .and_then(|href| href.to_string().strip_prefix('/').map(String::from))
            })
    }

    fn page_images(&self) -> impl Iterator<Item = String> + use<'a> {
        self.elements
            .find("div.film-poster > img")
            .into_iter()
            .filter_map(|element| {
                element
                    .get_attribute("data-src")
                    .map(|value| value.to_string())
            })
    }

    fn page_titles(&self) -> impl Iterator<Item = String> + use<'a> {
        self.elements
            .find("div.film-detail > h2.film-name > a")
            .into_iter()
            .filter_map(|element| {
                element
                    .get_attribute("title")
                    .map(|value| value.to_string())
            })
    }

    fn page_release_dates(&self) -> impl Iterator<Item = String> + use<'a> {
        self.elements
            .find("div.fd-infor > span:nth-child(1)")
            .into_iter()
            .map(|element| element.text())
    }

    fn page_episodes(&self) -> impl Iterator<Item = String> + use<'a> {
        self.elements
            .find("div.fd-infor > span:nth-child(3)")
            .into_iter()
            .map(|element| element.text())
    }

    fn media_type(&self, id: &str) -> Option<MediaType> {
        match id.split('/').next() {
            Some("tv") => Some(MediaType::Tv),
            Some("movie") => Some(MediaType::Movie),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
struct Search<'b> {
    elements: &'b Elements<'b>,
}

impl<'b> Search<'b> {
    fn new(elements: &'b Elements<'b>) -> Self {
        Self { elements }
    }

    fn image(&self) -> String {
        let image_attr = self
            .elements
            .find("div.m_i-d-poster > div > img")
            .attr("src");

        if let Some(image) = image_attr {
            return image.to_string();
        };

        String::new()
    }

    fn title(&self) -> String {
        self.elements
        .find(
            "#main-wrapper > div.movie_information > div > div.m_i-detail > div.m_i-d-content > h2",
        )
        .text()
        .trim()
        .to_owned()
    }
}

/// Remy clarke was here & some red guy
#[derive(Clone, Copy)]
struct Info<'b> {
    elements: &'b Elements<'b>,
}

impl<'b> Info<'b> {
    fn new(elements: &'b Elements<'b>) -> Self {
        Self { elements }
    }

    fn label(&self, index: usize, label: &str) -> Vec<String> {
        self.elements
            .find(&format!(
                "div.m_i-d-content > div.elements > div:nth-child({})",
                index
            ))
            .text()
            .replace(label, "")
            .split(',')
            .map(|s| s.trim().to_owned())
            .filter(|x| !x.is_empty())
            .collect()
    }

    pub fn duration(&self) -> String {
        self.elements
            .find("span.item:nth-child(3)")
            .text()
            .trim()
            .to_owned()
    }
}

struct Season<'a> {
    elements: Elements<'a>,
}

impl<'a> Season<'a> {
    fn new(html: &'a str) -> Self {
        let elements = create_html_fragment(html);
        Self { elements }
    }

    fn season_results(&self) -> Vec<Option<String>> {
        self.elements.find(".dropdown-menu > a").map(|_, element| {
            element
                .get_attribute("data-id")
                .map(|value| value.to_string())
        })
    }
}

struct Episode<'a> {
    elements: Elements<'a>,
}

impl<'a> Episode<'a> {
    fn new(html: &'a str) -> Self {
        let elements = create_html_fragment(html);
        Self { elements }
    }

    fn episode_title(&self) -> Vec<Option<String>> {
        self.elements.find("ul > li > a").map(|_, element| {
            element
                .get_attribute("title")
                .map(|value| value.to_string())
        })
    }

    fn episode_id(&self) -> Vec<Option<String>> {
        self.elements.find("ul > li > a").map(|_, element| {
            element
                .get_attribute("data-id")
                .map(|value| value.to_string())
        })
    }

    fn episode_results(&self) -> Vec<FlixHQEpisode> {
        let episode_titles = self.episode_title();
        let episode_ids = self.episode_id();

        let mut episodes: Vec<FlixHQEpisode> = vec![];

        for (id, title) in episode_ids.iter().zip(episode_titles.iter()) {
            if let Some(id) = id {
                episodes.push(FlixHQEpisode {
                    id: id.to_string(),
                    title: title.as_deref().unwrap_or("").to_string(),
                });
            }
        }

        episodes
    }
}

struct Server<'a> {
    elements: Elements<'a>,
}

impl<'a> Server<'a> {
    fn new(html: &'a str) -> Self {
        let elements = create_html_fragment(html);
        Self { elements }
    }

    fn parse_server_html(&self, media_id: &str) -> Vec<FlixHQServer> {
        self.elements.find("ul > li > a").map(|_, element| {
            let id = element
                .get_attribute("id")
                .map(|value| value.to_string().replace("watch-", ""))
                .unwrap_or(String::from(""));

            let name = element
                .get_attribute("title")
                .map(|value| value.to_string().trim_start_matches("Server ").to_owned());

            let url = format!("{}/watch-{}.{}", BASE_URL, media_id, id);
            let name = name.unwrap_or(String::from(""));

            FlixHQServer { name, url }
        })
    }
}

struct Recent<'a> {
    elements: Elements<'a>,
}

impl<'a> Recent<'a> {
    fn new(html: &'a str) -> Self {
        let elements = create_html_fragment(html);
        Self { elements }
    }
    fn recent_movie_ids(&self) -> impl Iterator<Item = String> + use<'a> {
        self.elements
        .find("#main-wrapper > div > section:nth-child(6) > div.block_area-content.block_area-list.film_list.film_list-grid > div > div.flw-item > div.film-poster > a")
        .into_iter()
        .filter_map(|element| {
            element
                .get_attribute("href")
                .and_then(|href| href.to_string().strip_prefix('/').map(String::from))
        })
    }

    fn recent_movie_images(&self) -> impl Iterator<Item = String> + use<'a> {
        self.elements
        .find("#main-wrapper > div > section:nth-child(6) > div.block_area-content.block_area-list.film_list.film_list-grid > div > div.flw-item > div.film-poster > img")
        .into_iter()
        .filter_map(|element| {
            element
                .get_attribute("data-src")
                .map(|value| value.to_string())
        })
    }

    fn recent_movie_titles(&self) -> impl Iterator<Item = String> + use<'a> {
        self.elements
        .find("#main-wrapper > div > section:nth-child(6) > div.block_area-content.block_area-list.film_list.film_list-grid > div > div.flw-item > div.film-detail > h3.film-name > a")
        .into_iter()
        .filter_map(|element| {
            element
                .get_attribute("title")
                .map(|value| value.to_string())
        })
    }

    fn recent_movie_release_dates(&self) -> impl Iterator<Item = String> + use<'a> {
        self.elements
        .find("#main-wrapper > div > section:nth-child(6) > div.block_area-content.block_area-list.film_list.film_list-grid > div > div.flw-item > div.film-detail > div.fd-infor > span:nth-child(1)")
        .into_iter()
        .map(|value| value.text())
    }

    fn recent_movie_durations(&self) -> impl Iterator<Item = String> + use<'a> {
        self.elements
        .find("#main-wrapper > div > section:nth-child(6) > div.block_area-content.block_area-list.film_list.film_list-grid > div > div.flw-item > div.film-detail > div.fd-infor > span:nth-child(3)")
        .into_iter()
        .map(|value| value.text())
    }

    fn recent_show_ids(&self) -> impl Iterator<Item = String> + use<'a> {
        self.elements
        .find("#main-wrapper > div > section:nth-child(7) > div.block_area-content.block_area-list.film_list.film_list-grid > div > div.flw-item > div.film-poster > a")
        .into_iter()
        .filter_map(|element| {
            element
                .get_attribute("href")
                .and_then(|href| href.to_string().strip_prefix('/').map(String::from))
        })
    }

    fn recent_show_titles(&self) -> impl Iterator<Item = String> + use<'a> {
        self.elements
        .find("#main-wrapper > div > section:nth-child(7) > div.block_area-content.block_area-list.film_list.film_list-grid > div > div.flw-item > div.film-detail > h3.film-name > a")
        .into_iter()
        .filter_map(|element| {
            element
                .get_attribute("title")
                .map(|value| value.to_string())
        })
    }

    fn recent_show_images(&self) -> impl Iterator<Item = String> + use<'a> {
        self.elements
        .find("#main-wrapper > div > section:nth-child(7) > div.block_area-content.block_area-list.film_list.film_list-grid > div > div.flw-item > div.film-poster > img")
        .into_iter()
        .filter_map(|element| {
            element
                .get_attribute("data-src")
                .map(|value| value.to_string())
        })
    }

    fn recent_show_episodes(&self) -> impl Iterator<Item = String> + use<'a> {
        self.elements
        .find("#main-wrapper > div > section:nth-child(7) > div.block_area-content.block_area-list.film_list.film_list-grid > div > div.flw-item > div.film-detail > div.fd-infor > span:nth-child(3)")
        .into_iter()
        .map(|value| value.text())
    }

    fn recent_show_seasons(&self) -> impl Iterator<Item = String> + use<'a> {
        self.elements
        .find("#main-wrapper > div > section:nth-child(7) > div.block_area-content.block_area-list.film_list.film_list-grid > div > div.flw-item > div.film-detail > div.fd-infor > span:nth-child(1)")
        .into_iter()
        .map(|value| value.text())
    }
}

struct Trending<'a> {
    elements: Elements<'a>,
}

impl<'a> Trending<'a> {
    fn new(html: &'a str) -> Self {
        let elements = create_html_fragment(html);
        Self { elements }
    }
    fn trending_movie_ids(&self) -> impl Iterator<Item = String> + use<'a> {
        self.elements
            .find("div#trending-movies div.film_list-wrap div.flw-item div.film-poster a")
            .into_iter()
            .filter_map(|element| {
                element
                    .get_attribute("href")
                    .and_then(|href| href.to_string().strip_prefix('/').map(String::from))
            })
    }

    fn trending_movie_images(&self) -> impl Iterator<Item = String> + use<'a> {
        self.elements
            .find("div#trending-movies div.film_list-wrap div.flw-item div.film-poster > img")
            .into_iter()
            .filter_map(|element| {
                element
                    .get_attribute("data-src")
                    .map(|value| value.to_string())
            })
    }

    fn trending_movie_release_dates(&self) -> impl Iterator<Item = String> + use<'a> {
        self.elements
            .find("div#trending-movies div.film_list-wrap div.flw-item > div.film-detail > div.fd-infor > span:nth-child(1)")
            .into_iter()
            .map(|value| value.text())
    }

    fn trending_movie_titles(&self) -> impl Iterator<Item = String> + use<'a> {
        self.elements
            .find("div#trending-movies div.film_list-wrap div.flw-item > div.film-detail > h3.film-name > a")
            .into_iter()
            .filter_map(|element| {
                element
                    .get_attribute("title")
                    .map(|value| value.to_string())
            })
    }

    fn trending_movie_duration(&self) -> impl Iterator<Item = String> + use<'a> {
        self.elements
            .find("div#trending-movies div.film_list-wrap div.flw-item > div.film-detail > div.fd-infor > span:nth-child(3)")
            .into_iter()
            .map(|value| value.text())
    }

    fn trending_show_ids(&self) -> impl Iterator<Item = String> + use<'a> {
        self.elements
            .find("div#trending-tv div.film_list-wrap div.flw-item div.film-poster a")
            .into_iter()
            .filter_map(|element| {
                element
                    .get_attribute("href")
                    .and_then(|href| href.to_string().strip_prefix('/').map(String::from))
            })
    }

    fn trending_show_images(&self) -> impl Iterator<Item = String> + use<'a> {
        self.elements
            .find("div#trending-tv div.film_list-wrap div.flw-item div.film-poster > img")
            .into_iter()
            .filter_map(|element| {
                element
                    .get_attribute("data-src")
                    .map(|value| value.to_string())
            })
    }

    fn trending_show_seasons(&self) -> impl Iterator<Item = String> + use<'a> {
        self.elements
            .find("div#trending-tv div.film_list-wrap div.flw-item > div.film-detail > div.fd-infor > span:nth-child(1)")
            .into_iter()
            .map(|value| value.text())
    }

    fn trending_show_titles(&self) -> impl Iterator<Item = String> + use<'a> {
        self.elements
            .find("div#trending-tv div.film_list-wrap div.flw-item > div.film-detail > h3.film-name > a")
            .into_iter()
            .filter_map(|element| {
                element
                    .get_attribute("title")
                    .map(|value| value.to_string())
            })
    }

    fn trending_show_episodes(&self) -> impl Iterator<Item = String> + use<'a> {
        self.elements
            .find("div#trending-tv div.film_list-wrap div.flw-item > div.film-detail > div.fd-infor > span:nth-child(3)")
            .into_iter()
            .map(|value| value.text())
    }
}
