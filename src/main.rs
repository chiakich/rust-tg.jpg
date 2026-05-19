use anyhow::Result;
use log::{error, info};
use regex::Regex;
use teloxide::prelude::*;
use teloxide::types::InputFile;
use tgjpg_rs::image_search::{initialize as initialize_image_search, search as image_search};
use tgjpg_rs::imgur_handler::{download_imgur_image, is_imgur_url};
use tgjpg_rs::inline_query_handler::{handle_chosen_inline_result, handle_inline_query};
use url::Url;

#[tokio::main]
async fn main() {
  pretty_env_logger::init();
  info!("Starting image search bot...");
  initialize_image_search().await;
  let bot = Bot::from_env();

  let handler = dptree::entry()
    .branch(Update::filter_message().endpoint(message_handler))
    .branch(Update::filter_inline_query().endpoint(handle_inline_query))
    .branch(Update::filter_chosen_inline_result().endpoint(handle_chosen_inline_result));

  Dispatcher::builder(bot, handler)
    .enable_ctrlc_handler()
    .build()
    .dispatch()
    .await;
}

async fn message_handler(bot: Bot, msg: Message) -> Result<(), anyhow::Error> {
  let text = match msg.text() {
    Some(text) => text,
    None => return Ok(()),
  };

  // Handle commands
  if text.starts_with('/') {
    return handle_command(&bot, &msg).await;
  }

  // Exclude URLs and only match patterns like "cat.jpg" or "dog.gif"
  if text.contains("://") {
    return Ok(());
  }

  let pattern = Regex::new(r"(?i)\.(jpg|png|gif)$")?;
  let captures = match pattern.captures(text) {
    Some(c) => c,
    None => return Ok(()),
  };

  let query = captures.get(1).unwrap().as_str();
  let is_gif = captures.get(2).unwrap().as_str().to_lowercase() == "gif";

  let image_urls = image_search(query, is_gif).await?;

  for image_url in image_urls.iter() {
    let result = if is_imgur_url(image_url) {
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

async fn handle_command(bot: &Bot, msg: &Message) -> Result<(), anyhow::Error> {
  let text = msg.text().unwrap();

  match text {
    "/start" => {
      bot
        .send_message(
          msg.chat.id,
          "Welcome! Send me a message like \"cat.jpg\" or \"dog.gif\" to search for images.\n\
           You can also use me in any chat by typing @botname followed by your search term!\n\
           See https://github.com/akira02/rust-tg.jpg for more information.",
        )
        .await?;
    }
    _ => {}
  }

  Ok(())
}
