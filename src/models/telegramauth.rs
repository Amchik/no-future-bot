use std::{
    collections::HashMap,
    fmt::Write,
    string::FromUtf8Error,
    time::{SystemTime, UNIX_EPOCH},
};

use hmac::{Hmac, Mac};
use itertools::Itertools;
use rocket::{
    http::Status,
    request::{FromRequest, Outcome},
    Request, State,
};
use serde::Deserialize;
use sha2::Sha256;
use telegrambot::teloxide::Bot;
use urlencoding::decode;

#[derive(Debug)]
/// Raw telegram authorization fields, can be parsed using
/// `TelegramAuth::authorize()` or `TelegramAuth::authorize_with_time()`.
pub struct TelegramAuth(pub HashMap<String, String>);

#[derive(Deserialize)]
/// Telegram user object, that should be parsed from `init_data`.
/// In telegram documentation, it called [`WebAppUser`](https://core.telegram.org/bots/webapps#webappuser).
pub struct TelegramUser {
    /// A unique identifier for the user or bot.
    pub id: i64,
    /// First name of the user or bot.
    pub first_name: String,
    /// Last name of the user or bot.
    pub last_name: Option<String>,
    /// Username of the user or bot.
    pub username: Option<String>,
    /// [IETF language](https://en.wikipedia.org/wiki/IETF_language_tag) tag of the user's language.
    pub language_code: Option<String>,
    /// True, if this user is a Telegram Premium user
    pub is_premium: Option<bool>,
    /// URL of the user’s profile photo. The photo can be in .jpeg or .svg formats.
    /// Only returned for Web Apps launched from the attachment menu.
    pub photo_url: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TelegramAuthError {
    /// Invalid `init_data` key=value format.
    /// For example, may occur on `init_data` like `foo=bar&invalid&bar=baz`
    InitDataFormat,
    /// Empty `init_data` or missing `hash=` field
    InitDataEmpty,
    /// `init_data` decode failture using `urlencoding::decode`
    Utf8Decode(FromUtf8Error),
    /// Calculated hash not equals telegram-provided hash
    HashMismatch,
    /// `auth_date` is too old.
    ///
    /// **Note:** this variant returned only in `TelegramAuth::authorize_with_time()`
    AuthorizationExpired,
}

impl std::fmt::Display for TelegramAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InitDataFormat => write!(f, "Invalid `init_data` key=value format"),
            Self::InitDataEmpty => write!(f, "Empty (or invalid) `init_data`"),
            Self::Utf8Decode(e) => e.fmt(f),
            Self::HashMismatch => write!(f, "Hash does not match"),
            Self::AuthorizationExpired => write!(f, "`auth_date=` field in `init_data` too old"),
        }
    }
}
impl std::error::Error for TelegramAuthError {}

impl TelegramAuth {
    /// Maximum time in seconds when the authorization is still valid
    pub const MAX_AUTH_TIME: u64 = 15 * 60;

