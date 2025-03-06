use anyhow::Result;
use log::{error, info};
use teloxide::prelude::*;
use teloxide::types::{
  ChosenInlineResult, InlineQuery, InlineQueryResult, InlineQueryResultArticle,
  InlineQueryResultGif, InlineQueryResultPhoto, InputMessageContent, InputMessageContentText,
};
use url::Url;
use uuid::Uuid;

use crate::local_image_finder::find_matching_images;

// Handle inline queries
pub async fn handle_inline_query(bot: Bot, q: InlineQuery) -> Result<(), anyhow::Error> {
  let query = q.query.clone();

  // If query is empty, return empty results
  if query.is_empty() {
    let results = Vec::<InlineQueryResult>::new();
    bot.answer_inline_query(&q.id, results).await?;
    return Ok(());
  }

  info!("Received inline query: {}", query);

  // Find matching images from local assets
  let matching_images = match find_matching_images(&query).await {
    Ok(images) => {
      info!(
        "Found {} matching images for query: {}",
        images.len(),
        query
      );
      images
    }
    Err(e) => {
      error!("Error finding matching images: {:?}", e);
      Vec::new()
    }
  };

  // Convert matching images to inline query results
  let mut results = Vec::new();

  // Limit the number of processed images to avoid timeout
  let max_results = 10; // Reduce the number of processed images

  // Create photo or gif results for each matching image
  for (image_path, score) in matching_images.iter().take(max_results) {
    info!("Processing image: {:?} with score: {}", image_path, score);

    let file_name = image_path
      .file_stem()
      .and_then(|s| s.to_str())
      .unwrap_or("image");

    let id = Uuid::new_v4().to_string();
    let file_path_str = image_path.to_string_lossy().to_string();

    // Get relative path from local path for constructing GitHub URL
    let relative_path = if let Some(assets_pos) = file_path_str.find("assets") {
      info!("Found assets in path: {}", file_path_str);
      &file_path_str[assets_pos..]
    } else {
      error!("Could not find 'assets' in path: {}", file_path_str);
      continue; // Skip this image if assets directory is not found
    };

    // Construct GitHub URL
    // Use the correct raw format
    let github_base_url = "https://raw.githubusercontent.com/akira02/rust-tg.jpg/main/src/";

    // URL encode the path
    let encoded_path = relative_path
      .split('/')
      .map(|segment| {
        // URL encode each path segment
        let encoded = urlencoding::encode(segment);
        info!("Encoded path segment: {} -> {}", segment, encoded);
        encoded
      })
      .collect::<Vec<_>>()
      .join("/");

    let github_url = format!("{}{}", github_base_url, encoded_path);
    info!("Constructed GitHub URL: {}", github_url);

    // Convert String URL to Url type
    let github_url_parsed = match Url::parse(&github_url) {
      Ok(url) => {
        info!("Successfully parsed URL: {}", url);
        url
      }
      Err(e) => {
        error!("Failed to parse URL {}: {:?}", github_url, e);
        continue; // Skip this image if URL parsing fails
      }
    };

    // Check file extension
    let file_extension = image_path
      .extension()
      .and_then(|ext| ext.to_str())
      .unwrap_or("")
      .to_lowercase();

    let is_gif = file_extension == "gif";
    info!("File extension: {}, is_gif: {}", file_extension, is_gif);

    // Create appropriate inline query result
    if is_gif {
      // GIF result
      info!("Creating GIF result for: {}", file_name);
      results.push(InlineQueryResult::Gif(InlineQueryResultGif {
        id,
        gif_url: github_url_parsed.clone(),
        thumbnail_url: github_url_parsed,
        gif_width: Some(320),  // Set reasonable width
        gif_height: Some(240), // Set reasonable height
        gif_duration: None,
        thumbnail_mime_type: None,
        title: Some(file_name.to_string()),
        caption: None,
        parse_mode: None,
        caption_entities: None,
        reply_markup: None,
        input_message_content: None,
      }));
    } else {
      // Photo result
      info!("Creating Photo result for: {}", file_name);
      info!("Photo URL: {}", github_url_parsed);
      results.push(InlineQueryResult::Photo(InlineQueryResultPhoto {
        id,
        photo_url: github_url_parsed.clone(),
        thumbnail_url: github_url_parsed,
        photo_width: Some(320),  // Set reasonable width
        photo_height: Some(240), // Set reasonable height
        title: Some(file_name.to_string()),
        description: None,
        caption: None,
        parse_mode: None,
        caption_entities: None,
        reply_markup: None,
        input_message_content: None,
      }));
    }
  }

  // If no results, add a message
  if results.is_empty() {
    info!("No results found for query: {}", query);
    let id = Uuid::new_v4().to_string();
    results.push(InlineQueryResult::Article(InlineQueryResultArticle {
      id,
      title: "No matching images found".to_string(),
      input_message_content: InputMessageContent::Text(InputMessageContentText {
        message_text: format!("No matching images found for \"{}\"", query),
        parse_mode: None,
        entities: None,
        link_preview_options: None,
      }),
      reply_markup: None,
      url: None,
      hide_url: None,
      description: Some("Try another search term".to_string()),
      thumbnail_url: None,
      thumbnail_width: None,
      thumbnail_height: None,
    }));
  } else {
    info!("Found {} results for query: {}", results.len(), query);
  }

  // Set cache time to 300 seconds (5 minutes)
  let cache_time = 300;

  // Answer the inline query with cache time
  info!("Answering inline query with {} results", results.len());
  match bot
    .answer_inline_query(&q.id, results)
    .cache_time(cache_time)
    .await
  {
    Ok(_) => info!("Successfully answered inline query: {}", q.id),
    Err(e) => error!("Failed to answer inline query: {:?}", e),
  }

  Ok(())
}

// Handle chosen inline results
pub async fn handle_chosen_inline_result(
  _bot: Bot,
  r: ChosenInlineResult,
) -> Result<(), anyhow::Error> {
  info!("Chosen inline result: {:?}", r);

  // Since we now display images directly in inline query
  // When user selects a result, the image is already sent to the chat
  // So we don't need to send the image again

  Ok(())
}
