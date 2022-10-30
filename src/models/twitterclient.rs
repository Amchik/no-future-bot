use std::{collections::HashMap, fmt::Display};

use json_structs::*;
use lazy_static::lazy_static;
use regex::Regex;

#[derive(Clone)]
pub struct TwitterClient {
    /// Twitter bearer token
    pub token: String,
}

#[derive(Debug)]
pub enum TwitterError {
    HttpError(reqwest::Error),
    APIError,
}

#[derive(Clone, Debug)]
pub struct TwitterTweet {
    pub id: i64,
    pub author_id: i64,
    pub author_name: String,
    pub author_username: String,
    pub text: String,
    pub media: Vec<TwitterMedia>,
}

#[derive(Clone, Debug)]
pub enum TwitterMedia {
    Photo(String),
    Video(String),
}

pub struct TwitterUser {
    pub id: i64,
    pub name: String,
    pub username: String,
    pub profile_image_url: Option<String>,
}

mod json_structs {
    use serde::Deserialize;

    #[derive(Deserialize)]
    pub struct TwitterUserResponse {
        pub data: TwitterUserResponseUser,
    }
    #[derive(Deserialize)]
    pub struct TwitterUserResponseUser {
        pub id: String,
        pub name: String,
        pub username: String,
        pub profile_image_url: Option<String>,
    }
    #[derive(Deserialize)]
    pub struct TwitterUserTimeline {
        #[serde(default)]
        pub data: Vec<TwitterRawTweet>,
        #[serde(default)]
        pub includes: TwitterTimelineIncludes,
    }
    #[derive(Deserialize)]
    pub struct TwitterRawTweet {
        pub id: String,
        pub text: String,
        pub attachments: Option<TwitterRawTweetAttachments>,
    }
    #[derive(Deserialize)]
    pub struct TwitterRawTweetAttachments {
        #[serde(default)]
        pub media_keys: Vec<String>,
    }
    #[derive(Deserialize, Default)]
    pub struct TwitterTimelineIncludes {
        #[serde(default)]
        pub media: Vec<TwitterTimelineMedia>,
        #[serde(default)]
        pub users: Vec<TwitterTimelineUser>,
    }
    #[derive(Deserialize)]
    pub struct TwitterTimelineUser {
        pub id: String,
        pub name: String,
        pub username: String,
    }
    #[derive(Deserialize)]
    pub struct TwitterTimelineMedia {
        pub media_key: String,
        pub r#type: String,
        pub url: Option<String>,
        pub variants: Option<Vec<TwitterTimelineMediaVariants>>,
    }
    #[derive(Deserialize)]
    pub struct TwitterTimelineMediaVariants {
        pub bitrate: Option<u32>,
        pub url: String,
    }
}

impl Display for TwitterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HttpError(e) => e.fmt(f),
            Self::APIError => write!(f, "API returned error"),
        }
    }
}
impl std::error::Error for TwitterError {}

impl TwitterClient {
    /// Create new instance of twitter client
    pub fn new(token: String) -> Self {
        Self { token }
    }

    /// Fetches user by id
    pub async fn fetch_user(&self, user_id: u64) -> Result<TwitterUser, TwitterError> {
        let client = reqwest::ClientBuilder::new()
            .use_rustls_tls()
            .build()
            .unwrap();
        let res = client
            .get(format!("https://api.twitter.com/2/users/{}", user_id))
            .query(&[("user.fields", "name,username,profile_image_url")])
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await
            .map_err(TwitterError::HttpError)?;

        let text = res.text().await.map_err(TwitterError::HttpError)?;
        let res: TwitterUserResponse = match serde_json::from_str(&text) {
            Ok(r) => r,
            Err(_) => return Err(TwitterError::APIError),
        };
        let user = TwitterUser {
            id: res.data.id.parse().unwrap(),
            name: res.data.name,
            username: res.data.username,
            profile_image_url: res.data.profile_image_url,
        };

        Ok(user)
    }

