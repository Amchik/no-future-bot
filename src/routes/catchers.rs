use rocket::{catch, catchers, http::Status, Catcher, Request};

use crate::models::response::APIResponse;

pub fn catchers() -> Vec<Catcher> {
    catchers![no_endpoint_catcher, default_catcher]
}

#[catch(404)]
fn no_endpoint_catcher() -> APIResponse {
    APIResponse::error(404, "Endpoint doesn't exists")
}

#[catch(default)]
fn default_catcher(status: Status, _: &Request) -> APIResponse {
    APIResponse::error(status.code, status.reason_lossy())
}
