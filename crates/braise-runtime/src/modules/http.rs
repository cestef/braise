use crate::{BraiseType, Result, RuntimeError};
use core::TypedValue;
use reqwest::blocking::Client;

pub struct HttpModule {
    client: Client,
}

impl HttpModule {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent(format!(
                    "Braise/{} ({}; {})",
                    env!("CARGO_PKG_VERSION"),
                    env!("CARGO_PKG_NAME"),
                    env!("CARGO_PKG_REPOSITORY")
                ))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    pub fn get(&self, url: TypedValue) -> Result<TypedValue> {
        let url = url.to_string();
        let response = self.client.get(&url).send().map_err(|e| {
            RuntimeError::builtin_error(format!("HTTP GET request failed: {e}"), "http")
        })?;
        let body = response.text().map_err(|e| {
            RuntimeError::builtin_error(format!("Failed to read response body: {e}"), "http")
        })?;
        Ok(TypedValue::new(body, BraiseType::String))
    }

    pub fn post(&self, url: TypedValue, body: TypedValue) -> Result<TypedValue> {
        let url = url.to_string();
        let response = self
            .client
            .post(&url)
            .body(body.to_string())
            .send()
            .map_err(|e| {
                RuntimeError::builtin_error(format!("HTTP POST request failed: {e}"), "http")
            })?;
        let response_body = response.text().map_err(|e| {
            RuntimeError::builtin_error(format!("Failed to read response body: {e}"), "http")
        })?;
        Ok(TypedValue::new(response_body, BraiseType::String))
    }

    pub fn put(&self, url: TypedValue, body: TypedValue) -> Result<TypedValue> {
        let url = url.to_string();
        let response = self
            .client
            .put(&url)
            .body(body.to_string())
            .send()
            .map_err(|e| {
                RuntimeError::builtin_error(format!("HTTP PUT request failed: {e}"), "http")
            })?;
        let response_body = response.text().map_err(|e| {
            RuntimeError::builtin_error(format!("Failed to read response body: {e}"), "http")
        })?;
        Ok(TypedValue::new(response_body, BraiseType::String))
    }
}

builtin_module! {
    HttpModule {
        functions: {
            "get" => get(BraiseType::String),
            "post" => post(BraiseType::String, BraiseType::String),
            "put" => put(BraiseType::String, BraiseType::String),
        }
    }
}