    /// Parse `init_data` and verify it using `bot_token`. This function does not verify
    /// `auth_date=` field.
    ///
    /// [Telegram documentation](https://core.telegram.org/bots/webapps#validating-data-received-via-the-web-app)
    pub fn authorize(init_data: &str, bot_token: &str) -> Result<Self, TelegramAuthError> {
        // Stolen from: https://stackoverflow.com/q/72044314 and first answer to it

        // Also, data_field here doesn't contains hash= field
        let (data_fields, telegram_hash) = {
            // Spliting at '&' before decode because after decode some field (ex user.name) may contains it
            let raw_fields = init_data.split('&').map(|f| f.split_once('='));

            let mut hs = Vec::new();
            let mut hash = None;

            // May be it can be more... readable... Anyway, its works (in my dreams)
            for field in raw_fields {
                if let Some((k, v)) = field {
                    let v = match decode(v) {
                        Ok(v) => v.to_string(),
                        Err(e) => return Err(TelegramAuthError::Utf8Decode(e)),
                    };
                    if k == "hash" {
                        hash = Some(v);
                    } else {
                        hs.push((k, v));
                    }
                } else {
                    return Err(TelegramAuthError::InitDataFormat);
                }
            }

            hs.sort_by_key(|f| f.0);

            (hs, hash)
        };

        // If there no hash= field, structure may be invalid (or empty...)
        let telegram_hash = match telegram_hash {
            Some(h) => h,
            None => return Err(TelegramAuthError::InitDataEmpty),
        };

        // Telegram-way data_check_string. See top comment
        let data_check_string = data_fields
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .join("\n");

        let secret_key = {
            let mut mac =
                Hmac::<Sha256>::new_from_slice(b"WebAppData").expect("Init 'WebAppData' mac");
            mac.update(bot_token.as_bytes());

            mac.finalize().into_bytes()
        };

        // Following telegram documentation, we need to check
        // hex(HMAC_SHA256(data_check_string, secret_key)) == telegram_hash
        // but hmac crate contains verify func... Copypasting code from top comment
        let calculated_hash = {
            let mut mac =
                Hmac::<Sha256>::new_from_slice(&secret_key[..]).expect("Init `secret_key` mac");
            mac.update(data_check_string.as_bytes());

            mac.finalize().into_bytes()
        };

        // Encode `calculated_hash` bytes to hex string, like `telegram_hash`
        let calculated_hash = {
            let bytes = &calculated_hash[..];
            let mut s = String::with_capacity(bytes.len() * 2);

            for &b in bytes {
                write!(&mut s, "{:02x}", b).unwrap();
            }

            s
        };

        if calculated_hash == telegram_hash {
            Ok(TelegramAuth(
                data_fields
                    .into_iter()
                    .map(|f| (f.0.to_owned(), f.1))
                    .collect(),
            ))
        } else {
            Err(TelegramAuthError::HashMismatch)
        }
    }

    /// Parse `init_data` and verify it using `bot_token`.
    ///
    /// [Telegram documentation](https://core.telegram.org/bots/webapps#validating-data-received-via-the-web-app)
    pub fn authorize_with_time(
        init_data: &str,
        bot_token: &str,
        allowed_auth_time: u64,
    ) -> Result<Self, TelegramAuthError> {
        let data_fields = Self::authorize(init_data, bot_token)?;

        if data_fields.validate_by_time(allowed_auth_time) {
            Ok(data_fields)
        } else {
            Err(TelegramAuthError::AuthorizationExpired)
        }
    }

    /// Validate authorization by time
    pub fn validate_by_time(&self, allowed_auth_time: u64) -> bool {
        let auth_time = self.0.get("auth_date").and_then(|f| f.parse::<u64>().ok());
        debug_assert!(auth_time.is_some());

        let auth_time = match auth_time {
            Some(a) => a,
            None => return false,
        };

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Duration since UNIX_EPOCH")
            .as_secs();

        auth_time + allowed_auth_time >= current_time
    }

