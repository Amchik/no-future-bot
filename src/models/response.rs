use rocket::{http::Status, response::Responder, Response};
use serde::Serialize;

#[allow(dead_code)]
#[derive(Serialize)]
#[serde(untagged)]
/// API Response
pub enum APIResponse {
    /// Request finished without errors and produce some result
    Ok { result: serde_json::Value },
    /// Request finished without errors and response
    NoContent,
    /// Request failed
    Error {
        error_code: u16,
        error_description: String,
    },
}

#[allow(dead_code)]
impl APIResponse {
    fn status(&self) -> Status {
        match self {
            APIResponse::Ok { .. } => Status::Ok,
            APIResponse::NoContent => Status::NoContent,
            APIResponse::Error {
                error_code: code, ..
            } => Status { code: *code },
        }
    }

    pub fn new<T>(result: T) -> Self
    where
        T: Serialize,
    {
        Self::try_new(result).unwrap()
    }

    pub fn try_new<T>(result: T) -> Result<Self, serde_json::Error>
    where
        T: Serialize,
    {
        Ok(Self::Ok {
            result: serde_json::to_value(result)?,
        })
    }

    pub fn error<T>(error_code: u16, error_description: T) -> Self
    where
        T: AsRef<str>,
    {
        Self::Error {
            error_code,
            error_description: error_description.as_ref().to_string(),
        }
    }
}

impl<'r, 'o: 'r> Responder<'r, 'o> for APIResponse {
    fn respond_to(self, request: &'r rocket::Request<'_>) -> rocket::response::Result<'o> {
        let json = serde_json::to_string_pretty(&self).expect("APIResponse serialize");

        Response::build_from(json.respond_to(request)?)
            .raw_header("Content-Type", "application/json")
            .status(self.status())
            .ok()
    }
}
