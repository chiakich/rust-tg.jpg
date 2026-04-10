use log::{debug, info};
use reqwest::Client;
use serde_json::Value;
use std::collections::HashSet;
use std::env;

use crate::image_search::{SearchError, MAX_RESULTS};

const ENGINE_NAME: &str = "SerpAPI";
const ENDPOINT: &str = "https://serpapi.com/search.json";

pub fn is_configured() -> bool {
  env::var("SERP_API")
    .map(|value| !value.trim().is_empty())
    .unwrap_or(false)
}

pub async fn search(query: &str, is_gif: bool) -> Result<Vec<String>, SearchError> {
  let api_key = env::var("SERP_API").map_err(|_| SearchError::NetworkFailed {
    engine: ENGINE_NAME,
    details: "SERP_API is not configured".to_string(),
  })?;

  let image_type = if is_gif { "animated" } else { "photo" };
  let params = [
    ("engine", "google_images"),
    ("q", query),
    ("api_key", api_key.as_str()),
    ("hl", "zh-tw"),
    ("gl", "tw"),
    ("google_domain", "google.com"),
    ("image_type", image_type),
  ];

  info!(
    "Searching SerpAPI Google Images for query: '{}', is_gif: {}",
    query, is_gif
  );

  let client = Client::new();
  let response = client
    .get(ENDPOINT)
    .query(&params)
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

  if let Some(error_message) = parsed.get("error").and_then(Value::as_str) {
    return Err(SearchError::Blocked {
      engine: ENGINE_NAME,
      details: format!("status={}, error={}", status, error_message),
    });
  }

  if parsed
    .get("search_metadata")
    .and_then(|value| value.get("status"))
    .and_then(Value::as_str)
    == Some("Error")
  {
    return Err(SearchError::Blocked {
      engine: ENGINE_NAME,
      details: format!("status={}, search_metadata.status=Error", status),
    });
  }

  let mut urls = Vec::new();
  let mut seen = HashSet::new();

  if let Some(results) = parsed.get("images_results").and_then(Value::as_array) {
    for result in results {
      if urls.len() >= MAX_RESULTS {
        break;
      }

      let candidate = result
        .get("original")
        .and_then(Value::as_str)
        .or_else(|| result.get("thumbnail").and_then(Value::as_str));

      if let Some(url) = candidate {
        let url = url.to_string();
        if seen.insert(url.clone()) {
          debug!("Extracted URL from SerpAPI: {}", url);
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
    "Successfully extracted {} image URLs from SerpAPI",
    urls.len()
  );
  Ok(urls)
}
