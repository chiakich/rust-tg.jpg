use log::{debug, info};
use reqwest::Client;
use serde_json::Value;
use std::collections::HashSet;
use std::env;

use crate::image_search::{SearchError, MAX_RESULTS};

const ENGINE_NAME: &str = "Serper";
const ENDPOINT: &str = "https://google.serper.dev/images";

pub fn is_configured() -> bool {
  env::var("SERPER_API")
    .map(|value| !value.trim().is_empty())
    .unwrap_or(false)
}

pub async fn search(query: &str, is_gif: bool) -> Result<Vec<String>, SearchError> {
  let api_key = env::var("SERPER_API").map_err(|_| SearchError::NetworkFailed {
    engine: ENGINE_NAME,
    details: "SERPER_API is not configured".to_string(),
  })?;

  info!(
    "Searching Serper images for query: '{}', is_gif: {}",
    query, is_gif
  );

  let client = Client::new();
  let payload = serde_json::json!({
    "q": query,
    "gl": "tw",
    "hl": "zh-tw",
    "type": if is_gif { "gif" } else { "images" }
  })
  .to_string();

  let response = client
    .post(ENDPOINT)
    .header("X-API-KEY", api_key)
    .header("Content-Type", "application/json")
    .body(payload)
    .send()
    .await
    .map_err(|err| SearchError::NetworkFailed {
      engine: ENGINE_NAME,
      details: err.to_string(),
    })?;

  let status = response.status();
  let body = response
    .text()
    .await
    .map_err(|err| SearchError::NetworkFailed {
      engine: ENGINE_NAME,
      details: err.to_string(),
    })?;

  let parsed: Value = serde_json::from_str(&body).map_err(|err| SearchError::ParseFailed {
    engine: ENGINE_NAME,
    details: format!("Invalid JSON: {}", err),
  })?;

  if let Some(message) = parsed.get("message").and_then(Value::as_str) {
    return Err(SearchError::Blocked {
      engine: ENGINE_NAME,
      details: format!("status={}, message={}", status, message),
    });
  }

  if let Some(error_message) = parsed.get("error").and_then(Value::as_str) {
    return Err(SearchError::Blocked {
      engine: ENGINE_NAME,
      details: format!("status={}, error={}", status, error_message),
    });
  }

  let mut urls = Vec::new();
  let mut seen = HashSet::new();

  if let Some(results) = parsed.get("images").and_then(Value::as_array) {
    for result in results {
      if urls.len() >= MAX_RESULTS {
        break;
      }

      let candidate = result
        .get("imageUrl")
        .and_then(Value::as_str)
        .or_else(|| result.get("thumbnailUrl").and_then(Value::as_str));

      if let Some(url) = candidate {
        let url = url.to_string();
        if seen.insert(url.clone()) {
          debug!("Extracted URL from Serper: {}", url);
          urls.push(url);
        }
      }
    }
  }

  if urls.is_empty() {
    return Err(SearchError::NoResults {
      engine: ENGINE_NAME,
    });
  }

  info!(
    "Successfully extracted {} image URLs from Serper",
    urls.len()
  );
  Ok(urls)
}
