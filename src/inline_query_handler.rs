use anyhow::Result;
use log::{error, info};
use teloxide::prelude::*;
use teloxide::types::{
  ChosenInlineResult, InlineQuery, InlineQueryResult, InlineQueryResultArticle,
  InlineQueryResultGif, InlineQueryResultPhoto, InputMessageContent, InputMessageContentText,
};
use url::Url;
use uuid::Uuid;

use crate::bing_image_searcher::search as image_search;

// Handle inline queries
pub async fn handle_inline_query(bot: Bot, q: InlineQuery) -> Result<(), anyhow::Error> {
  let query = q.query.trim().to_string();

  if query.is_empty() {
    bot
      .answer_inline_query(&q.id, Vec::<InlineQueryResult>::new())
      .await?;
    return Ok(());
  }

  info!("Received inline query: {}", query);

  // Detect if the user wants GIFs (query ends with .gif)
  let (search_query, is_gif) = if query.to_lowercase().ends_with(".gif") {
    (query.trim_end_matches(|c: char| c == '.' || c.is_alphabetic()).trim().to_string(), true)
  } else {
    (query.trim_end_matches(|c: char| c == '.' || c.is_alphabetic() && query.to_lowercase().ends_with(".jpg") || c.is_alphabetic() && query.to_lowercase().ends_with(".png")).trim().to_string(), false)
  };

  // Strip common image extensions from the search query
  let search_query = search_query
    .trim_end_matches(".jpg")
    .trim_end_matches(".jpeg")
    .trim_end_matches(".png")
    .trim_end_matches(".gif")
    .trim()
    .to_string();
  let search_query = if search_query.is_empty() { query.clone() } else { search_query };

  let image_urls = match image_search(&search_query, is_gif).await {
    Ok(urls) => {
      info!("Found {} image URLs for query: {}", urls.len(), search_query);
      urls
    }
    Err(e) => {
      error!("Error searching images: {:?}", e);
      Vec::new()
    }
  };

  let mut results = Vec::new();

  for url_str in image_urls.iter().take(10) {
    let parsed_url = match Url::parse(url_str) {
      Ok(url) => url,
      Err(e) => {
        error!("Failed to parse URL {}: {:?}", url_str, e);
        continue;
      }
    };

    let id = Uuid::new_v4().to_string();

    if is_gif {
      results.push(InlineQueryResult::Gif(InlineQueryResultGif {
        id,
        gif_url: parsed_url.clone(),
        thumbnail_url: parsed_url,
        gif_width: None,
        gif_height: None,
        gif_duration: None,
        thumbnail_mime_type: None,
        title: None,
        caption: None,
        parse_mode: None,
        caption_entities: None,
        reply_markup: None,
        input_message_content: None,
      }));
    } else {
      results.push(InlineQueryResult::Photo(InlineQueryResultPhoto {
        id,
        photo_url: parsed_url.clone(),
        thumbnail_url: parsed_url,
        photo_width: None,
        photo_height: None,
        title: None,
        description: None,
        caption: None,
        parse_mode: None,
        caption_entities: None,
        reply_markup: None,
        input_message_content: None,
      }));
    }
  }

  if results.is_empty() {
    info!("No results found for query: {}", query);
    results.push(InlineQueryResult::Article(InlineQueryResultArticle {
      id: Uuid::new_v4().to_string(),
      title: "No images found".to_string(),
      input_message_content: InputMessageContent::Text(InputMessageContentText {
        message_text: format!("No images found for \"{}\"", query),
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
  }

  info!("Answering inline query with {} results", results.len());
  match bot
    .answer_inline_query(&q.id, results)
    .cache_time(300)
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
  Ok(())
}
