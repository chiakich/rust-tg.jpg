# rust-tg.jpg

![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/akira02/rust-tg.jpg/fly-deploy.yml)

Sometimes words aren't enough to express your emotions, and searching for stickers or GIFs can be a hassle.  
Try tg.jpg! It's like an "I'm Feeling Lucky" bot for images: send a query like `mic drop.gif`, and it will search multiple image engines and reply with the first usable result it can send.  
"mic drop.gif"!

<img width="554" alt="image" src="https://github.com/user-attachments/assets/2bedf066-e1ee-4354-92d0-ca2e2c39e73e" />

## Features

- Listens for messages containing image file names (e.g., `example.jpg`, `example.png`, `example.gif`).
- Searches across multiple image engines and sends back the first usable result.
- Supports Google, DuckDuckGo, and Bing scraping backends.
- Optionally supports [Serper.dev](https://serper.dev/) when `SERPER_API` is configured. (Recommend)
- Optionally supports [SerpAPI](https://serpapi.com/google-images-api) when `SERP_API` is configured.
- Runs a startup health check and only enables search engines that pass.
- Supports both regular images and GIFs.

## Commands

- `/start` - Display welcome message and available commands

## Prerequisites

- Rust and Cargo installed on your system. You can install them from [rustup.rs](https://rustup.rs/).
- A Telegram bot token. You can create a bot and get a token by talking to [BotFather](https://t.me/botfather) on Telegram.

## Setup

1.  [Download Rust](http://rustup.rs/).
2.  Create a new bot using [@Botfather](https://t.me/botfather) to get a token in the format `123456789:blablabla`.
3.  Initialise the `TELOXIDE_TOKEN` environmental variable to your token:

```bash
# Unix-like
$ export TELOXIDE_TOKEN=<Your token here>

# Windows command line
$ set TELOXIDE_TOKEN=<Your token here>

# Windows PowerShell
$ $env:TELOXIDE_TOKEN=<Your token here>
```

4.  Optional: if you want to enable the Serper image backend, set `SERPER_API`:

```bash
# Unix-like
$ export SERPER_API=<Your Serper key here>

# Windows command line
$ set SERPER_API=<Your Serper key here>

# Windows PowerShell
$ $env:SERPER_API=<Your Serper key here>
```

5.  Optional: if you want to enable the SerpAPI image backend, set `SERP_API`:

```bash
# Unix-like
$ export SERP_API=<Your SerpAPI key here>

# Windows command line
$ set SERP_API=<Your SerpAPI key here>

# Windows PowerShell
$ $env:SERP_API=<Your SerpAPI key here>
```

6.  Make sure that your Rust compiler is up to date (`teloxide` currently requires rustc at least version 1.80):

```bash
# If you're using stable
$ rustup update stable
$ rustup override set stable

# If you're using nightly
$ rustup update nightly
$ rustup override set nightly
```

7. Build and run the project:

   ```bash
   cargo run
   ```

## Usage

- Start the bot on Telegram by searching for your bot's username and sending a message with an image file name (e.g., `example.jpg`).
- On startup, the bot health-checks each configured search engine and only enables the ones that currently work.
- When `SERPER_API` is set, Serper is included as the highest-priority search backend.
- When `SERP_API` is set, SerpAPI is included in the search order.
- The bot will respond with the first possible image result it finds from the enabled backends.

## Dependencies

- [Teloxide](https://github.com/teloxide/teloxide) for Telegram bot API interaction.
- [Reqwest](https://github.com/seanmonstar/reqwest) for making HTTP requests.
- [Regex](https://github.com/rust-lang/regex) for regular expression matching.
- [Anyhow](https://github.com/dtolnay/anyhow) for error handling.

## Contributing

Contributions are welcome! Please open an issue or submit a pull request for any improvements or bug fixes.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
