use anyhow::Result;
use log::{error, info};
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::InputFile;
use teloxide::RequestError;
use tokio::sync::Mutex;
use url::Url;

// Import the local image finder module
mod local_image_finder;
use local_image_finder::find_local_image;

// Import the Google image search module
mod google_image_searcher;
use google_image_searcher::search as google_image_search;

// Define a type for our chat settings
type ChatSettings = Arc<Mutex<HashMap<ChatId, bool>>>;

#[tokio::main]
async fn main() {
  pretty_env_logger::init();
  info!("Starting image search bot...");
  let bot = Bot::from_env();

  // Initialize chat settings (local search enabled by default)
  let chat_settings: ChatSettings = Arc::new(Mutex::new(HashMap::new()));

  teloxide::repl(bot, move |bot: Bot, msg: Message| {
    let chat_settings = Arc::clone(&chat_settings);
    async move {
      if let Err(e) = handle_message(&bot, &msg, &chat_settings).await {
        error!("Error handling message: {:?}", e);
      }
      Ok::<(), RequestError>(())
    }
  })
  .await;
}

async fn handle_message(
  bot: &Bot,
  msg: &Message,
  chat_settings: &ChatSettings,
) -> Result<(), anyhow::Error> {
  let text = match msg.text() {
    Some(text) => text,
    None => return Ok(()),
  };

  // Handle commands
  if text.starts_with('/') {
    return handle_command(bot, msg, chat_settings).await;
  }

  // Check if local search is enabled for this chat
  let local_search_enabled = {
    let settings = chat_settings.lock().await;
    *settings.get(&msg.chat.id).unwrap_or(&true) // Default to enabled
  };

  // Try to find a local image if local search is enabled
  if local_search_enabled {
    if let Some(local_image) = find_local_image(text).await? {
      info!("Found local image: {:?}", local_image);

      let file_extension = local_image
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");

      let is_gif = file_extension.to_lowercase() == "gif";

      let result = if is_gif {
        bot
          .send_animation(msg.chat.id, InputFile::file(local_image))
          .await
      } else {
        bot
          .send_photo(msg.chat.id, InputFile::file(local_image))
          .await
      };

      if let Err(e) = result {
        error!("Failed to send local image: {:?}", e);
      } else {
        return Ok(());
      }
    }
  }

  // If no local image is found, try online search
  let pattern = Regex::new(r"^(.+?)\.((?i)jpg|png|gif)$")?;
  let captures = match pattern.captures(text) {
    Some(c) => c,
    None => return Ok(()),
  };

  let query = captures.get(1).unwrap().as_str();
  let is_gif = captures.get(2).unwrap().as_str().to_lowercase() == "gif";

  let image_urls = google_image_search(query, is_gif).await?;

  for image_url in image_urls.iter() {
    let parsed_url = match Url::parse(image_url) {
      Ok(url) => url,
      Err(_) => {
        error!("Failed to parse URL: {}", image_url);
        continue;
      }
    };

    let result = if is_gif {
      bot
        .send_animation(msg.chat.id, InputFile::url(parsed_url))
        .await
    } else {
      bot
        .send_photo(msg.chat.id, InputFile::url(parsed_url))
        .await
    };

    match result {
      Ok(_) => break,
      Err(e) => {
        error!(
          "Failed to send {} {}: {:?}",
          if is_gif { "animation" } else { "photo" },
          image_url,
          e
        );
        continue;
      }
    }
  }

  Ok(())
}

// Handle bot commands
async fn handle_command(
  bot: &Bot,
  msg: &Message,
  chat_settings: &ChatSettings,
) -> Result<(), anyhow::Error> {
  let text = msg.text().unwrap();

  match text {
    "/start" => {
      bot
        .send_message(
          msg.chat.id,
          "Welcome! I can support images on google or from local collection.\n\
         See https://github.com/akira02/rust-tg.jpg for more information.\n\
         Use /enable_local to enable local image search\n\
         Use /disable_local to disable local image search\n\
         Use /status to check current settings",
        )
        .await?;
    }
    "/enable_local" => {
      {
        let mut settings = chat_settings.lock().await;
        settings.insert(msg.chat.id, true);
      }
      bot.send_message(
        msg.chat.id,
        "Local image search has been enabled! I will now search for images in my local collection."
      ).await?;
    }
    "/disable_local" => {
      {
        let mut settings = chat_settings.lock().await;
        settings.insert(msg.chat.id, false);
      }
      bot
        .send_message(
          msg.chat.id,
          "Local image search has been disabled! I will only search for images online.",
        )
        .await?;
    }
    "/status" => {
      let local_search_enabled = {
        let settings = chat_settings.lock().await;
        *settings.get(&msg.chat.id).unwrap_or(&true)
      };

      let status_message = if local_search_enabled {
        "Local image search is currently enabled."
      } else {
        "Local image search is currently disabled."
      };

      bot.send_message(msg.chat.id, status_message).await?;
    }
    _ => {
      // Unknown command, ignore
    }
  }

  Ok(())
}
