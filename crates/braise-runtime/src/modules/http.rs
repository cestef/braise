use crate::{BraiseType, Result, RuntimeError};
use core::TypedValue;
use ureq::Agent;

pub struct HttpModule {
    client: Agent,
}

impl HttpModule {
    pub fn new() -> Self {
        Self {
            client: Agent::new_with_config(
                Agent::config_builder()
                    .user_agent(format!(
                        "Braise/{} ({}; {})",
                        env!("CARGO_PKG_VERSION"),
                        env!("CARGO_PKG_NAME"),
                        env!("CARGO_PKG_REPOSITORY")
                    ))
                    .build(),
            ),
        }
    }

    pub fn get(&self, url: TypedValue) -> Result<TypedValue> {
        let url = url.to_string();
        let mut response = self.client.get(&url).call().map_err(|e| {
            RuntimeError::builtin_error(format!("HTTP GET request failed: {e}"), "http")
        })?;
        let body = response.body_mut().read_to_string().map_err(|e| {
            RuntimeError::builtin_error(format!("Failed to read response body: {e}"), "http")
        })?;
        Ok(TypedValue::new(body, BraiseType::String))
    }

    pub fn post(&self, url: TypedValue, body: TypedValue) -> Result<TypedValue> {
        let url = url.to_string();
        let mut response = self.client.post(&url).send(body.to_string()).map_err(|e| {
            RuntimeError::builtin_error(format!("HTTP POST request failed: {e}"), "http")
        })?;
        let response_body = response.body_mut().read_to_string().map_err(|e| {
            RuntimeError::builtin_error(format!("Failed to read response body: {e}"), "http")
        })?;
        Ok(TypedValue::new(response_body, BraiseType::String))
    }

    pub fn put(&self, url: TypedValue, body: TypedValue) -> Result<TypedValue> {
        let url = url.to_string();
        let mut response = self.client.put(&url).send(body.to_string()).map_err(|e| {
            RuntimeError::builtin_error(format!("HTTP PUT request failed: {e}"), "http")
        })?;
        let response_body = response.body_mut().read_to_string().map_err(|e| {
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
