use regex::Regex;
use serde::Deserialize;
use std::{collections::HashMap, time::Duration};
use ureq::{serde_json, Agent, AgentBuilder};
use url::{Host, Url};

use crate::utils::log;

#[derive(Deserialize)]
pub struct TraktMovie {
    pub title: String,
    pub year: u16,
    pub ids: TraktIds,
}

#[derive(Deserialize)]
pub struct TraktShow {
    pub title: String,
    pub year: u16,
    pub ids: TraktIds,
}

#[derive(Deserialize)]
pub struct TraktEpisode {
    pub season: u8,
    pub number: u8,
    pub title: String,
    pub ids: TraktIds,
}

#[derive(Deserialize)]
pub struct TraktIds {
    pub trakt: u32,
    pub slug: Option<String>,
    pub tvdb: Option<u32>,
    pub imdb: Option<String>,
    pub tmdb: Option<u32>,
    pub tvrage: Option<u32>,
}

#[derive(Deserialize)]
pub struct TraktWatchingResponse {
    pub expires_at: String,
    pub started_at: String,
    pub action: String,
    pub r#type: String,
    pub movie: Option<TraktMovie>,
    pub show: Option<TraktShow>,
    pub episode: Option<TraktEpisode>,
}

#[derive(Deserialize)]
pub struct TraktRatingsResponse {
    pub rating: f64,
    pub votes: u32,
    pub distribution: HashMap<String, u16>,
}

pub struct Trakt {
    rating_cache: HashMap<String, f64>,
    image_cache: HashMap<String, String>,
    agent: Agent,
    client_id: String,
    username: String,
}

impl Trakt {
    pub fn new(client_id: String, username: String) -> Trakt {
        Trakt {
            rating_cache: HashMap::default(),
            image_cache: HashMap::default(),
            agent: AgentBuilder::new()
                .timeout_read(Duration::from_secs(5))
                .timeout_write(Duration::from_secs(5))
                .build(),
            client_id,
            username,
        }
    }

    pub fn get_watching(&self) -> Option<TraktWatchingResponse> {
        let endpoint = format!("https://api.trakt.tv/users/{}/watching", self.username);

        let response = match self
            .agent
            .get(&endpoint)
            .set("Content-Type", "application/json")
            .set("trakt-api-version", "2")
            .set("trakt-api-key", &self.client_id)
            .call()
        {
            Ok(response) => response,
            Err(_) => return None,
        };

        match response.into_json() {
            Ok(body) => body,
            Err(_) => None,
        }
    }

    pub fn get_show_image(&mut self, imdb_id: String) -> Option<String> {
        match self.image_cache.get(&imdb_id) {
            Some(image_url) => Some(image_url.to_string()),
            None => {
                let endpoint = format!("https://imdb-api.com/en/API/Posters/k_jiv7uc8t/{imdb_id}");

                let response = match self.agent.get(&endpoint).call() {
                    Ok(response) => response,
                    Err(_) => {
                        log("Failed to get image from imdb-api");
                        return None;
                    }
                };

                match response.into_json::<serde_json::Value>() {
                    Ok(body) => {
                        let unredirected_image_url =
                            "https://imdb-api.com/posters/s470/ckLLIsNy3Z0Go1PYHA2PHzVymUA.jpg";

                        println!("HARDCODED: {}", unredirected_image_url);

                        match self.agent.get(&unredirected_image_url.clone()).call() {
                            Ok(response) => {
                                println!("{:?}", response);
                            }
                            Err(response) => {
                                println!("{:?}", response);
                                log("Failed to get image from imdb-api");
                            }
                        };

                        let image_url = body["posters"][0]["link"]
                            .to_string()
                            .replace("original", "s470")
                            .to_owned();

                        println!("FROM REQ: {}", image_url);

                        // let parsed_url = Url::parse(&unredirected_image_url).unwrap();

                        let base = Url::parse("https://imdb-api.com/posters/s470/").unwrap();
                        let re = Regex::new(r"/([^/]+)/?$").unwrap();
                        let mut relative_url = re.captures(&image_url).unwrap()[1].to_string();

                        println!("{:?}", relative_url);
                        relative_url.pop();
                        println!("{:?}", relative_url);

                        let image_url = base.join(&relative_url).unwrap();

                        println!("CLEAN: {:?}", image_url);

                        let image_url = match self.agent.request_url("get", &image_url).call() {
                            Ok(response) => {
                                println!("{:?}", response);
                                response.get_url().to_string()
                            }
                            Err(response) => {
                                println!("{:?}", response);
                                log("Failed to get image from imdb-api");
                                return None;
                            }
                        };

                        self.image_cache.insert(imdb_id, image_url.to_string());
                        Some(image_url)
                    }
                    Err(_) => {
                        log("Show image not correctly found");
                        None
                    }
                }
            }
        }
    }

    pub fn get_movie_rating(&mut self, movie_slug: String) -> f64 {
        match self.rating_cache.get(&movie_slug) {
            Some(rating) => *rating,
            None => {
                let endpoint = format!("https://api.trakt.tv/movies/{movie_slug}/ratings");

                let response = match self
                    .agent
                    .get(&endpoint)
                    .set("Content-Type", "application/json")
                    .set("trakt-api-version", "2")
                    .set("trakt-api-key", &self.client_id)
                    .call()
                {
                    Ok(response) => response,
                    Err(_) => return 0.0,
                };

                match response.into_json::<TraktRatingsResponse>() {
                    Ok(body) => {
                        self.rating_cache
                            .insert(movie_slug.to_string(), body.rating);
                        body.rating
                    }
                    Err(_) => 0.0,
                }
            }
        }
    }
}
