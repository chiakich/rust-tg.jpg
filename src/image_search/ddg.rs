use anyhow::Result;
use log::{debug, error, info, warn};
use reqwest::Client;

// Search for images using DuckDuckGo Image Search
//
// DDG image search requires two requests:
//   1. GET duckduckgo.com/?q=...&iax=images&ia=images  → extract vqd token from HTML
//   2. GET duckduckgo.com/i.js?q=...&vqd=TOKEN&o=json  → returns JSON with image URLs
pub async fn search(query: &str, is_gif: bool) -> Result<Vec<String>, anyhow::Error> {
  info!(
    "Searching DuckDuckGo Images for query: '{}', is_gif: {}",
    query, is_gif
  );

  let client = Client::builder().cookie_store(true).build()?;

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
    .await?;

  info!("i.js response status: {}", res.status());

  let body = res.text().await?;
  debug!(
    "i.js response (first 500 chars): {}",
    &body.chars().take(500).collect::<String>()
  );

  let urls = extract_image_urls(&body);

  if urls.is_empty() {
    error!("Failed to extract any image URLs from DDG response.");
    error!(
      "Response sample (first 2000 chars): {}",
      &body.chars().take(2000).collect::<String>()
    );
    if let Err(e) = std::fs::write("/tmp/ddg_search_debug.json", body.as_bytes()) {
      warn!("Could not write debug JSON to /tmp: {}", e);
    } else {
      info!("Wrote full DDG response to /tmp/ddg_search_debug.json for debugging");
    }
    return Err(anyhow::anyhow!(
      "DDG image search returned no results. Token or format may have changed."
    ));
  }

  info!("Successfully extracted {} image URLs", urls.len());
  Ok(urls)
}

// Step 1: load the DDG search page and pull out the vqd token
// The token appears as: vqd="3-..." or vqd='3-...' somewhere in the HTML/JS
async fn fetch_vqd(client: &Client, query: &str) -> Result<String> {
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
    .await?;

  info!("DDG search page status: {}", res.status());
  let html = res.text().await?;

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

  error!(
    "Could not find vqd token. HTML sample: {}",
    &html.chars().take(2000).collect::<String>()
  );
  if let Err(e) = std::fs::write("/tmp/ddg_vqd_debug.html", html.as_bytes()) {
    warn!("Could not write vqd debug HTML: {}", e);
  }
  Err(anyhow::anyhow!(
    "Could not extract vqd token from DuckDuckGo search page"
  ))
}

// Extract image URLs from the DDG i.js JSON response
// Response shape: {"results": [{"image": "https://...", ...}, ...], "next": "..."}
fn extract_image_urls(json: &str) -> Vec<String> {
  let mut urls = Vec::new();

  let image_regex = regex::Regex::new(r#""image"\s*:\s*"(https?://[^"]+)""#).unwrap();

  for cap in image_regex.captures_iter(json) {
    if urls.len() >= 10 {
      break;
    }
    if let Some(url_match) = cap.get(1) {
      let url = url_match
        .as_str()
        .replace("\\u0026", "&")
        .replace("\\u003d", "=");
      debug!("Extracted URL: {}", url);
      urls.push(url);
    }
  }

  info!("Extracted {} URLs from DDG response", urls.len());
  urls
}
