use super::{
    search::{FlixHQEpisode, FlixHQResult},
    FlixHQ,
};
use crate::{MediaType, BASE_URL};
use visdom::types::Elements;
use visdom::Vis;

fn create_html_fragment(html: &str) -> Elements<'_> {
    Vis::load(html).expect("Failed to load HTML")
}

pub(super) trait FlixHQHTML {
    fn parse_search(&self, html: &str) -> Vec<Option<String>>;
    fn single_page(&self, media_html: String, id: &str) -> FlixHQResult;
    fn info_season(&self, season_html: String) -> Vec<String>;
    fn info_episode(&self, episode_html: String) -> Vec<FlixHQEpisode>;
}

impl FlixHQHTML for FlixHQ {
    fn parse_search(&self, html: &str) -> Vec<Option<String>> {
        let page_parser = Page::new(html);

        page_parser.page_ids()
    }

    fn single_page(&self, media_html: String, id: &str) -> FlixHQResult {
        let elements = create_html_fragment(&media_html);

        let search_parser = Search::new(&elements, id);

        let info_parser = Info::new(&elements);

        FlixHQResult {
            title: search_parser.title(),
            image: search_parser.image(),

            year: info_parser.label(3, "Released:").join(""),
            media_type: search_parser.media_type(),
            id: id.to_string(),
        }
    }

    fn info_season(&self, season_html: String) -> Vec<String> {
        let elements = create_html_fragment(&season_html);

        let season_parser = Seasons::new(elements);

        season_parser
            .season_results()
            .into_iter()
            .flatten()
            .collect()
    }

    fn info_episode(&self, episode_html: String) -> Vec<FlixHQEpisode> {
        let elements = create_html_fragment(&episode_html);

        let episode_parser = Episodes::new(elements);

        episode_parser.episode_results()
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

    fn page_ids(&self) -> Vec<Option<String>> {
        self.elements.find("div.film-poster > a").map(|_, element| {
            element
                .get_attribute("href")?
                .to_string()
                .strip_prefix('/')
                .map(String::from)
        })
    }
}

#[derive(Clone, Copy)]
struct Search<'page, 'b> {
    elements: &'b Elements<'page>,
    id: &'b str,
}

impl<'page, 'b> Search<'page, 'b> {
    fn new(elements: &'b Elements<'page>, id: &'b str) -> Self {
        Self { elements, id }
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

    fn media_type(&self) -> Option<MediaType> {
        match self.id.split('/').next() {
            Some("tv") => Some(MediaType::Tv),
            Some("movie") => Some(MediaType::Movie),
            _ => None,
        }
    }
}

/// Remy clarke was here & some red guy
#[derive(Clone, Copy)]
struct Info<'page, 'b> {
    elements: &'b Elements<'page>,
}

impl<'page, 'b> Info<'page, 'b> {
    fn new(elements: &'b Elements<'page>) -> Self {
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
}

struct Seasons<'a> {
    elements: Elements<'a>,
}

impl<'a> Seasons<'a> {
    fn new(elements: Elements<'a>) -> Self {
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

struct Episodes<'a> {
    elements: Elements<'a>,
}

impl<'a> Episodes<'a> {
    fn new(elements: Elements<'a>) -> Self {
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
                let url = format!("{}/ajax/v2/episode/servers/{}", BASE_URL, id);
                episodes.push(FlixHQEpisode {
                    id: id.to_string(),
                    title: title.as_deref().unwrap_or("").to_string(),
                    url,
                });
            }
        }

        episodes
    }
}
