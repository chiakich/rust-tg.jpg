use anyhow::Result;
use log::{debug, error, info, warn};
use reqwest::Client;

// Search for images using Google Image Search
pub async fn search(query: &str, is_gif: bool) -> Result<Vec<String>, anyhow::Error> {
  let endpoint = "https://www.google.com/search";
  let tbs = if is_gif { "ift:gif" } else { "ift:jpg" };

  let params = [("q", query), ("tbs", tbs), ("tbm", "isch"), ("hl", "zh-TW")];

  info!(
    "Searching Google Images for query: '{}', is_gif: {}",
    query, is_gif
  );

  let client = Client::new();
  let res = client
    .get(endpoint)
    .query(&params)
    .header(
      "User-Agent",
      "Mozilla/5.0 (iPhone; CPU iPhone OS 16_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.0 Mobile/15E148 Safari/604.1",
    )
    .header(
      "Accept",
      "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
    )
    .header(
      "Accept-Language",
      "zh-TW,zh;q=0.9,en-US;q=0.8,en;q=0.7",
    )
    .send()
    .await?;

  info!("Received response with status: {}", res.status());

  // Use bytes() instead of text() for better performance
  // Only convert the portion we need to UTF-8
  let bytes = res.bytes().await?;
  info!("HTML response length: {} bytes", bytes.len());

  // Convert to string (this is the expensive part)
  let html = String::from_utf8_lossy(&bytes);

  // Log a snippet of the HTML for debugging (first 1000 chars)
  debug!(
    "HTML snippet: {}",
    &html.chars().take(1000).collect::<String>()
  );

  let urls = extract_image_urls(&html);
  if urls.is_empty() {
    error!("Failed to extract any image URLs. This likely means Google changed their HTML format.");
    error!(
      "HTML sample (first 2000 chars): {}",
      &html.chars().take(2000).collect::<String>()
    );
    return Err(anyhow::anyhow!(
      "Img array is empty. It might be because Google changed the search html format."
    ));
  }
  info!("Successfully extracted {} image URLs", urls.len());
  Ok(urls)
}

// Extract image URLs from Google search results HTML
fn extract_image_urls(text: &str) -> Vec<String> {
  let mut urls = Vec::new();

  // Try method 1: JSON-formatted image data (for Mobile Safari format)
  // Pattern: ["https://...image.jpg", width, height]
  let json_img_regex =
    regex::Regex::new(r#"\["(https?://[^"]+\.(?:jpg|jpeg|png|gif)[^"]*)"\s*,\s*\d+\s*,\s*\d+\]"#)
      .unwrap();

  // Use iterator directly without collecting - stops as soon as we have enough
  for cap in json_img_regex.captures_iter(text) {
    if urls.len() >= 10 {
      break; // Early termination once we have enough URLs
    }

    if let Some(url_match) = cap.get(1) {
      let url_str = url_match.as_str();
      // Filter out thumbnails and Google's own images
      if !url_str.contains("encrypted-tbn")
        && !url_str.contains("gstatic")
        && !url_str.contains("googlelogo")
      {
        // Decode unicode escapes if any
        let url = url_str.replace("\\u0026", "&").replace("\\u003d", "=");
        debug!("Extracted URL (method 1): {}", url);
        urls.push(url);
      }
    }
  }

  info!("Method 1 (JSON array): Found {} URLs", urls.len());

  // Try method 2: Simple quoted image URLs (fallback)
  if urls.is_empty() {
    info!("Method 1 failed, trying method 2 (quoted URLs)");
    let quoted_url_regex =
      regex::Regex::new(r#""(https?://[^"]+\.(?:jpg|jpeg|png|gif)[^"]*)""#).unwrap();

    for cap in quoted_url_regex.captures_iter(text) {
      if urls.len() >= 10 {
        break;
      }

      if let Some(url_match) = cap.get(1) {
        let url_str = url_match.as_str();
        // Filter out thumbnails
        if !url_str.contains("encrypted-tbn")
          && !url_str.contains("gstatic")
          && !url_str.contains("googlelogo")
        {
          let url = url_str.replace("\\u0026", "&").replace("\\u003d", "=");
          debug!("Extracted URL (method 2): {}", url);
          urls.push(url);
        }
      }
    }

    info!("Method 2 (quoted URLs): Found {} URLs", urls.len());
  }

  // Try method 3: data-ou attribute
  if urls.is_empty() {
    info!("Method 2 failed, trying method 3 (data-ou)");
    let data_ou_regex = regex::Regex::new(r#"data-ou="(.*?)""#).unwrap();
    let data_ou_matches: Vec<_> = data_ou_regex.captures_iter(text).take(10).collect();
    info!(
      "Method 3 (data-ou): Found {} matches",
      data_ou_matches.len()
    );

    for cap in data_ou_matches {
      if let Some(url_match) = cap.get(1) {
        let url = url_match.as_str().to_string();
        debug!("Extracted URL (method 3): {}", url);
        urls.push(url);
      }
    }
  }

  if urls.is_empty() {
    warn!("All extraction methods failed to find image URLs");
  } else {
    info!("Successfully extracted {} URLs total", urls.len());
  }

  urls
}
