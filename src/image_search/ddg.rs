use log::{debug, error, info, warn};
use reqwest::Client;
use reqwest::StatusCode;
use serde_json::Value;
use std::collections::HashSet;

use crate::image_search::{SearchError, MAX_RESULTS};

// Search for images using DuckDuckGo Image Search
//
// DDG image search requires two requests:
//   1. GET duckduckgo.com/?q=...&iax=images&ia=images  → extract vqd token from HTML
//   2. GET duckduckgo.com/i.js?q=...&vqd=TOKEN&o=json  → returns JSON with image URLs
pub async fn search(query: &str, is_gif: bool) -> Result<Vec<String>, SearchError> {
  info!(
    "Searching DuckDuckGo Images for query: '{}', is_gif: {}",
    query, is_gif
  );

  let client = Client::builder()
    .cookie_store(true)
    .build()
    .map_err(|err| SearchError::NetworkFailed {
      engine: "DDG",
      details: err.to_string(),
    })?;

  // Step 1: fetch the search page to obtain the vqd token
  let vqd = fetch_vqd(&client, query).await?;
  info!("Got vqd token: {}", vqd);

  // Step 2: fetch image results using the vqd token
  // type:gif or type:photo filter
  let type_filter = if is_gif { "type:gif" } else { "type:photo" };

  let params = [
    ("q", query),
    ("o", "json"),
    ("vqd", &vqd),
    ("f", type_filter),
    ("p", "1"),
  ];

  let res = client
    .get("https://duckduckgo.com/i.js")
    .query(&params)
    .header(
      "User-Agent",
      "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
    )
    .header("Accept", "application/json, text/javascript, */*; q=0.01")
    .header("Accept-Language", "zh-TW,zh;q=0.9,en-US;q=0.8,en;q=0.7")
    .header("Referer", "https://duckduckgo.com/")
    .header("X-Requested-With", "XMLHttpRequest")
    .send()
    .await
    .map_err(|err| SearchError::NetworkFailed {
      engine: "DDG",
      details: err.to_string(),
    })?;

  let status = res.status();
  info!("i.js response status: {}", status);

  let body = res.text().await.map_err(|err| SearchError::NetworkFailed {
    engine: "DDG",
    details: err.to_string(),
  })?;
  debug!(
    "i.js response (first 500 chars): {}",
    &body.chars().take(500).collect::<String>()
  );

  if is_blocked_i_js(status, &body) {
    error!("DuckDuckGo image search was blocked or rate-limited.");
    write_debug_file("/tmp/ddg_search_debug.json", body.as_bytes(), "debug JSON");
    return Err(SearchError::Blocked {
      engine: "DDG",
      details: format!("status={}, body matched block page", status),
    });
  }

  let urls = extract_image_urls(&body)?;

  if urls.is_empty() {
    return Err(SearchError::NoResults { engine: "DDG" });
  }

  info!("Successfully extracted {} image URLs", urls.len());
  Ok(urls)
}

// Step 1: load the DDG search page and pull out the vqd token
// The token appears as: vqd="3-..." or vqd='3-...' somewhere in the HTML/JS
async fn fetch_vqd(client: &Client, query: &str) -> Result<String, SearchError> {
  let params = [("q", query), ("iax", "images"), ("ia", "images")];

  let res = client
    .get("https://duckduckgo.com/")
    .query(&params)
    .header(
      "User-Agent",
      "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
    )
    .header(
      "Accept",
      "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8",
    )
    .header("Accept-Language", "zh-TW,zh;q=0.9,en-US;q=0.8,en;q=0.7")
    .send()
    .await
    .map_err(|err| SearchError::NetworkFailed {
      engine: "DDG",
      details: err.to_string(),
    })?;

  let status = res.status();
  info!("DDG search page status: {}", status);
  let html = res.text().await.map_err(|err| SearchError::NetworkFailed {
    engine: "DDG",
    details: err.to_string(),
  })?;

  if is_blocked_search_page(status, &html) {
    error!("DuckDuckGo search page was blocked or did not return a usable token.");
    write_debug_file("/tmp/ddg_vqd_debug.html", html.as_bytes(), "vqd debug HTML");
    return Err(SearchError::Blocked {
      engine: "DDG",
      details: format!("status={}, vqd page blocked", status),
    });
  }

  // Extract vqd token — DDG embeds it as vqd="<token>" in a script block
  let vqd_regex = regex::Regex::new(r#"vqd=["']([^"']+)["']"#).unwrap();
  if let Some(cap) = vqd_regex.captures(&html) {
    if let Some(token) = cap.get(1) {
      return Ok(token.as_str().to_string());
    }
  }

  // Fallback pattern: vqd:<token> (no quotes, sometimes seen in newer DDG HTML)
  let vqd_regex2 = regex::Regex::new(r#"vqd:([0-9a-zA-Z\-]+)"#).unwrap();
  if let Some(cap) = vqd_regex2.captures(&html) {
    if let Some(token) = cap.get(1) {
      return Ok(token.as_str().to_string());
    }
  }

  error!("Could not find a DDG vqd token in the search page.");
  write_debug_file("/tmp/ddg_vqd_debug.html", html.as_bytes(), "vqd debug HTML");
  Err(SearchError::ParseFailed {
    engine: "DDG",
    details: "Could not extract vqd token from search page".to_string(),
  })
}

// Extract image URLs from the DDG i.js JSON response
// Response shape: {"results": [{"image": "https://...", ...}, ...], "next": "..."}
fn extract_image_urls(json: &str) -> Result<Vec<String>, SearchError> {
  let parsed: Value = serde_json::from_str(json).map_err(|err| SearchError::ParseFailed {
    engine: "DDG",
    details: format!("Invalid i.js JSON: {}", err),
  })?;

  let mut urls = Vec::new();
  let mut seen = HashSet::new();

  if let Some(results) = parsed.get("results").and_then(Value::as_array) {
    for result in results {
      if urls.len() >= MAX_RESULTS {
        break;
      }

      if let Some(url) = result.get("image").and_then(Value::as_str) {
        let url = url.replace("\\u0026", "&").replace("\\u003d", "=");
        if seen.insert(url.clone()) {
          debug!("Extracted URL: {}", url);
          urls.push(url);
        }
      }
    }
  }

  info!("Extracted {} URLs from DDG response", urls.len());
  Ok(urls)
}

fn is_blocked_search_page(status: StatusCode, html: &str) -> bool {
  status == StatusCode::TOO_MANY_REQUESTS
    || status == StatusCode::FORBIDDEN
    || html.contains("ops@duckduckgo.com")
    || html.contains("anomaly")
}

fn is_blocked_i_js(status: StatusCode, body: &str) -> bool {
  status == StatusCode::TOO_MANY_REQUESTS
    || status == StatusCode::FORBIDDEN
    || body.contains("ops@duckduckgo.com")
}

fn write_debug_file(path: &str, bytes: &[u8], label: &str) {
  if let Err(e) = std::fs::write(path, bytes) {
    warn!("Could not write {} to {}: {}", label, path, e);
  } else {
    info!("Wrote full {} to {} for debugging", label, path);
  }
}
