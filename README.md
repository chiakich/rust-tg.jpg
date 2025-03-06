# rust-tg.jpg

![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/akira02/rust-tg.jpg/fly-deploy.yml)

Sometimes words aren't enough to express your emotions, and searching for stickers or GIFs can be a hassle.  
Try tg.jpg! It's like Google's "I'm Feeling Lucky" but for images, this bot will instantly reply with the first image it finds on Google.  
"mic drop.gif"!

<img width="554" alt="image" src="https://github.com/user-attachments/assets/2bedf066-e1ee-4354-92d0-ca2e2c39e73e" />

## Features

- Listens for messages containing image file names (e.g., `example.jpg`, `example.png`, `example.gif`).
- Searches for images on Google and sends the first result back to the user.
- Supports both regular images and GIFs.
- [New] MyGo mode! Support using local /src/assets image. (Good for MYGO!! meme pics)

## Commands

- `/start` - Display welcome message and available commands
- `/enable_mygo` - Enable local image search
- `/disable_mygo` - Disable local image search
- `/status` - Check current settings

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

4.  Make sure that your Rust compiler is up to date (`teloxide` currently requires rustc at least version 1.80):

```bash
# If you're using stable
$ rustup update stable
$ rustup override set stable

# If you're using nightly
$ rustup update nightly
$ rustup override set nightly
```

5. Build and run the project:

   ```bash
   cargo run
   ```

## Usage

- Start the bot on Telegram by searching for your bot's username and sending a message with an image file name (e.g., `example.jpg`).
- The bot will respond with the first possible image result it finds.

## Dependencies

- [Teloxide](https://github.com/teloxide/teloxide) for Telegram bot API interaction.
- [Reqwest](https://github.com/seanmonstar/reqwest) for making HTTP requests.
- [Regex](https://github.com/rust-lang/regex) for regular expression matching.
- [Anyhow](https://github.com/dtolnay/anyhow) for error handling.

## Contributing

Contributions are welcome! Please open an issue or submit a pull request for any improvements or bug fixes.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
