use anyhow::Result;
use log::{error, info};
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::InputFile;
use tokio::sync::Mutex;
use url::Url;

// Import the local image finder module
mod local_image_finder;
use local_image_finder::find_local_image;

// Import the Google image search module
mod google_image_searcher;
use google_image_searcher::search as google_image_search;

// Import the Imgur handler module
mod imgur_handler;
use imgur_handler::{download_imgur_image, is_imgur_url};

// Import the inline query handler module
mod inline_query_handler;
use inline_query_handler::{handle_chosen_inline_result, handle_inline_query};

// Define a type for our chat settings
type ChatSettings = Arc<Mutex<HashMap<ChatId, bool>>>;

#[tokio::main]
async fn main() {
  pretty_env_logger::init();
  info!("Starting image search bot...");
  let bot = Bot::from_env();

  // Initialize chat settings (mygo mode disabled by default)
  let chat_settings: ChatSettings = Arc::new(Mutex::new(HashMap::new()));

  let handler = dptree::entry()
    .branch(Update::filter_message().endpoint(message_handler))
    .branch(Update::filter_inline_query().endpoint(handle_inline_query))
    .branch(Update::filter_chosen_inline_result().endpoint(handle_chosen_inline_result));

  Dispatcher::builder(bot, handler)
    .dependencies(dptree::deps![chat_settings])
    .enable_ctrlc_handler()
    .build()
    .dispatch()
    .await;
}

async fn message_handler(
  bot: Bot,
  msg: Message,
  chat_settings: ChatSettings,
) -> Result<(), anyhow::Error> {
  let text = match msg.text() {
    Some(text) => text,
    None => return Ok(()),
  };

  // Handle commands
  if text.starts_with('/') {
    return handle_command(&bot, &msg, &chat_settings).await;
  }

  // Check if mygo mode is enabled for this chat
  let mygo_enabled = {
    let settings = chat_settings.lock().await;
    *settings.get(&msg.chat.id).unwrap_or(&false) // Default to disabled
  };

  // Try to find a local image if mygo mode is enabled
  if mygo_enabled {
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
    let result = if is_imgur_url(image_url) {
      // Download imgur image and send as file
      match download_imgur_image(image_url).await {
        Ok(data) => {
          let input_file = InputFile::memory(data);
          if is_gif {
            bot.send_animation(msg.chat.id, input_file).await
          } else {
            bot.send_photo(msg.chat.id, input_file).await
          }
        }
        Err(e) => {
          error!("Failed to download imgur image {}: {:?}", image_url, e);
          continue;
        }
      }
    } else {
      // Use URL for non-imgur images
      let parsed_url = match Url::parse(image_url) {
        Ok(url) => url,
        Err(_) => {
          error!("Failed to parse URL: {}", image_url);
          continue;
        }
      };

      if is_gif {
        bot
          .send_animation(msg.chat.id, InputFile::url(parsed_url))
          .await
      } else {
        bot
          .send_photo(msg.chat.id, InputFile::url(parsed_url))
          .await
      }
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
         Use /enable_mygo to enable mygo mode\n\
         Use /disable_mygo to disable mygo mode\n\
         Use /status to check current settings\n\n\
         You can also use me in any chat by typing @botname followed by your search term!",
        )
        .await?;
    }
    "/enable_mygo" => {
      {
        let mut settings = chat_settings.lock().await;
        settings.insert(msg.chat.id, true);
      }
      bot
        .send_message(
          msg.chat.id,
          "Mygo mode has been enabled! I will now search for images in my local collection.",
        )
        .await?;
    }
    "/disable_mygo" => {
      {
        let mut settings = chat_settings.lock().await;
        settings.insert(msg.chat.id, false);
      }
      bot
        .send_message(
          msg.chat.id,
          "Mygo mode has been disabled! I will only search for images online.",
        )
        .await?;
    }
    "/status" => {
      let mygo_enabled = {
        let settings = chat_settings.lock().await;
        *settings.get(&msg.chat.id).unwrap_or(&false)
      };

      let status_message = if mygo_enabled {
        "Mygo mode is currently enabled."
      } else {
        "Mygo mode is currently disabled."
      };

      bot.send_message(msg.chat.id, status_message).await?;
    }
    _ => {
      // Unknown command, ignore
    }
  }

  Ok(())
}
