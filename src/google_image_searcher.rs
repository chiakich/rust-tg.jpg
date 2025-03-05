use anyhow::Result;
use reqwest::Client;

// Search for images using Google Image Search
pub async fn search(query: &str, is_gif: bool) -> Result<Vec<String>, anyhow::Error> {
  let endpoint = "https://www.google.com/search";
  let tbs = if is_gif { "ift:gif" } else { "ift:jpg" };

  let params = [("q", query), ("tbs", tbs), ("tbm", "isch"), ("hl", "zh-TW")];

  let client = Client::new();
  let res = client
    .get(endpoint)
    .query(&params)
    .header(
      "User-Agent",
      "Opera/9.80 (J2ME/MIDP; Opera Mini/9.80 (J2ME/23.377; U; en) Presto/2.5.25 Version/10.54",
    )
    .header(
      "Accept-Language",
      "en-US,en-GB;q=0.9,en;q=0.8,zh-TW;q=0.7,zh;q=0.6,ja-JP;q=0.5",
    )
    .send()
    .await?;

  let html = res.text().await?;
  let urls = extract_image_urls(&html);
  if urls.is_empty() {
    return Err(anyhow::anyhow!(
      "Img array is empty. It might be because Google changed the search html format."
    ));
  }
  Ok(urls)
}

// Extract image URLs from Google search results HTML
fn extract_image_urls(text: &str) -> Vec<String> {
  let mut urls = Vec::new();

  let imgres_regex = regex::Regex::new(r#"/imgres\?imgurl=(.*?)(?:&|$)"#).unwrap();
  for cap in imgres_regex.captures_iter(text).take(10) {
    if let Some(url_match) = cap.get(1) {
      let decoded_url = urlencoding::decode(url_match.as_str())
        .unwrap_or_default()
        .into_owned();
      let clean_url = decoded_url.split('?').next().unwrap_or("").to_string();
      urls.push(clean_url);
    }
  }
  // fallback using data-ou
  if urls.is_empty() {
    let data_ou_regex = regex::Regex::new(r#"data-ou="(.*?)""#).unwrap();
    for cap in data_ou_regex.captures_iter(text).take(10) {
      if let Some(url_match) = cap.get(1) {
        urls.push(url_match.as_str().to_string());
      }
    }
  }
  urls
}
