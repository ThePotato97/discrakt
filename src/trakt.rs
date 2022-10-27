extern crate tmdb;

use serde::Deserialize;
use std::{collections::HashMap, time::Duration};
use ureq::{Agent, AgentBuilder};

use tmdb::model::*;
use tmdb::themoviedb::*;

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
    pub season: Option<u32>,
    pub number: Option<u32>,
    pub title: String,
    pub ids: TraktIds,
}

#[derive(Deserialize)]
pub struct TraktIds {
    pub trakt: u32,
    pub slug: Option<String>,
    pub tvdb: Option<u32>,
    pub imdb: Option<String>,
    pub tmdb: Option<u64>,
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
    tmdb_cache: HashMap<String, String>,
    agent: Agent,
    client_id: String,
    username: String,
}

impl Trakt {
    pub fn new(client_id: String, username: String) -> Trakt {
        Trakt {
            rating_cache: HashMap::default(),
            tmdb_cache: HashMap::default(),
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
    pub fn get_show_image_tmdb(&mut self, tmdb_id: u64, media: &str, season: Option<u32>) -> Option<String> {
        let formatted_name = if season.is_some() { format!("{}_{}", tmdb_id, season.unwrap()) } else { format!("{}", tmdb_id)};

        let cached_image = self.tmdb_cache.get(&formatted_name);
        let show_image = if cached_image.is_some() {
            cached_image.cloned()
        } else {
            let tmdb = TMDb {
                api_key: "caa20889fc40c9d85fe7ddfbca8166e7",
                language: "en",
            };

            let movie: Movie;
            let shows: TV;

            let image_url = match media {
                "movie" => {
                    movie = tmdb.fetch().id(tmdb_id).execute().unwrap();
                    movie.backdrop_path
                }
                "shows" => {
                    shows = tmdb.fetch().id(tmdb_id).execute().unwrap();
                    let season_image = shows.seasons[season.unwrap() as usize].poster_path.clone();
                    if season_image.is_some() {
                        season_image
                    } else {
                        shows.backdrop_path
                    }
                }
                &_ => todo!(),
            };
            if let Some(image_url) = &image_url {
                self.tmdb_cache.insert(formatted_name, image_url.to_string());
            }
            image_url
        };
        Some(format!("https://image.tmdb.org/t/p/w470_and_h470_face{}", show_image.as_ref().unwrap()))
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