    /// Get user from data fields. Panics if there no `user=` field.
    pub fn get_user(&self) -> Result<TelegramUser, serde_json::Error> {
        let user = self
            .0
            .get("user")
            .expect("`data_fields` doesn't contains `user=` field");

        serde_json::from_str(user)
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for TelegramUser {
    type Error = TelegramAuthError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let bot = match req.guard::<&State<Bot>>().await {
            Outcome::Success(a) => a,
            Outcome::Failure(_) => panic!("Failed to get guard &State<teloxide_core::bot::Bot>"),
            Outcome::Forward(_) => unreachable!(),
        };

        let init_data = match req.headers().get("X-InitData").next() {
            Some(h) => h,
            None => return Outcome::Forward(()),
        };

        let telegram_auth =
            TelegramAuth::authorize_with_time(init_data, bot.token(), TelegramAuth::MAX_AUTH_TIME);

        match telegram_auth {
            Ok(a) => Outcome::Success(a.get_user().expect("Invalid TelegramUser object")),
            Err(e) => Outcome::Failure((Status::Forbidden, e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TelegramAuth, TelegramAuthError};

    #[test]
    /// Check [`TelegramAuth::authorize`]  (without time check)
    fn test_authorize() {
        // Real (but revoked) bot token and init_data
        let bot_token = "5771507903:AAHYxg2LdN031SsY0urp0FFgiWPk4Jq4v_g";
        let init_data = "query_id=AAEmQYAUAAAAACZBgBSRrcZj&user=%7B%22id%22%3A343949606%2C%22first_name%22%3A%22ceheki%21%20%F0%9F%8C%BF%22%2C%22last_name%22%3A%22%CE%B6%22%2C%22username%22%3A%22ceheki%22%2C%22language_code%22%3A%22en%22%2C%22is_premium%22%3Atrue%7D&auth_date=1666961519&hash=d27168e5a02e8639308d526b060dd28cdc668e6c48a897bbd1c7e1b6df3022c2";

        let result = TelegramAuth::authorize(init_data, bot_token).unwrap();

        assert_eq!(
            result.0.get("query_id"),
            Some(&"AAEmQYAUAAAAACZBgBSRrcZj".to_owned())
        );
        assert_eq!(result.0.get("auth_date"), Some(&"1666961519".to_owned()));
        assert_eq!(result.0.get("hash"), None);
    }

    #[test]
    /// Check [`TelegramAuth::authorize`] with invalid data format
    fn test_authorize_invalid_format() {
        let init_data = "hash=foobarbaz&INVALID&yeees=12345";

        let result = TelegramAuth::authorize(init_data, "--not-used").unwrap_err();

        assert_eq!(result, TelegramAuthError::InitDataFormat);
    }

    #[test]
    /// Check [`TelegramAuth::authorize`] with empty data
    fn test_authorize_data_empty() {
        let init_data = "";

        let result = TelegramAuth::authorize(init_data, "--not-used").unwrap_err();

        assert_eq!(result, TelegramAuthError::InitDataFormat);
    }

    #[test]
    /// Check [`TelegramAuth::authorize`] with data without hash
    fn test_authorize_data_no_hash() {
        let init_data = "stilly=normal&data=butnohash";

        let result = TelegramAuth::authorize(init_data, "--not-used").unwrap_err();

        assert_eq!(result, TelegramAuthError::InitDataEmpty);
    }

    #[test]
    /// Check [`TelegramAuth::authorize`] with invalid hash
    fn test_authorize_invalid_hash() {
        let bot_data = "5771507903:AAHYxg2LdN031SsY0urp0FFgiWPk4Jq4v_g";
        let init_data = "query_id=AAEmQYAUAAAAACZBgBQMkN2a&user=%7B%22id%22%3A343949606%2C%22first_name%22%3A%22ceheki%21%20%F0%9F%8C%BF%22%2C%22last_name%22%3A%22%CE%B6%22%2C%22username%22%3A%22ceheki%22%2C%22language_code%22%3A%22en%22%2C%22is_premium%22%3Atrue%7D&auth_date=1666953069&hash=360df48b0006b02b2f5d8526c2a16dbd521ab59c7bad1b480738c45fbd9e3b8e";

        let result = TelegramAuth::authorize(init_data, bot_data).unwrap_err();

        assert_eq!(result, TelegramAuthError::HashMismatch);
    }

    #[test]
    /// Check [`TelegramAuth::authorize_with_time`]
    fn test_authorize_with_time() {
        let bot_token = "5771507903:AAHYxg2LdN031SsY0urp0FFgiWPk4Jq4v_g";
        let init_data = "query_id=AAEmQYAUAAAAACZBgBT6fMXb&user=%7B%22id%22%3A343949606%2C%22first_name%22%3A%22ceheki%21%20%F0%9F%8C%BF%22%2C%22last_name%22%3A%22%CE%B6%22%2C%22username%22%3A%22ceheki%22%2C%22language_code%22%3A%22en%22%2C%22is_premium%22%3Atrue%7D&auth_date=1666946118&hash=95b450d68547f74f61d8863d03de61b3f9c528da494631715d6ee52c20561076";

        let result =
            TelegramAuth::authorize_with_time(init_data, bot_token, TelegramAuth::MAX_AUTH_TIME)
                .unwrap_err();

        assert_eq!(result, TelegramAuthError::AuthorizationExpired);
    }
}