    /// Fetcher user by username
    pub async fn fetch_user_by_username(
        &self,
        username: &str,
    ) -> Result<TwitterUser, TwitterError> {
        let client = reqwest::ClientBuilder::new()
            .use_rustls_tls()
            .build()
            .unwrap();
        let res = client
            .get(format!(
                "https://api.twitter.com/2/users/by/username/{}",
                username
            ))
            .query(&[("user.fields", "name,username,profile_image_url")])
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await
            .map_err(TwitterError::HttpError)
            .unwrap();

        let text = res.text().await.map_err(TwitterError::HttpError)?;
        let res: TwitterUserResponse = match serde_json::from_str(&text) {
            Ok(r) => r,
            Err(_) => return Err(TwitterError::APIError),
        };
        let user = TwitterUser {
            id: res.data.id.parse().unwrap(),
            name: res.data.name,
            username: res.data.username,
            profile_image_url: res.data.profile_image_url,
        };

        Ok(user)
    }

    /// Fetch `user_id` timeline and remove from it all tweets after `last_id` (if present)
    pub async fn fetch_timeline(
        &self,
        user_id: &str,
        last_id: Option<i64>,
    ) -> Result<Vec<TwitterTweet>, TwitterError> {
        let query = {
            let mut query = HashMap::from([
                ("exclude", "replies,retweets".into()),
                ("tweet.fields", "attachments,author_id".into()),
                ("expansions", "attachments.media_keys,author_id".into()),
                ("media.fields", "type,url,variants".into()),
                ("max_results", "5".into()),
            ]);

            if let Some(last_id) = last_id {
                query.insert("since_id", last_id.to_string());
            }

            query
        };

        let client = reqwest::ClientBuilder::new()
            .use_rustls_tls()
            .build()
            .unwrap();
        let res = client
            .get(format!(
                "https://api.twitter.com/2/users/{}/tweets",
                user_id
            ))
            .query(&query)
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await
            .map_err(TwitterError::HttpError)?;

        let text = res.text().await.map_err(TwitterError::HttpError)?;
        let data: TwitterUserTimeline = match serde_json::from_str(&text) {
            Ok(r) => r,
            Err(_) => return Err(TwitterError::APIError),
        };

        if data.data.is_empty() {
            return Ok(vec![]);
        }

        let last_id = last_id.unwrap_or_default();

        let mut res = vec![];
        let author = data
            .includes
            .users
            .iter()
            .find(|f| f.id == user_id)
            .expect("api doesn't returned author object");

        for tweet in data.data {
            let id = tweet.id.parse().expect("tweet id is not a number");

            if id <= last_id {
                continue;
            }

            let media: Vec<TwitterMedia> = tweet
                .attachments
                .iter()
                .flat_map(|attachments| &attachments.media_keys)
                .map(|f| data.includes.media.iter().position(|r| &r.media_key == f))
                .map(|idx| &data.includes.media[idx.expect("api returned invalid media_key")])
                .map(|m| {
                    if m.r#type == "photo" {
                        TwitterMedia::Photo(m.url.clone().unwrap())
                    } else {
                        TwitterMedia::Video(
                            m.variants
                                .as_ref()
                                .unwrap()
                                .iter()
                                .max_by_key(|v| v.bitrate)
                                .expect("api doesn't returned normal video")
                                .url
                                .clone(),
                        )
                    }
                })
                .collect();

            lazy_static! {
                static ref RE: Regex = Regex::new("https://t\\.co/[^ ]+$").unwrap();
            }

            let text = if media.is_empty() {
                tweet.text
            } else {
                RE.replace(&tweet.text, "").to_string()
            };

            res.push(TwitterTweet {
                id,
                author_id: user_id.parse().unwrap(),
                author_name: author.name.clone(),
                author_username: author.username.clone(),
                text,
                media,
            });
        }

        Ok(res)
    }
}

impl TwitterMedia {
    /// Get type of media
    pub fn media_type(&self) -> entity::post_media::MediaType {
        match self {
            Self::Photo(_) => entity::post_media::MediaType::Photo,
            Self::Video(_) => entity::post_media::MediaType::Video,
        }
    }

    /// Get url to media
    pub fn media_url(self) -> String {
        match self {
            Self::Photo(s) => s,
            Self::Video(s) => s,
        }
    }
}
