use log::{debug, error, info, warn};
use reqwest::Client;
use reqwest::StatusCode;
use std::collections::HashSet;

use crate::image_search::{SearchError, MAX_RESULTS};

// Search for images using Bing Image Search
pub async fn search(query: &str, is_gif: bool) -> Result<Vec<String>, SearchError> {
  let endpoint = "https://www.bing.com/images/search";

  // filterui:photo-animatedgif for GIF, filterui:photo-photo for static images
  let qft = if is_gif {
    "+filterui:photo-animatedgif"
  } else {
    "+filterui:photo-photo"
  };

  let params = [
    ("q", query),
    ("form", "HDRSC2"),
    ("first", "1"),
    ("qft", qft),
  ];

  info!(
    "Searching Bing Images for query: '{}', is_gif: {}",
    query, is_gif
  );

  let client = Client::new();
  let res = client
    .get(endpoint)
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
    .header("Referer", "https://www.bing.com/")
    .send()
    .await
    .map_err(|err| SearchError::NetworkFailed {
      engine: "Bing",
      details: err.to_string(),
    })?;

  let status = res.status();
  info!("Received response with status: {}", status);

  let bytes = res
    .bytes()
    .await
    .map_err(|err| SearchError::NetworkFailed {
      engine: "Bing",
      details: err.to_string(),
    })?;
  info!("HTML response length: {} bytes", bytes.len());

  let html = String::from_utf8_lossy(&bytes);

  debug!(
    "HTML snippet: {}",
    &html.chars().take(1000).collect::<String>()
  );

  if is_blocked(status, &html) {
    error!("Bing image search was blocked or challenged.");
    write_debug_html("/tmp/bing_search_debug.html", bytes.as_ref());
    return Err(SearchError::Blocked {
      engine: "Bing",
      details: format!("status={}", status),
    });
  }

  let urls = extract_image_urls(&html);
  if urls.is_empty() {
    error!("Bing image search returned no parseable image URLs.");
    write_debug_html("/tmp/bing_search_debug.html", bytes.as_ref());
    return Err(SearchError::ParseFailed {
      engine: "Bing",
      details: "No image URLs extracted from HTML".to_string(),
    });
  }
  info!("Successfully extracted {} image URLs", urls.len());
  Ok(urls)
}

// Extract image URLs from Bing image search results HTML
// Bing embeds image metadata in <a class="iusc" m="{...}"> where the JSON contains murl
fn extract_image_urls(text: &str) -> Vec<String> {
  let mut urls = Vec::new();
  let mut seen = HashSet::new();

  // Method 1: iusc anchor tag with HTML-encoded JSON m attribute
  // <a class="iusc" ... m="{&quot;murl&quot;:&quot;https://...&quot;,...}">
  let iusc_regex = regex::Regex::new(r#"<a[^>]+class="iusc"[^>]+m="(\{[^"]+\})"[^>]*>"#).unwrap();

  for cap in iusc_regex.captures_iter(text) {
    if urls.len() >= MAX_RESULTS {
      break;
    }
    if let Some(m_match) = cap.get(1) {
      let m_str = m_match.as_str().replace("&quot;", "\"");
      if let Some(murl) = extract_murl(&m_str) {
        if seen.insert(murl.clone()) {
          debug!("Extracted URL (method 1 iusc): {}", murl);
          urls.push(murl);
        }
      }
    }
  }

  info!("Method 1 (iusc murl): Found {} URLs", urls.len());

  // Method 2: murl in raw JSON blobs (fallback)
  if urls.len() < 5 {
    info!("Trying method 2 (murl JSON)");
    let murl_regex = regex::Regex::new(r#""murl"\s*:\s*"(https?://[^"]+)""#).unwrap();

    for cap in murl_regex.captures_iter(text) {
      if urls.len() >= MAX_RESULTS {
        break;
      }
      if let Some(url_match) = cap.get(1) {
        let url = url_match
          .as_str()
          .replace("\\u0026", "&")
          .replace("\\u003d", "=");
        if seen.insert(url.clone()) {
          debug!("Extracted URL (method 2 murl): {}", url);
          urls.push(url);
        }
      }
    }

    info!("Method 2 (murl JSON): Found {} URLs total", urls.len());
  }

  if urls.is_empty() {
    warn!("All extraction methods failed to find image URLs");
  } else {
    info!("Successfully extracted {} URLs total", urls.len());
  }

  urls
}

fn extract_murl(json_str: &str) -> Option<String> {
  let key = "\"murl\":\"";
  let start = json_str.find(key)? + key.len();
  let rest = &json_str[start..];
  let end = rest.find('"')?;
  let url = &rest[..end];
  if url.starts_with("http") {
    Some(url.replace("\\u0026", "&").replace("\\u003d", "="))
  } else {
    None
  }
}

fn is_blocked(status: StatusCode, html: &str) -> bool {
  status == StatusCode::TOO_MANY_REQUESTS
    || status == StatusCode::FORBIDDEN
    || html.contains("Please verify you are a human")
    || html.contains("captcha")
}

fn write_debug_html(path: &str, bytes: &[u8]) {
  if let Err(e) = std::fs::write(path, bytes) {
    warn!("Could not write debug HTML to {}: {}", path, e);
  } else {
    info!("Wrote full HTML response to {} for debugging", path);
  }
}
