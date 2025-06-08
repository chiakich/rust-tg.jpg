use anyhow::Result;
use reqwest::Client;

// Check if the URL is from imgur
pub fn is_imgur_url(url: &str) -> bool {
  url.contains("imgur.com") || url.contains("i.imgur.com")
}

// Download image data from imgur URL
pub async fn download_imgur_image(url: &str) -> Result<Vec<u8>, anyhow::Error> {
  let client = Client::new();
  let response = client
    .get(url)
    .header(
      "User-Agent",
      "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
    )
    .send()
    .await?;

  if !response.status().is_success() {
    return Err(anyhow::anyhow!("HTTP error: {}", response.status()));
  }

  let bytes = response.bytes().await?;
  Ok(bytes.to_vec())
}
